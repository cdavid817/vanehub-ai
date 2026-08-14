use super::*;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

fn fixture() -> (NativeDatabase, SqliteEvolutionEvidenceRepository) {
    let directory = TempDirectory::new("evidence-query-scope");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    for (index, workspace, skill) in [
        (1, "workspace-a", "review"),
        (2, "workspace-b", "other"),
        (3, "workspace-a", "review"),
    ] {
        connection.execute(
            "INSERT INTO evolution_signals (signal_id, source_event_id, source_kind, extractor_id, extractor_version, discriminator, deduplication_key, category, polarity, severity, attribution_strength, attribution_rationale, targeting_eligibility, source_fidelity, occurred_at, ingested_at, workspace, safe_summary, sanitizer_version, task_fingerprint_version, task_fingerprint) VALUES (?1, ?2, 'native_execution', 'execution_failure', 1, ?2, ?2, 'execution_failure', 'negative', 'high', 'verified', 'exact_native_observation', 'automated_consideration', 'native', ?3, ?3, ?4, ?5, 1, 1, ?2)",
            rusqlite::params![format!("signal-{index}"), format!("event-{index}"), format!("2026-08-0{index}T00:00:00Z"), workspace, format!("safe summary {index}")],
        ).expect("signal");
        connection.execute(
            "INSERT INTO evolution_signal_skill_associations (signal_id, skill_id, skill_revision, association_kind, observed_at, attribution_strength, attribution_rationale, targeting_eligibility) VALUES (?1, ?2, 'rev-a', 'injected', '2026-08-01T00:00:00Z', 'verified', 'exact_native_observation', 'automated_consideration')",
            rusqlite::params![format!("signal-{index}"), skill],
        ).expect("association");
    }
    connection.execute(
        "UPDATE evolution_signals SET association_truncated_count = 4, source_link_truncated_count = 2 WHERE signal_id = 'signal-3'",
        [],
    ).expect("truncation disclosure");
    drop(connection);
    let repository = SqliteEvolutionEvidenceRepository::new(database.clone());
    (database, repository)
}

#[test]
fn queries_enforce_workspace_and_skill_scope_with_stable_cursor_pagination() {
    let (_database, repository) = fixture();
    let request = EvidencePageRequest {
        scope: EvidenceQueryScope {
            workspace: Some("workspace-a".to_string()),
            skill_id: Some("review".to_string()),
        },
        limit: 1,
        cursor: None,
    };
    let first = repository.query_overview(&request).expect("first page");
    assert_eq!(first.signal_count, 2);
    assert_eq!(first.signals.len(), 1);
    assert_eq!(first.signals[0].signal_id, "signal-3");
    assert_eq!(first.signals[0].association_truncated_count, 4);
    assert_eq!(first.signals[0].source_link_truncated_count, 2);
    assert_eq!(
        first.first_occurred_at.as_deref(),
        Some("2026-08-01T00:00:00Z")
    );
    assert_eq!(
        first.last_occurred_at.as_deref(),
        Some("2026-08-03T00:00:00Z")
    );
    let second = repository
        .query_overview(&EvidencePageRequest {
            cursor: first.next_cursor,
            ..request
        })
        .expect("second page");
    assert_eq!(second.signals[0].signal_id, "signal-1");
    assert!(second.next_cursor.is_none());
}

#[test]
fn unknown_workspace_isolated_as_a_valid_empty_result() {
    let (_database, repository) = fixture();
    let overview = repository
        .query_overview(&EvidencePageRequest {
            scope: EvidenceQueryScope {
                workspace: Some("unknown-workspace".to_string()),
                skill_id: None,
            },
            limit: 10,
            cursor: None,
        })
        .expect("isolated empty overview");
    assert_eq!(overview.signal_count, 0);
    assert_eq!(overview.seed_count, 0);
    assert!(overview.signals.is_empty());
    assert!(overview.seeds.is_empty());
}

#[test]
fn query_projection_contains_safe_summaries_but_no_prohibited_source_content_fields() {
    let (_database, repository) = fixture();
    let overview = repository
        .query_overview(&EvidencePageRequest {
            scope: EvidenceQueryScope {
                workspace: None,
                skill_id: None,
            },
            limit: 1000,
            cursor: None,
        })
        .expect("overview");
    assert_eq!(overview.signals.len(), 3);
    let wire = serde_json::to_string(&overview).expect("serialize");
    for prohibited in [
        "prompt",
        "toolArguments",
        "toolOutput",
        "terminalOutput",
        "thinking",
    ] {
        assert!(!wire.contains(prohibited));
    }
    assert!(wire.contains("safeSummary"));
}
