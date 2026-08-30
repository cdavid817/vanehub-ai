use super::CircuitBreakerRepositoryError;
use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, AutoApplyCircuitBreakerV1, AutomaticFailureCategory,
    AutomaticFailureSignalV1, BreakerTransitionError, CircuitBreakerStatus,
    EvolutionActorProvenance, BREAKER_HEALTH_CHECK_VERSION_V1, ROLLING_DAY_MS,
};
use rusqlite::{params, OptionalExtension, Transaction};

pub(super) fn validate_signal(
    value: &AutomaticFailureSignalV1,
) -> Result<(), CircuitBreakerRepositoryError> {
    validate_scope(&value.workspace_id, None)?;
    if value.occurred_at_ms < 0
        || value
            .source_run_id
            .as_ref()
            .is_some_and(|id| !is_safe_identifier(id, 256))
        || value
            .source_application_id
            .as_ref()
            .is_some_and(|id| !is_safe_identifier(id, 256))
        || value.source_run_id.is_none() && value.source_application_id.is_none()
    {
        return Err(CircuitBreakerRepositoryError::InvalidInput);
    }
    Ok(())
}

pub(super) fn validate_scope(
    workspace_id: &str,
    skill_id: Option<&str>,
) -> Result<(), CircuitBreakerRepositoryError> {
    if !is_safe_identifier(workspace_id, 256)
        || skill_id.is_some_and(|id| !is_safe_identifier(id, 256))
    {
        return Err(CircuitBreakerRepositoryError::InvalidInput);
    }
    Ok(())
}

pub(super) fn application_failure_times(
    transaction: &Transaction<'_>,
    signal: &AutomaticFailureSignalV1,
) -> Result<Vec<i64>, CircuitBreakerRepositoryError> {
    if signal.category != AutomaticFailureCategory::Application {
        return Ok(vec![]);
    }
    let mut statement = transaction
        .prepare(
            "SELECT occurred_at_ms FROM evolution_auto_application_failures
             WHERE workspace_id=?1 AND category='application_failure_threshold'
             AND occurred_at_ms>=?2 AND occurred_at_ms<=?3 ORDER BY occurred_at_ms,failure_id",
        )
        .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
    let result = statement
        .query_map(
            params![
                signal.workspace_id,
                signal.occurred_at_ms.saturating_sub(ROLLING_DAY_MS),
                signal.occurred_at_ms,
            ],
            |row| row.get(0),
        )
        .map_err(|_| CircuitBreakerRepositoryError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CircuitBreakerRepositoryError::Storage);
    result
}

pub(super) fn failures_before_current(
    failures: &[i64],
    signal: &AutomaticFailureSignalV1,
) -> Vec<i64> {
    if signal.category == AutomaticFailureCategory::Application {
        failures
            .iter()
            .copied()
            .take(failures.len().saturating_sub(1))
            .collect()
    } else {
        vec![]
    }
}

pub(super) fn closed_breaker(workspace_id: &str) -> AutoApplyCircuitBreakerV1 {
    AutoApplyCircuitBreakerV1 {
        breaker_id: format!("workspace-breaker-{workspace_id}"),
        workspace_id: workspace_id.into(),
        skill_id: None,
        status: CircuitBreakerStatus::Closed,
        safe_cause_code: None,
        source_run_id: None,
        source_application_id: None,
        health_check_version: BREAKER_HEALTH_CHECK_VERSION_V1.into(),
        health_probe_passed: false,
        acknowledged_by: None,
        opened_at_ms: None,
        updated_at_ms: 0,
        revision: 0,
    }
}

pub(super) fn persist(
    transaction: &Transaction<'_>,
    value: &AutoApplyCircuitBreakerV1,
    expected_revision: u64,
) -> Result<(), CircuitBreakerRepositoryError> {
    let acknowledged_actor = value.acknowledged_by.map(actor_name);
    let revision =
        i64::try_from(value.revision).map_err(|_| CircuitBreakerRepositoryError::InvalidInput)?;
    let expected_revision = i64::try_from(expected_revision)
        .map_err(|_| CircuitBreakerRepositoryError::InvalidInput)?;
    let changed = transaction
        .execute(
            "INSERT INTO evolution_auto_breakers VALUES
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(breaker_id) DO UPDATE SET status=excluded.status,
             safe_cause_code=excluded.safe_cause_code,source_run_id=excluded.source_run_id,
             source_application_id=excluded.source_application_id,
             health_check_version=excluded.health_check_version,
             health_probe_passed=excluded.health_probe_passed,
             acknowledged_actor=excluded.acknowledged_actor,opened_at_ms=excluded.opened_at_ms,
             updated_at_ms=excluded.updated_at_ms,revision=excluded.revision
             WHERE evolution_auto_breakers.revision=?14",
            params![
                value.breaker_id,
                value.workspace_id,
                value.skill_id,
                status_name(value.status),
                value.safe_cause_code,
                value.source_run_id,
                value.source_application_id,
                value.health_check_version,
                value.health_probe_passed,
                acknowledged_actor,
                value.opened_at_ms,
                value.updated_at_ms,
                revision,
                expected_revision,
            ],
        )
        .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
    if changed != 1 {
        return Err(CircuitBreakerRepositoryError::Conflict);
    }
    Ok(())
}

pub(super) fn load(
    transaction: &Transaction<'_>,
    workspace_id: &str,
    skill_id: Option<&str>,
) -> Result<Option<AutoApplyCircuitBreakerV1>, CircuitBreakerRepositoryError> {
    transaction
        .query_row(
            "SELECT breaker_id,workspace_id,skill_id,status,safe_cause_code,source_run_id,
             source_application_id,health_check_version,health_probe_passed,acknowledged_actor,
             opened_at_ms,updated_at_ms,revision FROM evolution_auto_breakers
             WHERE workspace_id=?1 AND skill_id IS ?2",
            params![workspace_id, skill_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            },
        )
        .optional()
        .map_err(|_| CircuitBreakerRepositoryError::Storage)?
        .map(from_row)
        .transpose()
}

type BreakerRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    bool,
    Option<String>,
    Option<i64>,
    i64,
    i64,
);

fn from_row(row: BreakerRow) -> Result<AutoApplyCircuitBreakerV1, CircuitBreakerRepositoryError> {
    Ok(AutoApplyCircuitBreakerV1 {
        breaker_id: row.0,
        workspace_id: row.1,
        skill_id: row.2,
        status: parse_status(&row.3)?,
        safe_cause_code: row.4,
        source_run_id: row.5,
        source_application_id: row.6,
        health_check_version: row.7,
        health_probe_passed: row.8,
        acknowledged_by: row.9.as_deref().map(parse_actor).transpose()?,
        opened_at_ms: row.10,
        updated_at_ms: row.11,
        revision: u64::try_from(row.12).map_err(|_| CircuitBreakerRepositoryError::Storage)?,
    })
}

fn status_name(value: CircuitBreakerStatus) -> &'static str {
    match value {
        CircuitBreakerStatus::Closed => "closed",
        CircuitBreakerStatus::Open => "open",
        CircuitBreakerStatus::AwaitingHealth => "awaiting_health",
        CircuitBreakerStatus::AwaitingAcknowledgement => "awaiting_acknowledgement",
    }
}
fn parse_status(value: &str) -> Result<CircuitBreakerStatus, CircuitBreakerRepositoryError> {
    match value {
        "closed" => Ok(CircuitBreakerStatus::Closed),
        "open" => Ok(CircuitBreakerStatus::Open),
        "awaiting_health" => Ok(CircuitBreakerStatus::AwaitingHealth),
        "awaiting_acknowledgement" => Ok(CircuitBreakerStatus::AwaitingAcknowledgement),
        _ => Err(CircuitBreakerRepositoryError::Storage),
    }
}
fn actor_name(value: EvolutionActorProvenance) -> &'static str {
    match value {
        EvolutionActorProvenance::InteractiveUser => "interactive_user",
        EvolutionActorProvenance::SystemPolicy => "system_policy",
        EvolutionActorProvenance::RuntimeTrigger => "runtime_trigger",
        EvolutionActorProvenance::Recovery => "recovery",
        EvolutionActorProvenance::WebMock => "web_mock",
    }
}
fn parse_actor(value: &str) -> Result<EvolutionActorProvenance, CircuitBreakerRepositoryError> {
    match value {
        "interactive_user" => Ok(EvolutionActorProvenance::InteractiveUser),
        "system_policy" => Ok(EvolutionActorProvenance::SystemPolicy),
        "runtime_trigger" => Ok(EvolutionActorProvenance::RuntimeTrigger),
        "recovery" => Ok(EvolutionActorProvenance::Recovery),
        "web_mock" => Ok(EvolutionActorProvenance::WebMock),
        _ => Err(CircuitBreakerRepositoryError::Storage),
    }
}
pub(super) fn map_transition(_: BreakerTransitionError) -> CircuitBreakerRepositoryError {
    CircuitBreakerRepositoryError::Conflict
}
