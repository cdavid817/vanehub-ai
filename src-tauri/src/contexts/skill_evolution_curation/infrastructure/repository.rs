use super::repository_support::*;
use super::{append_audit_event, safe_snapshot_json, *};
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

pub(crate) struct SqliteCuratorRepository<'a> {
    pub(super) connection: &'a mut Connection,
}

impl<'a> SqliteCuratorRepository<'a> {
    pub(crate) fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn insert_candidate(
        &mut self,
        snapshot: &CuratorCandidateSnapshot,
    ) -> Result<PersistCandidateOutcome, CuratorRepositoryError> {
        validate_snapshot(snapshot)?;
        let snapshot_json = safe_snapshot_json(snapshot)?;
        let snapshot_revision = sql_u64(snapshot.revision)?;
        let transaction = self.transaction()?;
        let inserted = transaction.execute(
            "INSERT INTO evolution_curator_candidates (
                candidate_id,schema_version,workspace_id,seed_id,seed_revision,
                assessment_attempt_id,assessment_revision,target_skill_id,target_revision,
                overlay_scope,route,risk,confidence,policy_witness_hash,witness_hash,snapshot_json,state,
                staleness_json,revision,created_at_ms,updated_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![
                snapshot.candidate_id,
                snapshot.schema_version,
                snapshot.workspace_id,
                snapshot.seed_id,
                snapshot.seed_revision,
                snapshot.assessment_attempt_id,
                snapshot.assessment_revision,
                snapshot.target_skill_id,
                snapshot.target_revision,
                snapshot.overlay_scope,
                route_name(snapshot.route),
                risk_name(snapshot.risk),
                confidence_name(snapshot.confidence),
                snapshot.policy_witness_hash,
                snapshot.witness_hash,
                snapshot_json,
                state_name(snapshot.state),
                serde_json::to_string(&snapshot.staleness)
                    .map_err(|_| CuratorRepositoryError::InvalidInput)?,
                snapshot_revision,
                snapshot.created_at_ms,
                snapshot.updated_at_ms,
            ],
        );
        if let Err(error) = inserted {
            if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                let candidate_id = transaction
                    .query_row(
                        "SELECT candidate_id FROM evolution_curator_candidates
                     WHERE assessment_attempt_id=?1 AND assessment_revision=?2
                       AND target_revision=?3 AND witness_hash=?4",
                        params![
                            snapshot.assessment_attempt_id,
                            snapshot.assessment_revision,
                            snapshot.target_revision,
                            snapshot.witness_hash
                        ],
                        |row| row.get(0),
                    )
                    .map_err(|_| CuratorRepositoryError::Storage)?;
                return Ok(PersistCandidateOutcome::Existing { candidate_id });
            }
            return Err(CuratorRepositoryError::Storage);
        }
        for source in &snapshot.evidence_sources {
            transaction
                .execute(
                    "INSERT INTO evolution_curator_candidate_sources
                 (candidate_id,evidence_id,evidence_revision,lineage_hash)
                 VALUES (?1,?2,?3,?4)",
                    params![
                        snapshot.candidate_id,
                        source.evidence_id,
                        source.evidence_revision,
                        source.lineage_hash
                    ],
                )
                .map_err(|_| CuratorRepositoryError::Storage)?;
        }
        transaction
            .commit()
            .map_err(|_| CuratorRepositoryError::Storage)?;
        Ok(PersistCandidateOutcome::Inserted)
    }

    pub(crate) fn transition_with_audit(
        &mut self,
        request: &CandidateTransitionRequest<'_>,
    ) -> Result<u64, CuratorRepositoryError> {
        let transaction = self.transaction()?;
        let (current_state, current_revision) =
            current_candidate(&transaction, request.candidate_id)?;
        if current_revision != request.expected_revision {
            return Err(CuratorRepositoryError::Conflict(CandidateConflict {
                current_revision,
                current_state,
            }));
        }
        let next_state = transition_candidate(current_state, request.transition)?;
        let next_revision = current_revision + 1;
        let current_revision_sql = sql_u64(current_revision)?;
        let next_revision_sql = sql_u64(next_revision)?;
        let updated = transaction
            .execute(
                "UPDATE evolution_curator_candidates SET state=?1, revision=?2, updated_at_ms=?3
             WHERE candidate_id=?4 AND revision=?5 AND state=?6",
                params![
                    state_name(next_state),
                    next_revision_sql,
                    request.audit.occurred_at_ms(),
                    request.candidate_id,
                    current_revision_sql,
                    state_name(current_state)
                ],
            )
            .map_err(|_| CuratorRepositoryError::Storage)?;
        if updated != 1 {
            let (state, revision) = current_candidate(&transaction, request.candidate_id)?;
            return Err(CuratorRepositoryError::Conflict(CandidateConflict {
                current_revision: revision,
                current_state: state,
            }));
        }
        append_audit_event(
            &transaction,
            request,
            current_state,
            next_state,
            next_revision,
        )?;
        transaction
            .commit()
            .map_err(|_| CuratorRepositoryError::Storage)?;
        Ok(next_revision)
    }

    pub(crate) fn persist_decision(
        &mut self,
        input: &DecisionPersistence<'_>,
    ) -> Result<PersistDecisionOutcome, CuratorRepositoryError> {
        if input.idempotency_key.trim().is_empty() || input.idempotency_key.len() > 160 {
            return Err(CuratorRepositoryError::InvalidInput);
        }
        let transaction = self.transaction()?;
        let decision_revision = sql_u64(input.decision.candidate_revision)?;
        if let Some(decision_id) = transaction
            .query_row(
                "SELECT decision_id FROM evolution_curator_decisions
             WHERE candidate_id=?1 AND decision_kind=?2 AND idempotency_key=?3",
                params![
                    input.decision.candidate_id,
                    decision_name(input.decision.kind),
                    input.idempotency_key
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| CuratorRepositoryError::Storage)?
        {
            return Ok(PersistDecisionOutcome::Existing { decision_id });
        }
        transaction
            .execute(
                "INSERT INTO evolution_curator_decisions
             (decision_id,candidate_id,candidate_revision,decision_kind,actor_class,reason_code,
              note_hash,preview_hash,review_after_ms,idempotency_key,decided_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    input.decision.decision_id,
                    input.decision.candidate_id,
                    decision_revision,
                    decision_name(input.decision.kind),
                    actor_name(input.decision.actor_class),
                    input.decision.reason_code,
                    input.decision.note_hash,
                    input.decision.preview_hash,
                    input.review_after_ms,
                    input.idempotency_key,
                    input.decision.decided_at_ms
                ],
            )
            .map_err(|_| CuratorRepositoryError::Storage)?;
        transaction
            .commit()
            .map_err(|_| CuratorRepositoryError::Storage)?;
        Ok(PersistDecisionOutcome::Inserted)
    }

    fn transaction(&mut self) -> Result<Transaction<'_>, CuratorRepositoryError> {
        self.connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorRepositoryError::Storage)
    }
}
