//! The SQLite journal against a temporary database: idempotent creation, exclusive claims,
//! compare-and-set updates, and the one transaction that deletes the rows.

use super::deletion_journal::SqliteDeletionJournal;
use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    DeletionClockPort, DeletionGroupStatus, DeletionJournalPort, DeletionOutcome, DeletionOwner,
    DeletionPhase, DeletionRuntimeEffect, GroupPatch, JournalCreateOutcome, NewDeletionGroup,
    NewDeletionOperation, OperationPatch, SessionDbEffect, SessionRecord, SessionRepository,
    SessionTransactionPort, SessionWorkspace, WorktreeDeletionPolicy, WorktreeEffect,
};
use crate::contexts::sessions::domain::{
    SessionActivation, SessionAggregate, SessionId, SessionLifecycle, SessionOwner,
    SessionPersonalizationMode, SessionSeat, SessionTitle,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::Arc;

struct FixedClock;

impl DeletionClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-09-05T00:00:00Z".to_string()
    }

    fn unix_now(&self) -> i64 {
        1_800_000_000
    }
}

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
    journal: SqliteDeletionJournal,
    repository: SqliteSessionsRepository,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        journal: SqliteDeletionJournal::new(database.clone(), Arc::new(FixedClock)),
        repository: SqliteSessionsRepository::new(database.clone()),
        database,
        _directory: directory,
    }
}

fn record(id: &str) -> SessionRecord {
    SessionRecord {
        personalization_mode: SessionPersonalizationMode::Standard,
        aggregate: SessionAggregate::rehydrate(
            SessionId::parse(id).expect("id"),
            SessionTitle::for_creation(Some(id)),
            SessionLifecycle::Idle,
            SessionOwner::desktop(),
            None,
            false,
            false,
        ),
        agent_id: "codex-cli".to_string(),
        seats: vec![SessionSeat {
            seat_id: "seat-1".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: None,
            role_snapshot: None,
            joined_at: "t".to_string(),
            left_at: None,
            provider_thread_id: None,
        }],
        interaction_mode: "interactive".to_string(),
        workspace: SessionWorkspace {
            folder: Some("/repo".to_string()),
            project_path: Some("/repo".to_string()),
            ..SessionWorkspace::default()
        },
        runtime_session_id: None,
        execution_origin_kind: "user".to_string(),
        execution_origin_id: None,
        created_at: "t".to_string(),
        updated_at: "t".to_string(),
    }
}

fn owner() -> DeletionOwner {
    DeletionOwner {
        instance_id: "me".to_string(),
        epoch: 1,
    }
}

fn operation(id: &str, request_id: &str, hash: &str, sessions: &[&str]) -> NewDeletionOperation {
    NewDeletionOperation {
        operation_id: id.to_string(),
        request_id: request_id.to_string(),
        request_hash: hash.to_string(),
        runtime_effect: DeletionRuntimeEffect::Native,
        owner: owner(),
        created_at: "t0".to_string(),
        operation_task_id: None,
        groups: vec![NewDeletionGroup {
            group_id: format!("{id}-g1"),
            worktree_key: Some("wt-1".to_string()),
            worktree_id: Some("wt-1".to_string()),
            policy: WorktreeDeletionPolicy::RemoveSafe,
            session_ids: sessions.iter().map(|s| (*s).to_string()).collect(),
            retained_path: Some("/repo-feature".to_string()),
            authorization: Some(
                serde_json::json!({ "identity": { "canonicalRoot": "/repo-feature" } }),
            ),
        }],
    }
}

#[test]
fn creation_is_idempotent_per_request_and_claims_every_session() {
    let fixture = fixture("journal-create");
    let created = match fixture
        .journal
        .create(&operation("op-1", "r1", "h1", &["s1", "s2"]))
        .expect("create")
    {
        JournalCreateOutcome::Created(operation) => operation,
        other => panic!("expected created, got {other:?}"),
    };
    assert_eq!(created.outcome, DeletionOutcome::Pending);
    assert_eq!(created.groups.len(), 1);
    assert_eq!(created.groups[0].status, DeletionGroupStatus::Pending);
    assert_eq!(
        created.groups[0].worktree_effect,
        WorktreeEffect::NotRequested
    );
    assert_eq!(
        fixture
            .journal
            .active_claim("s1")
            .unwrap()
            .unwrap()
            .operation_id,
        "op-1"
    );

    assert!(matches!(
        fixture.journal.create(&operation("op-2", "r1", "h1", &["s1", "s2"])).unwrap(),
        JournalCreateOutcome::Existing(existing) if existing.operation_id == "op-1"
    ));
    assert!(matches!(
        fixture
            .journal
            .create(&operation("op-3", "r1", "h2", &["s1", "s2"]))
            .unwrap(),
        JournalCreateOutcome::RequestConflict
    ));
    assert!(matches!(
        fixture.journal.create(&operation("op-4", "r4", "h4", &["s2", "s9"])).unwrap(),
        JournalCreateOutcome::SessionClaimed { session_id, operation_id } if session_id == "s2" && operation_id == "op-1"
    ));
    assert!(
        fixture.journal.load("op-4").unwrap().is_none(),
        "a refused request writes nothing"
    );
    assert_eq!(fixture.journal.list_pending().unwrap().len(), 1);
    let snapshot = fixture
        .journal
        .group_snapshot("op-1", "op-1-g1")
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot.authorization.unwrap()["identity"]["canonicalRoot"],
        "/repo-feature"
    );
}

#[test]
fn group_and_operation_updates_are_compare_and_set() {
    let fixture = fixture("journal-cas");
    fixture
        .journal
        .create(&operation("op-1", "r1", "h1", &["s1"]))
        .unwrap();
    let revision = fixture
        .journal
        .update_group(
            "op-1",
            "op-1-g1",
            1,
            &GroupPatch {
                status: Some(DeletionGroupStatus::Running),
                phase: Some(DeletionPhase::RemovingWorktree),
                worktree_effect: Some(WorktreeEffect::RemoveStarted),
                execution_snapshot: Some(serde_json::json!({ "startedAt": "t1" })),
                attempt: Some(1),
                ..GroupPatch::default()
            },
        )
        .expect("update");
    assert_eq!(revision, 2);
    assert!(fixture
        .journal
        .update_group("op-1", "op-1-g1", 1, &GroupPatch::default())
        .is_err());
    let loaded = fixture.journal.load("op-1").unwrap().unwrap();
    assert_eq!(
        loaded.groups[0].worktree_effect,
        WorktreeEffect::RemoveStarted
    );
    assert_eq!(loaded.groups[0].attempt, 1);
    assert_eq!(loaded.groups[0].revision, 2);
    let snapshot = fixture
        .journal
        .group_snapshot("op-1", "op-1-g1")
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.execution_snapshot.unwrap()["startedAt"], "t1");
    assert!(
        snapshot.authorization.is_some(),
        "an unpatched column is untouched"
    );

    let revision = fixture
        .journal
        .update_operation(
            "op-1",
            1,
            &OperationPatch {
                outcome: Some(DeletionOutcome::NeedsAttention),
                phase: Some(DeletionPhase::Completed),
                error_code: Some(Some("worktree_removal_unknown".to_string())),
                completed: true,
                owner: Some(DeletionOwner {
                    instance_id: "other".to_string(),
                    epoch: 7,
                }),
                last_retry_request_id: Some("retry-1".to_string()),
            },
        )
        .expect("update");
    assert_eq!(revision, 2);
    assert!(fixture
        .journal
        .update_operation(
            "op-1",
            1,
            &OperationPatch {
                outcome: None,
                phase: None,
                error_code: None,
                completed: false,
                owner: None,
                last_retry_request_id: None
            }
        )
        .is_err());
    let loaded = fixture.journal.load("op-1").unwrap().unwrap();
    assert_eq!(loaded.outcome, DeletionOutcome::NeedsAttention);
    assert!(loaded.completed_at.is_some());
    let ownership = fixture.journal.ownership("op-1").unwrap().unwrap();
    assert_eq!(ownership.owner.instance_id, "other");
    assert_eq!(ownership.last_retry_request_id.as_deref(), Some("retry-1"));
    assert!(fixture.journal.list_pending().unwrap().is_empty());
}

#[test]
fn completing_a_group_deletes_the_rows_clears_a_matching_active_session_and_releases_claims() {
    let fixture = fixture("journal-complete");
    fixture
        .repository
        .create_session(&record("s1"), SessionActivation::Activate)
        .expect("s1");
    fixture
        .repository
        .create_session(&record("s2"), SessionActivation::PreserveActive)
        .expect("s2");
    fixture
        .repository
        .create_session(&record("s3"), SessionActivation::PreserveActive)
        .expect("s3");
    assert_eq!(
        fixture
            .repository
            .active_session()
            .unwrap()
            .map(|s| s.id().to_string()),
        Some("s1".to_string())
    );
    fixture
        .journal
        .create(&operation("op-1", "r1", "h1", &["s1", "s2"]))
        .unwrap();

    let completion = fixture
        .journal
        .complete_group_deleting_sessions(
            "op-1",
            "op-1-g1",
            1,
            &["s1".to_string(), "s2".to_string()],
        )
        .expect("complete");
    assert!(completion.active_session_cleared);
    assert!(fixture
        .repository
        .find(&SessionId::parse("s1").unwrap())
        .unwrap()
        .is_none());
    assert!(fixture
        .repository
        .find(&SessionId::parse("s2").unwrap())
        .unwrap()
        .is_none());
    assert!(fixture
        .repository
        .find(&SessionId::parse("s3").unwrap())
        .unwrap()
        .is_some());
    assert!(fixture.repository.active_session().unwrap().is_none());
    assert!(fixture.journal.active_claim("s1").unwrap().is_none());
    let loaded = fixture.journal.load("op-1").unwrap().unwrap();
    assert_eq!(loaded.groups[0].status, DeletionGroupStatus::Succeeded);
    assert_eq!(loaded.groups[0].db_effect, SessionDbEffect::Deleted);

    // Deleting a session that is not active leaves the active selection alone.
    fixture
        .repository
        .create_session(&record("s4"), SessionActivation::Activate)
        .expect("s4");
    fixture
        .journal
        .create(&operation("op-2", "r2", "h2", &["s3"]))
        .unwrap();
    let completion = fixture
        .journal
        .complete_group_deleting_sessions("op-2", "op-2-g1", 1, &["s3".to_string()])
        .expect("complete");
    assert!(!completion.active_session_cleared);
    assert_eq!(
        fixture
            .repository
            .active_session()
            .unwrap()
            .map(|s| s.id().to_string()),
        Some("s4".to_string())
    );

    // A stale revision does nothing, and the journal survives the session rows it describes.
    assert!(fixture
        .journal
        .complete_group_deleting_sessions("op-2", "op-2-g1", 1, &["s4".to_string()])
        .is_err());
    assert!(fixture
        .repository
        .find(&SessionId::parse("s4").unwrap())
        .unwrap()
        .is_some());
    let count: i64 = fixture
        .database
        .connection()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM session_deletion_operations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn reclaiming_a_group_respects_claims_held_by_another_operation() {
    let fixture = fixture("journal-reclaim");
    fixture
        .journal
        .create(&operation("op-1", "r1", "h1", &["s1"]))
        .unwrap();
    fixture
        .journal
        .release_group_claims("op-1", "op-1-g1")
        .unwrap();
    assert!(fixture.journal.active_claim("s1").unwrap().is_none());
    fixture
        .journal
        .create(&operation("op-2", "r2", "h2", &["s1"]))
        .unwrap();
    let conflict = fixture
        .journal
        .reclaim_group("op-1", "op-1-g1", &["s1".to_string()])
        .expect("reclaim")
        .expect("conflict");
    assert_eq!(conflict.operation_id, "op-2");
    fixture
        .journal
        .release_group_claims("op-2", "op-2-g1")
        .unwrap();
    assert!(fixture
        .journal
        .reclaim_group("op-1", "op-1-g1", &["s1".to_string()])
        .unwrap()
        .is_none());
    assert_eq!(
        fixture
            .journal
            .active_claim("s1")
            .unwrap()
            .unwrap()
            .operation_id,
        "op-1"
    );
}
