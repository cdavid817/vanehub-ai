//! How many inspections may be happening at once, and what happens to the rest.
//!
//! A result cap and a work budget both bound one request. Neither bounds ten windows each starting
//! a search on every keystroke: without admission, each of those reaches `spawn_blocking` and the
//! blocking pool fills with walks whose answers nobody is waiting for any more. The pool has no
//! opinion about that — it schedules what it is given — so the opinion has to live here, before the
//! task exists.
//!
//! Two ceilings rather than one. A global ceiling keeps the process from starving unrelated work; a
//! per-workspace ceiling keeps one busy project from consuming the whole global allowance while a
//! second workspace waits behind it.
//!
//! A permit is held by the *worker*, not by the caller. If the caller's future is aborted the work
//! is still running on a blocking thread until it observes its token, and releasing the permit at
//! abort would let the next request start while the previous one is still holding a thread — which
//! is precisely the accumulation the ceiling exists to prevent.

use super::inspection_budget::WorkspaceInspectionReason;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// What the process will run at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkspaceInspectionAdmissionLimits {
    pub(crate) global_active: usize,
    pub(crate) per_workspace_active: usize,
    /// How long a request will wait for a permit before it is told the system is busy.
    ///
    /// Finite, and short. A queue with no deadline is an unbounded queue wearing a different name:
    /// the requests in it are still allocated, still holding their arguments, and still going to
    /// run long after the reader who asked has typed something else.
    pub(crate) wait: Duration,
}

impl Default for WorkspaceInspectionAdmissionLimits {
    /// Four concurrent inspections, two per workspace.
    ///
    /// Chosen from the shape of the work rather than from core count: these are I/O-bound walks on
    /// the blocking pool, and a fifth concurrent walk on one disk finishes no sooner than the first
    /// four would have. Two per workspace covers the real overlap — a directory listing while a
    /// content search runs — without letting one project hold the whole global allowance.
    fn default() -> Self {
        Self {
            global_active: 4,
            per_workspace_active: 2,
            wait: Duration::from_millis(750),
        }
    }
}

/// The right to be running. Dropped when the worker exits, and not before.
pub(crate) struct InspectionPermit {
    _workspace: OwnedSemaphorePermit,
    _global: OwnedSemaphorePermit,
}

/// Global and per-workspace ceilings on active inspection work.
pub(crate) struct WorkspaceInspectionAdmission {
    limits: WorkspaceInspectionAdmissionLimits,
    global: Arc<Semaphore>,
    /// One semaphore per workspace, created on first use.
    ///
    /// Never pruned while the process runs. A semaphore is two words and the number of workspaces a
    /// session touches is bounded by the number a person opens; reclaiming them would need a
    /// lifetime rule whose only failure mode is dropping one that is in use.
    per_workspace: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl Default for WorkspaceInspectionAdmission {
    fn default() -> Self {
        Self::new(WorkspaceInspectionAdmissionLimits::default())
    }
}

impl WorkspaceInspectionAdmission {
    pub(crate) fn new(limits: WorkspaceInspectionAdmissionLimits) -> Self {
        Self {
            global: Arc::new(Semaphore::new(limits.global_active.max(1))),
            per_workspace: Mutex::new(HashMap::new()),
            limits,
        }
    }

    pub(crate) fn limits(&self) -> WorkspaceInspectionAdmissionLimits {
        self.limits
    }

    fn workspace_semaphore(&self, workspace_key: &str) -> Arc<Semaphore> {
        let mut per_workspace = self
            .per_workspace
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(
            per_workspace
                .entry(workspace_key.to_string())
                .or_insert_with(|| {
                    Arc::new(Semaphore::new(self.limits.per_workspace_active.max(1)))
                }),
        )
    }

    /// A permit, or a typed refusal.
    ///
    /// Acquired before `spawn_blocking` or a remote launch, never after: a task that exists is a
    /// task the pool will run, and refusing it afterwards has already paid for it.
    ///
    /// The workspace permit is taken first. Taking the global one first would let a request that is
    /// going to be refused by its workspace ceiling occupy a global slot for the whole wait, which
    /// turns a per-workspace limit into a global one.
    pub(crate) async fn acquire(
        &self,
        workspace_key: &str,
    ) -> Result<InspectionPermit, WorkspaceInspectionReason> {
        let workspace = self.workspace_semaphore(workspace_key);
        let workspace_permit = tokio::time::timeout(self.limits.wait, workspace.acquire_owned())
            .await
            .map_err(|_| WorkspaceInspectionReason::InspectionBusy)?
            .map_err(|_| WorkspaceInspectionReason::ProviderUnavailable)?;
        let global_permit =
            tokio::time::timeout(self.limits.wait, Arc::clone(&self.global).acquire_owned())
                .await
                .map_err(|_| WorkspaceInspectionReason::InspectionBusy)?
                .map_err(|_| WorkspaceInspectionReason::ProviderUnavailable)?;

        Ok(InspectionPermit {
            _workspace: workspace_permit,
            _global: global_permit,
        })
    }

    /// How many global slots are free. A structural assertion for tests, not a product signal.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "occupancy is asserted by the admission tests; production only acquires"
        )
    )]
    pub(crate) fn available_global(&self) -> usize {
        self.global.available_permits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admission(global: usize, per_workspace: usize) -> WorkspaceInspectionAdmission {
        WorkspaceInspectionAdmission::new(WorkspaceInspectionAdmissionLimits {
            global_active: global,
            per_workspace_active: per_workspace,
            wait: Duration::from_millis(50),
        })
    }

    #[tokio::test]
    async fn a_workspace_beyond_its_ceiling_is_told_it_is_busy() {
        let admission = admission(8, 1);

        let held = admission
            .acquire("workspace-a")
            .await
            .expect("first permit");

        // Not an unbounded queue and not a silent `spawn_blocking`: a typed refusal, so the caller
        // returns `Unavailable/inspection_busy` rather than starting hidden work.
        assert_eq!(
            admission.acquire("workspace-a").await.err(),
            Some(WorkspaceInspectionReason::InspectionBusy)
        );

        drop(held);
        assert!(admission.acquire("workspace-a").await.is_ok());
    }

    #[tokio::test]
    async fn one_busy_workspace_does_not_block_another() {
        let admission = admission(4, 1);

        let _held = admission.acquire("workspace-a").await.expect("permit");

        // The whole reason there are two ceilings: a project mid-walk must not make a second
        // project's file tree unopenable.
        assert!(admission.acquire("workspace-b").await.is_ok());
    }

    #[tokio::test]
    async fn the_global_ceiling_bounds_every_workspace_together() {
        let admission = admission(2, 4);

        let _first = admission.acquire("workspace-a").await.expect("first");
        let _second = admission.acquire("workspace-b").await.expect("second");

        assert_eq!(admission.available_global(), 0);
        assert_eq!(
            admission.acquire("workspace-c").await.err(),
            Some(WorkspaceInspectionReason::InspectionBusy)
        );
    }

    #[tokio::test]
    async fn a_permit_is_released_only_when_its_holder_is_dropped() {
        let admission = admission(1, 1);

        let permit = admission.acquire("workspace-a").await.expect("permit");
        assert_eq!(admission.available_global(), 0);

        // The permit belongs to the worker, so a caller that walked away changes nothing until the
        // work itself exits. Releasing at abort would let the next request start while a blocking
        // thread is still held, which is exactly the accumulation the ceiling prevents.
        drop(permit);
        assert_eq!(admission.available_global(), 1);
    }

    #[tokio::test]
    async fn a_refused_workspace_permit_never_occupies_a_global_slot() {
        let admission = admission(2, 1);

        let _held = admission.acquire("workspace-a").await.expect("permit");
        assert_eq!(admission.available_global(), 1);

        assert!(admission.acquire("workspace-a").await.is_err());

        // Taking the global permit first would have parked this request in a global slot for the
        // whole wait, turning a per-workspace limit into a global one.
        assert_eq!(admission.available_global(), 1);
    }

    #[tokio::test]
    async fn repeated_requests_never_exceed_the_configured_ceiling() {
        let admission = admission(3, 3);
        let mut permits = Vec::new();

        for _ in 0..3 {
            permits.push(admission.acquire("workspace-a").await.expect("permit"));
        }
        for _ in 0..5 {
            assert_eq!(
                admission.acquire("workspace-a").await.err(),
                Some(WorkspaceInspectionReason::InspectionBusy)
            );
        }

        assert_eq!(permits.len(), 3);
        assert_eq!(admission.available_global(), 0);
    }

    #[test]
    fn the_default_policy_is_finite_in_every_dimension() {
        let limits = WorkspaceInspectionAdmissionLimits::default();

        assert!(limits.global_active > 0);
        assert!(limits.per_workspace_active > 0);
        assert!(limits.per_workspace_active <= limits.global_active);
        // A queue with no deadline is an unbounded queue wearing a different name.
        assert!(limits.wait > Duration::ZERO);
    }
}
