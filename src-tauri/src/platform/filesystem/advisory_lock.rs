//! Cross-process advisory file locks.
//!
//! Two application instances sharing one data directory cannot coordinate through an in-process
//! mutex, and a row in SQLite says who *claimed* something, not whether that claimant is still
//! alive. An OS-held lock on an open file handle answers the second question: it is released the
//! moment the holding process dies, however it dies, so "can I take the lock" is the same
//! question as "is the previous owner gone".

use std::fs::{File, TryLockError};
use std::path::PathBuf;

/// Why a lock could not be taken. Neither is ever a reason to proceed without it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdvisoryLockRejection {
    /// Another handle — in this process or another — holds it.
    Busy,
    /// The lock file could not be opened or locked at all.
    Unavailable,
}

/// A lock file at a fixed path.
///
/// The file is never deleted: removing it between an unlock and the next lock would let two
/// holders each create their own file and each believe they hold the lock. Its presence means
/// nothing on its own — only the OS lock on an open handle does.
#[derive(Debug, Clone)]
pub(crate) struct AdvisoryFileLock {
    path: PathBuf,
}

/// Held for as long as the lock is held; dropping it releases the OS lock and closes the handle.
#[derive(Debug)]
pub(crate) struct AdvisoryLockGuard {
    /// `Option` so `Drop` can take ownership and close the handle after unlocking.
    file: Option<File>,
}

impl AdvisoryFileLock {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Takes the lock without waiting. Callers that would rather wait retry with their own bound;
    /// nothing here ever blocks a thread on another process.
    pub(crate) fn try_acquire(&self) -> Result<AdvisoryLockGuard, AdvisoryLockRejection> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| AdvisoryLockRejection::Unavailable)?;
        }
        // Opened read/write purely to obtain a handle the OS will lock; nothing is ever written,
        // and `truncate(false)` keeps a concurrent holder's handle undisturbed.
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .map_err(|_| AdvisoryLockRejection::Unavailable)?;
        match file.try_lock() {
            Ok(()) => Ok(AdvisoryLockGuard { file: Some(file) }),
            Err(TryLockError::WouldBlock) => Err(AdvisoryLockRejection::Busy),
            Err(TryLockError::Error(_)) => Err(AdvisoryLockRejection::Unavailable),
        }
    }

    /// Whether the lock is currently held by someone — anyone — without keeping it.
    ///
    /// `Ok(true)` means a holder exists. `Ok(false)` means the lock could be taken (and was
    /// immediately released). `Err` means the question could not be answered, which callers must
    /// treat as "unknown", never as "free".
    pub(crate) fn is_held(&self) -> Result<bool, AdvisoryLockRejection> {
        match self.try_acquire() {
            Ok(guard) => {
                drop(guard);
                Ok(false)
            }
            Err(AdvisoryLockRejection::Busy) => Ok(true),
            Err(error) => Err(error),
        }
    }
}

impl Drop for AdvisoryLockGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn lock_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vanehub-advisory-lock-test-{}-{label}.lock",
            std::process::id()
        ))
    }

    #[test]
    fn a_held_lock_reports_busy_and_frees_on_drop() {
        let lock = AdvisoryFileLock::new(lock_path("busy"));
        let guard = lock.try_acquire().expect("first acquisition");
        assert_eq!(lock.is_held(), Ok(true));
        drop(guard);
        assert_eq!(lock.is_held(), Ok(false));
    }

    #[test]
    fn a_lock_held_by_another_process_is_visible_and_released_when_it_exits() {
        let path = lock_path("other-process");
        let lock = AdvisoryFileLock::new(&path);
        // A child holds the lock via a shell for a bounded time; the parent observes both states.
        #[cfg(unix)]
        {
            let holder = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "exec 9>>'{}'; flock -n 9 || exit 3; sleep 1",
                    path.display()
                ))
                .spawn()
                .expect("spawn holder");
            std::thread::sleep(std::time::Duration::from_millis(250));
            let held = lock.is_held().expect("lock query");
            let status = holder.wait_with_output().expect("holder exit");
            if status.status.code() == Some(3) {
                // `flock` unavailable on this host: nothing to assert beyond not panicking.
                return;
            }
            assert!(held, "lock held by a live process must report busy");
            assert_eq!(lock.is_held(), Ok(false));
        }
        #[cfg(not(unix))]
        {
            let _ = Command::new("cmd");
            assert_eq!(lock.is_held(), Ok(false));
        }
    }
}
