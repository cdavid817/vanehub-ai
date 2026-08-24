use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const RELAY_CACHE_VERSION: &str = "v1";
const STALE_RELAY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug)]
pub(crate) struct PrivateRelayDirectory {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedMcpRelayGuard {
    inner: Arc<RelayGuardInner>,
}

#[derive(Debug)]
struct RelayGuardInner {
    version_root: PathBuf,
    invocation_directory: PathBuf,
    cleaned: AtomicBool,
}

impl PrivateRelayDirectory {
    pub(crate) fn create() -> io::Result<Self> {
        Self::create_in(&default_relay_root()?)
    }

    pub(crate) fn create_in(relay_root: &Path) -> io::Result<Self> {
        let version_root = relay_root.join(RELAY_CACHE_VERSION);
        fs::create_dir_all(&version_root)?;
        restrict_to_current_user(&version_root)?;
        let path = version_root.join(format!(
            "invocation-{}-{}",
            std::process::id(),
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&path)?;
        if let Err(error) = restrict_to_current_user(&path) {
            let _ = fs::remove_dir(&path);
            return Err(error);
        }
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn create_file(&self, name: &str) -> io::Result<File> {
        validate_file_name(name)?;
        let path = self.path.join(name);
        let file = open_private_file(&path)?;
        if let Err(error) = restrict_to_current_user(&path) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(file)
    }

    pub(crate) fn guard(&self) -> io::Result<PreparedMcpRelayGuard> {
        PreparedMcpRelayGuard::new(&self.path)
    }

    pub(crate) fn scavenge_stale() -> io::Result<()> {
        let cutoff = SystemTime::now()
            .checked_sub(STALE_RELAY_AGE)
            .ok_or_else(|| io::Error::other("relay cleanup cutoff underflow"))?;
        scavenge_stale_in(&default_relay_root()?, cutoff)
    }
}

fn default_relay_root() -> io::Result<PathBuf> {
    dirs::cache_dir()
        .map(|cache| cache.join("vanehub-ai").join("mcp-relay"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "application cache directory unavailable",
            )
        })
}

fn scavenge_stale_in(relay_root: &Path, cutoff: SystemTime) -> io::Result<()> {
    let version_root = relay_root.join(RELAY_CACHE_VERSION);
    let root_metadata = match fs::symlink_metadata(&version_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "relay version root is not a real directory",
        ));
    }
    let canonical_root = fs::canonicalize(&version_root)?;
    let mut first_error = None;
    for entry in fs::read_dir(&version_root)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("invocation-")
        {
            continue;
        }
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        if modified > cutoff {
            continue;
        }
        let candidate = match fs::canonicalize(entry.path()) {
            Ok(candidate) => candidate,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        if candidate.parent() != Some(canonical_root.as_path()) {
            continue;
        }
        if let Err(error) = fs::remove_dir_all(candidate) {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

impl PreparedMcpRelayGuard {
    fn new(invocation_directory: &Path) -> io::Result<Self> {
        let version_root = invocation_directory.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "relay directory has no version root",
            )
        })?;
        let version_root = fs::canonicalize(version_root)?;
        let invocation_directory = fs::canonicalize(invocation_directory)?;
        if invocation_directory.parent() != Some(version_root.as_path()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "relay directory escaped its version root",
            ));
        }
        Ok(Self {
            inner: Arc::new(RelayGuardInner {
                version_root,
                invocation_directory,
                cleaned: AtomicBool::new(false),
            }),
        })
    }

    pub(crate) fn cleanup(&self) -> io::Result<()> {
        self.inner.cleanup()
    }
}

impl RelayGuardInner {
    fn cleanup(&self) -> io::Result<()> {
        if self.cleaned.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let invocation_directory = match fs::canonicalize(&self.invocation_directory) {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let version_root = fs::canonicalize(&self.version_root)?;
        if invocation_directory.parent() != Some(version_root.as_path()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "relay cleanup target escaped its version root",
            ));
        }
        fs::remove_dir_all(invocation_directory)
    }
}

impl Drop for RelayGuardInner {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn validate_file_name(name: &str) -> io::Result<()> {
    let mut components = Path::new(name).components();
    if name.is_empty()
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "relay file name must be one path component",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

/// Creates a new private file for feature adapters that need exclusive creation semantics.
/// Keeping handle construction here preserves the platform boundary enforced by architecture tests.
pub(crate) fn create_new_private_file(path: &Path) -> io::Result<File> {
    open_private_file(path)
}

#[cfg(unix)]
pub(crate) fn open_private_file_for_append(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new().append(true).mode(0o600).open(path)
}

#[cfg(windows)]
pub(crate) fn open_private_file_for_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().append(true).open(path)
}

#[cfg(windows)]
fn open_private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Opens `path` for writing, creating it if absent and truncating it if present, at mode `0o600`
/// on Unix from the moment it is created. This differs from `open_private_file`'s `create_new`
/// semantics on purpose: callers of this function name their file with a pid plus a
/// process-global counter, and a prior crash can leave a stale file under that exact name behind
/// once a pid gets recycled. Under `create_new` that collision becomes a permanent failure the
/// user has to fix by hand-deleting a hidden file; overwriting it here is safe instead, because
/// the current process already owns that name by construction and there is nothing in a stale
/// file worth preserving.
///
/// This module is named and organized around the MCP relay's own temp-file needs, but this
/// function is also a hard dependency of
/// `contexts::agent_runtime::infrastructure::tools::edit_tool::create_temp_file` --
/// `tests/architecture.rs` confines raw `OpenOptions` construction to the modules under
/// `platform/`, so `edit_tool.rs` cannot open its own file handle and calls this instead. A future
/// relay-focused refactor that removes, renames, or narrows this function without grepping for
/// that caller would silently break `edit`.
#[cfg(unix)]
pub(crate) fn create_or_truncate_private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(windows)]
pub(crate) fn create_or_truncate_private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

#[cfg(unix)]
fn restrict_to_current_user(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if path.is_dir() { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(windows)]
fn restrict_to_current_user(path: &Path) -> io::Result<()> {
    windows_acl::restrict_to_current_user(path)
}

#[cfg(windows)]
#[path = "private_relay_fs_windows.rs"]
mod windows_acl;

/// Read-only structural reading of a Windows DACL, used by the privacy contract test. Kept out
/// of `windows_acl` because that module applies access control and this one only observes it.
#[cfg(all(windows, test))]
#[path = "private_relay_fs_windows_acl_report.rs"]
mod windows_acl_report;

#[cfg(test)]
#[path = "private_relay_fs_tests.rs"]
mod tests;
