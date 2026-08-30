use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, orchestration_idempotency_key, EvolutionStageKind,
};
use rusqlite::{params, OptionalExtension};

use super::{OrchestrationPersistenceError, OrchestrationRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReserveStageItemOutcome {
    Reserved {
        item_id: String,
        idempotency_key: String,
    },
    Duplicate {
        item_id: String,
        committed_receipt_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommitStageItemOutcome {
    Committed { receipt_id: String },
    Duplicate { receipt_id: String },
}

impl OrchestrationRepository {
    pub(crate) fn reserve_stage_item(
        &self,
        run_id: &str,
        stage: EvolutionStageKind,
        source_id: &str,
        source_revision: u64,
        created_at_ms: i64,
    ) -> Result<ReserveStageItemOutcome, OrchestrationPersistenceError> {
        if !is_safe_identifier(run_id, 128)
            || !is_safe_identifier(source_id, 128)
            || created_at_ms < 0
        {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let source_revision_sql = i64::try_from(source_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let key = orchestration_idempotency_key(
            "stage-item",
            stage.as_str(),
            &(run_id, source_id, source_revision),
        )
        .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let item_id = format!("item:{key}");
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "INSERT OR IGNORE INTO evolution_run_items
                 (item_id,run_id,stage,subsystem_idempotency_key,source_id,source_revision,
                  committed_receipt_id,safe_failure_code,created_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,NULL,NULL,?7)",
                params![
                    item_id,
                    run_id,
                    stage.as_str(),
                    key,
                    source_id,
                    source_revision_sql,
                    created_at_ms,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 1 {
            return Ok(ReserveStageItemOutcome::Reserved {
                item_id,
                idempotency_key: key,
            });
        }
        let existing = connection
            .query_row(
                "SELECT item_id,run_id,stage,source_id,source_revision,committed_receipt_id
                 FROM evolution_run_items WHERE subsystem_idempotency_key=?1",
                [key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?
            .ok_or(OrchestrationPersistenceError::Conflict)?;
        if existing.1 != run_id
            || existing.2 != stage.as_str()
            || existing.3 != source_id
            || existing.4 != source_revision_sql
        {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        Ok(ReserveStageItemOutcome::Duplicate {
            item_id: existing.0,
            committed_receipt_id: existing.5,
        })
    }

    pub(crate) fn commit_stage_item(
        &self,
        item_id: &str,
        receipt_id: &str,
    ) -> Result<CommitStageItemOutcome, OrchestrationPersistenceError> {
        if !is_safe_identifier(item_id, 128) || !is_safe_identifier(receipt_id, 128) {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "UPDATE evolution_run_items SET committed_receipt_id=?1
                 WHERE item_id=?2 AND committed_receipt_id IS NULL",
                params![receipt_id, item_id],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 1 {
            return Ok(CommitStageItemOutcome::Committed {
                receipt_id: receipt_id.into(),
            });
        }
        let existing = connection
            .query_row(
                "SELECT committed_receipt_id FROM evolution_run_items WHERE item_id=?1",
                [item_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?
            .ok_or(OrchestrationPersistenceError::NotFound)?;
        match existing {
            Some(existing) if existing == receipt_id => Ok(CommitStageItemOutcome::Duplicate {
                receipt_id: existing,
            }),
            Some(_) => Err(OrchestrationPersistenceError::Conflict),
            None => Err(OrchestrationPersistenceError::Conflict),
        }
    }
}
