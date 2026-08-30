use super::CuratorCandidateState;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratorTransition {
    IntakeValidatedWithoutDraft,
    DraftBecameReady,
    DraftChanged,
    Defer,
    ResumeWithoutReadyDraft,
    ResumeWithReadyDraft,
    Reject,
    Approve,
    ApplySucceeded,
    ApplyFailed,
    RetryPrepared,
    Supersede,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorTransitionError {
    #[error("candidate state is terminal")]
    TerminalState,
    #[error("candidate transition is not allowed")]
    InvalidTransition,
}

pub(crate) fn transition_candidate(
    current: CuratorCandidateState,
    transition: CuratorTransition,
) -> Result<CuratorCandidateState, CuratorTransitionError> {
    use CuratorCandidateState as State;
    use CuratorTransition as Action;

    if is_terminal(current) {
        return Err(CuratorTransitionError::TerminalState);
    }
    match (current, transition) {
        (State::Pending, Action::IntakeValidatedWithoutDraft) => Ok(State::AwaitingDraft),
        (State::AwaitingDraft, Action::DraftBecameReady) => Ok(State::ReadyForReview),
        (State::ReadyForReview, Action::DraftChanged) => Ok(State::AwaitingDraft),
        (State::AwaitingDraft | State::ReadyForReview, Action::Defer) => Ok(State::Deferred),
        (State::Deferred, Action::ResumeWithoutReadyDraft) => Ok(State::AwaitingDraft),
        (State::Deferred, Action::ResumeWithReadyDraft) => Ok(State::ReadyForReview),
        (State::AwaitingDraft | State::ReadyForReview | State::Deferred, Action::Reject) => {
            Ok(State::Rejected)
        }
        (State::ReadyForReview, Action::Approve) => Ok(State::Applying),
        (State::Applying, Action::ApplySucceeded) => Ok(State::Applied),
        (State::Applying, Action::ApplyFailed) => Ok(State::ApplyFailed),
        (State::ApplyFailed, Action::RetryPrepared) => Ok(State::ReadyForReview),
        (
            State::Pending
            | State::AwaitingDraft
            | State::ReadyForReview
            | State::Deferred
            | State::ApplyFailed,
            Action::Supersede,
        ) => Ok(State::Superseded),
        _ => Err(CuratorTransitionError::InvalidTransition),
    }
}

pub(crate) fn is_terminal(state: CuratorCandidateState) -> bool {
    matches!(
        state,
        CuratorCandidateState::Applied
            | CuratorCandidateState::Rejected
            | CuratorCandidateState::Superseded
    )
}
