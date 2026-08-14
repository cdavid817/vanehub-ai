use super::SqliteContextQualityRepository;
use crate::contexts::agent_runtime::application::ContextQualityRepository;
use crate::contexts::agent_runtime::domain::{
    ContextAssessmentInvariants, ContextAssessmentMeasurementQuality, ContextAssessmentOutcome,
    ContextAssessmentPath, ContextAssessmentReason, ContextAssessmentTriggerSource,
    ContextQualityAssessment, ContextQualityAssessmentInput, ContextQualityAssessmentRecord,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

fn repository(
    name: &str,
) -> (
    TempDirectory,
    NativeDatabase,
    SqliteContextQualityRepository,
) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteContextQualityRepository::new(database.clone());
    (directory, database, repository)
}

fn record(
    correlation: &str,
    sequence: u64,
    recorded_at: &str,
    outcome: ContextAssessmentOutcome,
    before_tokens: Option<u64>,
    after_tokens: Option<u64>,
) -> ContextQualityAssessmentRecord {
    ContextQualityAssessmentRecord {
        session_correlation: Some(format!("session-{sequence}")),
        recorded_at: recorded_at.to_string(),
        assessment: ContextQualityAssessment::new(ContextQualityAssessmentInput {
            generation_correlation: correlation,
            decision_sequence: sequence,
            outcome,
            path: (outcome == ContextAssessmentOutcome::Compacted)
                .then_some(ContextAssessmentPath::Optimizer),
            reason: (outcome == ContextAssessmentOutcome::Bypassed)
                .then_some(ContextAssessmentReason::Cooldown),
            trigger_source: Some(ContextAssessmentTriggerSource::TokenAware),
            before_characters: 1_000,
            after_characters: 400,
            before_tokens,
            after_tokens,
            measurement_quality: if before_tokens.is_some() {
                ContextAssessmentMeasurementQuality::Reported
            } else {
                ContextAssessmentMeasurementQuality::CharactersOnly
            },
            invariants: (outcome == ContextAssessmentOutcome::Compacted)
                .then_some(ContextAssessmentInvariants::passed()),
            context_policy_version: "policy-v1",
            optimizer_version: "optimizer-v1",
            verifier_version: "verifier-v1",
        }),
    }
}

#[test]
fn appends_idempotently_and_paginates_with_stable_cursors() {
    let (_directory, _database, repository) = repository("context-quality-pagination");
    for item in [
        record(
            "generation-1",
            1,
            "2026-08-14T10:00:01Z",
            ContextAssessmentOutcome::Compacted,
            Some(900),
            Some(300),
        ),
        record(
            "generation-2",
            2,
            "2026-08-14T10:00:02Z",
            ContextAssessmentOutcome::Bypassed,
            None,
            None,
        ),
        record(
            "generation-3",
            3,
            "2026-08-14T10:00:03Z",
            ContextAssessmentOutcome::Failed,
            Some(700),
            Some(700),
        ),
    ] {
        repository
            .append_and_prune(&item, "2026-08-01T00:00:00Z", 10)
            .expect("append");
    }
    let duplicate = record(
        "generation-3",
        3,
        "2026-08-14T10:00:03Z",
        ContextAssessmentOutcome::Failed,
        Some(700),
        Some(700),
    );
    repository
        .append_and_prune(&duplicate, "2026-08-01T00:00:00Z", 10)
        .expect("duplicate append");

    let first = repository
        .list("2026-08-01T00:00:00Z", None, 2)
        .expect("first page");
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.items[0].assessment.decision_sequence, 3);
    assert_eq!(first.items[1].assessment.decision_sequence, 2);
    let second = repository
        .list("2026-08-01T00:00:00Z", first.next_cursor.as_deref(), 2)
        .expect("second page");
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].assessment.decision_sequence, 1);
    assert!(second.next_cursor.is_none());
}

#[test]
fn summarizes_empty_and_mixed_measurement_quality_without_fabricating_tokens() {
    let (_directory, _database, repository) = repository("context-quality-summary");
    let empty = repository
        .summarize("2026-08-01T00:00:00Z")
        .expect("empty summary");
    assert_eq!(empty.evaluated, 0);
    assert_eq!(empty.saved_tokens, 0);
    assert_eq!(empty.token_measurement_count, 0);

    let compacted = record(
        "generation-1",
        1,
        "2026-08-14T10:00:01Z",
        ContextAssessmentOutcome::Compacted,
        Some(900),
        Some(300),
    );
    let bypassed = record(
        "generation-2",
        2,
        "2026-08-14T10:00:02Z",
        ContextAssessmentOutcome::Bypassed,
        None,
        None,
    );
    for item in [&compacted, &bypassed] {
        repository
            .append_and_prune(item, "2026-08-01T00:00:00Z", 10)
            .expect("append");
    }
    let summary = repository
        .summarize("2026-08-01T00:00:00Z")
        .expect("summary");
    assert_eq!(summary.evaluated, 2);
    assert_eq!(summary.saved_characters, 1_200);
    assert_eq!(summary.saved_tokens, 600);
    assert_eq!(summary.token_measurement_count, 1);
    assert_eq!(summary.outcomes.get("compacted"), Some(&1));
    assert_eq!(summary.outcomes.get("bypassed"), Some(&1));
    assert_eq!(summary.qualities.get("characters-only"), Some(&1));
    assert_eq!(summary.policy_versions.get("policy-v1"), Some(&2));
}

#[test]
fn prunes_expired_rows_and_enforces_the_oldest_first_hard_ceiling() {
    let (_directory, _database, repository) = repository("context-quality-retention");
    let expired = record(
        "expired",
        1,
        "2026-07-01T00:00:00Z",
        ContextAssessmentOutcome::Compacted,
        None,
        None,
    );
    repository
        .append_and_prune(&expired, "2026-08-01T00:00:00Z", 2)
        .expect("append expired");
    for (sequence, timestamp) in [
        (2, "2026-08-14T10:00:01Z"),
        (3, "2026-08-14T10:00:02Z"),
        (4, "2026-08-14T10:00:03Z"),
    ] {
        repository
            .append_and_prune(
                &record(
                    &format!("generation-{sequence}"),
                    sequence,
                    timestamp,
                    ContextAssessmentOutcome::Compacted,
                    None,
                    None,
                ),
                "2026-08-01T00:00:00Z",
                2,
            )
            .expect("append retained");
    }
    let page = repository
        .list("2020-01-01T00:00:00Z", None, 10)
        .expect("history");
    let sequences = page
        .items
        .iter()
        .map(|item| item.assessment.decision_sequence)
        .collect::<Vec<_>>();
    assert_eq!(sequences, vec![4, 3]);
}

#[test]
fn rejects_invalid_queries_and_corrupt_persisted_enum_values() {
    let (_directory, database, repository) = repository("context-quality-corruption");
    assert!(repository.list("", None, 10).is_err());
    assert!(repository.list("2026-08-01T00:00:00Z", None, 0).is_err());
    assert!(repository.list("2026-08-01T00:00:00Z", None, 101).is_err());
    assert!(repository
        .list("2026-08-01T00:00:00Z", Some("unknown"), 10)
        .is_err());

    let item = record(
        "generation-1",
        1,
        "2026-08-14T10:00:01Z",
        ContextAssessmentOutcome::Compacted,
        None,
        None,
    );
    repository
        .append_and_prune(&item, "2026-08-01T00:00:00Z", 10)
        .expect("append");
    database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE context_quality_assessments SET reason = 'corrupt' WHERE attempt_id = ?1",
            [&item.assessment.attempt_id],
        )
        .expect("corrupt row");
    assert!(repository.list("2026-08-01T00:00:00Z", None, 10).is_err());
}
