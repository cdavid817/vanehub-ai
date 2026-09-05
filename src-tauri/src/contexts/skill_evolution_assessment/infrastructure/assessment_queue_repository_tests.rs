use super::*;
use crate::contexts::skill_evolution_assessment::application::{
    AssessmentQueueLane, AssessmentQueueRequest, QueueEnqueueOutcome,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

#[test]
fn deterministic_work_is_claimed_before_optional_work() {
    let (_directory, database) = database_with_seed("assessment-queue-priority");
    let repository = SqliteAssessmentQueueRepository::new(database, 4).expect("queue");
    assert!(matches!(
        repository.enqueue(&request(
            "optional",
            "optional-witness",
            AssessmentQueueLane::OptionalModel,
            100,
        )),
        Ok(QueueEnqueueOutcome::Scheduled { .. })
    ));
    assert!(matches!(
        repository.enqueue(&request(
            "deterministic",
            "deterministic-witness",
            AssessmentQueueLane::Deterministic,
            1,
        )),
        Ok(QueueEnqueueOutcome::Scheduled { .. })
    ));

    let lease = repository
        .claim_next("worker", 20, 100)
        .expect("claim")
        .expect("lease");
    assert_eq!(lease.queue_id, "deterministic");
    assert_eq!(lease.lane, AssessmentQueueLane::Deterministic);
}

#[test]
fn pressure_falls_back_optional_work_before_saturating_deterministic_work() {
    let (_directory, database) = database_with_seed("assessment-queue-pressure");
    let repository = SqliteAssessmentQueueRepository::new(database.clone(), 1).expect("queue");
    repository
        .enqueue(&request(
            "optional",
            "optional-witness",
            AssessmentQueueLane::OptionalModel,
            10,
        ))
        .expect("optional");
    assert_eq!(
        repository.enqueue(&request(
            "optional-2",
            "optional-witness-2",
            AssessmentQueueLane::OptionalModel,
            20,
        )),
        Ok(QueueEnqueueOutcome::OptionalFallback)
    );
    assert_eq!(
        repository.enqueue(&request(
            "deterministic",
            "deterministic-witness",
            AssessmentQueueLane::Deterministic,
            1,
        )),
        Ok(QueueEnqueueOutcome::Scheduled {
            queue_id: "deterministic".to_string()
        })
    );
    assert_eq!(
        repository.enqueue(&request(
            "deterministic-2",
            "deterministic-witness-2",
            AssessmentQueueLane::Deterministic,
            2,
        )),
        Ok(QueueEnqueueOutcome::Saturated)
    );
    let connection = database.connection().expect("connection");
    let states: Vec<(String, String)> = connection
        .prepare("SELECT queue_id,status FROM evolution_assessment_queue_state ORDER BY queue_id")
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("states");
    assert_eq!(
        states,
        vec![
            ("deterministic".to_string(), "queued".to_string()),
            ("optional".to_string(), "fallback".to_string()),
        ]
    );
}

#[test]
fn identical_queue_requests_coalesce() {
    let (_directory, database) = database_with_seed("assessment-queue-coalesce");
    let repository = SqliteAssessmentQueueRepository::new(database, 1).expect("queue");
    let request = request(
        "queue-original",
        "same-witness",
        AssessmentQueueLane::Deterministic,
        1,
    );
    assert!(matches!(
        repository.enqueue(&request),
        Ok(QueueEnqueueOutcome::Scheduled { .. })
    ));
    let mut duplicate = request;
    duplicate.queue_id = "queue-duplicate".to_string();
    assert_eq!(
        repository.enqueue(&duplicate),
        Ok(QueueEnqueueOutcome::Coalesced {
            queue_id: "queue-original".to_string()
        })
    );
}

fn request(
    queue_id: &str,
    witness_hash: &str,
    lane: AssessmentQueueLane,
    priority: i32,
) -> AssessmentQueueRequest {
    AssessmentQueueRequest {
        queue_id: queue_id.to_string(),
        seed_id: "seed-1".to_string(),
        witness_hash: witness_hash.to_string(),
        lane,
        priority,
        available_at_ms: 10,
        created_at_ms: 10,
    }
}

fn database_with_seed(name: &str) -> (TempDirectory, NativeDatabase) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection.execute(
        "INSERT INTO evolution_candidate_seeds (seed_id,seed_version,grouping_key_hash,workspace,category,readiness,readiness_reason,safe_summary,first_occurred_at,last_occurred_at,independent_run_count,signal_set_revision,has_recovery,lineage_summary_json,lineage_signal_count,lineage_truncated_count,created_at) VALUES ('seed-1',1,'group','workspace','verification_outcome','ready','ready','safe','now','now',2,1,1,'{}',1,0,'now')",
        [],
    ).expect("seed");
    drop(connection);
    (directory, database)
}
