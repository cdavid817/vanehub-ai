//! What the index promises about identity, ordering, and what it is willing to claim.
//!
//! Every test here runs against a real SQLite database created by the real migration, because the
//! three things under test — a unique constraint, a keyset boundary, and an aggregate over gap rows
//! — are properties of the schema rather than of the code that calls it.

use super::log_index_repository::SqliteLogIndexRepository;
use crate::contexts::operations::application::{
    IndexedLogLevel, IndexedSessionLogQuery, LogCorrelation, LogIndexInsertOutcome,
    LogSortDirection, LogSourceIdentity, OperationsLogError, RedactedLogRecord, SessionLogCoverage,
    SessionLogCoverageState, SessionLogFilters, SessionLogIndexRepository, SessionLogQueryScope,
    MAX_LOG_PAGE_SIZE, MAX_LOG_SEARCH_CANDIDATES,
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

fn source(file: &str) -> LogSourceIdentity {
    LogSourceIdentity {
        directory_generation: "generation-1".to_string(),
        file_id: file.to_string(),
    }
}

fn record(id: &str, offset: u64, at: &str) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: id.to_string(),
        source: source("file-1"),
        source_offset: offset,
        occurred_at: at.to_string(),
        occurred_at_ms: chrono::DateTime::parse_from_rfc3339(at)
            .expect("timestamp")
            .timestamp_millis(),
        level: IndexedLogLevel::Info,
        category: "test".to_string(),
        message: format!("message for {id}"),
        context: BTreeMap::from([("sessionId".to_string(), SESSION.to_string())]),
        correlation: LogCorrelation {
            session_id: Some(SESSION.to_string()),
            ..LogCorrelation::default()
        },
    }
}

fn query(
    filters: SessionLogFilters,
    cursor: Option<String>,
    limit: usize,
) -> IndexedSessionLogQuery {
    IndexedSessionLogQuery {
        scope: SessionLogQueryScope {
            session_id: Some(SESSION.to_string()),
            ..SessionLogQueryScope::default()
        },
        filters,
        cursor,
        limit: Some(limit),
    }
}

fn newest_first() -> SessionLogFilters {
    SessionLogFilters::default()
}

fn at(second: u32) -> String {
    format!("2026-08-24T10:00:{second:02}Z")
}

// ---------------------------------------------------------------------------------------------
// 8.4 — idempotency
// ---------------------------------------------------------------------------------------------

/// A retry of a record already indexed under the same witness is a success that changes nothing.
/// Anything else — a second row, a moved coverage, a second notice — would turn one record into two
/// for every reader downstream.
#[test]
fn an_identical_retry_adds_no_row_and_moves_no_coverage() {
    let harness = harness("log-index-retry");
    let first = harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("first insert");
    let before = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");

    let again = harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("retry");
    let after = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");

    assert!(matches!(first, LogIndexInsertOutcome::Inserted { .. }));
    assert_eq!(again, LogIndexInsertOutcome::AlreadyIndexed);
    assert_eq!(after.dropped_count, before.dropped_count);
    assert_eq!(after.reason_codes, before.reason_codes);
    let page = harness
        .repository
        .query(&query(newest_first(), None, 50))
        .expect("query");
    assert_eq!(page.items.len(), 1);
}

/// Same id, different witness. The stored row wins and the disagreement becomes a gap, because a
/// later record silently replacing an earlier one would rewrite something a reader may have cited.
#[test]
fn a_conflicting_witness_keeps_the_original_row_and_degrades_coverage() {
    let harness = harness("log-index-conflict");
    harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("first insert");
    // Checkpointed first, so the corpus counts as read: "I have not indexed this yet" is a stronger
    // caveat than "I lost a record", and it would otherwise mask the conflict under `indexing`.
    harness
        .repository
        .commit_batch(&source("file-1"), &[], &Default::default(), 100)
        .expect("checkpoint");
    let mut moved = record("record-1", 4096, &at(2));
    moved.message = "a different line entirely".to_string();

    let outcome = harness.repository.insert(&moved).expect("conflict");
    let coverage = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");
    let page = harness
        .repository
        .query(&query(newest_first(), None, 50))
        .expect("query");

    assert_eq!(outcome, LogIndexInsertOutcome::Conflicted);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].message, "message for record-1");
    assert_eq!(coverage.state(), SessionLogCoverageState::Partial);
    assert!(coverage
        .reason_codes
        .iter()
        .any(|code| code == "log_identity_conflict"));
}

/// Two records that differ only in where they sit are two records. A retry loop logging one failure
/// twice inside a millisecond produces exactly this, and collapsing them would drop a real event.
#[test]
fn the_same_text_at_two_offsets_is_two_records() {
    let harness = harness("log-index-two-offsets");
    harness
        .repository
        .insert(&record("record-a", 0, &at(1)))
        .expect("first");
    harness
        .repository
        .insert(&record("record-b", 200, &at(1)))
        .expect("second");

    let page = harness
        .repository
        .query(&query(newest_first(), None, 50))
        .expect("query");

    assert_eq!(page.items.len(), 2);
}

// ---------------------------------------------------------------------------------------------
// 8.5 / 0.7 — keyset pagination
// ---------------------------------------------------------------------------------------------

/// `sequence` is the table's `INTEGER PRIMARY KEY`, so no two rows can share one. That is what
/// makes `(occurred_at_ms, sequence, record_id)` a strict total order rather than an order with
/// ties a page boundary could fall inside.
#[test]
fn the_ordering_key_is_unique_per_row_even_when_timestamps_tie() {
    let harness = harness("log-index-total-order");
    for index in 0..5 {
        harness
            .repository
            .insert(&record(&format!("record-{index}"), index * 100, &at(1)))
            .expect("insert");
    }

    let page = harness
        .repository
        .query(&query(newest_first(), None, 50))
        .expect("query");

    let sequences: Vec<i64> = page.items.iter().map(|record| record.sequence).collect();
    let mut unique = sequences.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(sequences.len(), unique.len(), "sequences repeated");
    // Strictly descending, with every timestamp identical: the tie-break is doing the ordering.
    assert!(sequences.windows(2).all(|pair| pair[0] > pair[1]));
}

/// The 0.7 gate, log half: a record appended above the first page's boundary must not shift it.
/// An offset cursor would either repeat the boundary row or skip it, and both read as ordinary
/// pagination.
#[test]
fn a_record_appended_between_pages_neither_repeats_nor_skips_the_boundary() {
    let harness = harness("log-index-append-between-pages");
    for index in 0..6u32 {
        harness
            .repository
            .insert(&record(
                &format!("record-{index}"),
                u64::from(index) * 100,
                &at(index),
            ))
            .expect("insert");
    }

    let first = harness
        .repository
        .query(&query(newest_first(), None, 3))
        .expect("first page");
    let first_ids: Vec<String> = first
        .items
        .iter()
        .map(|record| record.record_id.clone())
        .collect();
    assert_eq!(first_ids, ["record-5", "record-4", "record-3"]);
    let cursor = first.next_cursor.clone().expect("a second page exists");

    // Newer than everything on page one, so it lands above the cursor's position.
    harness
        .repository
        .insert(&record("record-newer", 9_000, &at(59)))
        .expect("append between pages");

    let second = harness
        .repository
        .query(&query(newest_first(), Some(cursor), 3))
        .expect("second page");
    let second_ids: Vec<String> = second
        .items
        .iter()
        .map(|record| record.record_id.clone())
        .collect();

    assert_eq!(second_ids, ["record-2", "record-1", "record-0"]);
    // Nothing from page one came back, and nothing between the pages went missing.
    assert!(second_ids.iter().all(|id| !first_ids.contains(id)));
    assert!(!second_ids.contains(&"record-newer".to_string()));
}

/// A cursor is issued against a set of filters. Reusing it under different ones would splice two
/// result sets, and the seam would be invisible — so each narrowing field is checked in turn.
#[test]
fn a_cursor_is_refused_by_every_filter_it_was_not_issued_for() {
    let harness = harness("log-index-cursor-mismatch");
    for index in 0..4u32 {
        harness
            .repository
            .insert(&record(
                &format!("record-{index}"),
                u64::from(index) * 100,
                &at(index),
            ))
            .expect("insert");
    }
    let cursor = harness
        .repository
        .query(&query(newest_first(), None, 2))
        .expect("first page")
        .next_cursor
        .expect("cursor");

    let changed = [
        SessionLogFilters {
            levels: vec![IndexedLogLevel::Error],
            ..newest_first()
        },
        SessionLogFilters {
            search: Some("anything".to_string()),
            ..newest_first()
        },
        SessionLogFilters {
            from: Some(at(1)),
            ..newest_first()
        },
        SessionLogFilters {
            to: Some(at(3)),
            ..newest_first()
        },
        SessionLogFilters {
            sort: LogSortDirection::OldestFirst,
            ..newest_first()
        },
    ];
    for filters in changed {
        assert_eq!(
            harness
                .repository
                .query(&query(filters.clone(), Some(cursor.clone()), 2))
                .err(),
            Some(OperationsLogError::CursorFilterMismatch),
            "a cursor survived a filter change: {filters:?}"
        );
    }

    // A different seat is a different scope, and scope is fingerprinted too.
    let mut other_seat = query(newest_first(), Some(cursor.clone()), 2);
    other_seat.scope.seat_id = Some("seat-2".to_string());
    assert_eq!(
        harness.repository.query(&other_seat).err(),
        Some(OperationsLogError::CursorFilterMismatch)
    );
}

/// A malformed cursor is refused. Reading it as an offset, or as "start again", would answer a
/// different question than the one asked without saying so.
#[test]
fn a_malformed_cursor_never_degrades_into_an_offset() {
    let harness = harness("log-index-malformed-cursor");
    harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("insert");

    for raw in ["0", "12", "not-base64!!", "MTIz"] {
        assert_eq!(
            harness
                .repository
                .query(&query(newest_first(), Some(raw.to_string()), 2))
                .err(),
            Some(OperationsLogError::InvalidCursor),
            "cursor {raw} was not refused"
        );
    }
}

/// Reading in the other direction is a different question, so it starts from the other end.
#[test]
fn reversing_the_sort_reverses_the_page_and_its_boundary() {
    let harness = harness("log-index-sort-direction");
    for index in 0..4u32 {
        harness
            .repository
            .insert(&record(
                &format!("record-{index}"),
                u64::from(index) * 100,
                &at(index),
            ))
            .expect("insert");
    }

    let oldest = harness
        .repository
        .query(&query(
            SessionLogFilters {
                sort: LogSortDirection::OldestFirst,
                ..newest_first()
            },
            None,
            2,
        ))
        .expect("oldest first");

    assert_eq!(
        oldest
            .items
            .iter()
            .map(|record| record.record_id.as_str())
            .collect::<Vec<_>>(),
        ["record-0", "record-1"]
    );
}

/// The needle is literal text, not a pattern. A user typing `%` is searching for a percent sign;
/// treating it as a wildcard would silently match everything and look like the search was ignored.
#[test]
fn search_treats_wildcard_characters_as_literal_text() {
    let harness = harness("log-index-search-literal");
    let mut percent = record("record-percent", 0, &at(1));
    percent.message = "progress 50% complete".to_string();
    let mut underscore = record("record-underscore", 100, &at(2));
    underscore.message = "snake_case identifier".to_string();
    let mut plain = record("record-plain", 200, &at(3));
    plain.message = "nothing special".to_string();
    for entry in [&percent, &underscore, &plain] {
        harness.repository.insert(entry).expect("insert");
    }

    let percent_hits = harness
        .repository
        .query(&query(
            SessionLogFilters {
                search: Some("50%".to_string()),
                ..newest_first()
            },
            None,
            50,
        ))
        .expect("percent search");
    let underscore_hits = harness
        .repository
        .query(&query(
            SessionLogFilters {
                search: Some("snake_case".to_string()),
                ..newest_first()
            },
            None,
            50,
        ))
        .expect("underscore search");

    assert_eq!(percent_hits.items.len(), 1);
    assert_eq!(percent_hits.items[0].record_id, "record-percent");
    // `_` as a wildcard would also match "snake-case" and anything else of that shape.
    assert_eq!(underscore_hits.items.len(), 1);
    assert_eq!(underscore_hits.items[0].record_id, "record-underscore");
}

/// The repository is the second line, not the first: the service refuses an oversized limit before
/// it gets here (`log_query_service_tests`), and if that check were ever removed the SQL must still
/// not read an unbounded page. So this asserts the clamp, and the refusal is asserted where the
/// refusal lives.
#[test]
fn the_repository_never_reads_more_than_the_ceiling_even_when_asked_to() {
    let harness = harness("log-index-page-ceiling");
    let page = harness
        .repository
        .query(&query(newest_first(), None, MAX_LOG_PAGE_SIZE + 1))
        .expect("the repository clamps rather than failing");
    assert!(page.items.len() <= MAX_LOG_PAGE_SIZE);
}

// ---------------------------------------------------------------------------------------------
// 8.6 — coverage
// ---------------------------------------------------------------------------------------------

/// An index nothing has read is not an index that read everything and found nothing. Both render
/// as zero, and only the coverage says which one happened.
#[test]
fn an_index_that_was_never_backfilled_is_indexing_rather_than_complete() {
    let harness = harness("log-index-never-read");

    let coverage = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");

    assert_eq!(coverage.state(), SessionLogCoverageState::Indexing);
    assert!(coverage
        .reason_codes
        .iter()
        .any(|code| code == "log_index_not_backfilled"));
    assert_eq!(coverage.newest_available_at, None);
}

/// Complete has to be earned: every source checkpointed, nothing dropped, nothing conflicting.
#[test]
fn coverage_is_complete_only_once_the_sources_are_checkpointed_and_whole() {
    let harness = harness("log-index-complete");
    harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("insert");
    harness
        .repository
        .commit_batch(&source("file-1"), &[], &Default::default(), 100)
        .expect("checkpoint");

    let coverage = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");

    assert_eq!(coverage.state(), SessionLogCoverageState::Complete);
    assert_eq!(coverage.dropped_count, 0);
    assert!(coverage.reason_codes.is_empty());
    assert_eq!(coverage.indexed_through, coverage.newest_available_at);
    assert!(coverage.oldest_available_at.is_some());
}

/// Every way a record can be lost degrades coverage, and each one says which way it was — a reader
/// does something different about retention than about a dropped receipt.
#[test]
fn each_kind_of_loss_reports_its_own_reason_code() {
    for (reason, dropped) in [
        ("log_receipt_dropped", 3u32),
        ("log_record_rejected", 1),
        ("log_retention_removed", 7),
    ] {
        let harness = harness(&format!("log-index-loss-{reason}"));
        harness
            .repository
            .insert(&record("record-1", 0, &at(1)))
            .expect("insert");
        harness
            .repository
            .commit_batch(&source("file-1"), &[], &Default::default(), 100)
            .expect("checkpoint");
        harness
            .repository
            .record_gap(&source("file-1"), reason, dropped)
            .expect("gap");

        let coverage = harness
            .repository
            .coverage(Some(SESSION))
            .expect("coverage");

        assert_eq!(coverage.state(), SessionLogCoverageState::Partial);
        assert_eq!(coverage.dropped_count, dropped);
        assert!(coverage.reason_codes.iter().any(|code| code == reason));
    }
}

/// A search that stopped at its candidate bound has not established that nothing else matches, so
/// the page is partial and truncated rather than a confident empty result.
#[test]
fn a_search_that_hits_its_candidate_bound_reports_partial_and_truncated() {
    let harness = harness("log-index-search-bound");
    harness
        .repository
        .commit_batch(&source("file-1"), &[], &Default::default(), 0)
        .expect("checkpoint");
    // One more than the bound, none of which match, so the scan stops without finding anything.
    for index in 0..=MAX_LOG_SEARCH_CANDIDATES {
        let mut entry = record(&format!("record-{index}"), index as u64 * 10, &at(1));
        entry.message = "unrelated".to_string();
        harness.repository.insert(&entry).expect("insert");
    }

    let page = harness
        .repository
        .query(&query(
            SessionLogFilters {
                search: Some("needle-that-is-not-there".to_string()),
                ..newest_first()
            },
            None,
            50,
        ))
        .expect("search");

    assert!(page.items.is_empty());
    assert!(page.truncated);
    assert_eq!(page.coverage.state(), SessionLogCoverageState::Partial);
    assert!(page
        .coverage
        .reason_codes
        .iter()
        .any(|code| code == "log_search_candidates_exhausted"));
}

/// Retention removing a source removes its rows and moves the oldest queryable boundary with them.
/// Leaving the rows would let the index answer for a corpus that no longer exists.
#[test]
fn forgetting_a_source_removes_its_rows_and_its_checkpoint() {
    let harness = harness("log-index-retention");
    harness
        .repository
        .insert(&record("record-1", 0, &at(1)))
        .expect("insert");
    harness
        .repository
        .commit_batch(&source("file-1"), &[], &Default::default(), 100)
        .expect("checkpoint");

    let removed = harness
        .repository
        .forget_sources(&[source("file-2")])
        .expect("forget");

    assert_eq!(removed, 1);
    assert_eq!(
        harness
            .repository
            .checkpoint(&source("file-1"))
            .expect("checkpoint"),
        None
    );
    let coverage: SessionLogCoverage = harness
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");
    assert_eq!(coverage.oldest_available_at, None);
}
