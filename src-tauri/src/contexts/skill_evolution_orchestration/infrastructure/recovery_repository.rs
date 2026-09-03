use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, EvolutionCheckpointStatus, EvolutionStageKind, EVOLUTION_STAGE_ORDER_V1,
};
use rusqlite::OptionalExtension;

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunResumePositionV1 {
    pub(crate) run_id: String,
    pub(crate) next_stage: Option<EvolutionStageKind>,
    pub(crate) cursor_record_id: Option<String>,
    pub(crate) cursor_record_revision: Option<u64>,
    pub(crate) checkpoint_status: Option<EvolutionCheckpointStatus>,
}

impl OrchestrationRepository {
    pub(crate) fn run_resume_position(
        &self,
        run_id: &str,
    ) -> Result<RunResumePositionV1, OrchestrationPersistenceError> {
        if !is_safe_identifier(run_id, 128) {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM evolution_runs WHERE run_id=?1",
                [run_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?
            .is_some();
        if !exists {
            return Err(OrchestrationPersistenceError::NotFound);
        }
        let checkpoint = connection
            .query_row(
                "SELECT stage,status,cursor_record_id,cursor_record_revision
                 FROM evolution_run_checkpoints WHERE run_id=?1
                 ORDER BY committed_at_ms DESC,checkpoint_id DESC LIMIT 1",
                [run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let Some((stage, status, cursor_record_id, cursor_revision)) = checkpoint else {
            return Ok(RunResumePositionV1 {
                run_id: run_id.into(),
                next_stage: Some(EvolutionStageKind::Recover),
                cursor_record_id: None,
                cursor_record_revision: None,
                checkpoint_status: None,
            });
        };
        let stage = EvolutionStageKind::from_persisted(&stage)
            .map_err(|_| OrchestrationPersistenceError::Corrupt)?;
        let status = EvolutionCheckpointStatus::from_persisted(&status)
            .map_err(|_| OrchestrationPersistenceError::Corrupt)?;
        if status == EvolutionCheckpointStatus::Pending {
            return Err(OrchestrationPersistenceError::Corrupt);
        }
        let cursor_record_revision = cursor_revision
            .map(u64::try_from)
            .transpose()
            .map_err(|_| OrchestrationPersistenceError::Corrupt)?;
        if cursor_record_id.is_some() != cursor_record_revision.is_some() {
            return Err(OrchestrationPersistenceError::Corrupt);
        }
        let next_stage = match status {
            EvolutionCheckpointStatus::ContinuationRequired => Some(stage),
            EvolutionCheckpointStatus::Committed | EvolutionCheckpointStatus::Reconciled => {
                stage_after(stage)
            }
            EvolutionCheckpointStatus::Pending => None,
        };
        Ok(RunResumePositionV1 {
            run_id: run_id.into(),
            next_stage,
            cursor_record_id,
            cursor_record_revision,
            checkpoint_status: Some(status),
        })
    }
}

fn stage_after(stage: EvolutionStageKind) -> Option<EvolutionStageKind> {
    EVOLUTION_STAGE_ORDER_V1
        .iter()
        .position(|candidate| *candidate == stage)
        .and_then(|index| EVOLUTION_STAGE_ORDER_V1.get(index + 1))
        .copied()
}
