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
        authorize_reusable_guidance: false,
    }
}

fn authorized_correction(expected_revision: u64, note: &str) -> SaveFeedbackRequest {
    let mut request = request(
        expected_revision,
        Some(FeedbackState::Corrected),
        Some(note),
    );
    request.authorize_reusable_guidance = true;
    request
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

#[test]
fn reusable_guidance_authorization_is_explicit_revision_bound_and_replaced_atomically() {
    let (database, repository) = fixture("feedback-reusable-authorization");
    let saved = repository
        .save_feedback(
            &authorized_correction(0, "Use the verified retry boundary."),
            KEY,
        )
        .expect("authorized correction");
    let authorization = saved
        .reusable_guidance_authorization
        .expect("authorization");
    assert_eq!(authorization.feedback_revision, 1);
    assert_eq!(
        authorization.disclosure_version,
        REUSABLE_GUIDANCE_DISCLOSURE_VERSION_V1
    );
    let summaries = repository
        .feedback_for_messages(&["assistant-feedback".into()])
        .expect("feedback summary");
    assert_eq!(
        summaries["assistant-feedback"]
            .reusable_guidance_authorization
            .as_ref()
            .map(|value| value.authorization_id.as_str()),
        Some(authorization.authorization_id.as_str())
    );
    let source = repository
        .authorized_correction_guidance(&authorization.authorization_id)
        .expect("authorized source")
        .expect("current authorization");
    assert_eq!(
        source.sanitized_guidance,
        "Use the verified retry boundary."
    );
    assert_eq!(source.sanitizer_version, 1);
    assert!(!source.authorization_witness_hash.is_empty());

    repository
        .save_feedback(
            &authorized_correction(1, "Use the newer verified boundary."),
            KEY,
        )
        .expect("replacement correction");
    let connection = database.connection().expect("connection");
    let revoked: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_correction_authorizations
             WHERE feedback_id='assistant-feedback' AND feedback_revision=1
               AND authorized=0 AND revoked_at_ms IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("revoked authorization");
    assert_eq!(revoked, 1);
    assert_eq!(
        repository
            .authorized_correction_guidance(&authorization.authorization_id)
            .expect("revoked lookup"),
        None
    );
}

#[test]
fn explicit_revocation_is_conflict_safe_and_stales_derived_eligibility() {
    let (database, repository) = fixture("feedback-authorization-revocation");
    let saved = repository
        .save_feedback(
            &authorized_correction(0, "Use the verified retry boundary."),
            KEY,
        )
        .expect("authorized correction");
    let authorization_id = saved
        .reusable_guidance_authorization
        .expect("authorization")
        .authorization_id;
    seed_derived_eligibility(&database, &authorization_id);
    assert_eq!(
        repository.revoke_reusable_guidance_authorization(
            &RevokeReusableGuidanceAuthorizationRequest {
                message_id: "assistant-feedback".into(),
                expected_feedback_revision: 0,
            },
            KEY,
        ),
        Err(FeedbackTransitionError::Conflict {
            current_revision: 1
        })
    );
    repository
        .revoke_reusable_guidance_authorization(
            &RevokeReusableGuidanceAuthorizationRequest {
                message_id: "assistant-feedback".into(),
                expected_feedback_revision: 1,
            },
            KEY,
        )
        .expect("revoked");
    let connection = database.connection().expect("connection");
    let result: String = connection
        .query_row(
            "SELECT result FROM evolution_auto_eligibility
             WHERE eligibility_id='feedback-eligibility'",
            [],
            |row| row.get(0),
        )
        .expect("eligibility");
    assert_eq!(result, "ineligible");
}

#[test]
fn non_correction_feedback_cannot_authorize_reusable_guidance() {
    let (_, repository) = fixture("feedback-authorization-shape");
    let mut invalid = request(0, Some(FeedbackState::Helpful), None);
    invalid.authorize_reusable_guidance = true;
    assert_eq!(
        repository.save_feedback(&invalid, KEY),
        Err(FeedbackTransitionError::InvalidInput)
    );
}

fn seed_derived_eligibility(database: &NativeDatabase, authorization_id: &str) {
    let connection = database.connection().expect("connection");
    connection.execute("INSERT INTO evolution_run_requests VALUES ('feedback-request',1,'workspace:feedback','runtime_trigger','completed','{}',0,0,'feedback-run',0,1,1)", []).expect("request");
    connection.execute("INSERT INTO evolution_runs VALUES ('feedback-run',1,'feedback-request','workspace:feedback','completed',NULL,'policy','{}','{}',NULL,NULL,NULL,NULL,0,1,1)", []).expect("run");
    connection.execute("INSERT INTO evolution_deterministic_drafts VALUES ('feedback-draft','workspace:feedback','skill-one',?1,'assessment-one','v1','content',7,'deterministic_authorized_correction','source',1)", [authorization_id]).expect("draft");
    connection.execute("INSERT INTO evolution_auto_eligibility VALUES ('feedback-eligibility','feedback-run','feedback-draft','skill-one','eligible','[]','proof','preview',1,0)", []).expect("eligibility");
}
