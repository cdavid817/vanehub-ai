//! Complete worktree manifests for Loop scope evidence.
//!
//! A manifest covers every entry under the run root: tracked, untracked, ignored and hidden files,
//! directories, links and other node types, with content digests, sizes and modes. Git's own
//! bounded diff is a display aid and never an authorization input; this scan is what the runtime
//! compares. A scan that cannot inspect everything, exceeds its budget, is cancelled, or observes
//! the tree changing underneath it reports a failure rather than a partial result.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScanBudget {
    pub(crate) max_entries: usize,
    pub(crate) max_bytes: u64,
}

impl Default for ScanBudget {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_bytes: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanFailure {
    Unreadable(String),
    BudgetExceeded,
    Cancelled,
    Unstable,
    LinkTraversal(String),
    /// A name the manifest cannot represent losslessly; two different names must never share
    /// one manifest key, so the scan reports instead of guessing.
    NonUtf8Name(String),
}

impl ScanFailure {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Unreadable(_) => "scan-unreadable",
            Self::BudgetExceeded => "scan-budget-exceeded",
            Self::Cancelled => "scan-cancelled",
            Self::Unstable => "scan-unstable",
            Self::LinkTraversal(_) => "scan-link-traversal",
            Self::NonUtf8Name(_) => "scan-non-utf8-name",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Self::Unreadable(path) => format!("could not inspect {path}"),
            Self::BudgetExceeded => "the workspace exceeds the evidence scan budget".to_string(),
            Self::Cancelled => "the evidence scan was cancelled".to_string(),
            Self::Unstable => "the workspace changed while evidence was being captured".to_string(),
            Self::LinkTraversal(path) => format!("an entry changed into a link at {path}"),
            Self::NonUtf8Name(path) => {
                format!("an entry under {path} has a name that is not valid UTF-8")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManifestEntry {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) size: u64,
    pub(crate) mode: u32,
    /// sha256 of file contents, or of the link target for symlinks. Absent for directories.
    pub(crate) digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkspaceManifest {
    pub(crate) version: u32,
    pub(crate) entries: Vec<ManifestEntry>,
    pub(crate) digest: String,
    pub(crate) total_bytes: u64,
}

impl WorkspaceManifest {
    pub(crate) fn from_entries(mut entries: Vec<ManifestEntry>) -> Self {
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let total_bytes = entries
            .iter()
            .filter(|entry| entry.kind == "file")
            .map(|entry| entry.size)
            .sum();
        let digest = manifest_digest(&entries);
        Self {
            version: 1,
            entries,
            digest,
            total_bytes,
        }
    }

    /// Recomputes the digest from the entries so a tampered or truncated stored manifest is
    /// detected instead of trusted.
    pub(crate) fn verify_integrity(&self) -> bool {
        let mut sorted = self.entries.clone();
        sorted.sort_by(|left, right| left.path.cmp(&right.path));
        sorted == self.entries && manifest_digest(&self.entries) == self.digest
    }

    pub(crate) fn find(&self, path: &str) -> Option<&ManifestEntry> {
        self.entries
            .binary_search_by(|entry| entry.path.as_str().cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }
}

fn manifest_digest(entries: &[ManifestEntry]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"workspace-manifest-v1\0");
    for entry in entries {
        hasher.update(entry.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(entry.kind.as_bytes());
        hasher.update(b"\0");
        hasher.update(entry.size.to_le_bytes());
        hasher.update(entry.mode.to_le_bytes());
        hasher.update(entry.digest.as_deref().unwrap_or("").as_bytes());
        hasher.update(b"\n");
    }
    hex(&hasher.finalize())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ChangeKind {
    Added,
    Modified,
    Deleted,
    TypeChanged,
    ModeChanged,
}

impl ChangeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::TypeChanged => "type-changed",
            Self::ModeChanged => "mode-changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestChange {
    pub(crate) path: String,
    pub(crate) kind: ChangeKind,
}

pub(crate) fn diff_manifests(
    baseline: &WorkspaceManifest,
    current: &WorkspaceManifest,
) -> Vec<ManifestChange> {
    let before: BTreeMap<&str, &ManifestEntry> = baseline
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let after: BTreeMap<&str, &ManifestEntry> = current
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut changes = Vec::new();
    for (path, entry) in &after {
        match before.get(path) {
            None => changes.push(ManifestChange {
                path: (*path).to_string(),
                kind: ChangeKind::Added,
            }),
            Some(previous) => {
                let kind = if previous.kind != entry.kind {
                    Some(ChangeKind::TypeChanged)
                } else if previous.digest != entry.digest || previous.size != entry.size {
                    Some(ChangeKind::Modified)
                } else if previous.mode != entry.mode {
                    Some(ChangeKind::ModeChanged)
                } else {
                    None
                };
                if let Some(kind) = kind {
                    changes.push(ManifestChange {
                        path: (*path).to_string(),
                        kind,
                    });
                }
            }
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            changes.push(ManifestChange {
                path: (*path).to_string(),
                kind: ChangeKind::Deleted,
            });
        }
    }
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    changes
}

/// One complete pass over the root. Links are recorded, never followed.
pub(crate) fn scan_workspace(
    root: &Path,
    budget: &ScanBudget,
    cancel: &AtomicBool,
) -> Result<WorkspaceManifest, ScanFailure> {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut pending: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((directory, prefix)) = pending.pop() {
        if cancel.load(Ordering::SeqCst) {
            return Err(ScanFailure::Cancelled);
        }
        let listing = fs::read_dir(&directory)
            .map_err(|_| ScanFailure::Unreadable(display_path(&prefix, "")))?;
        for item in listing {
            let item = item.map_err(|_| ScanFailure::Unreadable(display_path(&prefix, "")))?;
            let Some(name) = item.file_name().to_str().map(str::to_string) else {
                return Err(ScanFailure::NonUtf8Name(display_path(&prefix, "")));
            };
            let relative = display_path(&prefix, &name);
            let metadata = fs::symlink_metadata(item.path())
                .map_err(|_| ScanFailure::Unreadable(relative.clone()))?;
            let file_type = metadata.file_type();
            entries.push(if file_type.is_symlink() {
                let target = fs::read_link(item.path())
                    .map_err(|_| ScanFailure::Unreadable(relative.clone()))?;
                ManifestEntry {
                    path: relative,
                    kind: "symlink".to_string(),
                    size: 0,
                    mode: mode_of(&metadata),
                    digest: Some(hex(&Sha256::digest(link_target_bytes(&target)?))),
                }
            } else if file_type.is_dir() {
                pending.push((item.path(), relative.clone()));
                ManifestEntry {
                    path: relative,
                    kind: "dir".to_string(),
                    size: 0,
                    mode: mode_of(&metadata),
                    digest: None,
                }
            } else if file_type.is_file() {
                let (digest, size) = hash_regular_file(&item.path(), &relative)?;
                total_bytes = total_bytes.saturating_add(size);
                if total_bytes > budget.max_bytes {
                    return Err(ScanFailure::BudgetExceeded);
                }
                ManifestEntry {
                    path: relative,
                    kind: "file".to_string(),
                    size,
                    mode: mode_of(&metadata),
                    digest: Some(digest),
                }
            } else {
                ManifestEntry {
                    path: relative,
                    kind: "other".to_string(),
                    size: 0,
                    mode: mode_of(&metadata),
                    digest: None,
                }
            });
            if entries.len() > budget.max_entries {
                return Err(ScanFailure::BudgetExceeded);
            }
        }
    }
    Ok(WorkspaceManifest::from_entries(entries))
}

/// Two identical consecutive passes are the only accepted proof of a stable tree. A writer that
/// is still active makes them differ, and the caller then reports unverifiable evidence.
pub(crate) fn scan_stable(
    root: &Path,
    budget: &ScanBudget,
    cancel: &AtomicBool,
) -> Result<WorkspaceManifest, ScanFailure> {
    let first = scan_workspace(root, budget, cancel)?;
    let second = scan_workspace(root, budget, cancel)?;
    if first.digest == second.digest {
        return Ok(second);
    }
    let third = scan_workspace(root, budget, cancel)?;
    if second.digest == third.digest {
        Ok(third)
    } else {
        Err(ScanFailure::Unstable)
    }
}

pub(crate) fn hash_regular_file(path: &Path, relative: &str) -> Result<(String, u64), ScanFailure> {
    let mut file = open_no_follow(path).map_err(|error| {
        if error.raw_os_error() == Some(loop_error_code()) {
            ScanFailure::LinkTraversal(relative.to_string())
        } else {
            ScanFailure::Unreadable(relative.to_string())
        }
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ScanFailure::Unreadable(relative.to_string()))?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((hex(&hasher.finalize()), size))
}

#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

#[cfg(not(unix))]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    // Without a no-follow open the read re-checks the entry type; a swap between the two is
    // reported as unstable by the double scan rather than silently trusted.
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "link traversal",
        ));
    }
    fs::File::open(path)
}

#[cfg(unix)]
fn loop_error_code() -> i32 {
    libc::ELOOP
}

#[cfg(not(unix))]
fn loop_error_code() -> i32 {
    -1
}

#[cfg(unix)]
fn mode_of(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o7777
}

#[cfg(not(unix))]
fn mode_of(metadata: &fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
}

/// The raw bytes of a link target: two targets that differ only in bytes a lossy string
/// would collapse must still produce different digests.
fn link_target_bytes(target: &Path) -> Result<Vec<u8>, ScanFailure> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(target.as_os_str().as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        target
            .to_str()
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| ScanFailure::NonUtf8Name(target.to_string_lossy().to_string()))
    }
}

fn display_path(prefix: &str, name: &str) -> String {
    match (prefix.is_empty(), name.is_empty()) {
        (true, true) => ".".to_string(),
        (true, false) => name.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}/{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn manifest_covers_hidden_ignored_links_and_metadata_and_diff_names_every_kind() {
        let workspace = TempDirectory::new("artifact-scan");
        workspace.write("src/app.ts", "a");
        workspace.write(".hidden/secret", "s");
        workspace.write("node_modules/dep/index.js", "d");
        workspace.write(".gitignore", "node_modules\n");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/hosts", workspace.path().join("link")).expect("link");
        let cancel = AtomicBool::new(false);
        let baseline =
            scan_stable(workspace.path(), &ScanBudget::default(), &cancel).expect("baseline");
        assert!(baseline.verify_integrity());
        assert!(baseline.find("src/app.ts").is_some());
        assert!(baseline.find(".hidden/secret").is_some());
        assert!(baseline.find("node_modules/dep/index.js").is_some());
        #[cfg(unix)]
        assert_eq!(
            baseline.find("link").map(|entry| entry.kind.as_str()),
            Some("symlink")
        );

        workspace.write("src/app.ts", "changed");
        workspace.write("src/new.ts", "n");
        fs::remove_file(workspace.path().join(".hidden/secret")).expect("delete");
        fs::remove_dir_all(workspace.path().join("node_modules/dep")).expect("dir delete");
        fs::create_dir_all(workspace.path().join("node_modules/dep")).expect("recreate");
        workspace.write("node_modules/dep/index.js", "d2");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                workspace.path().join(".gitignore"),
                fs::Permissions::from_mode(0o755),
            )
            .expect("chmod");
        }
        let current =
            scan_stable(workspace.path(), &ScanBudget::default(), &cancel).expect("current");
        let changes = diff_manifests(&baseline, &current);
        let kinds: BTreeMap<&str, ChangeKind> = changes
            .iter()
            .map(|change| (change.path.as_str(), change.kind))
            .collect();
        assert_eq!(kinds.get("src/app.ts"), Some(&ChangeKind::Modified));
        assert_eq!(kinds.get("src/new.ts"), Some(&ChangeKind::Added));
        assert_eq!(kinds.get(".hidden/secret"), Some(&ChangeKind::Deleted));
        assert_eq!(
            kinds.get("node_modules/dep/index.js"),
            Some(&ChangeKind::Modified)
        );
        #[cfg(unix)]
        assert_eq!(kinds.get(".gitignore"), Some(&ChangeKind::ModeChanged));
        assert_ne!(baseline.digest, current.digest);
    }

    #[test]
    fn budget_cancellation_and_corruption_are_reported_not_passed() {
        let workspace = TempDirectory::new("artifact-scan-budget");
        workspace.write("a.txt", "aaaa");
        workspace.write("b.txt", "bbbb");
        let cancel = AtomicBool::new(false);
        let tiny = ScanBudget {
            max_entries: 1,
            max_bytes: 1024,
        };
        assert_eq!(
            scan_workspace(workspace.path(), &tiny, &cancel).expect_err("entries"),
            ScanFailure::BudgetExceeded
        );
        let tiny_bytes = ScanBudget {
            max_entries: 100,
            max_bytes: 5,
        };
        assert_eq!(
            scan_workspace(workspace.path(), &tiny_bytes, &cancel).expect_err("bytes"),
            ScanFailure::BudgetExceeded
        );
        cancel.store(true, Ordering::SeqCst);
        assert_eq!(
            scan_workspace(workspace.path(), &ScanBudget::default(), &cancel).expect_err("cancel"),
            ScanFailure::Cancelled
        );
        cancel.store(false, Ordering::SeqCst);
        let mut manifest =
            scan_workspace(workspace.path(), &ScanBudget::default(), &cancel).expect("scan");
        manifest.entries[0].size += 1;
        assert!(!manifest.verify_integrity());
    }

    #[cfg(unix)]
    #[test]
    fn link_targets_are_compared_by_raw_bytes_and_non_utf8_names_are_reported() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let workspace = TempDirectory::new("artifact-scan-bytes");
        workspace.write("src/app.ts", "a");
        let cancel = AtomicBool::new(false);
        let link = workspace.path().join("link");
        std::os::unix::fs::symlink(OsStr::from_bytes(b"../target-\xff"), &link).expect("link");
        let first = scan_stable(workspace.path(), &ScanBudget::default(), &cancel).expect("first");
        std::fs::remove_file(&link).expect("unlink");
        std::os::unix::fs::symlink(OsStr::from_bytes(b"../target-\xfe"), &link).expect("relink");
        let second =
            scan_stable(workspace.path(), &ScanBudget::default(), &cancel).expect("second");
        assert_ne!(
            first.find("link").and_then(|entry| entry.digest.clone()),
            second.find("link").and_then(|entry| entry.digest.clone()),
            "two targets a lossy string would merge keep distinct digests"
        );
        assert_ne!(first.digest, second.digest);

        std::fs::write(
            workspace
                .path()
                .join(OsStr::from_bytes(b"src/bad-\xff.txt")),
            "x",
        )
        .expect("bad name");
        assert!(matches!(
            scan_stable(workspace.path(), &ScanBudget::default(), &cancel),
            Err(ScanFailure::NonUtf8Name(_))
        ));
    }
}
