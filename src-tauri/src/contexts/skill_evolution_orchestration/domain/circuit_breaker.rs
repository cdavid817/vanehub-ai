use super::{
    canonical_hash, is_safe_identifier, AutoApplyCircuitBreakerV1, CircuitBreakerStatus,
    EvolutionActorProvenance, ROLLING_DAY_MS,
};

pub(crate) const BREAKER_HEALTH_CHECK_VERSION_V1: &str = "auto-apply-health-v1";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BreakerHealthProbeInputV1 {
    pub(crate) workspace_id: String,
    pub(crate) skill_id: Option<String>,
    pub(crate) scanner_healthy: bool,
    pub(crate) overlay_integrity_healthy: bool,
    pub(crate) curator_audit_healthy: bool,
    pub(crate) idempotency_healthy: bool,
    pub(crate) storage_healthy: bool,
    pub(crate) checked_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BreakerHealthProbeV1 {
    pub(crate) workspace_id: String,
    pub(crate) skill_id: Option<String>,
    pub(crate) version: String,
    pub(crate) passed: bool,
    pub(crate) proof_hash: String,
    pub(crate) checked_at_ms: i64,
}

pub(crate) fn evaluate_breaker_health_probe(
    input: &BreakerHealthProbeInputV1,
) -> Result<BreakerHealthProbeV1, BreakerTransitionError> {
    if !is_safe_identifier(&input.workspace_id, 256)
        || input
            .skill_id
            .as_ref()
            .is_some_and(|id| !is_safe_identifier(id, 256))
        || input.checked_at_ms < 0
    {
        return Err(BreakerTransitionError::Conflict);
    }
    let passed = input.scanner_healthy
        && input.overlay_integrity_healthy
        && input.curator_audit_healthy
        && input.idempotency_healthy
        && input.storage_healthy;
    let proof_hash = canonical_hash(&(BREAKER_HEALTH_CHECK_VERSION_V1, input))
        .map_err(|_| BreakerTransitionError::Conflict)?;
    Ok(BreakerHealthProbeV1 {
        workspace_id: input.workspace_id.clone(),
        skill_id: input.skill_id.clone(),
        version: BREAKER_HEALTH_CHECK_VERSION_V1.into(),
        passed,
        proof_hash,
        checked_at_ms: input.checked_at_ms,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutomaticFailureCategory {
    Security,
    Integrity,
    Audit,
    Idempotency,
    Application,
}

impl AutomaticFailureCategory {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Security => "security_failure",
            Self::Integrity => "integrity_failure",
            Self::Audit => "audit_failure",
            Self::Idempotency => "idempotency_failure",
            Self::Application => "application_failure_threshold",
        }
    }

    pub(crate) const fn opens_immediately(self) -> bool {
        !matches!(self, Self::Application)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutomaticFailureSignalV1 {
    pub(crate) workspace_id: String,
    pub(crate) source_run_id: Option<String>,
    pub(crate) source_application_id: Option<String>,
    pub(crate) category: AutomaticFailureCategory,
    pub(crate) occurred_at_ms: i64,
}

pub(crate) fn should_open_workspace_breaker(
    signal: &AutomaticFailureSignalV1,
    prior_application_failure_times_ms: &[i64],
) -> bool {
    if signal.category.opens_immediately() {
        return true;
    }
    let window_start = signal.occurred_at_ms.saturating_sub(ROLLING_DAY_MS);
    prior_application_failure_times_ms
        .iter()
        .filter(|time| (window_start..=signal.occurred_at_ms).contains(*time))
        .count()
        >= 1
}

pub(crate) fn open_breaker(
    current: &AutoApplyCircuitBreakerV1,
    signal: &AutomaticFailureSignalV1,
) -> Result<AutoApplyCircuitBreakerV1, BreakerTransitionError> {
    if current.workspace_id != signal.workspace_id || signal.occurred_at_ms < current.updated_at_ms
    {
        return Err(BreakerTransitionError::Conflict);
    }
    Ok(AutoApplyCircuitBreakerV1 {
        status: CircuitBreakerStatus::Open,
        safe_cause_code: Some(signal.category.code().into()),
        source_run_id: signal.source_run_id.clone(),
        source_application_id: signal.source_application_id.clone(),
        health_check_version: BREAKER_HEALTH_CHECK_VERSION_V1.into(),
        health_probe_passed: false,
        acknowledged_by: None,
        opened_at_ms: current.opened_at_ms.or(Some(signal.occurred_at_ms)),
        updated_at_ms: signal.occurred_at_ms,
        revision: current.revision.saturating_add(1),
        ..current.clone()
    })
}

pub(crate) fn record_breaker_health(
    current: &AutoApplyCircuitBreakerV1,
    passed: bool,
    health_check_version: &str,
    now_ms: i64,
) -> Result<AutoApplyCircuitBreakerV1, BreakerTransitionError> {
    if current.status == CircuitBreakerStatus::Closed
        || health_check_version != BREAKER_HEALTH_CHECK_VERSION_V1
        || now_ms < current.updated_at_ms
    {
        return Err(BreakerTransitionError::Conflict);
    }
    Ok(AutoApplyCircuitBreakerV1 {
        status: if passed {
            CircuitBreakerStatus::AwaitingAcknowledgement
        } else {
            CircuitBreakerStatus::AwaitingHealth
        },
        health_probe_passed: passed,
        acknowledged_by: None,
        updated_at_ms: now_ms,
        revision: current.revision.saturating_add(1),
        ..current.clone()
    })
}

pub(crate) fn acknowledge_breaker(
    current: &AutoApplyCircuitBreakerV1,
    actor: EvolutionActorProvenance,
    now_ms: i64,
) -> Result<AutoApplyCircuitBreakerV1, BreakerTransitionError> {
    if current.status != CircuitBreakerStatus::AwaitingAcknowledgement
        || !current.health_probe_passed
        || actor != EvolutionActorProvenance::InteractiveUser
        || now_ms < current.updated_at_ms
    {
        return Err(BreakerTransitionError::HealthAndAcknowledgementRequired);
    }
    Ok(AutoApplyCircuitBreakerV1 {
        status: CircuitBreakerStatus::Closed,
        acknowledged_by: Some(actor),
        updated_at_ms: now_ms,
        revision: current.revision.saturating_add(1),
        ..current.clone()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BreakerTransitionError {
    Conflict,
    HealthAndAcknowledgementRequired,
}
