use super::CuratorRepositoryError;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::Transaction;

pub(super) fn current_candidate(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<(CuratorCandidateState, u64), CuratorRepositoryError> {
    transaction
        .query_row(
            "SELECT state,revision FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|_| CuratorRepositoryError::NotFound)
        .and_then(|(state, revision)| Ok((parse_state(&state)?, from_sql_u64(revision)?)))
}

pub(super) fn sql_u64(value: u64) -> Result<i64, CuratorRepositoryError> {
    i64::try_from(value).map_err(|_| CuratorRepositoryError::InvalidInput)
}

pub(super) fn from_sql_u64(value: i64) -> Result<u64, CuratorRepositoryError> {
    u64::try_from(value).map_err(|_| CuratorRepositoryError::Storage)
}

pub(super) fn validate_snapshot(
    snapshot: &CuratorCandidateSnapshot,
) -> Result<(), CuratorRepositoryError> {
    if snapshot.schema_version != CURATOR_SCHEMA_VERSION_V1
        || snapshot.candidate_id.trim().is_empty()
        || snapshot.assessment_attempt_id.trim().is_empty()
        || snapshot.revision == 0
        || snapshot.updated_at_ms < snapshot.created_at_ms
    {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    Ok(())
}

pub(super) fn state_name(value: CuratorCandidateState) -> &'static str {
    match value {
        CuratorCandidateState::Pending => "pending",
        CuratorCandidateState::AwaitingDraft => "awaiting_draft",
        CuratorCandidateState::ReadyForReview => "ready_for_review",
        CuratorCandidateState::Deferred => "deferred",
        CuratorCandidateState::Rejected => "rejected",
        CuratorCandidateState::Applying => "applying",
        CuratorCandidateState::Applied => "applied",
        CuratorCandidateState::ApplyFailed => "apply_failed",
        CuratorCandidateState::Superseded => "superseded",
    }
}

pub(super) fn parse_state(value: &str) -> Result<CuratorCandidateState, CuratorRepositoryError> {
    match value {
        "pending" => Ok(CuratorCandidateState::Pending),
        "awaiting_draft" => Ok(CuratorCandidateState::AwaitingDraft),
        "ready_for_review" => Ok(CuratorCandidateState::ReadyForReview),
        "deferred" => Ok(CuratorCandidateState::Deferred),
        "rejected" => Ok(CuratorCandidateState::Rejected),
        "applying" => Ok(CuratorCandidateState::Applying),
        "applied" => Ok(CuratorCandidateState::Applied),
        "apply_failed" => Ok(CuratorCandidateState::ApplyFailed),
        "superseded" => Ok(CuratorCandidateState::Superseded),
        _ => Err(CuratorRepositoryError::Storage),
    }
}

pub(super) fn route_name(value: CuratorRoute) -> &'static str {
    match value {
        CuratorRoute::Advance => "advance",
        CuratorRoute::NeedsHumanReview => "needs_human_review",
    }
}
pub(super) fn risk_name(value: CuratorRisk) -> &'static str {
    match value {
        CuratorRisk::Low => "low",
        CuratorRisk::Medium => "medium",
        CuratorRisk::High => "high",
    }
}
pub(super) fn confidence_name(value: CuratorConfidence) -> &'static str {
    match value {
        CuratorConfidence::Low => "low",
        CuratorConfidence::Medium => "medium",
        CuratorConfidence::High => "high",
    }
}
pub(super) fn actor_name(value: CuratorActorClass) -> &'static str {
    match value {
        CuratorActorClass::LocalInteractiveUser => "local_interactive_user",
        CuratorActorClass::System => "system",
        CuratorActorClass::WebMockInteractiveUser => "web_mock_interactive_user",
    }
}
pub(super) fn decision_name(value: CuratorDecisionKind) -> &'static str {
    match value {
        CuratorDecisionKind::Approve => "approve",
        CuratorDecisionKind::Reject => "reject",
        CuratorDecisionKind::Defer => "defer",
        CuratorDecisionKind::Resume => "resume",
    }
}
pub(super) fn draft_kind_name(value: CuratorDraftKind) -> &'static str {
    match value {
        CuratorDraftKind::LearnBlock => "learn_block",
        CuratorDraftKind::ExactPatch => "exact_patch",
    }
}
pub(super) fn event_name(value: CuratorEventKind) -> &'static str {
    match value {
        CuratorEventKind::Intake => "intake",
        CuratorEventKind::DraftRejected => "draft_rejected",
        CuratorEventKind::DraftRevised => "draft_revised",
        CuratorEventKind::DraftAssessed => "draft_assessed",
        CuratorEventKind::Previewed => "previewed",
        CuratorEventKind::PreviewInvalidated => "preview_invalidated",
        CuratorEventKind::Deferred => "deferred",
        CuratorEventKind::Resumed => "resumed",
        CuratorEventKind::Rejected => "rejected",
        CuratorEventKind::Approved => "approved",
        CuratorEventKind::ApplicationStarted => "application_started",
        CuratorEventKind::Applied => "applied",
        CuratorEventKind::ApplicationFailed => "application_failed",
        CuratorEventKind::Superseded => "superseded",
        CuratorEventKind::PolicyChanged => "policy_changed",
        CuratorEventKind::ContentPurged => "content_purged",
    }
}
