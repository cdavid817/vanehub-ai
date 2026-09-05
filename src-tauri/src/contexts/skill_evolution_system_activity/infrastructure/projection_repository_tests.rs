use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;
use rusqlite::Connection;

#[test]
fn global_lease_is_exclusive_renewable_and_takeover_requires_expiry() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);

    let first = repository
        .acquire_lease("projector-one", 10, 100)
        .expect("first lease");
    assert_eq!(first.revision, 1);
    assert_eq!(
        repository.acquire_lease("projector-two", 20, 120),
        Err(ActivityProjectionRepositoryError::LeaseHeld)
    );
    assert_eq!(
        repository.heartbeat_lease("projector-one", 0, 30, 130),
        Err(ActivityProjectionRepositoryError::Conflict)
    );

    let renewed = repository
        .heartbeat_lease("projector-one", 1, 30, 130)
        .expect("renewed lease");
    assert_eq!(renewed.revision, 2);
    let takeover = repository
        .acquire_lease("projector-two", 130, 230)
        .expect("expired takeover");
    assert_eq!(takeover.owner_id, "projector-two");
    assert_eq!(takeover.revision, 3);
}

#[test]
fn domain_checkpoints_are_optimistic_monotonic_and_failure_isolated() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);

    let evidence = repository
        .checkpoint(&checkpoint(EvolutionSourceDomain::Evidence, 3, 0))
        .expect("evidence checkpoint");
    let assessment = repository
        .checkpoint(&checkpoint(EvolutionSourceDomain::Assessment, 5, 0))
        .expect("assessment checkpoint");
    assert_eq!(evidence.revision, 1);
    assert_eq!(assessment.revision, 1);

    let failed = repository
        .record_failure(
            EvolutionSourceDomain::Assessment,
            Some(ActivityGapCode::MissingSequence),
            ActivityProjectionFailureCode::IntegrityFailed,
            1,
        )
        .expect("isolated failure");
    assert_eq!(failed.gap, Some(ActivityGapCode::MissingSequence));
    assert_eq!(failed.revision, 2);
    assert_eq!(
        repository
            .cursor(EvolutionSourceDomain::Evidence)
            .expect("evidence cursor")
            .expect("evidence state"),
        evidence
    );

    assert_eq!(
        repository.checkpoint(&checkpoint(EvolutionSourceDomain::Assessment, 6, 1)),
        Err(ActivityProjectionRepositoryError::Conflict)
    );
    let recovered = repository
        .checkpoint(&checkpoint(EvolutionSourceDomain::Assessment, 6, 2))
        .expect("recovered checkpoint");
    assert_eq!(recovered.gap, None);
    assert_eq!(recovered.failure_code, None);
    assert_eq!(recovered.last_sequence, 6);
}

#[test]
fn failure_state_can_be_created_before_a_domain_has_a_checkpoint() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);

    let failed = repository
        .record_failure(
            EvolutionSourceDomain::Retention,
            Some(ActivityGapCode::RetentionFloorAdvanced),
            ActivityProjectionFailureCode::InvalidCursor,
            0,
        )
        .expect("initial failure state");

    assert_eq!(failed.last_sequence, 0);
    assert_eq!(failed.gap, Some(ActivityGapCode::RetentionFloorAdvanced));
    assert_eq!(
        failed.failure_code,
        Some(ActivityProjectionFailureCode::InvalidCursor)
    );
}

fn checkpoint(
    source_domain: EvolutionSourceDomain,
    last_sequence: u64,
    expected_revision: u64,
) -> ActivityDomainCheckpoint {
    ActivityDomainCheckpoint {
        source_domain,
        opaque_cursor: OpaqueDomainCursor::parse(format!("cursor:{last_sequence}"))
            .expect("cursor"),
        last_sequence,
        last_source_hash: format!("sha256:{last_sequence}"),
        retention_floor: Some(OpaqueDomainCursor::parse("floor:1".into()).expect("floor")),
        pending_count: 2,
        oldest_pending_at_ms: Some(50),
        last_success_at_ms: 100,
        expected_revision,
    }
}
