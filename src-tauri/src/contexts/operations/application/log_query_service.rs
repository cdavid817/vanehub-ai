//! Interactive session-log queries, and the bounded repair that keeps them honest.
//!
//! Every read here goes to the index and only to the index. There is no fallback to scanning log
//! files: a fallback would be a second query implementation with different filters, different
//! bounds and different coverage semantics, reached exactly when a reader is least able to tell
//! which one answered. When the index cannot answer, it says so.

use super::log_cursor::{filter_fingerprint, LogPageCursor};
use super::log_index::{
    IndexedSessionLogDetail, IndexedSessionLogPage, IndexedSessionLogQuery, OperationsLogError,
    SafeLogExportPreparation, SessionLogBackfillState, SessionLogBackfillStatus,
    SessionLogCoverage, SessionLogSubscriptionBootstrap, SessionLogSummary, MAX_LOG_PAGE_SIZE,
};
use super::log_index_ports::{
    BackfillOperationPublisher, LogIndexClock, LogIndexDiagnostics, LogIndexIdGenerator,
    LogIndexInsertOutcome, RedactedLogSourceReader, SessionLogIndexRepository,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// How much one repair batch may read before yielding.
///
/// Three bounds rather than one, because a log file can be pathological in three ways: many tiny
/// records, few enormous ones, or one line that never ends. Whichever bound trips first stops the
/// batch, so no single read can hold the worker for an unbounded time.
pub(crate) const REPAIR_BATCH_RECORDS: usize = 500;
pub(crate) const REPAIR_BATCH_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) struct SessionLogQueryService {
    index: Arc<dyn SessionLogIndexRepository>,
    sources: Arc<dyn RedactedLogSourceReader>,
    clock: Arc<dyn LogIndexClock>,
    ids: Arc<dyn LogIndexIdGenerator>,
    diagnostics: Arc<dyn LogIndexDiagnostics>,
    backfill: Arc<dyn BackfillOperationPublisher>,
    /// The repair currently running, if any. A second request joins the first rather than starting
    /// a competing pass over the same files.
    active_repair: Mutex<Option<SessionLogBackfillStatus>>,
    cancelled: AtomicBool,
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
        self.index.query(query)
    }

    /// The authoritative row behind a live notice.
    pub(crate) fn record(
        &self,
        record_id: &str,
    ) -> Result<IndexedSessionLogDetail, OperationsLogError> {
        Ok(IndexedSessionLogDetail {
            record: self.index.record(record_id)?,
            coverage: self.index.coverage(None)?,
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
            coverage: self.index.coverage(Some(session_id))?,
            session_id: session_id.to_string(),
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
            coverage: self.index.coverage(None)?,
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
        self.index.coverage(session_id)
    }

    pub(crate) fn backfill_status(&self) -> SessionLogBackfillStatus {
        self.active_repair
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_else(|| self.idle_status())
    }

    fn idle_status(&self) -> SessionLogBackfillStatus {
        SessionLogBackfillStatus {
            operation_id: String::new(),
            state: SessionLogBackfillState::Idle,
            files_completed: 0,
            files_total: 0,
            records_indexed: 0,
            started_at: None,
            updated_at: Some(self.clock.now()),
            reason_code: None,
        }
    }

    /// Asks the running repair to stop.
    ///
    /// Committed checkpoints stay. A cancel that rolled back would make cancelling more expensive
    /// than finishing, which is the opposite of what a cancel is for.
    pub(crate) fn cancel_repair(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Indexes retained source records that the index does not hold yet.
    ///
    /// Reads and parses outside any database transaction, in bounded batches, resuming from the
    /// checkpoint each source last reached. Returns when the corpus is caught up, when a bound is
    /// reached, or when cancellation is observed.
    pub(crate) fn repair(&self) -> SessionLogBackfillStatus {
        self.cancelled.store(false, Ordering::SeqCst);
        let operation_id = self.ids.next_operation_id();
        let started_at = self.clock.now();
        let sources = match self.sources.sources() {
            Ok(sources) => sources,
            Err(error) => return self.finish(operation_id, started_at, 0, 0, 0, Some(error)),
        };
        // Rows whose source is gone stop counting toward the corpus, and the oldest queryable
        // boundary moves with them. Skipping this would let retention silently shrink the files
        // while the index kept reporting the old span as available.
        if let Err(error) = self.index.forget_sources(&sources) {
            self.diagnostics.report(
                error.code(),
                BTreeMap::from([("stage".into(), "retention".into())]),
            );
        }

        let files_total = u32::try_from(sources.len()).unwrap_or(u32::MAX);
        let mut files_completed = 0u32;
        let mut records_indexed = 0u64;
        for source in &sources {
            if self.cancelled.load(Ordering::SeqCst) {
                return self
                    .finish(
                        operation_id,
                        started_at,
                        files_completed,
                        files_total,
                        records_indexed,
                        None,
                    )
                    .with_state(SessionLogBackfillState::Cancelled);
            }
            let mut offset = self.index.checkpoint(source).ok().flatten().unwrap_or(0);
            loop {
                if self.cancelled.load(Ordering::SeqCst) {
                    break;
                }
                let batch = match self.sources.read_batch(
                    source,
                    offset,
                    REPAIR_BATCH_RECORDS,
                    REPAIR_BATCH_BYTES,
                ) {
                    Ok(batch) => batch,
                    Err(error) => {
                        self.diagnostics.report(
                            error.code(),
                            BTreeMap::from([
                                ("source".into(), source.as_key()),
                                ("offset".into(), offset.to_string()),
                            ]),
                        );
                        break;
                    }
                };
                for record in &batch.records {
                    match self.index.insert(record) {
                        Ok(LogIndexInsertOutcome::Inserted { .. }) => records_indexed += 1,
                        // A repeated pass over the same line is the normal case, not an error: the
                        // whole point of a deterministic id is that finding it again is a no-op.
                        Ok(LogIndexInsertOutcome::AlreadyIndexed) => {}
                        Ok(LogIndexInsertOutcome::Conflicted) => {
                            let _ = self.index.record_gap(source, "log_identity_conflict", 1);
                        }
                        Err(error) => {
                            self.diagnostics.report(
                                error.code(),
                                BTreeMap::from([("source".into(), source.as_key())]),
                            );
                        }
                    }
                }
                if batch.rejected > 0 {
                    // Counted, never quoted: the text that failed to parse is source content.
                    let _ = self
                        .index
                        .record_gap(source, "log_record_rejected", batch.rejected);
                }
                // Advanced only past complete lines. A partial trailing line is one the writer has
                // not finished, and consuming it would index half a record and never see the rest.
                let advanced = batch.next_offset > offset;
                offset = batch.next_offset;
                if let Err(error) = self.index.save_checkpoint(source, offset) {
                    self.diagnostics.report(
                        error.code(),
                        BTreeMap::from([("stage".into(), "checkpoint".into())]),
                    );
                }
                if batch.reached_end || !advanced {
                    break;
                }
            }
            files_completed += 1;
            self.backfill.publish(SessionLogBackfillStatus {
                operation_id: operation_id.clone(),
                state: SessionLogBackfillState::Running,
                files_completed,
                files_total,
                records_indexed,
                started_at: Some(started_at.clone()),
                updated_at: Some(self.clock.now()),
                reason_code: None,
            });
        }
        self.finish(
            operation_id,
            started_at,
            files_completed,
            files_total,
            records_indexed,
            None,
        )
    }

    fn finish(
        &self,
        operation_id: String,
        started_at: String,
        files_completed: u32,
        files_total: u32,
        records_indexed: u64,
        error: Option<OperationsLogError>,
    ) -> SessionLogBackfillStatus {
        let status = SessionLogBackfillStatus {
            operation_id,
            state: if error.is_some() {
                SessionLogBackfillState::Failed
            } else {
                SessionLogBackfillState::Completed
            },
            files_completed,
            files_total,
            records_indexed,
            started_at: Some(started_at),
            updated_at: Some(self.clock.now()),
            reason_code: error.map(|error| error.code().to_string()),
        };
        if let Ok(mut slot) = self.active_repair.lock() {
            *slot = Some(status.clone());
        }
        self.backfill.publish(status.clone());
        status
    }
}

impl SessionLogBackfillStatus {
    fn with_state(mut self, state: SessionLogBackfillState) -> Self {
        self.state = state;
        self
    }
}
