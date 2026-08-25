//! Interactive session-log queries, and the bounded repair that keeps them honest.
//!
//! Every read here goes to the index and only to the index. There is no fallback to scanning log
//! files: a fallback would be a second query implementation with different filters, different
//! bounds and different coverage semantics, reached exactly when a reader is least able to tell
//! which one answered. When the index cannot answer, it says so.

use super::log_cursor::{filter_fingerprint, LogPageCursor};
use super::log_index::{
    IndexedSessionLogDetail, IndexedSessionLogPage, IndexedSessionLogQuery, LogFailureQuery,
    LogFailureSummary, OperationsLogError, SafeLogExportPreparation, SessionLogBackfillStatus,
    SessionLogCoverage, SessionLogCoverageState, SessionLogCoverageStateHolder,
    SessionLogSubscriptionBootstrap, SessionLogSummary, MAX_LOG_FAILURE_ROWS, MAX_LOG_PAGE_SIZE,
};
use super::log_index_ports::{
    BackfillOperationPublisher, LogIndexClock, LogIndexDiagnostics, LogIndexIdGenerator,
    RedactedLogSourceReader, SessionLogIndexRepository,
};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Every bound one repair pass answers to.
///
/// A log file can be pathological in more than one way — many tiny records, a few enormous ones,
/// one line that never ends, a directory holding hundreds of rotations — and a single bound only
/// catches the shape it was written for. Whichever trips first stops the work, so no input decides
/// how long the worker runs or how much it holds.
///
/// The line ceiling lives with the reader (`MAX_LOG_LINE_BYTES`) because that is where a line is
/// allocated; the rest live here because this is where the loops are.
pub(crate) const REPAIR_BATCH_RECORDS: usize = 500;
pub(crate) const REPAIR_BATCH_BYTES: u64 = 4 * 1024 * 1024;
/// How many source files one pass will touch. A pass that stopped short leaves the rest for the
/// next one, and the checkpoints mean the next one starts where this stopped.
pub(crate) const REPAIR_FILES_PER_PASS: usize = 64;
/// How many batches one file gets before the pass moves on. Without it a single enormous file
/// would be indexed to its end before any other file was looked at once.
pub(crate) const REPAIR_BATCHES_PER_FILE: usize = 64;
/// How many rows one prune transaction removes. The caller loops; each call is a transaction that
/// ends, so pruning a large generation never holds the write lock across all of it.
pub(crate) const REPAIR_PRUNE_ROWS: u32 = 500;

pub(crate) struct SessionLogQueryService {
    pub(super) index: Arc<dyn SessionLogIndexRepository>,
    pub(super) sources: Arc<dyn RedactedLogSourceReader>,
    pub(super) clock: Arc<dyn LogIndexClock>,
    pub(super) ids: Arc<dyn LogIndexIdGenerator>,
    pub(super) diagnostics: Arc<dyn LogIndexDiagnostics>,
    pub(super) backfill: Arc<dyn BackfillOperationPublisher>,
    /// The repair currently running, if any, and which directory generation it claimed.
    ///
    /// The generation is what makes single-flight mean something. Two passes over one corpus race
    /// each other to the same checkpoints and each undoes the other's progress; two passes over
    /// *different* corpora share nothing, so admitting the second is correct. Keying on the
    /// generation says which of those a second request is.
    pub(super) active_repair: Mutex<Option<ActiveRepair>>,
    pub(super) cancelled: AtomicBool,
}

/// The claim a running repair holds.
pub(super) struct ActiveRepair {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) directory_generation: String,
    pub(super) status: SessionLogBackfillStatus,
}

impl SessionLogQueryService {
    pub(crate) fn new(
        index: Arc<dyn SessionLogIndexRepository>,
        sources: Arc<dyn RedactedLogSourceReader>,
        clock: Arc<dyn LogIndexClock>,
        ids: Arc<dyn LogIndexIdGenerator>,
        diagnostics: Arc<dyn LogIndexDiagnostics>,
        backfill: Arc<dyn BackfillOperationPublisher>,
    ) -> Self {
        Self {
            index,
            sources,
            clock,
            ids,
            diagnostics,
            backfill,
            active_repair: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        }
    }

    /// One page, newest first.
    ///
    /// The cursor is validated against the current filters before it is used. A cursor issued for
    /// other filters is refused rather than applied: continuing would splice two result sets, and
    /// the seam would look like ordinary pagination.
    pub(crate) fn query(
        &self,
        query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        let fingerprint = filter_fingerprint(&query.scope, &query.filters);
        if let Some(cursor) = query.cursor.as_deref() {
            LogPageCursor::decode(cursor, &fingerprint)?;
        }
        if query.limit.is_some_and(|limit| limit > MAX_LOG_PAGE_SIZE) {
            return Err(OperationsLogError::InvalidQuery("log_page_limit_exceeded"));
        }
        let mut page = self.index.query(query)?;
        page.coverage = self.with_repair_state(page.coverage);
        Ok(page)
    }

    /// Downgrades a confident coverage while a repair is running.
    ///
    /// The rows a page returned are real; the set is not final. `complete` would claim otherwise,
    /// and a reader acting on a "complete" that was really "still filling in" draws exactly the
    /// conclusion this whole type exists to prevent. A `partial` is left alone: it says something
    /// is *known* to be missing, which is a stronger and different fact that `indexing` would blur.
    pub(super) fn with_repair_state(&self, mut coverage: SessionLogCoverage) -> SessionLogCoverage {
        if let Some(state) = self.repair_coverage_state() {
            if coverage.state() == SessionLogCoverageState::Complete {
                coverage.state = SessionLogCoverageStateHolder(state);
            }
        }
        coverage
    }

    /// The authoritative row behind a live notice.
    pub(crate) fn record(
        &self,
        record_id: &str,
    ) -> Result<IndexedSessionLogDetail, OperationsLogError> {
        Ok(IndexedSessionLogDetail {
            record: self.index.record(record_id)?,
            coverage: self.with_repair_state(self.index.coverage(None)?),
        })
    }

    /// The narrow figure the workspace summary badge shows.
    ///
    /// Carries its coverage, so a zero next to `partial` renders as "not observed" rather than as
    /// "no errors happened".
    pub(crate) fn summary(
        &self,
        session_id: &str,
    ) -> Result<SessionLogSummary, OperationsLogError> {
        Ok(SessionLogSummary {
            new_errors: self.index.error_count(session_id)?,
            coverage: self.with_repair_state(self.index.coverage(Some(session_id))?),
            session_id: session_id.to_string(),
        })
    }

    /// Error rows grouped by category, for a session-run report.
    ///
    /// Carries the same coverage the badge count does, and for the same reason: a report section
    /// showing no failures over an index that is still building would read as a clean session.
    pub(crate) fn failure_summary(
        &self,
        query: &LogFailureQuery,
    ) -> Result<LogFailureSummary, OperationsLogError> {
        let mut rows = self.index.failure_counts(query, MAX_LOG_FAILURE_ROWS)?;
        let truncated = rows.len() > MAX_LOG_FAILURE_ROWS;
        rows.truncate(MAX_LOG_FAILURE_ROWS);
        Ok(LogFailureSummary {
            rows,
            coverage: self.with_repair_state(self.index.coverage(Some(&query.session_id))?),
            truncated,
        })
    }

    /// Where a subscriber resumes from.
    ///
    /// Read after the listener is already registered, which is the caller's responsibility and the
    /// reason this is a separate call rather than part of subscribing: reading first would lose
    /// every notice published in the window, and the sequences the subscriber then saw would be
    /// contiguous.
    pub(crate) fn subscription_bootstrap(
        &self,
    ) -> Result<SessionLogSubscriptionBootstrap, OperationsLogError> {
        Ok(SessionLogSubscriptionBootstrap {
            watermark_sequence: self.index.watermark()?,
            coverage: self.with_repair_state(self.index.coverage(None)?),
        })
    }

    /// What an export may read: redacted files, never the index.
    pub(crate) fn export_preparation(
        &self,
    ) -> Result<SafeLogExportPreparation, OperationsLogError> {
        let coverage = self.index.coverage(None)?;
        Ok(SafeLogExportPreparation {
            source_files: self.sources.export_sources()?,
            oldest_available_at: coverage.oldest_available_at,
            newest_available_at: coverage.newest_available_at,
            redacted: true,
        })
    }

    pub(crate) fn coverage(
        &self,
        session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        Ok(self.with_repair_state(self.index.coverage(session_id)?))
    }
}
