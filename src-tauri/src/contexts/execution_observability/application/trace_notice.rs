//! What a subscriber is told when a run or span changes state.
//!
//! Identifiers and a status, never a name and never an attribute. A Traces tab that received the
//! span itself would have two shapes for the same span — the one on the notice and the one the
//! timeline query returns — and the two can disagree about anything a later write changed. So the
//! notice says *which* run changed and *how*, and the view refetches the timeline it already knows
//! how to ask for.
//!
//! Published after the local store has committed, never before. A notice that arrived first would
//! send a subscriber to fetch a timeline that does not yet contain the change it was told about,
//! and the refetch would return the old state — which reads as "the notice was wrong" rather than
//! as "the notice was early".

use super::super::domain::ExecutionStatus;

/// Which transition happened.
///
/// Run and span are separate because a subscriber does different things with them: a run
/// transition changes the run list, and a span transition changes at most the timeline currently
/// open. Collapsing them would make every span in a busy run invalidate the list as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraceTransitionKind {
    RunStarted,
    RunFinished,
    SpanStarted,
    SpanFinished,
}

impl TraceTransitionKind {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::RunStarted => "run-started",
            Self::RunFinished => "run-finished",
            Self::SpanStarted => "span-started",
            Self::SpanFinished => "span-finished",
        }
    }

    /// Whether this transition changes the run list rather than one open timeline.
    pub(crate) fn affects_run_list(self) -> bool {
        matches!(self, Self::RunStarted | Self::RunFinished)
    }
}

/// One transition, as identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceTransitionNotice {
    pub(crate) kind: TraceTransitionKind,
    pub(crate) run_id: String,
    /// Always present, so a client never has to branch on whether it was included. A notice whose
    /// correlation is sometimes there is one every consumer has to handle twice.
    pub(crate) trace_id: String,
    /// The span that changed, for a span transition. Absent for a run transition.
    pub(crate) span_id: Option<String>,
    pub(crate) status: ExecutionStatus,
    /// When the transition happened, for a finish. Absent for a start, which happens now.
    pub(crate) occurred_at: Option<String>,
}

/// Where a committed transition is announced.
///
/// Failing to publish is not a reason to fail the write. The transition is already durable and the
/// subscriber's next query will find it; failing the commit to keep a notification honest would
/// lose the record that the notification was about.
pub(crate) trait TraceTransitionPublisherPort: Send + Sync {
    fn publish(&self, notice: &TraceTransitionNotice);
}
