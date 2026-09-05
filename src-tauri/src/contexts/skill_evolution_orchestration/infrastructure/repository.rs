use crate::{
    contexts::skill_evolution_orchestration::domain::{
        canonical_json, is_safe_identifier, EvolutionActorProvenance, EvolutionTriggerEnvelopeV1,
        ORCHESTRATION_SCHEMA_VERSION_V1,
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrchestrationPersistenceError {
    InvalidInput,
    Conflict,
    NotFound,
    Corrupt,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistTriggerOutcome {
    Inserted { receipt_id: String },
    Duplicate { receipt_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeaseAcquisition {
    pub(crate) run_id: String,
    pub(crate) revision: u64,
    pub(crate) lease_owner: String,
    pub(crate) lease_expires_at_ms: i64,
}

#[derive(Clone)]
pub(crate) struct OrchestrationRepository {
    pub(super) database: NativeDatabase,
}

impl OrchestrationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn persist_trigger(
        &self,
        trigger: &EvolutionTriggerEnvelopeV1,
        created_at_ms: i64,
    ) -> Result<PersistTriggerOutcome, OrchestrationPersistenceError> {
        validate_trigger(trigger)?;
        let source_revision = i64::try_from(trigger.source_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let safe_reason_codes = canonical_json(&trigger.safe_reason_codes)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "INSERT OR IGNORE INTO evolution_trigger_receipts
                 (receipt_id,schema_version,family,workspace_id,source_kind,source_id,
                  source_revision,occurred_at_ms,priority,safe_reason_codes_json,actor,created_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    trigger.trigger_id,
                    trigger.schema_version,
                    trigger.family.as_str(),
                    trigger.workspace_id,
                    trigger.source_kind,
                    trigger.source_id,
                    source_revision,
                    trigger.occurred_at_ms,
                    trigger.priority,
                    safe_reason_codes,
                    actor_name(trigger.actor),
                    created_at_ms,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 1 {
            return Ok(PersistTriggerOutcome::Inserted {
                receipt_id: trigger.trigger_id.clone(),
            });
        }
        let receipt_id = connection
            .query_row(
                "SELECT receipt_id FROM evolution_trigger_receipts
                 WHERE family=?1 AND source_kind=?2 AND source_id=?3 AND source_revision=?4",
                params![
                    trigger.family.as_str(),
                    trigger.source_kind,
                    trigger.source_id,
                    source_revision,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?
            .ok_or(OrchestrationPersistenceError::Conflict)?;
        Ok(PersistTriggerOutcome::Duplicate { receipt_id })
    }

    pub(crate) fn acquire_run_lease(
        &self,
        run_id: &str,
        expected_revision: u64,
        lease_owner: &str,
        now_ms: i64,
        lease_expires_at_ms: i64,
    ) -> Result<LeaseAcquisition, OrchestrationPersistenceError> {
        if !is_safe_identifier(run_id, 128)
            || !is_safe_identifier(lease_owner, 128)
            || lease_expires_at_ms <= now_ms
        {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or(OrchestrationPersistenceError::InvalidInput)?;
        let expected_revision_sql = i64::try_from(expected_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let next_revision_sql = i64::try_from(next_revision)
            .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let changed = connection
            .execute(
                "UPDATE evolution_runs SET lease_owner=?1,lease_expires_at_ms=?2,
                 revision=?3,updated_at_ms=?4 WHERE run_id=?5 AND revision=?6
                 AND status IN ('requested','waiting_idle','running','partial','cancel_requested')
                 AND (lease_owner IS NULL OR lease_owner=?1 OR lease_expires_at_ms<=?4)",
                params![
                    lease_owner,
                    lease_expires_at_ms,
                    next_revision_sql,
                    now_ms,
                    run_id,
                    expected_revision_sql,
                ],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if changed == 0 {
            let exists = connection
                .query_row(
                    "SELECT 1 FROM evolution_runs WHERE run_id=?1",
                    [run_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|_| OrchestrationPersistenceError::Storage)?
                .is_some();
            return Err(if exists {
                OrchestrationPersistenceError::Conflict
            } else {
                OrchestrationPersistenceError::NotFound
            });
        }
        Ok(LeaseAcquisition {
            run_id: run_id.into(),
            revision: next_revision,
            lease_owner: lease_owner.into(),
            lease_expires_at_ms,
        })
    }

    pub(crate) fn trigger_safe_reason_codes(
        &self,
        receipt_id: &str,
    ) -> Result<Vec<String>, OrchestrationPersistenceError> {
        if !is_safe_identifier(receipt_id, 128) {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let json = connection
            .query_row(
                "SELECT safe_reason_codes_json FROM evolution_trigger_receipts WHERE receipt_id=?1",
                [receipt_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| OrchestrationPersistenceError::Storage)?
            .ok_or(OrchestrationPersistenceError::NotFound)?;
        let values: Vec<String> =
            serde_json::from_str(&json).map_err(|_| OrchestrationPersistenceError::Corrupt)?;
        if values.len() > 8 || values.iter().any(|value| !is_safe_identifier(value, 64)) {
            return Err(OrchestrationPersistenceError::Corrupt);
        }
        Ok(values)
    }
}

pub(super) fn validate_trigger(
    trigger: &EvolutionTriggerEnvelopeV1,
) -> Result<(), OrchestrationPersistenceError> {
    let identifiers = [
        trigger.trigger_id.as_str(),
        trigger.workspace_id.as_str(),
        trigger.source_kind.as_str(),
        trigger.source_id.as_str(),
    ];
    if trigger.schema_version != ORCHESTRATION_SCHEMA_VERSION_V1
        || identifiers
            .iter()
            .any(|value| !is_safe_identifier(value, 128))
        || trigger.safe_reason_codes.len() > 8
        || trigger
            .safe_reason_codes
            .iter()
            .any(|value| !is_safe_identifier(value, 64))
    {
        return Err(OrchestrationPersistenceError::InvalidInput);
    }
    Ok(())
}

pub(super) fn actor_name(actor: EvolutionActorProvenance) -> &'static str {
    match actor {
        EvolutionActorProvenance::InteractiveUser => "interactive_user",
        EvolutionActorProvenance::SystemPolicy => "system_policy",
        EvolutionActorProvenance::RuntimeTrigger => "runtime_trigger",
        EvolutionActorProvenance::Recovery => "recovery",
        EvolutionActorProvenance::WebMock => "web_mock",
    }
}

pub(super) fn validate_lease_input(
    run_id: &str,
    lease_owner: &str,
    now_ms: i64,
    lease_expires_at_ms: i64,
) -> Result<(), OrchestrationPersistenceError> {
    if !is_safe_identifier(run_id, 128)
        || !is_safe_identifier(lease_owner, 128)
        || now_ms < 0
        || lease_expires_at_ms <= now_ms
    {
        return Err(OrchestrationPersistenceError::InvalidInput);
    }
    Ok(())
}

pub(super) fn sql_revisions(expected: u64) -> Result<(i64, i64), OrchestrationPersistenceError> {
    let next = expected
        .checked_add(1)
        .ok_or(OrchestrationPersistenceError::InvalidInput)?;
    Ok((
        i64::try_from(expected).map_err(|_| OrchestrationPersistenceError::InvalidInput)?,
        i64::try_from(next).map_err(|_| OrchestrationPersistenceError::InvalidInput)?,
    ))
}
