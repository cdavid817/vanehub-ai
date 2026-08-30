use super::audit_chain::{append_system_event, SystemAuditEvent};
use super::intake_persistence::*;
use super::intake_source::*;
use super::CuratorRepositoryError;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{Connection, TransactionBehavior};

pub(crate) struct SqliteCuratorIntakeRepository<'a> {
    connection: &'a mut Connection,
}

impl<'a> SqliteCuratorIntakeRepository<'a> {
    pub(crate) fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn consume(
        &mut self,
        envelope: &AssessmentCompletionEnvelopeV1,
        now_ms: i64,
    ) -> Result<CuratorIntakeOutcome, CuratorRepositoryError> {
        validate_envelope(envelope, now_ms)?;
        let envelope_hash = hash_envelope(envelope)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorRepositoryError::Storage)?;
        if let Some(outcome) = existing_receipt(&transaction, &envelope_hash)? {
            return Ok(outcome);
        }
        let assessment = load_assessment(&transaction, &envelope.assessment_attempt_id)?;
        validate_authoritative(envelope, &assessment)?;
        if !envelope.current || !assessment.is_current {
            record_receipt(
                &transaction,
                envelope,
                &envelope_hash,
                "non_current",
                None,
                now_ms,
            )?;
            transaction
                .commit()
                .map_err(|_| CuratorRepositoryError::Storage)?;
            return Ok(CuratorIntakeOutcome::NonCurrentRejected);
        }
        let Some(evidence) = load_evidence(&transaction, envelope, &assessment.lineage_hash)?
        else {
            supersede_open(&transaction, &assessment.seed_id, None, now_ms)?;
            record_receipt(
                &transaction,
                envelope,
                &envelope_hash,
                "purged_evidence",
                None,
                now_ms,
            )?;
            transaction
                .commit()
                .map_err(|_| CuratorRepositoryError::Storage)?;
            return Ok(CuratorIntakeOutcome::PurgedEvidenceRejected);
        };
        if approvable_route(envelope.route).is_none() {
            supersede_open(&transaction, &assessment.seed_id, None, now_ms)?;
            record_receipt(
                &transaction,
                envelope,
                &envelope_hash,
                "non_approvable",
                None,
                now_ms,
            )?;
            transaction
                .commit()
                .map_err(|_| CuratorRepositoryError::Storage)?;
            return Ok(CuratorIntakeOutcome::NonApprovableRecorded);
        }
        let target = load_target(&transaction, &envelope.assessment_attempt_id)?;
        let policy_hash = load_policy_hash(&transaction, &assessment.workspace_id)?;
        let checks = load_checks(&transaction, &envelope.assessment_attempt_id)?;
        if checks.len() != 9 {
            return Err(CuratorRepositoryError::InvalidInput);
        }
        let snapshot = build_snapshot(
            envelope,
            &assessment,
            &target,
            evidence,
            checks,
            policy_hash,
            now_ms,
        )?;
        let candidate_id = snapshot.candidate_id.clone();
        insert_candidate(&transaction, &snapshot)?;
        supersede_open(
            &transaction,
            &assessment.seed_id,
            Some(&candidate_id),
            now_ms,
        )?;
        append_system_event(
            &transaction,
            &SystemAuditEvent {
                candidate_id: &candidate_id,
                event_kind: CuratorEventKind::Intake,
                occurred_at_ms: now_ms,
                prior_state: None,
                next_state: CuratorCandidateState::AwaitingDraft,
                object_revision: 1,
                reason_code: "assessment_intake_validated",
            },
        )?;
        record_receipt(
            &transaction,
            envelope,
            &envelope_hash,
            "candidate_created",
            Some(&candidate_id),
            now_ms,
        )?;
        transaction
            .commit()
            .map_err(|_| CuratorRepositoryError::Storage)?;
        Ok(CuratorIntakeOutcome::CandidateCreated { candidate_id })
    }
}
