use crate::contexts::skill_evolution_orchestration::domain::{
    canonical_json, is_safe_identifier, orchestration_idempotency_key, EvolutionCheckpointStatus,
    EvolutionRunUsageV1, EvolutionStageKind,
};
use rusqlite::{params, TransactionBehavior};

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdleDeferralCheckpoint {
    pub(crate) checkpoint_id: String,
    pub(crate) run_id: String,
    pub(crate) revision: u64,
    pub(crate) continuation_not_before_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageCheckpointCommitV1 {
    pub(crate) run_id: String,
    pub(crate) expected_run_revision: u64,
    pub(crate) stage: EvolutionStageKind,
    pub(crate) status: EvolutionCheckpointStatus,
    pub(crate) cursor_record_id: Option<String>,
    pub(crate) cursor_record_revision: Option<u64>,
    pub(crate) usage: EvolutionRunUsageV1,
    pub(crate) continuation_not_before_ms: Option<i64>,
    pub(crate) committed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageCheckpointCommitOutcome {
    pub(crate) checkpoint_id: String,
    pub(crate) run_revision: u64,
}

impl OrchestrationRepository {
    pub(crate) fn commit_stage_checkpoint(
        &self,
        request: &StageCheckpointCommitV1,
    ) -> Result<StageCheckpointCommitOutcome, OrchestrationPersistenceError> {
        validate_stage_checkpoint(request)?;
        let expected_revision = i64::try_from(request.expected_run_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let run_revision = request
            .expected_run_revision
            .checked_add(1)
            .ok_or(OrchestrationPersistenceError::InvalidInput)?;
        let run_revision_sql =
            i64::try_from(run_revision).map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let cursor_revision = request
            .cursor_record_revision
            .map(i64::try_from)
            .transpose()
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let usage_json = canonical_json(&request.usage)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let checkpoint_id = stage_checkpoint_id(request)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = transaction
            .execute(
                "UPDATE evolution_runs SET current_stage=?1,usage_json=?2,revision=?3,
                 updated_at_ms=?4 WHERE run_id=?5 AND revision=?6 AND status IN
                 ('requested','waiting_idle','running','partial','recovered')",
                params![
                    request.stage.as_str(),
                    usage_json,
                    run_revision_sql,
                    request.committed_at_ms,
                    request.run_id,
                    expected_revision,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 0 {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        transaction
            .execute(
                "INSERT INTO evolution_run_checkpoints
                 (checkpoint_id,run_id,stage,status,cursor_record_id,cursor_record_revision,
                  usage_json,continuation_not_before_ms,committed_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    checkpoint_id,
                    request.run_id,
                    request.stage.as_str(),
                    request.status.as_str(),
                    request.cursor_record_id,
                    cursor_revision,
                    usage_json,
                    request.continuation_not_before_ms,
                    request.committed_at_ms,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(StageCheckpointCommitOutcome {
            checkpoint_id,
            run_revision,
        })
    }

    pub(crate) fn defer_run_for_idle(
        &self,
        run_id: &str,
        expected_revision: u64,
        stage: EvolutionStageKind,
        usage: &EvolutionRunUsageV1,
        committed_at_ms: i64,
        continuation_not_before_ms: i64,
    ) -> Result<IdleDeferralCheckpoint, OrchestrationPersistenceError> {
        if !is_safe_identifier(run_id, 128)
            || committed_at_ms < 0
            || continuation_not_before_ms <= committed_at_ms
        {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let expected_revision_sql = i64::try_from(expected_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let revision = expected_revision
            .checked_add(1)
            .ok_or(OrchestrationPersistenceError::InvalidInput)?;
        let revision_sql =
            i64::try_from(revision).map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let checkpoint_id = checkpoint_id(run_id, expected_revision, stage)?;
        let usage_json =
            canonical_json(usage).map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = transaction
            .execute(
                "UPDATE evolution_runs SET status='partial',current_stage=?1,revision=?2,
                 updated_at_ms=?3 WHERE run_id=?4 AND revision=?5 AND status IN
                 ('requested','waiting_idle','running','partial')",
                params![
                    stage.as_str(),
                    revision_sql,
                    committed_at_ms,
                    run_id,
                    expected_revision_sql,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 0 {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        transaction
            .execute(
                "INSERT INTO evolution_run_checkpoints
                 (checkpoint_id,run_id,stage,status,cursor_record_id,cursor_record_revision,
                  usage_json,continuation_not_before_ms,committed_at_ms)
                 VALUES (?1,?2,?3,'continuation_required',NULL,NULL,?4,?5,?6)",
                params![
                    checkpoint_id,
                    run_id,
                    stage.as_str(),
                    usage_json,
                    continuation_not_before_ms,
                    committed_at_ms,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(IdleDeferralCheckpoint {
            checkpoint_id,
            run_id: run_id.into(),
            revision,
            continuation_not_before_ms,
        })
    }
}

fn validate_stage_checkpoint(
    request: &StageCheckpointCommitV1,
) -> Result<(), OrchestrationPersistenceError> {
    let cursor_shape_valid = match (
        request.cursor_record_id.as_deref(),
        request.cursor_record_revision,
    ) {
        (None, None) => true,
        (Some(record_id), Some(_)) => is_safe_identifier(record_id, 128),
        _ => false,
    };
    let continuation_valid = match request.status {
        EvolutionCheckpointStatus::ContinuationRequired => request
            .continuation_not_before_ms
            .is_some_and(|value| value > request.committed_at_ms),
        EvolutionCheckpointStatus::Committed | EvolutionCheckpointStatus::Reconciled => {
            request.continuation_not_before_ms.is_none()
        }
        EvolutionCheckpointStatus::Pending => false,
    };
    if !is_safe_identifier(&request.run_id, 128)
        || request.committed_at_ms < 0
        || !cursor_shape_valid
        || !continuation_valid
    {
        return Err(OrchestrationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn stage_checkpoint_id(
    request: &StageCheckpointCommitV1,
) -> Result<String, OrchestrationPersistenceError> {
    let digest = orchestration_idempotency_key(
        "checkpoint",
        request.stage.as_str(),
        &(
            &request.run_id,
            request.expected_run_revision,
            &request.cursor_record_id,
            request.cursor_record_revision,
        ),
    )
    .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
    Ok(format!("checkpoint:{digest}"))
}

fn checkpoint_id(
    run_id: &str,
    expected_revision: u64,
    stage: EvolutionStageKind,
) -> Result<String, OrchestrationPersistenceError> {
    let digest = orchestration_idempotency_key(
        "checkpoint",
        "idle-deferral",
        &(run_id, expected_revision, stage.as_str()),
    )
    .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
    Ok(format!("checkpoint:{digest}"))
}
