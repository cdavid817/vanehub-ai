//! What the bridge must never do to a log that is already written.

use super::log_index_bridge::{start_log_index_bridge, LOG_INDEX_QUEUE_CAPACITY};
use crate::contexts::operations::application::{
    IndexedSessionLogPage, IndexedSessionLogQuery, IndexedSessionLogRecord, LogCorrelation,
    LogIndexInsertOutcome, LogSourceIdentity, OperationsLogError, PostCommitLogNoticePublisher,
    RedactedLogRecord, SessionLogCoverage, SessionLogCoverageState, SessionLogIndexRepository,
    SessionLogNotice, SessionLogNoticeKind,
};
use crate::platform::log_receipts::{
    LogSourceWitness, RedactedLogAppendReceipt, RedactedLogAppendSink,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// An index that never returns. Stands in for a database that is busy, locked, or gone.
#[derive(Default)]
struct StalledIndex {
    release: Arc<Mutex<()>>,
    seen: AtomicU32,
}

impl SessionLogIndexRepository for StalledIndex {
    fn insert(
        &self,
        _record: &RedactedLogRecord,
    ) -> Result<LogIndexInsertOutcome, OperationsLogError> {
        self.seen.fetch_add(1, Ordering::SeqCst);
        // Blocks for as long as the test holds the lock, which is what makes the queue fill.
        let _held = self.release.lock();
        Ok(LogIndexInsertOutcome::Inserted { sequence: 1 })
    }

    fn query(
        &self,
        _query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("stalled"))
    }

    fn record(&self, _record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError> {
        Err(OperationsLogError::RecordNotFound)
    }

    fn coverage(
        &self,
        _session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        Ok(SessionLogCoverage::with_state(
            SessionLogCoverageState::Indexing,
        ))
    }

    fn watermark(&self) -> Result<i64, OperationsLogError> {
        Ok(0)
    }

    fn error_count(&self, _session_id: &str) -> Result<u32, OperationsLogError> {
        Ok(0)
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

    crate::contexts::operations::application::inert_repair_methods!();

    fn forget_sources(&self, _retained: &[LogSourceIdentity]) -> Result<u32, OperationsLogError> {
        Ok(0)
    }
}

/// An index that always fails. Stands in for a projection that cannot be written at all.
struct FailingIndex;

impl SessionLogIndexRepository for FailingIndex {
    fn insert(
        &self,
        _record: &RedactedLogRecord,
    ) -> Result<LogIndexInsertOutcome, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn query(
        &self,
        _query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn record(&self, _record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError> {
        Err(OperationsLogError::RecordNotFound)
    }

    fn coverage(
        &self,
        _session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn watermark(&self) -> Result<i64, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn error_count(&self, _session_id: &str) -> Result<u32, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn checkpoint(&self, _source: &LogSourceIdentity) -> Result<Option<u64>, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    fn record_gap(
        &self,
        _source: &LogSourceIdentity,
        _reason_code: &str,
        _dropped: u32,
    ) -> Result<(), OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }

    crate::contexts::operations::application::inert_repair_methods!();

    fn forget_sources(&self, _retained: &[LogSourceIdentity]) -> Result<u32, OperationsLogError> {
        Err(OperationsLogError::IndexUnavailable("closed"))
    }
}

#[derive(Default)]
struct RecordingNotices(Mutex<Vec<SessionLogNotice>>);

impl PostCommitLogNoticePublisher for RecordingNotices {
    fn publish(&self, notice: SessionLogNotice) {
        self.0.lock().expect("notices").push(notice);
    }
}

fn receipt(index: usize) -> RedactedLogAppendReceipt {
    RedactedLogAppendReceipt {
        record_id: format!("record-{index}"),
        source: LogSourceWitness {
            directory_generation: "generation-1".to_string(),
            file_id: "file-1".to_string(),
            offset: index as u64 * 100,
        },
        timestamp: "2026-08-24T10:00:00Z".to_string(),
        level: "info",
        category: "test".to_string(),
        message: "hello".to_string(),
        context: BTreeMap::from([("sessionId".to_string(), "session-1".to_string())]),
    }
}

/// The record is already on disk by the time a receipt exists. A full queue may cost the index a
/// row — repair will find it again in the file — but it must never reach back into the append.
#[test]
fn a_full_queue_does_not_fail_the_log_append_it_describes() {
    let index = Arc::new(StalledIndex::default());
    let held = index.release.lock().expect("hold the index");
    let (bridge, worker) =
        start_log_index_bridge(index.clone(), Arc::new(RecordingNotices::default()));

    // Far more than the queue holds, published from the caller's thread exactly as a log write does.
    for entry in 0..(LOG_INDEX_QUEUE_CAPACITY * 2) {
        bridge.record_appended(receipt(entry));
    }

    // Every publish returned. Nothing blocked, nothing panicked, nothing was reported to the caller.
    assert!(
        worker.counters().dropped_full.load(Ordering::SeqCst) > 0,
        "the queue never filled, so this test proved nothing about back-pressure"
    );
    drop(held);
    drop(bridge);
    worker.shutdown();
}

/// A dropped receipt reaches the live stream, not only the coverage table.
///
/// Coverage tells a *page* that it is short. A subscriber watching the live stream never asks for a
/// page — it accumulates notices — so a loss that only ever appeared in coverage would leave that
/// view missing exactly the records nobody could deliver, with nothing to say so.
#[test]
fn a_dropped_receipt_is_announced_as_a_gap_rather_than_only_recorded() {
    let index = Arc::new(StalledIndex::default());
    let held = index.release.lock().expect("hold the index");
    let notices = Arc::new(RecordingNotices::default());
    let (bridge, worker) = start_log_index_bridge(index.clone(), notices.clone());

    for entry in 0..(LOG_INDEX_QUEUE_CAPACITY * 2) {
        bridge.record_appended(receipt(entry));
    }
    assert!(
        worker.counters().dropped_full.load(Ordering::SeqCst) > 0,
        "the queue never filled, so no gap could be announced"
    );

    // Releasing the index lets the worker drain, and draining is when the queued gaps are written
    // down and announced.
    drop(held);
    drop(bridge);
    worker.shutdown();

    let published = notices.0.lock().expect("notices");
    let gaps = published
        .iter()
        .filter(|notice| notice.kind == SessionLogNoticeKind::Gap)
        .collect::<Vec<_>>();
    assert!(!gaps.is_empty(), "receipts were dropped in silence");
    for gap in gaps {
        assert_eq!(gap.reason_code.as_deref(), Some("log_receipt_dropped"));
        assert!(gap.dropped_count > 0);
        // Nothing to fetch, and nothing attributed: the receipt that carried the correlation is
        // the thing that was lost, so naming a session here would be a guess stated as a fact.
        assert!(gap.record_id.is_empty());
        assert_eq!(gap.correlation, LogCorrelation::default());
    }
}

/// An index that cannot be written is a projection problem. The log is unaffected, and the failure
/// is counted rather than logged — logging it would write a record, which produces a receipt, which
/// fails to index, which logs.
#[test]
fn an_index_that_cannot_be_written_is_counted_rather_than_propagated() {
    let (bridge, worker) = start_log_index_bridge(
        Arc::new(FailingIndex),
        Arc::new(RecordingNotices::default()),
    );

    for entry in 0..3 {
        bridge.record_appended(receipt(entry));
    }
    drop(bridge);
    worker.shutdown();

    assert_eq!(worker.counters().failed_inserts.load(Ordering::SeqCst), 3);
    assert_eq!(worker.counters().dropped_full.load(Ordering::SeqCst), 0);
}

/// A notice is published only for a row that was actually added, and it carries identifiers rather
/// than the line: a subscriber that heard about a row it cannot find would read that as a gap.
#[test]
fn a_notice_follows_the_row_that_was_written_and_carries_no_content() {
    let notices = Arc::new(RecordingNotices::default());
    let index = Arc::new(StalledIndex::default());
    let (bridge, worker) = start_log_index_bridge(index, notices.clone());

    bridge.record_appended(receipt(1));
    drop(bridge);
    worker.shutdown();

    let published = notices.0.lock().expect("notices");
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].record_id, "record-1");
    assert_eq!(
        published[0].correlation.session_id.as_deref(),
        Some("session-1")
    );
    // The correlation the log carried, not the text it carried.
    assert_eq!(
        published[0].coverage_state,
        SessionLogCoverageState::Indexing
    );
}
