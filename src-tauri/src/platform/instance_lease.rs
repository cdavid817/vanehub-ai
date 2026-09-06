//! The identity of this running process, provable to other processes.
//!
//! A row in a shared database can say "instance X owns this"; it cannot say whether X is still
//! running. This lease holds an OS advisory lock on a per-instance file for the life of the
//! process, so any other process can answer "is X alive" by trying to take X's lock. Lock files
//! are never deleted — a deleted file would let two processes each create their own and each
//! believe they hold it.

use super::filesystem::advisory_lock::{
    AdvisoryFileLock, AdvisoryLockGuard, AdvisoryLockRejection,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const LOCK_DIRECTORY: &str = "instance-locks";

#[derive(Clone)]
pub(crate) struct InstanceLease {
    id: Arc<str>,
    epoch: u64,
    lock_directory: PathBuf,
    _guard: Arc<AdvisoryLockGuard>,
}

impl InstanceLease {
    /// Takes a fresh identity under `data_directory`. Fails only when the lock directory cannot
    /// be used at all, in which case no cross-instance guarantee can be made and the caller must
    /// not pretend otherwise.
    pub(crate) fn acquire(data_directory: &Path) -> Result<Self, AdvisoryLockRejection> {
        let id = uuid::Uuid::new_v4().to_string();
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0);
        let lock_directory = data_directory.join(LOCK_DIRECTORY);
        let guard =
            AdvisoryFileLock::new(Self::lock_path_in(&lock_directory, &id)).try_acquire()?;
        Ok(Self {
            id: Arc::from(id),
            epoch,
            lock_directory,
            _guard: Arc::new(guard),
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Whether the instance with `id` still holds its lock. `Err` is "could not tell", which a
    /// caller must treat as alive.
    pub(crate) fn is_alive(&self, id: &str) -> Result<bool, AdvisoryLockRejection> {
        if id == self.id.as_ref() {
            return Ok(true);
        }
        AdvisoryFileLock::new(Self::lock_path_in(&self.lock_directory, id)).is_held()
    }

    fn lock_path_in(lock_directory: &Path, id: &str) -> PathBuf {
        // The id is a UUID of our own making, but sanitizing it costs nothing and keeps a
        // corrupted row from naming a path outside the directory.
        let safe: String = id
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
            .collect();
        lock_directory.join(format!("instance-{safe}.lock"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lease_is_alive_for_itself_and_dead_for_an_unknown_id() {
        let directory = crate::test_support::TempDirectory::new("instance-lease");
        let lease = InstanceLease::acquire(directory.path()).expect("lease");
        assert_eq!(lease.is_alive(lease.id()), Ok(true));
        assert_eq!(
            lease.is_alive("00000000-0000-0000-0000-000000000000"),
            Ok(false)
        );
        assert!(lease.epoch() > 0);
    }

    #[test]
    fn two_leases_in_one_directory_see_each_other_until_one_drops() {
        let directory = crate::test_support::TempDirectory::new("instance-lease-pair");
        let first = InstanceLease::acquire(directory.path()).expect("first");
        let second = InstanceLease::acquire(directory.path()).expect("second");
        assert_eq!(first.is_alive(second.id()), Ok(true));
        let second_id = second.id().to_string();
        drop(second);
        assert_eq!(first.is_alive(&second_id), Ok(false));
    }

    #[test]
    fn lock_paths_never_escape_the_lock_directory() {
        let path = InstanceLease::lock_path_in(Path::new("/locks"), "../../etc/passwd");
        assert!(path.starts_with("/locks"));
        assert_eq!(path.file_name().unwrap(), "instance-etcpasswd.lock");
    }
}
