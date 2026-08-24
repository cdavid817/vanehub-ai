use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use crate::contexts::personalization::application::PersonalizationApplicationError;

/// The application-owned lock file.
///
/// A fixed name, never derived from user input, and never deleted: removing it between an unlock
/// and the next lock would let two holders each create their own file and each believe they hold
/// the directory. Its presence means nothing on its own — only the OS lock on an open handle does.
pub(crate) const MEMORY_LOCK_FILE_NAME: &str = ".personalization-memory.lock";

/// The startup-maintenance lock, deliberately a different file.
///
/// Maintenance runs for the length of a whole migration and calls the ordinary write paths while it
/// does, and those take the directory lock themselves. One lock for both would mean maintenance
/// deadlocking against its own writes — reported as busy by the non-blocking acquisition, which is
/// worse than a hang because it looks like contention that is not there. Two locks with distinct
/// scopes is the honest shape: this one answers "is another process running maintenance", the other
/// answers "is another writer touching this directory right now".
pub(crate) const MAINTENANCE_LOCK_FILE_NAME: &str = ".personalization-maintenance.lock";

/// Bounded retry budget for callers that would rather wait briefly than fail.
///
/// Deliberately bounded: nothing here ever calls the blocking `File::lock`, so no path can park a
/// runtime thread indefinitely waiting on another process.
const DEFAULT_RETRY_ATTEMPTS: usize = 10;
const DEFAULT_RETRY_BACKOFF: Duration = Duration::from_millis(25);

/// Serializes every mutation of one memory directory, within this process and across processes.
///
/// # Lock order
///
/// Every mutating path takes these in exactly this order, and releases in reverse:
///
/// 1. the in-process directory mutex (below);
/// 2. the cross-process OS file lock (below);
/// 3. the authoritative Markdown file mutation;
/// 4. the SQLite projection transaction;
/// 5. derived `MEMORY.md` and retrieval-index coordination.
///
/// Steps 1 and 2 are what this type owns; 3 through 5 are the order the coordination service calls
/// them in. A path that took the projection transaction before this lock could deadlock against a
/// path that took them the other way round, which is why the order is stated once, here, rather
/// than rediscovered at each call site.
///
/// The in-process mutex is not redundant with the OS lock. On Unix, `flock` is held per open file
/// description, so two threads in one process sharing one handle would not contend at all; the
/// mutex is what makes the guarantee uniform.
pub(crate) struct MemoryDirectoryLock {
    lock_path: PathBuf,
    in_process: Mutex<()>,
}

/// Why a lock could not be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryLockRejection {
    /// Someone else holds it. Expected, retryable, and never a reason to proceed without it.
    Busy,
    /// The lock file itself could not be opened or locked. Also never a reason to proceed.
    Unavailable,
}

impl From<MemoryLockRejection> for PersonalizationApplicationError {
    fn from(rejection: MemoryLockRejection) -> Self {
        match rejection {
            MemoryLockRejection::Busy => Self::MaintenanceBusy,
            MemoryLockRejection::Unavailable => {
                Self::Storage("the memory directory lock is unavailable".to_string())
            }
        }
    }
}

impl MemoryDirectoryLock {
    pub(crate) fn new(root: &Path) -> Self {
        Self::named(root, MEMORY_LOCK_FILE_NAME)
    }

    /// Builds a lock over a named file in the same directory.
    ///
    /// The name is chosen from the constants above and never derived from user input, so no caller
    /// can point a lock at a memory file and have it truncated or held open.
    pub(crate) fn named(root: &Path, file_name: &str) -> Self {
        Self {
            lock_path: root.join(file_name),
            in_process: Mutex::new(()),
        }
    }

    pub(crate) fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Takes the lock or reports why it could not, without ever blocking.
    pub(crate) fn try_acquire(&self) -> Result<MemoryDirectoryGuard<'_>, MemoryLockRejection> {
        // `try_lock` rather than `lock` on the in-process mutex too: a reentrant acquisition is a
        // caller bug, and reporting it as busy is far better than deadlocking a thread against
        // itself with no diagnostic.
        let in_process = self
            .in_process
            .try_lock()
            .map_err(|_| MemoryLockRejection::Busy)?;

        // `File::options()` rather than `OpenOptions::new()`: identical semantics, but the
        // architecture rule that keeps append-log construction inside the platform adapter matches
        // `OpenOptions::new` syntactically, and a lock file is not an append log — it is opened
        // read/write purely to obtain a handle the OS will lock, and nothing is ever written to it.
        // `create(true)` without `truncate`, so a concurrent holder's handle is never truncated.
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            // Explicitly never truncate. Nothing is stored in this file, so truncation would be
            // harmless in content terms, but it would mean touching a file another holder has open
            // for the sole purpose of not disturbing them.
            .truncate(false)
            .open(&self.lock_path)
            .map_err(|_| MemoryLockRejection::Unavailable)?;

        match file.try_lock() {
            Ok(()) => Ok(MemoryDirectoryGuard {
                _in_process: in_process,
                file: Some(file),
            }),
            Err(TryLockError::WouldBlock) => Err(MemoryLockRejection::Busy),
            Err(TryLockError::Error(_)) => Err(MemoryLockRejection::Unavailable),
        }
    }

    /// Takes the cross-process lock and hands back the handle holding it.
    ///
    /// Used only by startup maintenance, which has to hold the lock across a whole run and past the
    /// borrow a `MemoryDirectoryGuard` would impose. The in-process mutex is deliberately not taken
    /// here: this lock is never contended within a process — one process runs maintenance once —
    /// and taking a guard that cannot be returned would leave it poisoned for the process lifetime.
    pub(crate) fn try_acquire_owned(&self) -> Result<File, MemoryLockRejection> {
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(|_| MemoryLockRejection::Unavailable)?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(TryLockError::WouldBlock) => Err(MemoryLockRejection::Busy),
            Err(TryLockError::Error(_)) => Err(MemoryLockRejection::Unavailable),
        }
    }

    /// Releases a handle taken by `try_acquire_owned`.
    ///
    /// Best effort: closing the handle releases the operating-system lock regardless, so a failure
    /// here cannot leave maintenance permanently locked out.
    pub(crate) fn release_owned(&self, file: File) {
        let _ = file.unlock();
    }

    /// Takes the lock, retrying a bounded number of times before reporting busy.
    ///
    /// Used by interactive paths, where a brief wait beats surfacing a transient conflict. Startup
    /// migration deliberately does not use this: it reports busy immediately and leaves long-term
    /// memory unavailable rather than delaying application start.
    pub(crate) fn acquire_with_retry(
        &self,
    ) -> Result<MemoryDirectoryGuard<'_>, MemoryLockRejection> {
        self.acquire_with_budget(DEFAULT_RETRY_ATTEMPTS, DEFAULT_RETRY_BACKOFF)
    }

    pub(crate) fn acquire_with_budget(
        &self,
        attempts: usize,
        backoff: Duration,
    ) -> Result<MemoryDirectoryGuard<'_>, MemoryLockRejection> {
        let mut last = MemoryLockRejection::Busy;
        for attempt in 0..attempts.max(1) {
            match self.try_acquire() {
                Ok(guard) => return Ok(guard),
                // An unavailable lock file will not become available by waiting, and retrying
                // would turn a configuration problem into a stall.
                Err(MemoryLockRejection::Unavailable) => {
                    return Err(MemoryLockRejection::Unavailable)
                }
                Err(rejection) => {
                    last = rejection;
                    if attempt + 1 < attempts.max(1) {
                        std::thread::sleep(backoff);
                    }
                }
            }
        }
        Err(last)
    }
}

/// Holds the directory lock for as long as it lives.
///
/// Releasing on `Drop` rather than through an explicit call is what makes an early return or a
/// panic release the lock; the OS also releases it when the handle closes, so a process that dies
/// outright leaves nothing stale behind.
pub(crate) struct MemoryDirectoryGuard<'a> {
    _in_process: MutexGuard<'a, ()>,
    /// `Option` so `Drop` can take ownership and close the handle after unlocking.
    file: Option<File>,
}

impl Drop for MemoryDirectoryGuard<'_> {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            // Best effort: the handle closing releases the lock regardless, so a failure here
            // cannot leave the directory permanently locked.
            let _ = file.unlock();
        }
    }
}

/// Whether a directory entry is the lock file.
///
/// Enumeration excludes it explicitly rather than relying on its extension, so it can never be
/// classified, listed, counted, or deleted as if it were a memory.
pub(crate) fn is_lock_file(file_name: &str) -> bool {
    file_name == MEMORY_LOCK_FILE_NAME || file_name == MAINTENANCE_LOCK_FILE_NAME
}

/// Ensures the directory exists before a lock file is created inside it.
pub(crate) fn ensure_directory(root: &Path) -> Result<(), PersonalizationApplicationError> {
    fs::create_dir_all(root).map_err(|error| {
        PersonalizationApplicationError::Storage(format!(
            "memory directory {} is unavailable: {error}",
            root.display()
        ))
    })
}
