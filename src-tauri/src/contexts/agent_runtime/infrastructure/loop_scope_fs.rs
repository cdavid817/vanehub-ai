//! The race-resistant execution boundary for mediated Loop mutations.
//!
//! Every check and every effect goes through directory handles opened component by component with
//! `O_NOFOLLOW`, so the resource that was admitted is the resource that changes. Canonicalizing a
//! string and reopening it later is exactly the pattern this module exists to replace: a parent
//! swapped for a link between the check and the write would redirect such a write, whereas a
//! handle keeps pointing at the directory that was authorized.
//!
//! Only Unix has a proven implementation. Other platforms report the capability as unavailable
//! and the runtime refuses to claim enforcement it cannot deliver.

use crate::contexts::agent_runtime::domain::{
    CaseRule, LoopScopeConfig, LoopScopePath, ScopeRejection,
};
use std::fmt;
use std::path::{Path, PathBuf};

pub(crate) const MAX_MEDIATED_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopedFsError {
    /// Constructed only by the non-unix fallback; unix hosts never report it.
    #[allow(dead_code)]
    Unsupported(&'static str),
    Rejected(ScopeRejection),
    InvalidPath(String),
    LinkTraversal(String),
    NotFound(String),
    NotRegularFile(String),
    IsDirectory(String),
    TooLarge(String),
    RootChanged,
    Io(String),
}

impl ScopedFsError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Unsupported(_) => "scope-platform-unsupported",
            Self::Rejected(rejection) => rejection.code,
            Self::InvalidPath(_) => "scope-invalid-path",
            Self::LinkTraversal(_) => "scope-link-traversal",
            Self::NotFound(_) => "scope-not-found",
            Self::NotRegularFile(_) => "scope-not-regular-file",
            Self::IsDirectory(_) => "scope-is-directory",
            Self::TooLarge(_) => "scope-file-too-large",
            Self::RootChanged => "scope-root-changed",
            Self::Io(_) => "scope-io-error",
        }
    }
}

impl fmt::Display for ScopedFsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(detail) => write!(f, "scope enforcement unavailable: {detail}"),
            Self::Rejected(rejection) => write!(f, "scope rejected: {}", rejection.message()),
            Self::InvalidPath(detail) => write!(f, "invalid path: {detail}"),
            Self::LinkTraversal(path) => write!(f, "refusing to traverse a link at {path}"),
            Self::NotFound(path) => write!(f, "{path} does not exist"),
            Self::NotRegularFile(path) => write!(f, "{path} is not a regular file"),
            Self::IsDirectory(path) => write!(f, "{path} is a directory"),
            Self::TooLarge(path) => write!(f, "{path} exceeds the mediated file size limit"),
            Self::RootChanged => write!(f, "the worktree root no longer has its bound identity"),
            Self::Io(detail) => write!(f, "filesystem operation failed: {detail}"),
        }
    }
}

/// The device/inode pair that names a resource independently of any path string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ResourceIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

/// What this build can prove on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopPlatformCapability {
    pub(crate) platform: &'static str,
    pub(crate) safe_mutation: bool,
    pub(crate) detail: &'static str,
}

pub(crate) fn platform_capability() -> LoopPlatformCapability {
    LoopPlatformCapability {
        platform: std::env::consts::OS,
        safe_mutation: cfg!(unix),
        detail: if cfg!(unix) {
            "handle-relative no-follow delivery"
        } else {
            "no verified handle-relative delivery on this platform"
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteReceipt {
    pub(crate) created: bool,
    /// The previous inode had other names; the replacement left that alias untouched.
    pub(crate) replaced_shared_inode: bool,
}

/// Splits a requested path into worktree-relative components. Absolute requests must start with
/// the root's own path; the string is only used to derive components, never as authority.
pub(crate) fn relative_scope_path(
    root: &Path,
    requested: &str,
) -> Result<LoopScopePath, ScopedFsError> {
    let requested = requested.trim();
    if requested.is_empty() || requested.chars().any(char::is_control) {
        return Err(ScopedFsError::InvalidPath(
            "empty or control characters".to_string(),
        ));
    }
    let relative = if Path::new(requested).is_absolute() {
        let root_text = root.to_string_lossy();
        let stripped = requested
            .strip_prefix(root_text.as_ref())
            .ok_or_else(|| ScopedFsError::InvalidPath("outside the bound root".to_string()))?;
        if !stripped.is_empty() && !stripped.starts_with(['/', '\\']) {
            return Err(ScopedFsError::InvalidPath(
                "outside the bound root".to_string(),
            ));
        }
        let trimmed = stripped.trim_start_matches(['/', '\\']);
        if trimmed.is_empty() {
            "."
        } else {
            trimmed
        }
    } else {
        requested
    };
    LoopScopePath::parse(relative).map_err(|error| ScopedFsError::InvalidPath(error.to_string()))
}

#[cfg(unix)]
pub(crate) use unix::ScopedWorkspaceRoot;

#[cfg(not(unix))]
pub(crate) use portable::ScopedWorkspaceRoot;

#[cfg(not(unix))]
mod portable {
    use super::*;

    /// No verified handle-relative primitives on this platform: every operation reports the
    /// capability as unavailable instead of degrading to lexical checks.
    #[derive(Debug)]
    pub(crate) struct ScopedWorkspaceRoot {
        root: PathBuf,
    }

    impl ScopedWorkspaceRoot {
        pub(crate) fn open(_root: &Path) -> Result<Self, ScopedFsError> {
            Err(ScopedFsError::Unsupported(
                "handle-relative delivery is only verified on Unix",
            ))
        }
        pub(crate) fn root(&self) -> &Path {
            &self.root
        }
        pub(crate) fn identity(&self) -> ResourceIdentity {
            ResourceIdentity {
                device: 0,
                inode: 0,
            }
        }
        pub(crate) fn case_rule(&self) -> CaseRule {
            CaseRule::Sensitive
        }
        pub(crate) fn verify_identity(&self) -> Result<(), ScopedFsError> {
            Err(ScopedFsError::Unsupported("unsupported platform"))
        }
        pub(crate) fn read_file(&self, _: &LoopScopePath) -> Result<Vec<u8>, ScopedFsError> {
            Err(ScopedFsError::Unsupported("unsupported platform"))
        }
        pub(crate) fn write_file(
            &self,
            _: &LoopScopeConfig,
            _: &LoopScopePath,
            _: &[u8],
        ) -> Result<WriteReceipt, ScopedFsError> {
            Err(ScopedFsError::Unsupported("unsupported platform"))
        }
        pub(crate) fn rename(
            &self,
            _: &LoopScopeConfig,
            _: &LoopScopePath,
            _: &LoopScopePath,
        ) -> Result<(), ScopedFsError> {
            Err(ScopedFsError::Unsupported("unsupported platform"))
        }
        pub(crate) fn remove(
            &self,
            _: &LoopScopeConfig,
            _: &LoopScopePath,
        ) -> Result<(), ScopedFsError> {
            Err(ScopedFsError::Unsupported("unsupported platform"))
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug)]
    pub(crate) struct ScopedWorkspaceRoot {
        root: PathBuf,
        fd: OwnedFd,
        identity: ResourceIdentity,
        case: CaseRule,
    }

    struct DirHandle {
        fd: OwnedFd,
    }

    #[derive(Clone, Copy)]
    enum EntryKind {
        Regular,
        Directory,
        Symlink,
        Other,
    }

    struct EntryStat {
        kind: EntryKind,
        mode: u32,
        nlink: u64,
        size: u64,
        identity: ResourceIdentity,
    }

    impl ScopedWorkspaceRoot {
        pub(crate) fn open(root: &Path) -> Result<Self, ScopedFsError> {
            let root = std::fs::canonicalize(root).map_err(io_error)?;
            let fd = open_directory_at(libc::AT_FDCWD, &c_path(&root)?, &root.to_string_lossy())?;
            let stat = fstat(fd.as_raw_fd())?;
            let mut this = Self {
                root,
                fd,
                identity: stat.identity,
                case: CaseRule::Sensitive,
            };
            this.case = this.probe_case_rule()?;
            Ok(this)
        }

        pub(crate) fn root(&self) -> &Path {
            &self.root
        }

        pub(crate) fn identity(&self) -> ResourceIdentity {
            self.identity
        }

        pub(crate) fn case_rule(&self) -> CaseRule {
            self.case
        }

        /// The bound handle must still be what the path names. A replaced root (a rename, a
        /// mount change) invalidates every path-derived assumption a caller might make.
        pub(crate) fn verify_identity(&self) -> Result<(), ScopedFsError> {
            let live = stat_path(&self.root)?;
            let handle = fstat(self.fd.as_raw_fd())?;
            if live.identity == self.identity && handle.identity == self.identity {
                Ok(())
            } else {
                Err(ScopedFsError::RootChanged)
            }
        }

        /// Creates and removes a probe entry to learn whether this volume folds ASCII case.
        /// A host administration step that runs before the baseline is captured.
        fn probe_case_rule(&self) -> Result<CaseRule, ScopedFsError> {
            let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let name = format!(".vanehub-case-probe-{}-{nonce}", std::process::id());
            let upper = name.to_ascii_uppercase();
            let c_name = c_str(&name)?;
            let c_upper = c_str(&upper)?;
            // SAFETY: valid directory fd and NUL-terminated names.
            let fd = unsafe {
                libc::openat(
                    self.fd.as_raw_fd(),
                    c_name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(io_error(std::io::Error::last_os_error()));
            }
            // SAFETY: fd was just returned by openat and is owned here.
            let created = unsafe { OwnedFd::from_raw_fd(fd) };
            let created_identity = fstat(created.as_raw_fd())?.identity;
            let folded = match fstatat(self.fd.as_raw_fd(), &c_upper) {
                Ok(stat) => stat.identity == created_identity,
                Err(ScopedFsError::NotFound(_)) => false,
                Err(error) => {
                    let _ = unlinkat(self.fd.as_raw_fd(), &c_name, false);
                    return Err(error);
                }
            };
            drop(created);
            unlinkat(self.fd.as_raw_fd(), &c_name, false)?;
            Ok(if folded {
                CaseRule::InsensitiveAscii
            } else {
                CaseRule::Sensitive
            })
        }

        fn walk(
            &self,
            components: &[String],
            create_missing: Option<&LoopScopeConfig>,
        ) -> Result<DirHandle, ScopedFsError> {
            let mut current = DirHandle {
                fd: dup(self.fd.as_raw_fd())?,
            };
            let mut walked: Vec<String> = Vec::new();
            for component in components {
                let name = c_str(component)?;
                walked.push(component.clone());
                let display = walked.join("/");
                let next = match open_directory_at(current.fd.as_raw_fd(), &name, &display) {
                    Ok(handle) => handle,
                    Err(ScopedFsError::NotFound(_)) if create_missing.is_some() => {
                        let Some(scope) = create_missing else {
                            return Err(ScopedFsError::NotFound(display));
                        };
                        scope
                            .admit_mutation(
                                &LoopScopePath::from_components(walked.clone()),
                                self.case,
                            )
                            .map_err(ScopedFsError::Rejected)?;
                        // SAFETY: valid directory fd and NUL-terminated name.
                        let created =
                            unsafe { libc::mkdirat(current.fd.as_raw_fd(), name.as_ptr(), 0o755) };
                        if created != 0 {
                            let error = std::io::Error::last_os_error();
                            if error.raw_os_error() != Some(libc::EEXIST) {
                                return Err(io_error(error));
                            }
                        }
                        open_directory_at(current.fd.as_raw_fd(), &name, &display)?
                    }
                    Err(error) => return Err(error),
                };
                current = DirHandle { fd: next };
            }
            Ok(current)
        }

        pub(crate) fn read_file(&self, path: &LoopScopePath) -> Result<Vec<u8>, ScopedFsError> {
            let Some((name, parents)) = path.components().split_last() else {
                return Err(ScopedFsError::IsDirectory(".".to_string()));
            };
            let parent = self.walk(parents, None)?;
            let c_name = c_str(name)?;
            // SAFETY: valid directory fd and NUL-terminated name.
            let fd = unsafe {
                libc::openat(
                    parent.fd.as_raw_fd(),
                    c_name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(open_error(std::io::Error::last_os_error(), &path.display()));
            }
            // SAFETY: fd was just returned by openat and is owned here.
            let file = unsafe { OwnedFd::from_raw_fd(fd) };
            let stat = fstat(file.as_raw_fd())?;
            if !matches!(stat.kind, EntryKind::Regular) {
                return Err(ScopedFsError::NotRegularFile(path.display()));
            }
            if stat.size > MAX_MEDIATED_FILE_BYTES {
                return Err(ScopedFsError::TooLarge(path.display()));
            }
            let mut file = std::fs::File::from(file);
            let mut bytes = Vec::with_capacity(stat.size as usize);
            std::io::Read::read_to_end(&mut file, &mut bytes).map_err(io_error)?;
            Ok(bytes)
        }

        /// Admits and delivers one write. The parent handle is resolved once and reused for the
        /// temporary file, the fsync and the rename, so nothing between admission and delivery
        /// can redirect the effect.
        pub(crate) fn write_file(
            &self,
            scope: &LoopScopeConfig,
            path: &LoopScopePath,
            content: &[u8],
        ) -> Result<WriteReceipt, ScopedFsError> {
            let ticket = self.admit_write(scope, path)?;
            ticket.deliver(content)
        }

        pub(crate) fn admit_write(
            &self,
            scope: &LoopScopeConfig,
            path: &LoopScopePath,
        ) -> Result<WriteTicket, ScopedFsError> {
            if content_too_large(0) {
                return Err(ScopedFsError::TooLarge(path.display()));
            }
            let Some((name, parents)) = path.components().split_last() else {
                return Err(ScopedFsError::IsDirectory(".".to_string()));
            };
            scope
                .admit_mutation(path, self.case)
                .map_err(ScopedFsError::Rejected)?;
            self.verify_identity()?;
            let parent = self.walk(parents, Some(scope))?;
            let c_name = c_str(name)?;
            let existing = match fstatat(parent.fd.as_raw_fd(), &c_name) {
                Ok(stat) => Some(stat),
                Err(ScopedFsError::NotFound(_)) => None,
                Err(error) => return Err(error),
            };
            if let Some(stat) = &existing {
                match stat.kind {
                    EntryKind::Symlink => return Err(ScopedFsError::LinkTraversal(path.display())),
                    EntryKind::Directory => return Err(ScopedFsError::IsDirectory(path.display())),
                    EntryKind::Other => return Err(ScopedFsError::NotRegularFile(path.display())),
                    EntryKind::Regular => {}
                }
            }
            Ok(WriteTicket {
                parent,
                name: name.clone(),
                display: path.display(),
                existing_mode: existing.as_ref().map(|stat| stat.mode),
                shared_inode: existing.as_ref().is_some_and(|stat| stat.nlink > 1),
            })
        }

        /// Renames one entry. Both endpoints and, for a directory, every descendant's new
        /// location are admitted before anything moves.
        /// No mediated tool issues this operation yet; the boundary keeps the check so a future
        /// channel cannot reach the filesystem without it. Exercised by the sentinel tests.
        #[allow(dead_code)]
        pub(crate) fn rename(
            &self,
            scope: &LoopScopeConfig,
            from: &LoopScopePath,
            to: &LoopScopePath,
        ) -> Result<(), ScopedFsError> {
            let (Some((from_name, from_parents)), Some((to_name, to_parents))) =
                (from.components().split_last(), to.components().split_last())
            else {
                return Err(ScopedFsError::IsDirectory(".".to_string()));
            };
            scope
                .admit_mutation(from, self.case)
                .map_err(ScopedFsError::Rejected)?;
            scope
                .admit_mutation(to, self.case)
                .map_err(ScopedFsError::Rejected)?;
            self.verify_identity()?;
            let from_parent = self.walk(from_parents, None)?;
            let from_c = c_str(from_name)?;
            let source = fstatat(from_parent.fd.as_raw_fd(), &from_c)?;
            if matches!(source.kind, EntryKind::Directory) {
                let source_dir =
                    open_directory_at(from_parent.fd.as_raw_fd(), &from_c, &from.display())?;
                for descendant in collect_descendants(source_dir.as_raw_fd(), Vec::new())? {
                    let mut moved_from = from.components().to_vec();
                    moved_from.extend(descendant.iter().cloned());
                    let mut moved_to = to.components().to_vec();
                    moved_to.extend(descendant);
                    scope
                        .admit_mutation(&LoopScopePath::from_components(moved_from), self.case)
                        .map_err(ScopedFsError::Rejected)?;
                    scope
                        .admit_mutation(&LoopScopePath::from_components(moved_to), self.case)
                        .map_err(ScopedFsError::Rejected)?;
                }
            }
            let to_parent = self.walk(to_parents, Some(scope))?;
            let to_c = c_str(to_name)?;
            if let Ok(stat) = fstatat(to_parent.fd.as_raw_fd(), &to_c) {
                if matches!(stat.kind, EntryKind::Symlink) {
                    return Err(ScopedFsError::LinkTraversal(to.display()));
                }
            }
            // SAFETY: valid directory fds and NUL-terminated names.
            let renamed = unsafe {
                libc::renameat(
                    from_parent.fd.as_raw_fd(),
                    from_c.as_ptr(),
                    to_parent.fd.as_raw_fd(),
                    to_c.as_ptr(),
                )
            };
            if renamed != 0 {
                return Err(io_error(std::io::Error::last_os_error()));
            }
            Ok(())
        }

        /// Removes one entry, recursively for directories. Every affected descendant is admitted
        /// before the first unlink; links are removed as entries and never followed.
        /// No mediated tool issues this operation yet; the boundary keeps the check so a future
        /// channel cannot reach the filesystem without it. Exercised by the sentinel tests.
        #[allow(dead_code)]
        pub(crate) fn remove(
            &self,
            scope: &LoopScopeConfig,
            path: &LoopScopePath,
        ) -> Result<(), ScopedFsError> {
            let Some((name, parents)) = path.components().split_last() else {
                return Err(ScopedFsError::IsDirectory(".".to_string()));
            };
            scope
                .admit_mutation(path, self.case)
                .map_err(ScopedFsError::Rejected)?;
            self.verify_identity()?;
            let parent = self.walk(parents, None)?;
            let c_name = c_str(name)?;
            let target = fstatat(parent.fd.as_raw_fd(), &c_name)?;
            if !matches!(target.kind, EntryKind::Directory) {
                return unlinkat(parent.fd.as_raw_fd(), &c_name, false);
            }
            let dir = open_directory_at(parent.fd.as_raw_fd(), &c_name, &path.display())?;
            let descendants = collect_descendants(dir.as_raw_fd(), Vec::new())?;
            for descendant in &descendants {
                let mut full = path.components().to_vec();
                full.extend(descendant.iter().cloned());
                scope
                    .admit_mutation(&LoopScopePath::from_components(full), self.case)
                    .map_err(ScopedFsError::Rejected)?;
            }
            remove_tree(dir.as_raw_fd())?;
            drop(dir);
            unlinkat(parent.fd.as_raw_fd(), &c_name, true)
        }
    }

    pub(crate) struct WriteTicket {
        parent: DirHandle,
        name: String,
        display: String,
        existing_mode: Option<u32>,
        shared_inode: bool,
    }

    impl WriteTicket {
        pub(crate) fn deliver(self, content: &[u8]) -> Result<WriteReceipt, ScopedFsError> {
            if content_too_large(content.len() as u64) {
                return Err(ScopedFsError::TooLarge(self.display));
            }
            let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temporary = format!(
                ".{}.vanehub-scope-{}-{nonce}.tmp",
                self.name,
                std::process::id()
            );
            let c_temp = c_str(&temporary)?;
            let c_name = c_str(&self.name)?;
            // SAFETY: valid directory fd and NUL-terminated name.
            let fd = unsafe {
                libc::openat(
                    self.parent.fd.as_raw_fd(),
                    c_temp.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(io_error(std::io::Error::last_os_error()));
            }
            // SAFETY: fd was just returned by openat and is owned here.
            let file = unsafe { OwnedFd::from_raw_fd(fd) };
            let outcome = (|| -> Result<(), ScopedFsError> {
                let mut handle = std::fs::File::from(dup(file.as_raw_fd())?);
                std::io::Write::write_all(&mut handle, content).map_err(io_error)?;
                handle.sync_all().map_err(io_error)?;
                let mode = self.existing_mode.unwrap_or(0o644) & 0o7777;
                // SAFETY: valid file descriptor.
                if unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } != 0 {
                    return Err(io_error(std::io::Error::last_os_error()));
                }
                // SAFETY: valid directory fd and NUL-terminated names.
                let renamed = unsafe {
                    libc::renameat(
                        self.parent.fd.as_raw_fd(),
                        c_temp.as_ptr(),
                        self.parent.fd.as_raw_fd(),
                        c_name.as_ptr(),
                    )
                };
                if renamed != 0 {
                    return Err(io_error(std::io::Error::last_os_error()));
                }
                Ok(())
            })();
            if outcome.is_err() {
                let _ = unlinkat(self.parent.fd.as_raw_fd(), &c_temp, false);
            }
            outcome.map(|()| WriteReceipt {
                created: self.existing_mode.is_none(),
                replaced_shared_inode: self.shared_inode,
            })
        }
    }

    /// Reached only through the recursive boundary operations above.
    #[allow(dead_code)]
    fn collect_descendants(
        dir_fd: RawFd,
        prefix: Vec<String>,
    ) -> Result<Vec<Vec<String>>, ScopedFsError> {
        let mut found = Vec::new();
        for (name, kind) in list_directory(dir_fd)? {
            let mut relative = prefix.clone();
            relative.push(name.clone());
            found.push(relative.clone());
            if matches!(kind, EntryKind::Directory) {
                let c_name = c_str(&name)?;
                let child = open_directory_at(dir_fd, &c_name, &relative.join("/"))?;
                found.extend(collect_descendants(child.as_raw_fd(), relative)?);
            }
        }
        Ok(found)
    }

    /// Reached only through the recursive boundary operations above.
    #[allow(dead_code)]
    fn remove_tree(dir_fd: RawFd) -> Result<(), ScopedFsError> {
        for (name, kind) in list_directory(dir_fd)? {
            let c_name = c_str(&name)?;
            if matches!(kind, EntryKind::Directory) {
                let child = open_directory_at(dir_fd, &c_name, &name)?;
                remove_tree(child.as_raw_fd())?;
                drop(child);
                unlinkat(dir_fd, &c_name, true)?;
            } else {
                unlinkat(dir_fd, &c_name, false)?;
            }
        }
        Ok(())
    }

    /// Reached only through the recursive boundary operations above.
    #[allow(dead_code)]
    fn list_directory(dir_fd: RawFd) -> Result<Vec<(String, EntryKind)>, ScopedFsError> {
        let duplicate = dup(dir_fd)?;
        // SAFETY: the duplicated fd is a valid open directory owned by this call.
        let stream = unsafe { libc::fdopendir(duplicate.as_raw_fd()) };
        if stream.is_null() {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // fdopendir owns the descriptor from here; closedir releases it. The duplicate shares
        // the original offset, which an earlier listing left at the end.
        std::mem::forget(duplicate);
        // SAFETY: stream is a valid DIR pointer.
        unsafe { libc::rewinddir(stream) };
        let mut entries = Vec::new();
        loop {
            // SAFETY: stream is a valid DIR pointer.
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break;
            }
            // SAFETY: readdir returned a valid dirent pointer.
            let raw_name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            let name = String::from_utf8_lossy(raw_name.to_bytes()).to_string();
            if name == "." || name == ".." {
                continue;
            }
            let c_name = match c_str(&name) {
                Ok(value) => value,
                Err(error) => {
                    // SAFETY: stream is a valid DIR pointer.
                    unsafe { libc::closedir(stream) };
                    return Err(error);
                }
            };
            let kind = match fstatat(dir_fd, &c_name) {
                Ok(stat) => stat.kind,
                Err(error) => {
                    // SAFETY: stream is a valid DIR pointer.
                    unsafe { libc::closedir(stream) };
                    return Err(error);
                }
            };
            entries.push((name, kind));
        }
        // SAFETY: stream is a valid DIR pointer.
        unsafe { libc::closedir(stream) };
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(entries)
    }

    fn open_directory_at(
        dir_fd: RawFd,
        name: &CString,
        display: &str,
    ) -> Result<OwnedFd, ScopedFsError> {
        // SAFETY: valid directory fd (or AT_FDCWD) and NUL-terminated name.
        let fd = unsafe {
            libc::openat(
                dir_fd,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::ENOTDIR) | Some(libc::ELOOP)
            ) && matches!(fstatat(dir_fd, name), Ok(stat) if matches!(stat.kind, EntryKind::Symlink))
            {
                return Err(ScopedFsError::LinkTraversal(display.to_string()));
            }
            return Err(open_error(error, display));
        }
        // SAFETY: fd was just returned by openat and is owned here.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn open_error(error: std::io::Error, display: &str) -> ScopedFsError {
        match error.raw_os_error() {
            Some(libc::ELOOP) => ScopedFsError::LinkTraversal(display.to_string()),
            Some(libc::ENOENT) => ScopedFsError::NotFound(display.to_string()),
            Some(libc::ENOTDIR) => ScopedFsError::NotRegularFile(display.to_string()),
            _ => io_error(error),
        }
    }

    fn fstat(fd: RawFd) -> Result<EntryStat, ScopedFsError> {
        // SAFETY: stat is a plain C struct fully initialized by the call.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: valid fd and out-pointer.
        if unsafe { libc::fstat(fd, &mut stat) } != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(entry_stat(&stat))
    }

    fn fstatat(dir_fd: RawFd, name: &CString) -> Result<EntryStat, ScopedFsError> {
        // SAFETY: stat is a plain C struct fully initialized by the call.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: valid directory fd, NUL-terminated name and out-pointer.
        if unsafe { libc::fstatat(dir_fd, name.as_ptr(), &mut stat, libc::AT_SYMLINK_NOFOLLOW) }
            != 0
        {
            let error = std::io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::ENOENT) => ScopedFsError::NotFound(name.to_string_lossy().to_string()),
                _ => io_error(error),
            });
        }
        Ok(entry_stat(&stat))
    }

    fn stat_path(path: &Path) -> Result<EntryStat, ScopedFsError> {
        let c = c_path(path)?;
        // SAFETY: stat is a plain C struct fully initialized by the call.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: NUL-terminated path and out-pointer.
        if unsafe { libc::lstat(c.as_ptr(), &mut stat) } != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(entry_stat(&stat))
    }

    // stat field widths differ between unix targets; the casts are needed off Linux.
    #[allow(clippy::unnecessary_cast)]
    fn entry_stat(stat: &libc::stat) -> EntryStat {
        let kind = match stat.st_mode & libc::S_IFMT {
            libc::S_IFREG => EntryKind::Regular,
            libc::S_IFDIR => EntryKind::Directory,
            libc::S_IFLNK => EntryKind::Symlink,
            _ => EntryKind::Other,
        };
        EntryStat {
            kind,
            mode: stat.st_mode as u32,
            nlink: stat.st_nlink as u64,
            size: stat.st_size as u64,
            identity: ResourceIdentity {
                device: stat.st_dev as u64,
                inode: stat.st_ino as u64,
            },
        }
    }

    fn unlinkat(dir_fd: RawFd, name: &CString, directory: bool) -> Result<(), ScopedFsError> {
        let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
        // SAFETY: valid directory fd and NUL-terminated name.
        if unsafe { libc::unlinkat(dir_fd, name.as_ptr(), flags) } != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn dup(fd: RawFd) -> Result<OwnedFd, ScopedFsError> {
        // SAFETY: valid fd.
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // SAFETY: fd was just returned by fcntl and is owned here.
        Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
    }

    fn c_str(value: &str) -> Result<CString, ScopedFsError> {
        CString::new(value).map_err(|_| ScopedFsError::InvalidPath("embedded NUL".to_string()))
    }

    fn c_path(path: &Path) -> Result<CString, ScopedFsError> {
        CString::new(path.as_os_str().as_bytes())
            .map_err(|_| ScopedFsError::InvalidPath("embedded NUL".to_string()))
    }

    fn content_too_large(bytes: u64) -> bool {
        bytes > MAX_MEDIATED_FILE_BYTES
    }

    fn io_error(error: std::io::Error) -> ScopedFsError {
        ScopedFsError::Io(error.to_string())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::fs;

    fn scope(allowed: &[&str], protected: &[&str]) -> LoopScopeConfig {
        LoopScopeConfig::parse(
            &allowed
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            &protected
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("scope")
    }

    fn rel(value: &str) -> LoopScopePath {
        LoopScopePath::parse(value).expect("path")
    }

    fn sentinel() -> String {
        format!("sentinel-{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn allowed_write_lands_and_out_of_scope_writes_have_zero_side_effects() {
        let workspace = TempDirectory::new("scope-fs-basic");
        let untouched = sentinel();
        workspace.write("src-old/a.txt", &untouched);
        workspace.write("src/generated/a.ts", &untouched);
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        let scope = scope(&["src"], &["src/generated"]);

        let receipt = root
            .write_file(&scope, &rel("src/app.ts"), b"export const a = 1;\n")
            .expect("allowed write");
        assert!(receipt.created);
        assert_eq!(
            fs::read_to_string(workspace.path().join("src/app.ts")).expect("written"),
            "export const a = 1;\n"
        );
        assert_eq!(
            root.write_file(&scope, &rel("src-old/a.txt"), b"x")
                .expect_err("outside")
                .code(),
            "scope-outside-allowed"
        );
        assert_eq!(
            root.write_file(&scope, &rel("src/generated/a.ts"), b"x")
                .expect_err("protected")
                .code(),
            "scope-protected-path"
        );
        assert_eq!(
            root.write_file(&scope, &rel("src/.git/config"), b"x")
                .expect_err("reserved")
                .code(),
            "scope-reserved-resource"
        );
        assert_eq!(
            fs::read_to_string(workspace.path().join("src-old/a.txt")).expect("sentinel"),
            untouched
        );
        assert_eq!(
            fs::read_to_string(workspace.path().join("src/generated/a.ts")).expect("sentinel"),
            untouched
        );
        // No temporary file survives a completed or refused write.
        assert!(!fs::read_dir(workspace.path().join("src"))
            .expect("dir")
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp")));
    }

    #[test]
    fn links_are_never_followed_for_delivery() {
        let workspace = TempDirectory::new("scope-fs-links");
        let outside = TempDirectory::new("scope-fs-links-outside");
        let victim = sentinel();
        let victim_path = outside.path().join("victim.txt");
        fs::write(&victim_path, &victim).expect("victim");
        fs::create_dir_all(workspace.path().join("src")).expect("src");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("src/linked-dir"))
            .expect("dir link");
        std::os::unix::fs::symlink(&victim_path, workspace.path().join("src/linked.txt"))
            .expect("file link");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        let scope = scope(&["."], &[]);

        assert_eq!(
            root.write_file(&scope, &rel("src/linked-dir/new.txt"), b"x")
                .expect_err("dir link")
                .code(),
            "scope-link-traversal"
        );
        assert_eq!(
            root.write_file(&scope, &rel("src/linked.txt"), b"x")
                .expect_err("file link")
                .code(),
            "scope-link-traversal"
        );
        assert!(!outside.path().join("new.txt").exists());
        assert_eq!(fs::read_to_string(&victim_path).expect("victim"), victim);
        assert_eq!(
            root.read_file(&rel("src/linked.txt"))
                .expect_err("read link")
                .code(),
            "scope-link-traversal"
        );
    }

    #[test]
    fn a_parent_swapped_after_admission_cannot_redirect_delivery() {
        let workspace = TempDirectory::new("scope-fs-swap");
        let outside = TempDirectory::new("scope-fs-swap-outside");
        fs::create_dir_all(workspace.path().join("src")).expect("src");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        let scope = scope(&["src"], &[]);
        let ticket = root
            .admit_write(&scope, &rel("src/app.ts"))
            .expect("ticket");

        // Between admission and delivery the directory is replaced with a link outside.
        fs::rename(
            workspace.path().join("src"),
            workspace.path().join("src-moved"),
        )
        .expect("move");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("src")).expect("swap");

        ticket
            .deliver(b"payload")
            .expect("deliver through the held handle");
        assert!(
            !outside.path().join("app.ts").exists(),
            "link target untouched"
        );
        assert_eq!(
            fs::read_to_string(workspace.path().join("src-moved/app.ts")).expect("held dir"),
            "payload"
        );
    }

    #[test]
    fn hardlink_aliases_outside_scope_are_not_mutated_in_place() {
        let workspace = TempDirectory::new("scope-fs-hardlink");
        let original = sentinel();
        workspace.write("shared/secret.txt", &original);
        fs::create_dir_all(workspace.path().join("src")).expect("src");
        fs::hard_link(
            workspace.path().join("shared/secret.txt"),
            workspace.path().join("src/alias.txt"),
        )
        .expect("hard link");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        let scope = scope(&["src"], &[]);

        let receipt = root
            .write_file(&scope, &rel("src/alias.txt"), b"changed")
            .expect("replace");
        assert!(receipt.replaced_shared_inode);
        assert_eq!(
            fs::read_to_string(workspace.path().join("shared/secret.txt")).expect("alias"),
            original,
            "the out-of-scope alias keeps its content"
        );
        assert_eq!(
            fs::read_to_string(workspace.path().join("src/alias.txt")).expect("target"),
            "changed"
        );
    }

    #[test]
    fn rename_and_recursive_delete_check_every_affected_endpoint() {
        let workspace = TempDirectory::new("scope-fs-rename");
        let keep = sentinel();
        workspace.write("src/a/keep.txt", "a");
        workspace.write("src/a/generated/out.js", &keep);
        workspace.write("docs/readme.md", &keep);
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        let scope = scope(&["src"], &["src/a/generated"]);

        assert_eq!(
            root.rename(&scope, &rel("src/a/keep.txt"), &rel("docs/keep.txt"))
                .expect_err("destination outside")
                .code(),
            "scope-outside-allowed"
        );
        assert_eq!(
            root.rename(&scope, &rel("src/a"), &rel("src/b"))
                .expect_err("moves a protected descendant")
                .code(),
            "scope-protected-path"
        );
        assert_eq!(
            root.remove(&scope, &rel("src/a"))
                .expect_err("protected descendant")
                .code(),
            "scope-protected-path"
        );
        assert!(workspace.path().join("src/a/keep.txt").exists());
        assert_eq!(
            fs::read_to_string(workspace.path().join("src/a/generated/out.js")).expect("kept"),
            keep
        );
        root.rename(&scope, &rel("src/a/keep.txt"), &rel("src/moved.txt"))
            .expect("allowed rename");
        assert!(workspace.path().join("src/moved.txt").exists());
        workspace.write("src/c/deep/x.txt", "x");
        root.remove(&scope, &rel("src/c"))
            .expect("allowed recursive delete");
        assert!(!workspace.path().join("src/c").exists());
    }

    #[test]
    fn root_identity_is_verified_and_case_rule_comes_from_a_probe() {
        let workspace = TempDirectory::new("scope-fs-identity");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        assert!(matches!(
            root.case_rule(),
            CaseRule::Sensitive | CaseRule::InsensitiveAscii
        ));
        assert!(fs::read_dir(workspace.path())
            .expect("dir")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().contains("case-probe")));
        root.verify_identity().expect("same root");
        let moved = workspace.path().with_extension("moved");
        fs::rename(workspace.path(), &moved).expect("move root");
        fs::create_dir_all(workspace.path()).expect("replacement");
        assert_eq!(
            root.verify_identity().expect_err("replaced").code(),
            "scope-root-changed"
        );
        fs::remove_dir_all(&moved).expect("cleanup");
        assert_eq!(
            relative_scope_path(root.root(), &format!("{}/src/a.ts", root.root().display()))
                .expect("absolute")
                .display(),
            "src/a.ts"
        );
        assert!(relative_scope_path(root.root(), "/etc/passwd").is_err());
        assert!(relative_scope_path(root.root(), "../x").is_err());
    }
}
