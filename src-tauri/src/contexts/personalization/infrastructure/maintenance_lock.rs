use std::cell::RefCell;
use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use super::memory_directory_lock::{ensure_directory, MAINTENANCE_LOCK_FILE_NAME};
use crate::contexts::personalization::application::{
    MaintenanceGatePort, MaintenanceLease, MutationAdmission, PersonalizationApplicationError,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

thread_local! {
    /// Which directory this thread has admitted, and how deep.
    ///
    /// Re-entrance is the whole reason this exists. Maintenance calls the ordinary write paths, and
    /// an ordinary coordinated write takes the gate once around the whole operation and again
    /// inside the store. Neither could take a non-re-entrant lock twice — it would report itself
    /// busy, which is worse than a hang because it looks like contention that is not there.
    ///
    /// Keyed by path rather than a bare counter: a thread that holds one directory must not be
    /// waved through on another. One directory at a time per thread is a real invariant — nothing
    /// maintains two memory directories at once — so a single entry is enough.
    static HELD: RefCell<Option<(PathBuf, usize)>> = const { RefCell::new(None) };
}

fn enter(path: &Path) -> bool {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        match held.as_mut() {
            Some((existing, depth)) if existing == path => {
                *depth += 1;
                true
            }
            Some(_) => false,
            None => {
                *held = Some((path.to_path_buf(), 1));
                false
            }
        }
    })
}

/// Undoes one `enter`. Reports whether this was the outermost, and therefore whether the caller
/// still owns an operating-system lock to release.
fn leave(path: &Path) -> bool {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        let Some((existing, depth)) = held.as_mut() else {
            return false;
        };
        if existing != path {
            return false;
        }
        *depth -= 1;
        if *depth == 0 {
            *held = None;
            return true;
        }
        false
    })
}

/// Decides who owns one memory directory right now.
///
/// # Why a gate rather than a health check
///
/// Reading `MemoryRuntimeHealth` and then taking the directory lock is two steps with a window
/// between them. A writer that read `Ready`, was descheduled, and resumed after another process
/// started maintenance would take the directory lock — maintenance releases it between every one of
/// its own operations, and holds none at all during the derived rebuild — and mutate underneath it.
/// The concrete damage is a delete that reconciliation then undoes from a snapshot taken before it,
/// putting a memory the user removed back into the projection and the index.
///
/// So the health check is not the exclusion. This is: an ordinary mutation holds a shared admission
/// for the whole operation *including the health check*, and maintenance holds the gate
/// exclusively for its whole run. A `Ready` read taken inside an admission cannot go stale while
/// that admission lives.
///
/// # Lock order
///
/// One order, everywhere, and releases in reverse:
///
/// 1. this maintenance gate — shared for an ordinary mutation, exclusive for maintenance;
/// 2. the memory directory lock, which serializes concurrent ordinary mutations;
/// 3. the authoritative Markdown file;
/// 4. the SQLite projection;
/// 5. the retrieval index and `MEMORY.md`.
pub(crate) struct MaintenanceGate {
    lock_path: PathBuf,
}

impl MaintenanceGate {
    pub(crate) fn new(root: &Path) -> Result<Self> {
        ensure_directory(root)?;
        Ok(Self {
            lock_path: root.join(MAINTENANCE_LOCK_FILE_NAME),
        })
    }

    /// `File::options()` rather than `OpenOptions::new()`: identical semantics, but the
    /// architecture rule that keeps append-log construction inside the platform adapter matches
    /// `OpenOptions::new` syntactically, and a lock file is not an append log. Never truncated, so
    /// opening it can never disturb a holder.
    fn open(&self) -> Result<File> {
        File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(|_| {
                PersonalizationApplicationError::Storage("maintenance_gate_unavailable".to_string())
            })
    }
}

impl MaintenanceGatePort for MaintenanceGate {
    fn enter_mutation(&self) -> Result<Box<dyn MutationAdmission>> {
        if enter(&self.lock_path) {
            // Already admitted on this thread. The outermost holder owns the release.
            return Ok(Box::new(Admission {
                lock_path: self.lock_path.clone(),
                file: None,
            }));
        }
        let file = match self.open() {
            Ok(file) => file,
            Err(error) => {
                leave(&self.lock_path);
                return Err(error);
            }
        };
        // Shared, so concurrent ordinary mutations do not exclude each other here — the directory
        // lock below is what serializes them. Only maintenance is excluded, and only because it
        // takes this same lock exclusively.
        match file.try_lock_shared() {
            Ok(()) => Ok(Box::new(Admission {
                lock_path: self.lock_path.clone(),
                file: Some(file),
            })),
            Err(TryLockError::WouldBlock) => {
                leave(&self.lock_path);
                Err(PersonalizationApplicationError::MaintenanceBusy)
            }
            Err(TryLockError::Error(_)) => {
                leave(&self.lock_path);
                Err(PersonalizationApplicationError::Storage(
                    "maintenance_gate_unavailable".to_string(),
                ))
            }
        }
    }

    fn try_enter_maintenance(&self) -> Result<Option<Box<dyn MaintenanceLease>>> {
        // Never re-entrant. Maintenance inside maintenance would run two migrations over one
        // directory, and admitting it because the thread already holds an admission is exactly the
        // kind of convenience that turns a guard into a formality.
        if HELD.with(|held| held.borrow().is_some()) {
            return Err(PersonalizationApplicationError::Storage(
                "maintenance_reentered".to_string(),
            ));
        }
        let file = self.open()?;
        match file.try_lock() {
            Ok(()) => {
                enter(&self.lock_path);
                Ok(Some(Box::new(Lease {
                    lock_path: self.lock_path.clone(),
                    file: Some(file),
                })))
            }
            // Someone else is migrating. Expected at every second launch, and never a reason to
            // proceed without it.
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(_)) => Err(PersonalizationApplicationError::Storage(
                "maintenance_gate_unavailable".to_string(),
            )),
        }
    }
}

/// One ordinary mutation's claim on the directory. Released on drop, including on panic.
struct Admission {
    lock_path: PathBuf,
    /// `None` for a re-entrant admission, which owns no operating-system lock.
    file: Option<File>,
}

impl MutationAdmission for Admission {}

impl Drop for Admission {
    fn drop(&mut self) {
        let outermost = leave(&self.lock_path);
        if let Some(file) = self.file.take() {
            debug_assert!(
                outermost,
                "only the outermost admission holds the lock file"
            );
            // Best effort: closing the handle releases the lock regardless, so a failure here
            // cannot leave the directory permanently claimed.
            let _ = file.unlock();
        }
    }
}

/// Maintenance's exclusive claim, held for a whole run.
struct Lease {
    lock_path: PathBuf,
    file: Option<File>,
}

impl MaintenanceLease for Lease {}

impl Drop for Lease {
    fn drop(&mut self) {
        leave(&self.lock_path);
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}
