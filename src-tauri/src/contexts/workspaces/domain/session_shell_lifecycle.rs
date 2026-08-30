//! The generation token, the bounded close budget, and the stable reason vocabulary a retained
//! Shell's lifecycle is written in.
//!
//! These three live together because they answer the same question from different sides: *which*
//! attempt at a Shell a fact belongs to, *how long* an attempt to end it may take, and *what* to
//! call the outcome. A late worker completion carrying no generation, a close with no ceiling, and
//! a failure reported as free text are the three ways ownership of a live process gets lost.

use std::time::Duration;

/// Which attempt at a Shell identity a fact belongs to.
///
/// Shell ids are opaque and never reused, so this is not needed to tell two ids apart. It is needed
/// to tell two *lives* of the same lookup apart: a reader thread, a reaper attempt, and a route
/// entry can all outlive the Shell they were created for, and a completion that arrives without one
/// cannot be distinguished from a completion that is still current. Comparing the generation is
/// what makes a late arrival a no-op instead of a close of somebody else's Shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ShellGeneration(u64);

impl ShellGeneration {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }

    /// Saturating rather than wrapping. A wrapped generation would compare equal to one that is
    /// 2^64 attempts old, which is exactly the comparison this type exists to make trustworthy.
    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// How long each stage of ending a Shell may take before ownership moves on.
///
/// Five finite windows rather than one, because the stages fail differently: a child that has
/// already exited answers the first observation immediately, a child ignoring a terminate needs the
/// force stage, and a reader thread blocked inside a driver read may never complete at all. One
/// combined number would make the common case pay the worst case's wait, and the worst case would
/// still have no ceiling of its own.
///
/// `total` is the command path's promise. Every stage sum is bounded by it, so a caller can state
/// what a close costs without reading the stage list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellCloseBudget {
    /// After input is stopped, before anything is signalled — a shell that was already finishing.
    pub(crate) graceful: Duration,
    /// After a termination request.
    pub(crate) terminate: Duration,
    /// After a forceful kill, waiting for the operating system to reap.
    pub(crate) force: Duration,
    /// For workers to report themselves complete. Never a blocking join.
    pub(crate) worker: Duration,
    /// The whole command path, including every stage above.
    pub(crate) total: Duration,
    /// How often an observation is retried inside a stage.
    pub(crate) poll: Duration,
}

impl Default for ShellCloseBudget {
    fn default() -> Self {
        Self {
            graceful: Duration::from_millis(150),
            terminate: Duration::from_millis(600),
            force: Duration::from_millis(600),
            worker: Duration::from_millis(250),
            total: Duration::from_millis(1_800),
            poll: Duration::from_millis(10),
        }
    }
}

impl ShellCloseBudget {
    /// A budget every stage of which is one poll interval, for tests that count observations rather
    /// than measure durations.
    #[cfg(test)]
    pub(crate) fn immediate() -> Self {
        Self {
            graceful: Duration::from_millis(1),
            terminate: Duration::from_millis(1),
            force: Duration::from_millis(1),
            worker: Duration::from_millis(1),
            total: Duration::from_millis(8),
            poll: Duration::from_millis(1),
        }
    }

    /// Whether the budget can bound anything at all. A zero total would make every stage report a
    /// deadline it had already passed on entry, which reads as "the child would not die" for a child
    /// nobody ever asked to.
    pub(crate) fn is_finite_and_positive(&self) -> bool {
        !self.total.is_zero() && !self.poll.is_zero()
    }
}

/// The stable, non-sensitive codes a lifecycle failure is reported as.
///
/// Written once here rather than spelled at each site: these cross the command boundary, the
/// frontend switches on them, and a code that differed by one character between the local and the
/// remote adapter would be a state the UI silently cannot render. Nothing here names a path, a host,
/// a command, or an operating-system message.
pub(crate) mod shell_reason_code {
    /// Every applicable capacity limit could not be reserved together.
    pub(crate) const CAPACITY_EXHAUSTED: &str = "shell_capacity_exhausted";
    /// Startup acquired something and then failed before commit.
    pub(crate) const OPEN_SETUP_FAILED: &str = "shell_open_setup_failed";
    /// Startup rollback could not confirm cleanup inside the startup budget.
    pub(crate) const STARTUP_CLEANUP_PENDING: &str = "shell_startup_cleanup_pending";
    /// The command path's close budget ran out with the resource still unconfirmed.
    pub(crate) const CLOSE_DEADLINE_REACHED: &str = "shell_close_deadline_reached";
    /// A termination primitive returned an error rather than a refusal to die.
    pub(crate) const TERMINATE_FAILED: &str = "shell_terminate_failed";
    /// A worker had not reported itself complete when its window closed.
    pub(crate) const WORKER_COMPLETION_PENDING: &str = "shell_worker_completion_pending";
    /// The platform stopped being able to say whether the child is still alive.
    pub(crate) const REAP_DEADLINE_REACHED: &str = "shell_reap_deadline_reached";
    /// The Reaper is full, so ownership stayed where it was.
    pub(crate) const REAPER_CAPACITY_EXHAUSTED: &str = "shell_reaper_capacity_exhausted";
    /// A completion named a generation that is no longer current.
    pub(crate) const GENERATION_STALE: &str = "shell_generation_stale";
    /// A session could not be finalized because one of its Shells is unconfirmed.
    pub(crate) const SESSION_CLEANUP_INCOMPLETE: &str = "session_shell_cleanup_incomplete";
    /// Output arrived during `Opening` faster than the bounded startup gate could hold.
    pub(crate) const STARTUP_BUFFER_OVERFLOW: &str = "shell_startup_buffer_overflow";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generation_advances_and_never_wraps_back_onto_an_old_one() {
        let first = ShellGeneration::new(1);

        assert_eq!(first.next().value(), 2);
        // A wrapped generation would compare equal to one that is 2^64 attempts old, which is the
        // one comparison this type exists to make trustworthy.
        assert_eq!(ShellGeneration::new(u64::MAX).next().value(), u64::MAX);
        assert!(first < first.next());
    }

    #[test]
    fn the_default_close_budget_bounds_the_command_path() {
        let budget = ShellCloseBudget::default();

        assert!(budget.is_finite_and_positive());
        // Every stage has to fit inside the promise the command path makes, or the promise is not
        // one: a caller stating "close returns within total" would be wrong on the slowest path.
        let stages = budget.graceful + budget.terminate + budget.force + budget.worker;
        assert!(
            stages <= budget.total,
            "{stages:?} exceeds {:?}",
            budget.total
        );
    }
}
