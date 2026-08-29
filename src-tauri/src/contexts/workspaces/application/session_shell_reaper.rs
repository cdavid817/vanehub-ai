//! The bounded queue of Shells whose cleanup outlived the command that asked for it.
//!
//! What this is *not* is the important part. It is not a task spawned per Shell: a thread per
//! stuck child is how a handful of unkillable processes becomes a thread pool nobody sized. It is
//! not fire-and-forget either — a detached task holding the only reference to a child is a leak
//! with a comment on it.
//!
//! It is a bounded queue of work identities, drained a fixed number at a time by whoever is already
//! running a periodic sweep. The handles themselves stay exactly where they were: the retained
//! runtime still owns the child, the PTY, and the workers, and the store entry still holds the
//! capacity lease. That is what makes a full queue safe to refuse — nothing was ever moved out of
//! an owner, so refusing the handoff drops nothing. The Shell stays `CloseFailed`, addressable, and
//! retryable by hand.

use super::evidence::WorkspaceShellCloseReason;
use crate::contexts::workspaces::domain::{ShellGeneration, ShellId};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Every ceiling the Reaper obeys.
///
/// All five are finite and none of them is a timeout on a thread. Queue depth bounds memory,
/// per-tick drain bounds how long one sweep costs, and the attempt count bounds how long a Shell
/// that will never die keeps being asked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellReaperLimits {
    /// How many Shells may be waiting at once. Sized against the Shell ceiling itself: every live
    /// Shell failing to close at the same time still fits, and nothing beyond that is reachable.
    pub(crate) queue_capacity: usize,
    /// How many attempts one drain may make. Bounds a sweep's cost, and is the concurrency limit:
    /// attempts run in the caller's bounded loop, never on threads of their own.
    pub(crate) max_active_per_drain: usize,
    /// How many automatic attempts a Shell gets before it is left `CloseFailed` for a person.
    pub(crate) max_attempts: u32,
    pub(crate) initial_backoff_millis: u64,
    pub(crate) max_backoff_millis: u64,
}

impl Default for ShellReaperLimits {
    fn default() -> Self {
        Self {
            queue_capacity: 32,
            max_active_per_drain: 4,
            max_attempts: 5,
            initial_backoff_millis: 250,
            max_backoff_millis: 8_000,
        }
    }
}

/// One Shell awaiting another cleanup attempt.
///
/// Identity and bookkeeping only. The item deliberately carries no handle: the runtime that failed
/// to close the Shell has not let go of it, so a handle here would be a second owner of the same
/// child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellReaperItem {
    pub(crate) shell_id: ShellId,
    pub(crate) generation: ShellGeneration,
    pub(crate) session_id: String,
    pub(crate) origin: WorkspaceShellCloseReason,
    /// How many bounded attempts have already been made, including the command-path one.
    pub(crate) attempts: u32,
    /// Monotonic milliseconds before which this item is not retried.
    pub(crate) due_at_millis: u64,
}

/// Why a handoff was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellReaperRejection {
    /// The queue is at its ceiling. Ownership stays with the caller.
    QueueFull,
    /// This `(shell_id, generation)` is already queued; the existing attempt continues.
    AlreadyQueued,
}

pub(crate) struct ShellReaperQueue {
    limits: ShellReaperLimits,
    pending: Mutex<VecDeque<ShellReaperItem>>,
}

impl ShellReaperQueue {
    pub(crate) fn new(limits: ShellReaperLimits) -> Self {
        Self {
            limits,
            pending: Mutex::new(VecDeque::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<ShellReaperItem>> {
        match self.pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn limits(&self) -> ShellReaperLimits {
        self.limits
    }

    /// How many Shells are waiting. Published because it is the metric that tells an operator the
    /// difference between "one shell would not die" and "cleanup has stopped working".
    pub(crate) fn depth(&self) -> usize {
        self.lock().len()
    }

    /// Takes ownership of the *continuation* of a close, not of the resources.
    ///
    /// A repeated offer for a generation already queued is accepted as a no-op rather than queued
    /// twice: two entries for one Shell would produce two competing attempts and, worse, two
    /// finalizations.
    pub(crate) fn offer(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        session_id: &str,
        origin: WorkspaceShellCloseReason,
        attempts: u32,
        now_millis: u64,
    ) -> Result<(), ShellReaperRejection> {
        let mut pending = self.lock();
        if pending
            .iter()
            .any(|item| &item.shell_id == shell_id && item.generation == generation)
        {
            return Err(ShellReaperRejection::AlreadyQueued);
        }
        if pending.len() >= self.limits.queue_capacity {
            return Err(ShellReaperRejection::QueueFull);
        }
        pending.push_back(ShellReaperItem {
            shell_id: shell_id.clone(),
            generation,
            session_id: session_id.to_string(),
            origin,
            attempts,
            due_at_millis: now_millis.saturating_add(self.backoff_millis(attempts)),
        });
        Ok(())
    }

    /// The items whose backoff has elapsed, at most `max_active_per_drain` of them.
    ///
    /// Removed from the queue as they are handed out, so a drain that is still in progress cannot
    /// hand the same Shell to a second caller. Whatever the caller does not finish it returns
    /// through `requeue`.
    pub(crate) fn drain_due(&self, now_millis: u64) -> Vec<ShellReaperItem> {
        let mut pending = self.lock();
        let mut due = Vec::new();
        let mut skipped = VecDeque::new();
        while let Some(item) = pending.pop_front() {
            if due.len() >= self.limits.max_active_per_drain || item.due_at_millis > now_millis {
                skipped.push_back(item);
                continue;
            }
            due.push(item);
        }
        *pending = skipped;
        due
    }

    /// Returns an unfinished item for another attempt, unless it has used them all up.
    ///
    /// Answers whether it will be retried. `false` is not a failure to record — it is the moment a
    /// Shell stops being retried automatically and starts waiting for a person, and the caller has
    /// to mark it as such rather than forget it.
    pub(crate) fn requeue(&self, item: ShellReaperItem, now_millis: u64) -> bool {
        let attempts = item.attempts.saturating_add(1);
        if attempts >= self.limits.max_attempts {
            return false;
        }
        let mut pending = self.lock();
        if pending.len() >= self.limits.queue_capacity {
            return false;
        }
        pending.push_back(ShellReaperItem {
            attempts,
            due_at_millis: now_millis.saturating_add(self.backoff_millis(attempts)),
            ..item
        });
        true
    }

    /// Forgets every entry for a Shell id, whatever its generation.
    ///
    /// Used when a generation has been finalized: leaving its entry behind would spend attempts on
    /// a Shell that no longer exists and, once the id is looked up again, report a stale result
    /// against whatever answers to it.
    pub(crate) fn forget(&self, shell_id: &ShellId) {
        self.lock().retain(|item| &item.shell_id != shell_id);
    }

    /// Exponential, capped, and derived from the attempt count rather than stored per item, so the
    /// schedule cannot drift between the offer path and the requeue path.
    fn backoff_millis(&self, attempts: u32) -> u64 {
        let steps = attempts.saturating_sub(1).min(16);
        self.limits
            .initial_backoff_millis
            .saturating_mul(1u64 << steps)
            .min(self.limits.max_backoff_millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(id: &str) -> ShellId {
        ShellId::parse(id).expect("shell id")
    }

    fn queue(limits: ShellReaperLimits) -> ShellReaperQueue {
        ShellReaperQueue::new(limits)
    }

    fn offer(queue: &ShellReaperQueue, id: &str, now: u64) -> Result<(), ShellReaperRejection> {
        queue.offer(
            &shell(id),
            ShellGeneration::new(1),
            "session-1",
            WorkspaceShellCloseReason::ExplicitClose,
            1,
            now,
        )
    }

    /// A full queue refuses rather than growing. Nothing was moved out of an owner to offer it, so
    /// the refusal drops nothing — which is what makes refusing safe.
    #[test]
    fn a_full_queue_refuses_the_handoff_instead_of_accepting_it() {
        let queue = queue(ShellReaperLimits {
            queue_capacity: 2,
            ..ShellReaperLimits::default()
        });
        offer(&queue, "shell-1", 0).expect("first");
        offer(&queue, "shell-2", 0).expect("second");

        assert_eq!(
            offer(&queue, "shell-3", 0),
            Err(ShellReaperRejection::QueueFull)
        );
        assert_eq!(queue.depth(), 2);
    }

    /// Two entries for one Shell would produce two competing attempts and two finalizations.
    #[test]
    fn offering_the_same_generation_twice_continues_the_existing_attempt() {
        let queue = queue(ShellReaperLimits::default());
        offer(&queue, "shell-1", 0).expect("first");

        assert_eq!(
            offer(&queue, "shell-1", 0),
            Err(ShellReaperRejection::AlreadyQueued)
        );
        assert_eq!(queue.depth(), 1);
    }

    /// A newer generation of the same id is a different Shell life, and queuing it is correct.
    #[test]
    fn a_newer_generation_of_the_same_id_is_its_own_work_item() {
        let queue = queue(ShellReaperLimits::default());
        offer(&queue, "shell-1", 0).expect("first");

        queue
            .offer(
                &shell("shell-1"),
                ShellGeneration::new(2),
                "session-1",
                WorkspaceShellCloseReason::ExplicitClose,
                1,
                0,
            )
            .expect("second generation");

        assert_eq!(queue.depth(), 2);
    }

    /// Virtual time throughout: the schedule is asserted by advancing a number, never by sleeping
    /// for a production backoff.
    #[test]
    fn an_item_is_not_retried_before_its_backoff_elapses() {
        let queue = queue(ShellReaperLimits {
            initial_backoff_millis: 100,
            ..ShellReaperLimits::default()
        });
        offer(&queue, "shell-1", 1_000).expect("offer");

        assert!(queue.drain_due(1_050).is_empty());
        assert_eq!(queue.drain_due(1_100).len(), 1);
    }

    #[test]
    fn a_drain_is_bounded_and_leaves_the_rest_queued() {
        let queue = queue(ShellReaperLimits {
            max_active_per_drain: 2,
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        });
        for index in 0..5 {
            offer(&queue, &format!("shell-{index}"), 0).expect("offer");
        }

        let drained = queue.drain_due(0);

        assert_eq!(drained.len(), 2);
        assert_eq!(queue.depth(), 3);
        // Order is preserved across a bounded drain: the two that were skipped are still ahead of
        // the three that follow them.
        assert_eq!(drained[0].shell_id.as_str(), "shell-0");
    }

    /// The moment automatic retry stops. Reported rather than silent, because the caller has to
    /// mark the Shell as needing a person instead of forgetting it.
    #[test]
    fn attempts_are_exhausted_rather_than_retried_forever() {
        let queue = queue(ShellReaperLimits {
            max_attempts: 3,
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        });
        offer(&queue, "shell-1", 0).expect("offer");

        let mut item = queue.drain_due(0).pop().expect("due");
        assert_eq!(item.attempts, 1);
        assert!(queue.requeue(item.clone(), 0), "second attempt");
        item = queue.drain_due(0).pop().expect("due again");
        assert_eq!(item.attempts, 2);

        assert!(!queue.requeue(item, 0), "the third exhausts the budget");
        assert_eq!(queue.depth(), 0);
    }

    /// Backoff grows and then stops growing. An uncapped schedule would eventually stop retrying in
    /// practice while still claiming it would.
    #[test]
    fn backoff_is_exponential_and_capped() {
        let queue = queue(ShellReaperLimits {
            initial_backoff_millis: 100,
            max_backoff_millis: 400,
            ..ShellReaperLimits::default()
        });

        assert_eq!(queue.backoff_millis(1), 100);
        assert_eq!(queue.backoff_millis(2), 200);
        assert_eq!(queue.backoff_millis(3), 400);
        assert_eq!(queue.backoff_millis(9), 400);
    }

    /// Leaving a finalized generation queued would spend attempts on a Shell that no longer exists.
    #[test]
    fn forgetting_a_shell_removes_every_generation_of_it() {
        let queue = queue(ShellReaperLimits {
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        });
        offer(&queue, "shell-1", 0).expect("first");
        queue
            .offer(
                &shell("shell-1"),
                ShellGeneration::new(2),
                "session-1",
                WorkspaceShellCloseReason::ExplicitClose,
                1,
                0,
            )
            .expect("second");
        offer(&queue, "shell-2", 0).expect("other");

        queue.forget(&shell("shell-1"));

        assert_eq!(queue.depth(), 1);
        assert_eq!(queue.drain_due(0)[0].shell_id.as_str(), "shell-2");
    }
}
