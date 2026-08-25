//! Error rows grouped by category, and the scope that narrows them.
//!
//! Against the real index because everything under test is SQL: a grouped count, an `IN` clause, a
//! millisecond comparison. The last is the one that bites — `occurred_at_ms` is an integer column,
//! and a text bound compared against it matches nothing and reports a clean session.

use super::log_index_repository::SqliteLogIndexRepository;
use crate::contexts::operations::application::{
    IndexedLogLevel, LogCorrelation, LogFailureQuery, RedactedLogRecord, SessionLogIndexRepository,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;

const SESSION: &str = "session-1";

struct Harness {
    _directory: TempDirectory,
    repository: SqliteLogIndexRepository,
}

fn harness(label: &str) -> Harness {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    Harness {
        repository: SqliteLogIndexRepository::new(database),
        _directory: directory,
    }
}

fn log_record(
    id: &str,
    offset: u64,
    at: &str,
    level: IndexedLogLevel,
    category: &str,
) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: id.to_string(),
        source: crate::contexts::operations::application::LogSourceIdentity {
            directory_generation: "generation-1".to_string(),
            file_id: "file-1".to_string(),
        },
        source_offset: offset,
        occurred_at: at.to_string(),
        occurred_at_ms: chrono::DateTime::parse_from_rfc3339(at)
            .expect("timestamp")
            .timestamp_millis(),
        level,
        category: category.to_string(),
        message: format!("message for {id}"),
        context: BTreeMap::from([("sessionId".to_string(), SESSION.to_string())]),
        correlation: LogCorrelation {
            session_id: Some(SESSION.to_string()),
            ..LogCorrelation::default()
        },
    }
}

fn query() -> LogFailureQuery {
    LogFailureQuery {
        session_id: SESSION.to_string(),
        ..LogFailureQuery::default()
    }
}

fn seed(harness: &Harness) {
    for (id, offset, at, category) in [
        ("error-1", 0_u64, "2026-01-01T00:00:00Z", "cli.launch"),
        ("error-2", 1, "2026-01-01T00:01:00Z", "cli.launch"),
        ("error-3", 2, "2026-01-01T00:02:00Z", "sdk.operation"),
    ] {
        harness
            .repository
            .insert(&log_record(
                id,
                offset,
                at,
                IndexedLogLevel::Error,
                category,
            ))
            .expect("insert");
    }
    harness
        .repository
        .insert(&log_record(
            "info-1",
            3,
            "2026-01-01T00:03:00Z",
            IndexedLogLevel::Info,
            "cli.launch",
        ))
        .expect("insert");
}

#[test]
fn errors_are_counted_per_category_heaviest_first() {
    let harness = harness("log-failure-counts");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(&query(), 20)
        .expect("failure counts");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].category, "cli.launch");
    assert_eq!(rows[0].count, 2);
    // The info row under the same category is not a failure. Counting every level would make "what
    // went wrong" the same question as "what happened".
    assert_eq!(rows[1].category, "sdk.operation");
    assert_eq!(rows[1].count, 1);
}

#[test]
fn a_time_range_narrows_the_count() {
    let harness = harness("log-failure-range");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(
            &LogFailureQuery {
                from: Some("2026-01-01T00:02:00Z".to_string()),
                ..query()
            },
            20,
        )
        .expect("failure counts");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].category, "sdk.operation");
}

#[test]
fn an_unreadable_time_bound_is_dropped_rather_than_matching_nothing() {
    let harness = harness("log-failure-bad-range");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(
            &LogFailureQuery {
                from: Some("last tuesday".to_string()),
                ..query()
            },
            20,
        )
        .expect("failure counts");

    // Comparing that text against the millisecond column would match nothing, and a session with
    // three errors would report as clean — which is the worse of the two ways to be wrong.
    assert_eq!(rows.len(), 2);
}

#[test]
fn one_extra_row_is_returned_so_the_caller_can_see_a_cut_tail() {
    let harness = harness("log-failure-tail");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(&query(), 1)
        .expect("failure counts");

    // Two categories exist and the limit is one, so the caller receives two and learns the tail was
    // cut without a second `COUNT(DISTINCT category)`.
    assert_eq!(rows.len(), 2);
}

#[test]
fn another_session_is_not_counted() {
    let harness = harness("log-failure-session-scope");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(
            &LogFailureQuery {
                session_id: "session-2".to_string(),
                ..query()
            },
            20,
        )
        .expect("failure counts");

    assert!(rows.is_empty());
}

#[test]
fn a_run_filter_excludes_other_runs() {
    let harness = harness("log-failure-run-scope");
    seed(&harness);

    let rows = harness
        .repository
        .failure_counts(
            &LogFailureQuery {
                run_ids: vec!["run-that-recorded-nothing".to_string()],
                ..query()
            },
            20,
        )
        .expect("failure counts");

    assert!(rows.is_empty());
}
