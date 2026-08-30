use super::repository_support::{actor_name, decision_name, sql_u64, state_name};
use super::{
    append_audit_event, append_system_event, CandidateTransitionRequest, SystemAuditEvent,
    TrustedAuditContext,
};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::params;

pub(super) fn validate_intent(
    intent: &CuratorApplicationIntent,
) -> Result<(), CuratorApplicationStoreError> {
    if intent.application_id.trim().is_empty()
        || intent.outbox_id.trim().is_empty()
        || intent.idempotency_key.trim().is_empty()
        || intent.decision.kind != CuratorDecisionKind::Approve
        || intent.decision.preview_hash.as_deref() != Some(&intent.approved_witness_hash)
        || intent.approved_diff_hash.trim().is_empty()
        || intent.expected_effective_hash.trim().is_empty()
        || intent.expected_state != CuratorCandidateState::ReadyForReview
    {
        return Err(CuratorApplicationStoreError::InvalidInput);
    }
    match (
        intent.decision.actor_class,
        intent.system_policy_authorization.as_ref(),
    ) {
        (CuratorActorClass::System, Some(authorization))
            if intent.decision.reason_code == "system_policy_authorized" =>
        {
            validate_system_authorization(authorization)?;
        }
        (
            CuratorActorClass::LocalInteractiveUser | CuratorActorClass::WebMockInteractiveUser,
            None,
        ) => {}
        _ => return Err(CuratorApplicationStoreError::InvalidInput),
    }
    Ok(())
}

fn validate_system_authorization(
    value: &CuratorSystemPolicyAuthorizationV1,
) -> Result<(), CuratorApplicationStoreError> {
    let identifiers = [
        value.run_id.as_str(),
        value.eligibility_id.as_str(),
        value.rate_reservation_id.as_str(),
    ];
    let hashes = [
        value.eligibility_proof_hash.as_str(),
        value.preflight_witness_hash.as_str(),
        value.policy_witness_hash.as_str(),
    ];
    if value.authorized_at_ms < 0
        || identifiers
            .iter()
            .any(|value| value.is_empty() || value.len() > 256)
        || hashes
            .iter()
            .any(|value| value.is_empty() || value.len() > 256)
    {
        return Err(CuratorApplicationStoreError::InvalidInput);
    }
    Ok(())
}

pub(super) fn insert_system_policy_authorization(
    transaction: &rusqlite::Transaction<'_>,
    intent: &CuratorApplicationIntent,
) -> Result<(), CuratorApplicationStoreError> {
    let Some(value) = &intent.system_policy_authorization else {
        return Ok(());
    };
    transaction
        .execute(
            "INSERT INTO evolution_curator_system_policy_authorizations VALUES
             (?1,?2,?3,?4,?5,?6,?7,'system_policy',?8)",
            params![
                intent.application_id,
                value.run_id,
                value.eligibility_id,
                value.eligibility_proof_hash,
                value.preflight_witness_hash,
                value.policy_witness_hash,
                value.rate_reservation_id,
                value.authorized_at_ms,
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    Ok(())
}

pub(super) fn validate_binding(
    intent: &CuratorApplicationIntent,
    binding: &CuratorApplicationBinding,
) -> Result<(), CuratorApplicationStoreError> {
    let preview = binding
        .decision
        .current_preview
        .as_ref()
        .ok_or(CuratorApplicationStoreError::Conflict)?;
    if binding.decision.state != intent.expected_state
        || binding.decision.candidate_revision != intent.decision.candidate_revision
        || preview.witness_hash != intent.approved_witness_hash
        || preview.effective_diff_hash != intent.approved_diff_hash
        || binding.overlay_witnesses.proposed_effective_hash != intent.expected_effective_hash
    {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    Ok(())
}

pub(super) fn insert_decision(
    transaction: &rusqlite::Transaction<'_>,
    intent: &CuratorApplicationIntent,
) -> Result<(), CuratorApplicationStoreError> {
    transaction
        .execute(
            "INSERT INTO evolution_curator_decisions
         (decision_id,candidate_id,candidate_revision,decision_kind,actor_class,reason_code,
          note_hash,preview_hash,review_after_ms,idempotency_key,decided_at_ms)
         VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,NULL,?8,?9)",
            params![
                intent.decision.decision_id,
                intent.decision.candidate_id,
                sql_u64(intent.decision.candidate_revision).map_err(map_repository_error)?,
                decision_name(intent.decision.kind),
                actor_name(intent.decision.actor_class),
                intent.decision.reason_code,
                intent.decision.preview_hash,
                intent.idempotency_key,
                intent.decision.decided_at_ms
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    Ok(())
}

pub(super) fn invalidate_preview_and_transition(
    transaction: &rusqlite::Transaction<'_>,
    intent: &CuratorApplicationIntent,
    next_revision: u64,
) -> Result<(), CuratorApplicationStoreError> {
    transaction
        .execute(
            "UPDATE evolution_curator_previews SET invalidated_at_ms=?1
         WHERE candidate_id=?2 AND witness_hash=?3 AND invalidated_at_ms IS NULL",
            params![
                intent.decision.decided_at_ms,
                intent.decision.candidate_id,
                intent.approved_witness_hash
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    let updated = transaction
        .execute(
            "UPDATE evolution_curator_candidates SET state='applying',current_preview_id=NULL,
         revision=?1,updated_at_ms=?2 WHERE candidate_id=?3 AND state=?4 AND revision=?5",
            params![
                sql_u64(next_revision).map_err(map_repository_error)?,
                intent.decision.decided_at_ms,
                intent.decision.candidate_id,
                state_name(intent.expected_state),
                sql_u64(intent.decision.candidate_revision).map_err(map_repository_error)?
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    if updated != 1 {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    Ok(())
}

pub(super) fn append_intent_events(
    transaction: &rusqlite::Transaction<'_>,
    intent: &CuratorApplicationIntent,
    next_revision: u64,
) -> Result<(), CuratorApplicationStoreError> {
    append_audit_event(
        transaction,
        &CandidateTransitionRequest {
            candidate_id: &intent.decision.candidate_id,
            expected_revision: intent.decision.candidate_revision,
            transition: CuratorTransition::Approve,
            event_kind: CuratorEventKind::Approved,
            reason_code: Some(&intent.decision.reason_code),
            audit: audit_context(&intent.decision),
        },
        intent.expected_state,
        CuratorCandidateState::Applying,
        next_revision,
    )
    .map_err(map_repository_error)?;
    append_system_event(
        transaction,
        &SystemAuditEvent {
            candidate_id: &intent.decision.candidate_id,
            event_kind: CuratorEventKind::ApplicationStarted,
            occurred_at_ms: intent.decision.decided_at_ms,
            prior_state: Some(CuratorCandidateState::Applying),
            next_state: CuratorCandidateState::Applying,
            object_revision: next_revision,
            reason_code: "application_intent_recorded",
        },
    )
    .map_err(map_repository_error)
}

fn audit_context(decision: &CuratorDecision) -> TrustedAuditContext {
    match decision.actor_class {
        CuratorActorClass::LocalInteractiveUser => {
            TrustedAuditContext::local_interactive_user(decision.decided_at_ms)
        }
        CuratorActorClass::WebMockInteractiveUser => {
            TrustedAuditContext::web_mock_interactive_user(decision.decided_at_ms)
        }
        CuratorActorClass::System => TrustedAuditContext::system(decision.decided_at_ms),
    }
}

fn map_repository_error(error: super::CuratorRepositoryError) -> CuratorApplicationStoreError {
    match error {
        super::CuratorRepositoryError::Conflict(_) => CuratorApplicationStoreError::Conflict,
        super::CuratorRepositoryError::NotFound => CuratorApplicationStoreError::NotFound,
        super::CuratorRepositoryError::Storage => CuratorApplicationStoreError::Storage,
        _ => CuratorApplicationStoreError::InvalidInput,
    }
}
