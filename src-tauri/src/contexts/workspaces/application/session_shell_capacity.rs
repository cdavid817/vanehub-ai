//! Admission control for retained Shells.
//!
//! Counting the live Shells and then opening one is not admission control: between the count and
//! the open there is a window in which every concurrent request sees the same free slot. The
//! ceilings are small — eight per session, thirty-two in total — so the window does not need to be
//! wide to be crossed by two tabs mounting at once.
//!
//! This module closes that window by making the decision and the claim the same operation. What
//! comes back is a lease, and the lease is the thing that occupies the slot: it is held from before
//! the first external side effect until cleanup is *confirmed*, which is why a Shell that is
//! closing, reaping, or failed to close still counts against the ceiling. Releasing it earlier
//! would let the application admit a new process while the old one is still running, which is the
//! overcommit this exists to prevent.

use super::session_shell::ShellCapacities;
use crate::contexts::workspaces::domain::{
    SessionShellError, ShellCapacityScope, ShellGeneration, ShellId,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// One reservation, as the ledger holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reservation {
    session_id: String,
}

#[derive(Default)]
struct CapacityLedger {
    /// Keyed by generation as well as id, so a reservation for a superseded attempt can never be
    /// released by, or confused with, the reservation for the attempt that replaced it.
    held: BTreeMap<(ShellId, u64), Reservation>,
    /// How many releases actually took effect. A counter rather than a flag because "exactly once"
    /// is the property under test, and a boolean cannot distinguish one release from three.
    releases: u64,
}

/// The single synchronization boundary for every Shell capacity limit.
pub(crate) struct ShellCapacityController {
    capacities: ShellCapacities,
    ledger: Mutex<CapacityLedger>,
}

impl ShellCapacityController {
    pub(crate) fn new(capacities: ShellCapacities) -> Self {
        Self {
            capacities,
            ledger: Mutex::new(CapacityLedger::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, CapacityLedger> {
        match self.ledger.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn capacities(&self) -> ShellCapacities {
        self.capacities
    }

    /// Reserves every applicable limit, or none of them.
    ///
    /// The application ceiling is checked before the per-session one so that a caller who is over
    /// both is told about the one they cannot act on: closing another Shell in this session would
    /// not help if the application as a whole is full.
    pub(crate) fn reserve(
        self: &Arc<Self>,
        session_id: &str,
        shell_id: &ShellId,
        generation: ShellGeneration,
    ) -> Result<ShellCapacityLease, SessionShellError> {
        let mut ledger = self.lock();
        if ledger.held.len() >= self.capacities.total {
            return Err(SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Application,
            });
        }
        let for_session = ledger
            .held
            .values()
            .filter(|reservation| reservation.session_id == session_id)
            .count();
        if for_session >= self.capacities.per_session {
            return Err(SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Session,
            });
        }
        ledger.held.insert(
            (shell_id.clone(), generation.value()),
            Reservation {
                session_id: session_id.to_string(),
            },
        );
        drop(ledger);
        Ok(ShellCapacityLease {
            controller: self.clone(),
            shell_id: shell_id.clone(),
            generation,
            released: AtomicBool::new(false),
        })
    }

    /// How many slots are occupied, counting Shells whose cleanup is still unconfirmed.
    pub(crate) fn active(&self) -> usize {
        self.lock().held.len()
    }

    pub(crate) fn active_for_session(&self, session_id: &str) -> usize {
        self.lock()
            .held
            .values()
            .filter(|reservation| reservation.session_id == session_id)
            .count()
    }

    /// How many releases took effect. Published so a test can assert exactly-once across the paths
    /// that all legitimately try to release: terminal close, startup rollback, reaper completion,
    /// a duplicate close, and a stale completion.
    pub(crate) fn releases(&self) -> u64 {
        self.lock().releases
    }

    fn release(&self, shell_id: &ShellId, generation: ShellGeneration) -> bool {
        let mut ledger = self.lock();
        if ledger
            .held
            .remove(&(shell_id.clone(), generation.value()))
            .is_none()
        {
            return false;
        }
        ledger.releases = ledger.releases.saturating_add(1);
        true
    }
}

/// One occupied slot, belonging to exactly one `(shell_id, generation)`.
///
/// Deliberately not `Clone`. A cloned lease would be two owners of one slot, and the second drop
/// would either double-release — admitting one Shell too many — or have to be made a silent no-op,
/// which is how a leak becomes invisible. Move it into the retained lifecycle record on commit;
/// drop it to roll a failed startup back.
pub(crate) struct ShellCapacityLease {
    controller: Arc<ShellCapacityController>,
    shell_id: ShellId,
    generation: ShellGeneration,
    released: AtomicBool,
}

impl ShellCapacityLease {
    pub(crate) fn shell_id(&self) -> &ShellId {
        &self.shell_id
    }

    pub(crate) fn generation(&self) -> ShellGeneration {
        self.generation
    }

    /// Gives the slot back. Idempotent, and reports whether this call was the one that did it.
    ///
    /// Idempotent because the release points are genuinely plural — a confirmed close, a reaper
    /// completion, and the drop that backstops both can all reach here for the same lease — and a
    /// release that panicked or double-counted on the second caller would turn a correct redundancy
    /// into a defect.
    pub(crate) fn release(&self) -> bool {
        if self.released.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.controller.release(&self.shell_id, self.generation)
    }
}

impl Drop for ShellCapacityLease {
    fn drop(&mut self) {
        self.release();
    }
}

impl std::fmt::Debug for ShellCapacityLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ShellCapacityLease")
            .field("shell_id", &self.shell_id.as_str())
            .field("generation", &self.generation.value())
            .field("released", &self.released.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(per_session: usize, total: usize) -> Arc<ShellCapacityController> {
        Arc::new(ShellCapacityController::new(ShellCapacities {
            per_session,
            total,
        }))
    }

    fn shell(id: &str) -> ShellId {
        ShellId::parse(id).expect("shell id")
    }

    #[test]
    fn a_reservation_that_hits_a_limit_takes_no_slot_at_all() {
        let controller = controller(1, 4);
        let _first = controller
            .reserve("session-1", &shell("shell-1"), ShellGeneration::new(1))
            .expect("first");

        let error = controller
            .reserve("session-1", &shell("shell-2"), ShellGeneration::new(2))
            .expect_err("per-session limit");

        assert!(matches!(
            error,
            SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Session
            }
        ));
        // Either all applicable limits are reserved or none are: a refused request that had already
        // taken the application slot would starve every other session.
        assert_eq!(controller.active(), 1);
        assert_eq!(controller.active_for_session("session-1"), 1);
    }

    /// A caller over both ceilings is told about the one they cannot act on. Closing another Shell
    /// in this session would not help when the application as a whole is full.
    #[test]
    fn the_application_ceiling_is_reported_ahead_of_the_session_one() {
        let controller = controller(1, 1);
        let _first = controller
            .reserve("session-1", &shell("shell-1"), ShellGeneration::new(1))
            .expect("first");

        let error = controller
            .reserve("session-1", &shell("shell-2"), ShellGeneration::new(2))
            .expect_err("full");

        assert!(matches!(
            error,
            SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Application
            }
        ));
    }

    /// The property the lease exists for. Every one of these paths is a legitimate release point
    /// for the same slot, and the ledger must count one.
    #[test]
    fn a_slot_is_released_exactly_once_however_many_owners_ask() {
        let controller = controller(4, 4);
        let lease = controller
            .reserve("session-1", &shell("shell-1"), ShellGeneration::new(1))
            .expect("reserve");

        assert!(lease.release(), "the first release takes effect");
        assert!(!lease.release(), "a duplicate release is a no-op");
        drop(lease);

        assert_eq!(controller.releases(), 1);
        assert_eq!(controller.active(), 0);
    }

    /// Dropping without an explicit release is the startup-rollback path: the launch guard falls
    /// out of scope and the slot has to come back with it.
    #[test]
    fn dropping_a_lease_returns_its_slot() {
        let controller = controller(1, 1);
        {
            let _lease = controller
                .reserve("session-1", &shell("shell-1"), ShellGeneration::new(1))
                .expect("reserve");
            assert_eq!(controller.active(), 1);
        }

        assert_eq!(controller.active(), 0);
        assert_eq!(controller.releases(), 1);
        controller
            .reserve("session-1", &shell("shell-2"), ShellGeneration::new(2))
            .expect("the slot is usable again");
    }

    /// A late completion for a superseded attempt must not free the slot of the attempt that
    /// replaced it.
    #[test]
    fn releasing_a_stale_generation_does_not_touch_the_current_one() {
        let controller = controller(2, 2);
        let old = controller
            .reserve("session-1", &shell("shell-1"), ShellGeneration::new(1))
            .expect("old");
        let current = controller
            .reserve("session-1", &shell("shell-1"), ShellGeneration::new(2))
            .expect("current");

        assert!(old.release());

        assert_eq!(controller.active(), 1);
        assert_eq!(current.generation(), ShellGeneration::new(2));
        assert_eq!(controller.active_for_session("session-1"), 1);
    }

    /// The last free slot under contention. A hundred threads racing for one slot is the shape of
    /// the defect — every one of them counted, every one of them saw room.
    #[test]
    fn a_hundred_concurrent_reservations_admit_exactly_the_configured_number() {
        let controller = controller(3, 3);
        let barrier = Arc::new(std::sync::Barrier::new(100));
        let granted = Arc::new(std::sync::Mutex::new(Vec::new()));

        let threads = (0..100)
            .map(|index| {
                let controller = controller.clone();
                let barrier = barrier.clone();
                let granted = granted.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    if let Ok(lease) = controller.reserve(
                        "session-1",
                        &shell(&format!("shell-{index}")),
                        ShellGeneration::new(index + 1),
                    ) {
                        // Held rather than dropped, so the ledger reflects admission rather than
                        // admission-and-immediate-return.
                        granted.lock().expect("granted").push(lease);
                    }
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().expect("join");
        }

        assert_eq!(granted.lock().expect("granted").len(), 3);
        assert_eq!(controller.active(), 3);
    }
}
