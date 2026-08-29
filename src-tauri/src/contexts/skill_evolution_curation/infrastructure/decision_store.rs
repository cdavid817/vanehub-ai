use super::decision_binding_store::load_decision_binding;
use super::repository_support::{
    actor_name, current_candidate, decision_name, event_name, sql_u64, state_name,
};
use super::{
    append_audit_event, CandidateTransitionRequest, SqliteCuratorRepository, TrustedAuditContext,
};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

impl CuratorDecisionStore for SqliteCuratorRepository<'_> {
    fn existing_decision(
        &mut self,
        candidate_id: &str,
        kind: CuratorDecisionKind,
        idempotency_key: &str,
    ) -> Result<Option<CuratorDecisionOutcome>, CuratorDecisionStoreError> {
        existing_outcome(self.connection, candidate_id, kind, idempotency_key)
    }

    fn decision_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDecisionBinding, CuratorDecisionStoreError> {
        load_decision_binding(self.connection, candidate_id)
    }

    fn persist_decision_mutation(
        &mut self,
        mutation: &CuratorDecisionMutation<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionStoreError> {
        validate_mutation(mutation)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorDecisionStoreError::Storage)?;
        if let Some(existing) = existing_outcome(
            &transaction,
            &mutation.decision.candidate_id,
            mutation.decision.kind,
            mutation.idempotency_key,
        )? {
            return Ok(existing);
        }
        let (current_state, current_revision) =
            current_candidate(&transaction, &mutation.decision.candidate_id).map_err(map_error)?;
        if current_state != mutation.expected_state
            || current_revision != mutation.decision.candidate_revision
        {
            return Err(CuratorDecisionStoreError::Conflict);
        }
        let next_state = transition_candidate(current_state, mutation.transition)
            .map_err(|_| CuratorDecisionStoreError::Conflict)?;
        let next_revision = current_revision
            .checked_add(1)
            .ok_or(CuratorDecisionStoreError::InvalidInput)?;
        insert_decision(&transaction, mutation)?;
        transaction
            .execute(
                "UPDATE evolution_curator_previews SET invalidated_at_ms=?1
                 WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
                params![
                    mutation.decision.decided_at_ms,
                    mutation.decision.candidate_id
                ],
            )
            .map_err(|_| CuratorDecisionStoreError::Storage)?;
        let updated = transaction
            .execute(
                "UPDATE evolution_curator_candidates SET state=?1,current_preview_id=NULL,
                 revision=?2,updated_at_ms=?3 WHERE candidate_id=?4 AND revision=?5 AND state=?6",
                params![
                    state_name(next_state),
                    sql_u64(next_revision).map_err(map_error)?,
                    mutation.decision.decided_at_ms,
                    mutation.decision.candidate_id,
                    sql_u64(current_revision).map_err(map_error)?,
                    state_name(current_state)
                ],
            )
            .map_err(|_| CuratorDecisionStoreError::Storage)?;
        if updated != 1 {
            return Err(CuratorDecisionStoreError::Conflict);
        }
        append_audit_event(
            &transaction,
            &CandidateTransitionRequest {
                candidate_id: &mutation.decision.candidate_id,
                expected_revision: current_revision,
                transition: mutation.transition,
                event_kind: mutation.event_kind,
                reason_code: Some(&mutation.decision.reason_code),
                audit: audit_context(mutation.decision),
            },
            current_state,
            next_state,
            next_revision,
        )
        .map_err(map_error)?;
        transaction
            .commit()
            .map_err(|_| CuratorDecisionStoreError::Storage)?;
        Ok(CuratorDecisionOutcome {
            decision_id: mutation.decision.decision_id.clone(),
            candidate_revision: next_revision,
            state: next_state,
            duplicate: false,
        })
    }
}

fn insert_decision(
    transaction: &rusqlite::Transaction<'_>,
    mutation: &CuratorDecisionMutation<'_>,
) -> Result<(), CuratorDecisionStoreError> {
    transaction
        .execute(
            "INSERT INTO evolution_curator_decisions
         (decision_id,candidate_id,candidate_revision,decision_kind,actor_class,reason_code,
          note_hash,preview_hash,review_after_ms,idempotency_key,decided_at_ms)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                mutation.decision.decision_id,
                mutation.decision.candidate_id,
                sql_u64(mutation.decision.candidate_revision).map_err(map_error)?,
                decision_name(mutation.decision.kind),
                actor_name(mutation.decision.actor_class),
                mutation.decision.reason_code,
                mutation.decision.note_hash,
                mutation.decision.preview_hash,
                mutation.review_after_ms,
                mutation.idempotency_key,
                mutation.decision.decided_at_ms
            ],
        )
        .map_err(|_| CuratorDecisionStoreError::Storage)?;
    Ok(())
}

fn existing_outcome(
    connection: &rusqlite::Connection,
    candidate_id: &str,
    kind: CuratorDecisionKind,
    idempotency_key: &str,
) -> Result<Option<CuratorDecisionOutcome>, CuratorDecisionStoreError> {
    connection
        .query_row(
            "SELECT d.decision_id,e.object_revision,e.next_state FROM evolution_curator_decisions d
         JOIN evolution_curator_events e ON e.candidate_id=d.candidate_id
          AND e.object_revision=d.candidate_revision+1
          AND e.event_kind=?4
         WHERE d.candidate_id=?1 AND d.decision_kind=?2 AND d.idempotency_key=?3",
            params![
                candidate_id,
                decision_name(kind),
                idempotency_key,
                event_name(decision_event(kind))
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| CuratorDecisionStoreError::Storage)?
        .map(|(decision_id, revision, state)| {
            Ok(CuratorDecisionOutcome {
                decision_id,
                candidate_revision: u64::try_from(revision)
                    .map_err(|_| CuratorDecisionStoreError::Storage)?,
                state: parse_state(&state)?,
                duplicate: true,
            })
        })
        .transpose()
}

fn parse_state(value: &str) -> Result<CuratorCandidateState, CuratorDecisionStoreError> {
    super::repository_support::parse_state(value).map_err(map_error)
}

fn validate_mutation(
    mutation: &CuratorDecisionMutation<'_>,
) -> Result<(), CuratorDecisionStoreError> {
    if mutation.decision.decision_id.trim().is_empty()
        || mutation.decision.candidate_id.trim().is_empty()
        || mutation.decision.reason_code.trim().is_empty()
        || mutation.decision.reason_code.len() > 160
        || mutation.decision.decided_at_ms < 0
        || mutation.decision.actor_class == CuratorActorClass::System
        || mutation.decision.kind == CuratorDecisionKind::Approve
        || mutation.decision.preview_hash.is_some()
        || !review_after_is_valid(mutation)
    {
        return Err(CuratorDecisionStoreError::InvalidInput);
    }
    Ok(())
}

fn review_after_is_valid(mutation: &CuratorDecisionMutation<'_>) -> bool {
    match (mutation.decision.kind, mutation.review_after_ms) {
        (CuratorDecisionKind::Defer, None) => true,
        (CuratorDecisionKind::Defer, Some(review_after_ms)) => {
            let Some(minimum) = mutation
                .decision
                .decided_at_ms
                .checked_add(CURATOR_MIN_DEFER_MS)
            else {
                return false;
            };
            let Some(maximum) = mutation
                .decision
                .decided_at_ms
                .checked_add(CURATOR_MAX_DEFER_MS)
            else {
                return false;
            };
            (minimum..=maximum).contains(&review_after_ms)
        }
        (_, None) => true,
        (_, Some(_)) => false,
    }
}

fn decision_event(kind: CuratorDecisionKind) -> CuratorEventKind {
    match kind {
        CuratorDecisionKind::Approve => CuratorEventKind::Approved,
        CuratorDecisionKind::Reject => CuratorEventKind::Rejected,
        CuratorDecisionKind::Defer => CuratorEventKind::Deferred,
        CuratorDecisionKind::Resume => CuratorEventKind::Resumed,
    }
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

fn map_error(error: super::CuratorRepositoryError) -> CuratorDecisionStoreError {
    match error {
        super::CuratorRepositoryError::NotFound => CuratorDecisionStoreError::NotFound,
        super::CuratorRepositoryError::Conflict(_) => CuratorDecisionStoreError::Conflict,
        super::CuratorRepositoryError::Storage => CuratorDecisionStoreError::Storage,
        _ => CuratorDecisionStoreError::InvalidInput,
    }
}
