use rusqlite::params;

use super::*;
use crate::contexts::skill_evolution_evidence::domain::FeedbackState;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const KEY: &[u8; 32] = &[17; 32];

fn fixture(name: &str) -> (NativeDatabase, SqliteEvolutionEvidenceRepository) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection.execute(
        "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state, folder, source_kind, pinned, archived, created_at, updated_at) VALUES ('session-feedback', 'Feedback', 'onepiece', 'agent', 'active', 'workspace-a', 'local', 0, 0, '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
        [],
    ).expect("session");
    connection.execute(
        "INSERT INTO messages (id, session_id, role, status, content, tool_use, rich_blocks, file_references, created_at, updated_at, session_sequence) VALUES ('assistant-feedback', 'session-feedback', 'assistant', 'completed', 'Answer', '[]', '[]', '[]', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z', 1)",
        [],
    ).expect("message");
    drop(connection);
    let repository = SqliteEvolutionEvidenceRepository::new(database.clone());
    (database, repository)
}

fn request(
    expected_revision: u64,
    state: Option<FeedbackState>,
    correction_note: Option<&str>,
) -> SaveFeedbackRequest {
    SaveFeedbackRequest {
        message_id: "assistant-feedback".to_string(),
        expected_revision,
        state,
        correction_note: correction_note.map(str::to_string),
    }
}

#[test]
fn create_replace_and_clear_use_monotonic_compare_and_swap_versions() {
    let (database, repository) = fixture("feedback-cas");
    let created = repository
        .save_feedback(&request(0, Some(FeedbackState::Helpful), None), KEY)
        .expect("create");
    assert_eq!(created.revision, 1);
    assert_eq!(created.state, Some(FeedbackState::Helpful));

    assert_eq!(
        repository.save_feedback(&request(0, Some(FeedbackState::Unhelpful), None), KEY),
        Err(FeedbackTransitionError::Conflict {
            current_revision: 1
        })
    );
    let replaced = repository
        .save_feedback(
            &request(
                1,
                Some(FeedbackState::Corrected),
                Some("Email alice@example.com must not persist."),
            ),
            KEY,
        )
        .expect("replace");
    assert_eq!(replaced.revision, 2);
    assert!(replaced
        .sanitized_note
        .as_deref()
        .is_some_and(|note| !note.contains("alice@example.com")));
    let cleared = repository
        .save_feedback(&request(2, None, None), KEY)
        .expect("clear");
    assert_eq!(cleared.revision, 3);

    let connection = database.connection().expect("connection");
    let (events, current, active, superseded): (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM evolution_feedback_events), (SELECT COUNT(*) FROM evolution_feedback_current), (SELECT COUNT(*) FROM evolution_signals WHERE lineage_status = 'active'), (SELECT COUNT(*) FROM evolution_signals WHERE lineage_status = 'superseded')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("counts");
    assert_eq!((events, current, active, superseded), (3, 0, 0, 2));
}

#[test]
fn feedback_and_signal_roll_back_together_on_evidence_failure() {
    let (database, repository) = fixture("feedback-atomic-failure");
    database
        .connection()
        .expect("connection")
        .execute_batch(
            "CREATE TRIGGER fail_feedback_signal BEFORE INSERT ON evolution_signals BEGIN SELECT RAISE(ABORT, 'fail'); END;",
        )
        .expect("trigger");
    assert_eq!(
        repository.save_feedback(&request(0, Some(FeedbackState::Helpful), None), KEY),
        Err(FeedbackTransitionError::Storage)
    );
    let connection = database.connection().expect("connection");
    for table in ["evolution_feedback_current", "evolution_feedback_events"] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 0, "{table}");
    }
}

#[test]
fn only_completed_assistant_messages_and_valid_correction_shapes_are_eligible() {
    let (database, repository) = fixture("feedback-eligibility");
    database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE messages SET status = 'streaming' WHERE id = ?1",
            params!["assistant-feedback"],
        )
        .expect("streaming");
    assert_eq!(
        repository.save_feedback(&request(0, Some(FeedbackState::Helpful), None), KEY),
        Err(FeedbackTransitionError::MessageNotEligible)
    );
    assert_eq!(
        repository.save_feedback(&request(0, Some(FeedbackState::Corrected), None), KEY),
        Err(FeedbackTransitionError::InvalidInput)
    );
}

#[test]
fn batched_feedback_lookup_preserves_state_revision_and_cleared_history() {
    let (_, repository) = fixture("feedback-batched-lookup");
    repository
        .save_feedback(&request(0, Some(FeedbackState::Helpful), None), KEY)
        .expect("create feedback");

    let summaries = repository
        .feedback_for_messages(&[
            "assistant-feedback".to_owned(),
            "missing-message".to_owned(),
        ])
        .expect("lookup feedback");
    let current = summaries
        .get("assistant-feedback")
        .expect("current feedback");
    assert_eq!(current.state, Some(FeedbackState::Helpful));
    assert_eq!(current.revision, 1);
    assert!(!summaries.contains_key("missing-message"));

    repository
        .save_feedback(&request(1, None, None), KEY)
        .expect("clear feedback");
    let cleared = repository
        .feedback_for_messages(&["assistant-feedback".to_owned()])
        .expect("lookup cleared feedback");
    assert_eq!(cleared["assistant-feedback"].state, None);
    assert_eq!(cleared["assistant-feedback"].revision, 2);
}

#[test]
fn batched_feedback_lookup_surfaces_storage_failure() {
    let (database, repository) = fixture("feedback-batched-storage-failure");
    database
        .connection()
        .expect("connection")
        .execute_batch("DROP TABLE evolution_feedback_current;")
        .expect("drop feedback table");

    assert!(matches!(
        repository.feedback_for_messages(&["assistant-feedback".to_owned()]),
        Err(EvidenceRepositoryError::Storage)
    ));
}
