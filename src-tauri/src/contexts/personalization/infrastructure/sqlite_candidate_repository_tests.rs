use chrono::{DateTime, Duration, TimeZone, Utc};
use tempfile::TempDir;

use super::sqlite_candidate_repository::SqliteCandidateRepository;
use super::sqlite_migration_state::SqliteMigrationState;
use crate::contexts::personalization::application::{
    CandidateRepository, MigrationStatePort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    AgentId, ArchiveMemoryCandidate, CandidateReviewStatus, CreateMemoryCandidate, MemoryAudience,
    MemoryCandidate, MemoryCandidateOperation, MemoryId, MemoryProvenance, MemoryScope,
    MemorySource, MemoryType, MigrationState, SessionId, UpdateMemoryCandidate, WorkspaceKey,
};
use crate::platform::database::NativeDatabase;

struct Fixture {
    _directory: TempDir,
    candidates: SqliteCandidateRepository,
    migration_state: SqliteMigrationState,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-candidate-{label}-"))
        .expect("temporary directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        _directory: directory,
        candidates: SqliteCandidateRepository::new(database.clone()),
        migration_state: SqliteMigrationState::new(database),
    }
}

fn base_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn candidate_id(index: usize) -> MemoryId {
    MemoryId::parse(&format!("01K9CAND{index:018}")).expect("candidate id")
}

fn target_id() -> MemoryId {
    MemoryId::parse("01K2ABCDEFGHJKMNPQRSTVWXYZ").expect("memory id")
}

fn create_candidate(index: usize) -> MemoryCandidate {
    MemoryCandidate {
        id: candidate_id(index),
        operation: MemoryCandidateOperation::Create(CreateMemoryCandidate {
            name: format!("Proposal {index}"),
            description: "extracted from a completed turn".to_string(),
            memory_type: MemoryType::Project,
            content: "The user prefers npm.".to_string(),
            scope: MemoryScope::Workspace {
                workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
            },
            audience: MemoryAudience::SelectedAgents {
                agent_ids: vec![AgentId::parse("claude-code").expect("agent")],
            },
        }),
        source: MemorySource::CliAutomatic,
        provenance: MemoryProvenance {
            source_agent_id: Some(AgentId::parse("claude-code").expect("agent")),
            source_session_id: Some(SessionId::parse("ses_1").expect("session")),
            source_message_id: Some("msg_1".to_string()),
            source_workspace_key: None,
            ..MemoryProvenance::default()
        },
        status: CandidateReviewStatus::Pending,
        created_at: base_time() + Duration::minutes(index as i64),
    }
}

#[test]
fn a_create_candidate_round_trips_with_its_provenance() {
    let fixture = fixture("create");
    let subject = create_candidate(1);
    fixture.candidates.insert(&subject).expect("insert");

    let loaded = fixture
        .candidates
        .get(&candidate_id(1))
        .expect("get")
        .expect("candidate exists");
    assert_eq!(loaded, subject);
    assert!(loaded.is_pending());
}

#[test]
fn update_and_archive_candidates_preserve_their_target_revision() {
    // The revision is the whole point of the proposal: without it, approving minutes later could
    // overwrite an edit the user made in between.
    let fixture = fixture("targets");
    let update = MemoryCandidate {
        id: candidate_id(2),
        operation: MemoryCandidateOperation::Update(UpdateMemoryCandidate {
            target_id: target_id(),
            expected_target_revision: 4,
            name: None,
            description: None,
            content: Some("corrected body".to_string()),
        }),
        ..create_candidate(2)
    };
    let archive = MemoryCandidate {
        id: candidate_id(3),
        operation: MemoryCandidateOperation::Archive(ArchiveMemoryCandidate {
            target_id: target_id(),
            expected_target_revision: 7,
        }),
        ..create_candidate(3)
    };
    fixture.candidates.insert(&update).expect("insert update");
    fixture.candidates.insert(&archive).expect("insert archive");

    let loaded_update = fixture
        .candidates
        .get(&candidate_id(2))
        .expect("get")
        .expect("exists");
    assert_eq!(loaded_update, update);
    assert_eq!(loaded_update.check_target_revision(4), Ok(()));
    assert!(loaded_update.check_target_revision(5).is_err());

    let loaded_archive = fixture
        .candidates
        .get(&candidate_id(3))
        .expect("get")
        .expect("exists");
    assert_eq!(loaded_archive, archive);
    assert!(loaded_archive.check_target_revision(8).is_err());
}

#[test]
fn only_pending_candidates_are_listed_and_counted() {
    let fixture = fixture("pending");
    for index in 1..=3 {
        fixture
            .candidates
            .insert(&create_candidate(index))
            .expect("insert");
    }
    assert_eq!(fixture.candidates.count_pending().expect("count"), 3);

    fixture
        .candidates
        .mark_reviewed(
            &candidate_id(2),
            CandidateReviewStatus::Approved,
            base_time(),
        )
        .expect("approve");
    assert_eq!(fixture.candidates.count_pending().expect("count"), 2);

    let pending = fixture.candidates.list_pending(10).expect("list");
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(MemoryCandidate::is_pending));
    // Oldest first: the review queue is a queue.
    assert_eq!(pending[0].id, candidate_id(1));
    assert_eq!(pending[1].id, candidate_id(3));
}

#[test]
fn listing_pending_candidates_respects_its_limit() {
    let fixture = fixture("limit");
    for index in 1..=5 {
        fixture
            .candidates
            .insert(&create_candidate(index))
            .expect("insert");
    }
    assert_eq!(fixture.candidates.list_pending(2).expect("list").len(), 2);
}

#[test]
fn marking_an_unknown_candidate_reviewed_is_not_found() {
    let fixture = fixture("unknown");
    let error = fixture
        .candidates
        .mark_reviewed(
            &candidate_id(9),
            CandidateReviewStatus::Rejected,
            base_time(),
        )
        .expect_err("no such candidate");
    assert!(matches!(error, PersonalizationApplicationError::NotFound));
}

#[test]
fn a_review_outcome_cannot_be_pending() {
    let fixture = fixture("pending-outcome");
    fixture
        .candidates
        .insert(&create_candidate(1))
        .expect("insert");
    assert!(fixture
        .candidates
        .mark_reviewed(
            &candidate_id(1),
            CandidateReviewStatus::Pending,
            base_time()
        )
        .is_err());
}

#[test]
fn pruning_drops_reviewed_bodies_beyond_the_retention_bound() {
    let fixture = fixture("prune");
    for index in 1..=4 {
        fixture
            .candidates
            .insert(&create_candidate(index))
            .expect("insert");
        fixture
            .candidates
            .mark_reviewed(
                &candidate_id(index),
                CandidateReviewStatus::Rejected,
                base_time() + Duration::minutes(index as i64),
            )
            .expect("reject");
    }

    let pruned = fixture.candidates.prune_reviewed(2).expect("prune");
    assert_eq!(pruned, 2, "the two oldest reviewed bodies are dropped");

    // The newest two keep their proposed body.
    for index in 3..=4 {
        let candidate = fixture
            .candidates
            .get(&candidate_id(index))
            .expect("get")
            .expect("exists");
        match candidate.operation {
            MemoryCandidateOperation::Create(create) => {
                assert_eq!(create.content, "The user prefers npm.")
            }
            other => panic!("unexpected operation {other:?}"),
        }
    }

    // The pruned ones keep their audit metadata but lose the text the user declined to keep.
    for index in 1..=2 {
        let candidate = fixture
            .candidates
            .get(&candidate_id(index))
            .expect("get")
            .expect("the audit row survives");
        assert_eq!(candidate.status, CandidateReviewStatus::Rejected);
        match candidate.operation {
            MemoryCandidateOperation::Create(create) => {
                assert!(create.content.is_empty(), "the proposed body is gone");
                assert_eq!(create.name, format!("Proposal {index}"));
            }
            other => panic!("unexpected operation {other:?}"),
        }
    }

    // Pruning again finds nothing left to prune.
    assert_eq!(fixture.candidates.prune_reviewed(2).expect("prune"), 0);
}

#[test]
fn pruning_never_touches_a_pending_candidate() {
    let fixture = fixture("prune-pending");
    fixture
        .candidates
        .insert(&create_candidate(1))
        .expect("insert");

    assert_eq!(fixture.candidates.prune_reviewed(0).expect("prune"), 0);
    let candidate = fixture
        .candidates
        .get(&candidate_id(1))
        .expect("get")
        .expect("exists");
    match candidate.operation {
        MemoryCandidateOperation::Create(create) => {
            assert_eq!(create.content, "The user prefers npm.")
        }
        other => panic!("unexpected operation {other:?}"),
    }
}

#[test]
fn the_migration_state_row_is_seeded_and_round_trips() {
    let fixture = fixture("migration-state");
    let initial = fixture.migration_state.load().expect("load");
    assert_eq!(initial, MigrationState::not_started());
    assert!(!initial.is_complete());

    let updated = MigrationState {
        generation: 3,
        started_at: Some(base_time()),
        completed_at: Some(base_time() + Duration::seconds(30)),
        last_error_code: Some("quarantined_entries".to_string()),
        repair_required: true,
    };
    fixture.migration_state.save(&updated).expect("save");

    let reloaded = fixture.migration_state.load().expect("reload");
    assert_eq!(reloaded, updated);
    assert!(reloaded.is_complete());
}
