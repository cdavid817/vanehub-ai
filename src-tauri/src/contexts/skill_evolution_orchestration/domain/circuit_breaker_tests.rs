use super::*;

fn breaker() -> AutoApplyCircuitBreakerV1 {
    AutoApplyCircuitBreakerV1 {
        breaker_id: "breaker-1".into(),
        workspace_id: "workspace-1".into(),
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

fn signal(category: AutomaticFailureCategory, occurred_at_ms: i64) -> AutomaticFailureSignalV1 {
    AutomaticFailureSignalV1 {
        workspace_id: "workspace-1".into(),
        source_run_id: Some("run-1".into()),
        source_application_id: Some("application-1".into()),
        category,
        occurred_at_ms,
    }
}

#[test]
fn severe_categories_open_immediately_and_application_failures_require_two_in_24h() {
    for category in [
        AutomaticFailureCategory::Security,
        AutomaticFailureCategory::Integrity,
        AutomaticFailureCategory::Audit,
        AutomaticFailureCategory::Idempotency,
    ] {
        assert!(should_open_workspace_breaker(&signal(category, 100), &[]));
    }
    let application = signal(AutomaticFailureCategory::Application, ROLLING_DAY_MS + 100);
    assert!(!should_open_workspace_breaker(&application, &[]));
    assert!(!should_open_workspace_breaker(&application, &[50]));
    assert!(should_open_workspace_breaker(&application, &[101]));
}

#[test]
fn closing_requires_current_health_and_interactive_acknowledgement() {
    let opened =
        open_breaker(&breaker(), &signal(AutomaticFailureCategory::Integrity, 10)).expect("open");
    let unhealthy = record_breaker_health(&opened, false, BREAKER_HEALTH_CHECK_VERSION_V1, 11)
        .expect("unhealthy probe");
    assert_eq!(unhealthy.status, CircuitBreakerStatus::AwaitingHealth);
    assert_eq!(
        acknowledge_breaker(&unhealthy, EvolutionActorProvenance::InteractiveUser, 12),
        Err(BreakerTransitionError::HealthAndAcknowledgementRequired)
    );
    let healthy = record_breaker_health(&unhealthy, true, BREAKER_HEALTH_CHECK_VERSION_V1, 13)
        .expect("healthy probe");
    assert_eq!(
        acknowledge_breaker(&healthy, EvolutionActorProvenance::SystemPolicy, 14),
        Err(BreakerTransitionError::HealthAndAcknowledgementRequired)
    );
    let closed = acknowledge_breaker(&healthy, EvolutionActorProvenance::InteractiveUser, 14)
        .expect("interactive acknowledgement");
    assert_eq!(closed.status, CircuitBreakerStatus::Closed);
}

#[test]
fn health_probe_is_deterministic_and_every_check_is_mandatory() {
    let input = BreakerHealthProbeInputV1 {
        workspace_id: "workspace-1".into(),
        skill_id: None,
        scanner_healthy: true,
        overlay_integrity_healthy: true,
        curator_audit_healthy: true,
        idempotency_healthy: true,
        storage_healthy: true,
        checked_at_ms: 20,
    };
    let first = evaluate_breaker_health_probe(&input).expect("probe");
    assert_eq!(
        first,
        evaluate_breaker_health_probe(&input).expect("repeat")
    );
    assert!(first.passed);
    let failed = evaluate_breaker_health_probe(&BreakerHealthProbeInputV1 {
        curator_audit_healthy: false,
        ..input
    })
    .expect("failed probe");
    assert!(!failed.passed);
    assert_ne!(failed.proof_hash, first.proof_hash);
}
