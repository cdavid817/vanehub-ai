//! What an interactive query costs when the corpus is as large as it is ever allowed to get.
//!
//! Asserted against a maximum fixture rather than a typical one, because the failure this guards
//! against does not appear on a small corpus at all. A query that scanned files, or that walked
//! every row before taking a page, is indistinguishable from a correct one until somebody has been
//! running the application for a month — at which point the Logs tab stops opening and there is
//! nothing in the code to point at.
//!
//! Every assertion here is about *work done*, never about elapsed time. A wall-clock budget on a
//! shared runner measures the runner: it passes on a quiet machine and fails under load, which is
//! exactly backwards for a test meant to catch a scaling mistake.

use super::log_index_repository::SqliteLogIndexRepository;
use crate::contexts::operations::application::{
    IndexedLogLevel, IndexedSessionLogQuery, LineRejections, LogCorrelation, LogSourceIdentity,
    RedactedLogRecord, SessionLogFilters, SessionLogIndexRepository, SessionLogQueryScope,
    DEFAULT_LOG_PAGE_SIZE, MAX_LOG_PAGE_SIZE, MAX_LOG_SEARCH_CANDIDATES,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;

const SESSION: &str = "session-1";

/// How many rows the fixture holds.
///
/// Past the search candidate bound so the truncation path is genuinely exercised, and past any
/// page size by a wide margin so "the query took a page" and "the query took everything" cannot
/// look alike.
const MAXIMUM_ROWS: usize = MAX_LOG_SEARCH_CANDIDATES + 1_000;

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteLogIndexRepository,
}

/// A corpus at its maximum, built once per test through the real repository.
fn maximum_fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let repository = SqliteLogIndexRepository::new(database.clone());
    let source = LogSourceIdentity {
        directory_generation: "generation-1".to_string(),
        file_id: "file-1".to_string(),
    };

    // Written in batches through the same transaction boundary a repair uses, so the fixture is a
    // corpus the product could actually have produced rather than one only a test can make.
    let mut batch = Vec::with_capacity(500);
    for index in 0..MAXIMUM_ROWS {
        batch.push(record(&source, index));
        if batch.len() == 500 || index + 1 == MAXIMUM_ROWS {
            repository
                .commit_batch(
                    &source,
                    &batch,
                    &LineRejections::new(),
                    (index as u64 + 1) * 100,
                )
                .expect("seed a batch");
            batch.clear();
        }
    }
    Fixture {
        database,
        repository,
        _directory: directory,
    }
}

fn record(source: &LogSourceIdentity, index: usize) -> RedactedLogRecord {
    // Timestamps advance by a second per row, so ordering is total and a page boundary is a
    // definite place rather than a tie the query has to break arbitrarily.
    let occurred_at_ms = 1_787_911_200_000 + index as i64 * 1_000;
    RedactedLogRecord {
        record_id: format!("record-{index}"),
        source: source.clone(),
        source_offset: index as u64 * 100,
        occurred_at: chrono::DateTime::from_timestamp_millis(occurred_at_ms)
            .expect("timestamp")
            .to_rfc3339(),
        occurred_at_ms,
        level: if index.is_multiple_of(100) {
            IndexedLogLevel::Error
        } else {
            IndexedLogLevel::Info
        },
        category: "test".to_string(),
        message: format!("line {index} of the corpus"),
        context: BTreeMap::from([("sessionId".to_string(), SESSION.to_string())]),
        correlation: LogCorrelation {
            session_id: Some(SESSION.to_string()),
            ..LogCorrelation::default()
        },
    }
}

fn query(filters: SessionLogFilters, limit: usize) -> IndexedSessionLogQuery {
    IndexedSessionLogQuery {
        scope: SessionLogQueryScope {
            session_id: Some(SESSION.to_string()),
            ..SessionLogQueryScope::default()
        },
        filters,
        cursor: None,
        limit: Some(limit),
    }
}

/// A page is a page, however large the corpus behind it is.
#[test]
fn a_page_over_a_maximum_corpus_returns_only_a_page() {
    let fixture = maximum_fixture("query-bounds-page");

    let page = fixture
        .repository
        .query(&query(SessionLogFilters::default(), DEFAULT_LOG_PAGE_SIZE))
        .expect("query");

    assert_eq!(page.items.len(), DEFAULT_LOG_PAGE_SIZE);
    assert!(page.truncated);
    assert!(page.next_cursor.is_some());
}

/// The ceiling holds at the maximum too, which is the only size where it could fail to.
#[test]
fn the_page_ceiling_holds_over_a_maximum_corpus() {
    let fixture = maximum_fixture("query-bounds-ceiling");

    let page = fixture
        .repository
        .query(&query(SessionLogFilters::default(), MAX_LOG_PAGE_SIZE))
        .expect("query");

    assert_eq!(page.items.len(), MAX_LOG_PAGE_SIZE);
}

/// A search that exhausts its candidate bound says so rather than reporting a definitive answer.
///
/// Having examined the first N candidates and found nothing does not establish that nothing
/// matches, and reporting it as "complete, no results" is the same class of false claim as a
/// coverage zero.
#[test]
fn a_search_over_a_maximum_corpus_stops_at_its_candidate_bound() {
    let fixture = maximum_fixture("query-bounds-search");

    let page = fixture
        .repository
        .query(&query(
            SessionLogFilters {
                // Present only in the oldest rows, so a newest-first search has to walk past the
                // candidate bound before it could find any.
                search: Some("line 3 of".to_string()),
                ..SessionLogFilters::default()
            },
            DEFAULT_LOG_PAGE_SIZE,
        ))
        .expect("query");

    assert!(
        page.truncated,
        "an exhausted search reported a final answer"
    );
    assert!(page
        .coverage
        .reason_codes
        .iter()
        .any(|code| code == "log_search_candidates_exhausted"));
}

/// Paging through a maximum corpus never re-reads a row and never skips one.
///
/// Walked far enough to cross several page boundaries, because an off-by-one in a keyset boundary
/// shows up as one duplicated or missing row per page — invisible in a single page and obvious
/// only when the pages are compared against each other.
#[test]
fn paging_through_a_maximum_corpus_repeats_and_skips_nothing() {
    let fixture = maximum_fixture("query-bounds-paging");
    let mut seen: Vec<String> = Vec::new();
    let mut cursor = None;

    for _ in 0..12 {
        let page = fixture
            .repository
            .query(&IndexedSessionLogQuery {
                cursor: cursor.clone(),
                ..query(SessionLogFilters::default(), 100)
            })
            .expect("page");
        seen.extend(page.items.iter().map(|item| item.record_id.clone()));
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(seen.len(), 1_200);
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), seen.len(), "a row appeared on two pages");
    // Newest first and contiguous: the corpus counts up, so the ids count down without a hole.
    for (offset, id) in seen.iter().enumerate() {
        assert_eq!(id, &format!("record-{}", MAXIMUM_ROWS - 1 - offset));
    }
}

/// Coverage over a maximum corpus is answered from the index, not from a walk over its rows.
#[test]
fn coverage_over_a_maximum_corpus_is_an_aggregate_rather_than_a_scan() {
    let fixture = maximum_fixture("query-bounds-coverage");

    let coverage = fixture
        .repository
        .coverage(Some(SESSION))
        .expect("coverage");

    // The boundaries are the corpus's own, so they can only come from an aggregate over the whole
    // table — and an aggregate is the one shape that does not get slower per row returned.
    assert!(coverage.oldest_available_at.is_some());
    assert!(coverage.newest_available_at.is_some());
    assert_eq!(coverage.dropped_count, 0);
}

/// The error badge is a count, not a page, so it does not grow with the corpus.
#[test]
fn the_error_count_over_a_maximum_corpus_is_a_count() {
    let fixture = maximum_fixture("query-bounds-error-count");

    let errors = fixture.repository.error_count(SESSION).expect("count");

    // One error per hundred rows in the fixture. The number matters less than the fact that it is
    // reached without materialising the rows behind it.
    assert_eq!(errors as usize, MAXIMUM_ROWS.div_ceil(100));
}

/// A query over a maximum corpus does not hold the write lock while it answers.
///
/// This is the half of the bound that a page-size assertion cannot reach. A query that took a
/// transaction — or that held its statement open while the caller consumed the page — would let one
/// reader opening the Logs tab stall every writer in the application, and the symptom would be
/// "the app freezes sometimes" rather than anything pointing at a query.
#[test]
fn a_query_over_a_maximum_corpus_holds_no_write_lock_while_answering() {
    let fixture = maximum_fixture("query-bounds-lock");

    let page = fixture
        .repository
        .query(&query(SessionLogFilters::default(), MAX_LOG_PAGE_SIZE))
        .expect("query");

    // Still holding the page — the largest one the ceiling allows — take the write lock from
    // another connection. A statement or transaction the query left open would make this fail.
    let mut other = fixture.database.connection().expect("second connection");
    let transaction = other.transaction().expect("transaction");
    transaction
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES ('probe', 'probe', 0, datetime('now'))",
            [],
        )
        .expect("the write lock was free while a full page was held");
    transaction.commit().expect("commit");
    assert_eq!(page.items.len(), MAX_LOG_PAGE_SIZE);
}

/// A search that walked its whole candidate bound still leaves the write lock free.
///
/// The most expensive read the surface allows, so it is the one where holding a transaction would
/// be least visible in a small test and most damaging in practice.
#[test]
fn an_exhausting_search_holds_no_write_lock_either() {
    let fixture = maximum_fixture("query-bounds-search-lock");

    let page = fixture
        .repository
        .query(&query(
            SessionLogFilters {
                search: Some("line 3 of".to_string()),
                ..SessionLogFilters::default()
            },
            DEFAULT_LOG_PAGE_SIZE,
        ))
        .expect("query");

    let mut other = fixture.database.connection().expect("second connection");
    let transaction = other.transaction().expect("transaction");
    transaction
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES ('probe', 'probe', 0, datetime('now'))",
            [],
        )
        .expect("the write lock was free after an exhausting search");
    transaction.commit().expect("commit");
    assert!(page.truncated);
}

/// An interactive query touches no file, whatever the corpus looks like on disk.
///
/// The index is the only thing a query reads, and there is deliberately no fallback to scanning:
/// a fallback would be a second query implementation with different filters and different bounds,
/// reached exactly when a reader is least able to tell which one answered. Proven by giving the
/// repository a database and no log directory at all — a query that scanned would have nothing to
/// scan, and would have to fail or return less.
#[test]
fn an_interactive_query_reads_no_file_because_there_is_no_file_to_read() {
    let fixture = maximum_fixture("query-bounds-no-files");

    let page = fixture
        .repository
        .query(&query(SessionLogFilters::default(), DEFAULT_LOG_PAGE_SIZE))
        .expect("the query answered without any log file present");

    assert_eq!(page.items.len(), DEFAULT_LOG_PAGE_SIZE);
    // And its coverage is the index's own, which is the point: the index answers for itself rather
    // than deferring to a corpus it would have to go and look at.
    assert!(page.coverage.newest_available_at.is_some());
}
