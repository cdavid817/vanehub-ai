use std::path::Path;
use std::sync::Arc;

use super::memory_directory_lock::{
    ensure_directory, MemoryDirectoryLock, MemoryLockRejection, MAINTENANCE_LOCK_FILE_NAME,
};
use crate::contexts::personalization::application::{
    MaintenanceLease, MaintenanceLockPort, PersonalizationApplicationError,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Cross-process ownership of startup maintenance for one memory directory.
///
/// Backed by an operating-system lock on an open handle rather than by the presence of a file: a
/// process that dies mid-migration releases it when its handles close, so the next launch proceeds
/// instead of finding a stale marker nobody can prove is stale.
pub(crate) struct FileMaintenanceLock {
    lock: Arc<MemoryDirectoryLock>,
}

impl FileMaintenanceLock {
    pub(crate) fn new(root: &Path) -> Result<Self> {
        ensure_directory(root)?;
        Ok(Self {
            lock: Arc::new(MemoryDirectoryLock::named(root, MAINTENANCE_LOCK_FILE_NAME)),
        })
    }
}

impl MaintenanceLockPort for FileMaintenanceLock {
    fn try_acquire(&self) -> Result<Option<Box<dyn MaintenanceLease>>> {
        // The lease borrows nothing, so the guard has to own its lock. An `Arc` clone is what lets
        // the lease outlive this call while still releasing on drop.
        let lock = self.lock.clone();
        match OwnedMaintenanceLease::acquire(lock) {
            Ok(lease) => Ok(Some(Box::new(lease))),
            // Someone else is migrating. Expected at every second launch, and never a reason to
            // proceed without the lock.
            Err(MemoryLockRejection::Busy) => Ok(None),
            Err(MemoryLockRejection::Unavailable) => Err(PersonalizationApplicationError::Storage(
                "maintenance_lock_unavailable".to_string(),
            )),
        }
    }
}

/// A held maintenance lock that owns the lock it was taken from.
///
/// `MemoryDirectoryGuard` borrows its lock, which cannot cross a `Box<dyn MaintenanceLease>`
/// boundary. Holding the `Arc` and releasing in `Drop` gives the same guarantee — including on
/// panic — without the lifetime.
struct OwnedMaintenanceLease {
    lock: Arc<MemoryDirectoryLock>,
    /// `None` once released, so `Drop` is idempotent.
    file: Option<std::fs::File>,
}

impl OwnedMaintenanceLease {
    fn acquire(lock: Arc<MemoryDirectoryLock>) -> std::result::Result<Self, MemoryLockRejection> {
        let file = lock.try_acquire_owned()?;
        Ok(Self {
            lock,
            file: Some(file),
        })
    }
}

impl MaintenanceLease for OwnedMaintenanceLease {}

impl Drop for OwnedMaintenanceLease {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            self.lock.release_owned(file);
        }
    }
}
