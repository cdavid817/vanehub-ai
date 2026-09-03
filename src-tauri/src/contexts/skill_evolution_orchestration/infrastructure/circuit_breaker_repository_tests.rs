use super::*;
use crate::{
    contexts::skill_evolution_orchestration::domain::*, platform::database::NativeDatabase,
    test_support::TempDirectory,
};

fn repository(name: &str) -> (SqliteCircuitBreakerRepository, TempDirectory) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (SqliteCircuitBreakerRepository::new(database), directory)
}

fn signal(
    category: AutomaticFailureCategory,
    application_id: &str,
    occurred_at_ms: i64,
) -> AutomaticFailureSignalV1 {
    AutomaticFailureSignalV1 {
        workspace_id: "workspace-1".into(),
        source_run_id: Some(format!("run-{application_id}")),
        source_application_id: Some(application_id.into()),
        category,
        occurred_at_ms,
    }
}

#[test]
fn severe_failure_opens_once_and_requires_health_then_interactive_ack() {
    let (repository, _directory) = repository("breaker-severe");
    let failure = signal(AutomaticFailureCategory::Integrity, "application-1", 10);
    let opened = repository.record_failure(&failure).expect("open");
    assert!(!opened.duplicate);
    let breaker = opened.breaker.expect("breaker");
    assert_eq!(breaker.status, CircuitBreakerStatus::Open);
    assert!(
        repository
            .record_failure(&failure)
            .expect("duplicate")
            .duplicate
    );

    let healthy = repository
        .record_health(
            "workspace-1",
            None,
            breaker.revision,
            &evaluate_breaker_health_probe(&BreakerHealthProbeInputV1 {
                workspace_id: "workspace-1".into(),
                skill_id: None,
                scanner_healthy: true,
                overlay_integrity_healthy: true,
                curator_audit_healthy: true,
                idempotency_healthy: true,
                storage_healthy: true,
                checked_at_ms: 11,
            })
            .expect("probe"),
        )
        .expect("health");
    assert_eq!(
        repository.acknowledge(
            "workspace-1",
            None,
            healthy.revision,
            EvolutionActorProvenance::SystemPolicy,
            12,
        ),
        Err(CircuitBreakerRepositoryError::Conflict)
    );
    let closed = repository
        .acknowledge(
            "workspace-1",
            None,
            healthy.revision,
            EvolutionActorProvenance::InteractiveUser,
            12,
        )
        .expect("acknowledge");
    assert_eq!(closed.status, CircuitBreakerStatus::Closed);
}

#[test]
fn second_application_failure_inside_24_hours_opens_workspace_breaker() {
    let (repository, _directory) = repository("breaker-window");
    let first = repository
        .record_failure(&signal(AutomaticFailureCategory::Application, "one", 10))
        .expect("first");
    assert!(first.breaker.is_none());
    let second = repository
        .record_failure(&signal(AutomaticFailureCategory::Application, "two", 11))
        .expect("second");
    assert_eq!(
        second.breaker.expect("opened").status,
        CircuitBreakerStatus::Open
    );
}
