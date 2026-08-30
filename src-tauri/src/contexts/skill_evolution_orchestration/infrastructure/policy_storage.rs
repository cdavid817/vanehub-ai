use crate::contexts::skill_evolution_orchestration::domain::{
    validate_policy_integrity, EvolutionOrchestrationPolicyV1, EvolutionPolicyError,
    EvolutionPolicyMode, EvolutionRunBudgetV1,
};
use rusqlite::{params, OptionalExtension, Transaction};

use super::OrchestrationPersistenceError;

type PolicyRow = (
    u16,
    String,
    String,
    Option<String>,
    String,
    String,
    i64,
    i64,
    bool,
    i64,
    i64,
    i64,
);

pub(super) fn map_policy_error(error: EvolutionPolicyError) -> OrchestrationPersistenceError {
    match error {
        EvolutionPolicyError::RevisionConflict => OrchestrationPersistenceError::Conflict,
        EvolutionPolicyError::Integrity => OrchestrationPersistenceError::Corrupt,
        _ => OrchestrationPersistenceError::InvalidInput,
    }
}

pub(super) fn load_policy(
    connection: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Option<EvolutionOrchestrationPolicyV1>, OrchestrationPersistenceError> {
    let row: Option<PolicyRow> = connection
        .query_row(
            "SELECT schema_version,mode,allowed_skill_ids_json,consent_json,automatic_budget_json,manual_budget_json,user_idle_ms,maximum_idle_wait_ms,notify_routine_completion,revision,created_at_ms,updated_at_ms FROM evolution_orchestration_policy WHERE workspace_id=?1",
            [workspace_id],
            |row| {
                Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                    row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                    row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?,
                ))
            },
        )
        .optional()
        .map_err(map_read_error)?;
    row.map(|row| decode_policy(workspace_id, row)).transpose()
}

pub(super) fn persist_policy(
    transaction: &Transaction<'_>,
    policy: &EvolutionOrchestrationPolicyV1,
    expected_revision: u64,
    exists: bool,
) -> Result<(), OrchestrationPersistenceError> {
    validate_policy_integrity(policy).map_err(map_policy_error)?;
    let allowed = json(&policy.allowed_skill_ids)?;
    let consent = policy.consent.as_ref().map(json).transpose()?;
    let automatic = json(&policy.automatic_budget)?;
    let manual = json(&policy.manual_budget)?;
    let user_idle_ms = sql_u64(policy.user_idle_ms)?;
    let maximum_idle_wait_ms = sql_u64(policy.maximum_idle_wait_ms)?;
    let revision = sql_u64(policy.revision)?;
    let expected_revision = sql_u64(expected_revision)?;
    let changed = if exists {
        transaction.execute("UPDATE evolution_orchestration_policy SET schema_version=?1,mode=?2,allowed_skill_ids_json=?3,consent_json=?4,automatic_budget_json=?5,manual_budget_json=?6,user_idle_ms=?7,maximum_idle_wait_ms=?8,notify_routine_completion=?9,revision=?10,updated_at_ms=?11 WHERE workspace_id=?12 AND revision=?13", params![policy.schema_version, mode_name(policy.mode), allowed, consent, automatic, manual, user_idle_ms, maximum_idle_wait_ms, policy.notify_routine_completion, revision, policy.updated_at_ms, policy.workspace_id, expected_revision])
    } else {
        transaction.execute("INSERT OR IGNORE INTO evolution_orchestration_policy VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)", params![policy.workspace_id, policy.schema_version, mode_name(policy.mode), allowed, consent, automatic, manual, user_idle_ms, maximum_idle_wait_ms, policy.notify_routine_completion, revision, policy.created_at_ms, policy.updated_at_ms])
    }.map_err(|_| OrchestrationPersistenceError::Storage)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(OrchestrationPersistenceError::Conflict)
    }
}

fn decode_policy(
    workspace_id: &str,
    row: PolicyRow,
) -> Result<EvolutionOrchestrationPolicyV1, OrchestrationPersistenceError> {
    let policy = EvolutionOrchestrationPolicyV1 {
        schema_version: row.0,
        workspace_id: workspace_id.into(),
        mode: parse_mode(&row.1)?,
        allowed_skill_ids: from_json(&row.2)?,
        consent: row.3.as_deref().map(from_json).transpose()?,
        automatic_budget: from_json::<EvolutionRunBudgetV1>(&row.4)?,
        manual_budget: from_json::<EvolutionRunBudgetV1>(&row.5)?,
        user_idle_ms: stored_u64(row.6)?,
        maximum_idle_wait_ms: stored_u64(row.7)?,
        notify_routine_completion: row.8,
        revision: stored_u64(row.9)?,
        created_at_ms: row.10,
        updated_at_ms: row.11,
    };
    validate_policy_integrity(&policy).map_err(|_| OrchestrationPersistenceError::Corrupt)?;
    Ok(policy)
}

fn mode_name(mode: EvolutionPolicyMode) -> &'static str {
    match mode {
        EvolutionPolicyMode::Off => "off",
        EvolutionPolicyMode::Observe => "observe",
        EvolutionPolicyMode::Enabled => "enabled",
    }
}

fn parse_mode(value: &str) -> Result<EvolutionPolicyMode, OrchestrationPersistenceError> {
    match value {
        "off" => Ok(EvolutionPolicyMode::Off),
        "observe" => Ok(EvolutionPolicyMode::Observe),
        "enabled" => Ok(EvolutionPolicyMode::Enabled),
        _ => Err(OrchestrationPersistenceError::Corrupt),
    }
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, OrchestrationPersistenceError> {
    serde_json::to_string(value).map_err(|_| OrchestrationPersistenceError::InvalidInput)
}

fn from_json<T: serde::de::DeserializeOwned>(
    value: &str,
) -> Result<T, OrchestrationPersistenceError> {
    serde_json::from_str(value).map_err(|_| OrchestrationPersistenceError::Corrupt)
}

fn sql_u64(value: u64) -> Result<i64, OrchestrationPersistenceError> {
    i64::try_from(value).map_err(|_| OrchestrationPersistenceError::InvalidInput)
}

fn stored_u64(value: i64) -> Result<u64, OrchestrationPersistenceError> {
    u64::try_from(value).map_err(|_| OrchestrationPersistenceError::Corrupt)
}

fn map_read_error(error: rusqlite::Error) -> OrchestrationPersistenceError {
    match error {
        rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::InvalidColumnType(..) => OrchestrationPersistenceError::Corrupt,
        _ => OrchestrationPersistenceError::Storage,
    }
}
