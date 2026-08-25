//! What the query service refuses before it reaches the index.
//!
//! The repository's own tests prove the SQL is right. These prove the calls that must never reach
//! the SQL at all: a bound the caller exceeded, a cursor issued for a different question. Both are
//! refusals rather than best-effort answers, because the failure mode of a best-effort answer here
//! is a reader who cannot tell that they were served something other than what they asked for.

use super::log_cursor::{filter_fingerprint, LogPageCursor, LogSortDirection};
use super::log_index::{
    IndexedLogLevel, IndexedSessionLogPage, IndexedSessionLogQuery, IndexedSessionLogRecord,
    LogFailureCount, LogFailureQuery, OperationsLogError, SessionLogBackfillStatus,
    SessionLogCoverage, SessionLogCoverageState, SessionLogCoverageStateHolder, SessionLogFilters,
    SessionLogQueryScope, MAX_LOG_PAGE_SIZE,
};
use super::log_index_ports::{
    BackfillOperationPublisher, LogIndexClock, LogIndexDiagnostics, LogIndexIdGenerator,
    LogIndexInsertOutcome, LogSourceIdentity, LogSourceSnapshot, RedactedLogBatch,
    RedactedLogRecord, RedactedLogSourceReader, SessionLogIndexRepository,
};
use super::log_query_service::SessionLogQueryService;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// An index that answers nothing and counts how often it was asked.
///
/// The count is the assertion: a refusal that still queried would have done the work it was
/// supposed to avoid, and on a real corpus that work is the whole cost.
#[derive(Default)]
struct CountingIndex {
    queries: AtomicUsize,
}

impl SessionLogIndexRepository for CountingIndex {
    fn insert(
        &self,
        _record: &RedactedLogRecord,
    ) -> Result<LogIndexInsertOutcome, OperationsLogError> {
        Ok(LogIndexInsertOutcome::Inserted { sequence: 1 })
    }

    fn query(
        &self,
        _query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        Ok(IndexedSessionLogPage {
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
            coverage: SessionLogCoverage {
                state: SessionLogCoverageStateHolder(SessionLogCoverageState::Complete),
                ..SessionLogCoverage::default()
            },
        })
    }

    fn record(&self, _record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError> {
        Err(OperationsLogError::RecordNotFound)
    }

    fn coverage(
        &self,
        _session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        Ok(SessionLogCoverage::default())
    }

    fn watermark(&self) -> Result<i64, OperationsLogError> {
        Ok(0)
    }

    fn error_count(&self, _session_id: &str) -> Result<u32, OperationsLogError> {
        Ok(0)
    }

    fn failure_counts(
        &self,
        _query: &LogFailureQuery,
        _limit: usize,
    ) -> Result<Vec<LogFailureCount>, OperationsLogError> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    fn checkpoint(&self, _source: &LogSourceIdentity) -> Result<Option<u64>, OperationsLogError> {
        Ok(None)
    }

    fn record_gap(
        &self,
        _source: &LogSourceIdentity,
        _reason_code: &str,
        _dropped: u32,
    ) -> Result<(), OperationsLogError> {
        Ok(())
    }

    super::log_index_test_doubles::inert_repair_methods!();

    fn expire_sources(
        &self,
        _retained: &[LogSourceIdentity],
        _limit: u32,
    ) -> Result<u32, OperationsLogError> {
        Ok(0)
    }
}

struct EmptySources;

impl RedactedLogSourceReader for EmptySources {
    fn sources(&self) -> Result<Vec<LogSourceSnapshot>, OperationsLogError> {
        Ok(Vec::new())
    }

    fn read_batch(
        &self,
        _source: &LogSourceIdentity,
        _from_offset: u64,
        _max_records: usize,
        _max_bytes: u64,
    ) -> Result<RedactedLogBatch, OperationsLogError> {
        Ok(RedactedLogBatch {
            records: Vec::new(),
            next_offset: 0,
            reached_end: true,
            rejections: Default::default(),
        })
    }

    fn export_sources(&self) -> Result<Vec<String>, OperationsLogError> {
        Ok(Vec::new())
    }
}

struct FixedClock;

impl LogIndexClock for FixedClock {
    fn now(&self) -> String {
        "2026-08-24T10:00:00Z".to_string()
    }
}

struct FixedIds;

impl LogIndexIdGenerator for FixedIds {
    fn next_operation_id(&self) -> String {
        "operation-1".to_string()
    }
}

struct SilentDiagnostics;

impl LogIndexDiagnostics for SilentDiagnostics {
    fn report(&self, _reason_code: &str, _context: BTreeMap<String, String>) {}
}

struct SilentBackfill;

impl BackfillOperationPublisher for SilentBackfill {
    fn publish(&self, _status: SessionLogBackfillStatus) {}
}

fn service() -> (SessionLogQueryService, Arc<CountingIndex>) {
    let index = Arc::new(CountingIndex::default());
    let service = SessionLogQueryService::new(
        index.clone(),
        Arc::new(EmptySources),
        Arc::new(FixedClock),
        Arc::new(FixedIds),
        Arc::new(SilentDiagnostics),
        Arc::new(SilentBackfill),
    );
    (service, index)
}

fn scope(session: &str) -> SessionLogQueryScope {
    SessionLogQueryScope {
        session_id: Some(session.to_string()),
        ..SessionLogQueryScope::default()
    }
}

/// A limit above the ceiling is an error, not a smaller page.
///
/// Silently clamping would hand the caller five hundred rows in answer to a request for a thousand
/// and no way to know it happened, so their next cursor would skip the five hundred they never saw.
#[test]
fn a_page_limit_above_the_ceiling_is_refused_before_the_index_is_touched() {
    let (service, index) = service();

    let refused = service.query(&IndexedSessionLogQuery {
        scope: scope("session-1"),
        filters: SessionLogFilters::default(),
        cursor: None,
        limit: Some(MAX_LOG_PAGE_SIZE + 1),
    });

    assert!(matches!(
        refused,
        Err(OperationsLogError::InvalidQuery("log_page_limit_exceeded"))
    ));
    assert_eq!(
        index.queries.load(Ordering::SeqCst),
        0,
        "the refusal still ran the query"
    );
}

/// The ceiling itself is allowed. An off-by-one here would make the documented maximum unusable.
#[test]
fn a_page_limit_at_the_ceiling_is_served() {
    let (service, index) = service();

    service
        .query(&IndexedSessionLogQuery {
            scope: scope("session-1"),
            filters: SessionLogFilters::default(),
            cursor: None,
            limit: Some(MAX_LOG_PAGE_SIZE),
        })
        .expect("the ceiling is a valid request");

    assert_eq!(index.queries.load(Ordering::SeqCst), 1);
}

/// A cursor from one question cannot continue another.
///
/// The service checks the fingerprint before the index sees the cursor, so a mismatched cursor
/// costs a rejection rather than a page assembled from two different result sets.
#[test]
fn a_cursor_issued_for_other_filters_is_refused_before_the_index_is_touched() {
    let (service, index) = service();
    let issued_scope = scope("session-1");
    let issued_filters = SessionLogFilters {
        levels: vec![IndexedLogLevel::Error],
        sort: LogSortDirection::NewestFirst,
        ..SessionLogFilters::default()
    };
    let cursor = LogPageCursor {
        occurred_at_ms: 1,
        sequence: 1,
        record_id: "record-1".to_string(),
        filter_fingerprint: filter_fingerprint(&issued_scope, &issued_filters),
    }
    .encode();

    // Same cursor, one level added: a different question, and a different fingerprint.
    let refused = service.query(&IndexedSessionLogQuery {
        scope: issued_scope.clone(),
        filters: SessionLogFilters {
            levels: vec![IndexedLogLevel::Error, IndexedLogLevel::Warn],
            ..issued_filters.clone()
        },
        cursor: Some(cursor.clone()),
        limit: Some(10),
    });

    assert!(matches!(
        refused,
        Err(OperationsLogError::CursorFilterMismatch)
    ));
    assert_eq!(
        index.queries.load(Ordering::SeqCst),
        0,
        "a mismatched cursor still reached the index"
    );

    // The same cursor against the question it was issued for is accepted.
    service
        .query(&IndexedSessionLogQuery {
            scope: issued_scope,
            filters: issued_filters,
            cursor: Some(cursor),
            limit: Some(10),
        })
        .expect("its own filters accept it");
    assert_eq!(index.queries.load(Ordering::SeqCst), 1);
}

/// Text that is not a cursor is refused as a cursor, never treated as absent.
///
/// Falling back to "no cursor" would silently restart the reader at the newest page, which reads
/// as a refresh rather than as the error it is.
#[test]
fn a_malformed_cursor_is_an_error_rather_than_a_fresh_first_page() {
    let (service, index) = service();

    for candidate in ["", "not-a-cursor", "0:0:", "eyJhIjoxfQ=="] {
        let refused = service.query(&IndexedSessionLogQuery {
            scope: scope("session-1"),
            filters: SessionLogFilters::default(),
            cursor: Some(candidate.to_string()),
            limit: Some(10),
        });
        assert!(
            matches!(
                refused,
                Err(OperationsLogError::InvalidCursor | OperationsLogError::CursorFilterMismatch)
            ),
            "{candidate:?} was accepted"
        );
    }

    assert_eq!(index.queries.load(Ordering::SeqCst), 0);
}
