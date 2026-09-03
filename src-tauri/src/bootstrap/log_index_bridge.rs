//! From a durable log append to an indexed row, without either side knowing the other.
//!
//! `platform::logging` publishes a receipt and returns. This picks receipts up on a bounded queue,
//! translates them into the operations context's vocabulary, and writes them to the index on a
//! worker thread. Neither side imports the other; the bridge is the only thing that knows both.
//!
//! Everything about the shape follows from one rule: **the log has already been written**. By the
//! time a receipt exists, the user's record is durably on disk. Nothing here may turn a problem
//! with the projection into a problem with the record — not a full queue, not a failed insert, not
//! a database that is gone. So the queue is bounded and lossy, the send never blocks, and a failure
//! is counted rather than propagated.

use crate::contexts::operations::application::{
    IndexedLogLevel, LogCorrelation, LogIndexInsertOutcome, LogSourceIdentity,
    PostCommitLogNoticePublisher, RedactedLogRecord, SessionLogCoverageState,
    SessionLogIndexRepository, SessionLogNotice, SessionLogNoticeKind,
};
use crate::platform::log_receipts::{RedactedLogAppendReceipt, RedactedLogAppendSink};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How many receipts may wait to be indexed.
///
/// Deep enough that an ordinary burst — a failing retry loop, a noisy startup — is absorbed, and
/// shallow enough that a stalled indexer costs bounded memory rather than growing until the process
/// dies. A dropped receipt costs a row that repair will find again in the file it came from.
pub(crate) const LOG_INDEX_QUEUE_CAPACITY: usize = 512;

/// Correlation keys the logging store already carries in its safe context.
///
/// Read from context rather than added to the receipt: these are values producers already attach,
/// and duplicating them into a second field would let the two disagree.
const SESSION_KEY: &str = "sessionId";
const RUN_KEY: &str = "runId";
const TRACE_KEY: &str = "traceId";
const SPAN_KEY: &str = "spanId";
const OPERATION_KEY: &str = "operationId";
const AGENT_KEY: &str = "agentId";
const SEAT_KEY: &str = "seatId";

/// What the bridge could not do, counted rather than logged.
///
/// Logging an indexing failure would write a log record, which produces a receipt, which fails to
/// index, which logs — one failure becoming a loop. Counters are read by whoever asks instead.
#[derive(Debug, Default)]
pub(crate) struct LogIndexBridgeCounters {
    pub(crate) dropped_full: AtomicU32,
    pub(crate) failed_inserts: AtomicU32,
    pub(crate) conflicts: AtomicU32,
}

/// Sources whose receipts were dropped, waiting to be written down as gaps.
///
/// Recorded here rather than in the drop itself because the drop happens on the caller's thread —
/// the one that just wrote a log — and reaching into SQLite there is exactly the back-pressure this
/// whole design refuses. The worker drains them next time it is awake.
type DroppedSources = Arc<Mutex<BTreeSet<LogSourceIdentity>>>;

/// The producer-facing half. Cheap to clone, never blocks.
pub(crate) struct LogIndexBridge {
    sender: SyncSender<RedactedLogAppendReceipt>,
    counters: Arc<LogIndexBridgeCounters>,
    dropped: DroppedSources,
}

impl Clone for LogIndexBridge {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            counters: self.counters.clone(),
            dropped: self.dropped.clone(),
        }
    }
}

impl RedactedLogAppendSink for LogIndexBridge {
    fn record_appended(&self, receipt: RedactedLogAppendReceipt) {
        match self.sender.try_send(receipt) {
            Ok(()) => {}
            // Full or disconnected are the same decision here: the record is already durable, and
            // the index is rebuildable from the file it is in. What must not happen is the index
            // going on to report `complete` — so the drop is queued as a gap the same way it is
            // counted, and coverage degrades until a repair fills it back in.
            Err(TrySendError::Full(receipt)) | Err(TrySendError::Disconnected(receipt)) => {
                self.counters.dropped_full.fetch_add(1, Ordering::Relaxed);
                self.dropped
                    .lock()
                    .map(|mut pending| {
                        pending.insert(LogSourceIdentity {
                            directory_generation: receipt.source.directory_generation,
                            file_id: receipt.source.file_id,
                        })
                    })
                    .ok();
            }
        }
    }
}

/// How long a shutdown waits for the worker to drain.
///
/// A backstop rather than the mechanism: the sink is uninstalled first, so the ordinary path ends in
/// microseconds. This exists for the day somebody adds a second sender.
const SHUTDOWN_GRACE: Duration = Duration::from_millis(500);

/// The consumer half, kept so the worker can be joined at shutdown.
pub(crate) struct LogIndexBridgeWorker {
    worker: Mutex<Option<JoinHandle<()>>>,
    #[cfg_attr(not(test), allow(dead_code))]
    counters: Arc<LogIndexBridgeCounters>,
}

impl LogIndexBridgeWorker {
    /// Waits for the worker to finish the receipts it already holds.
    ///
    /// The caller uninstalls the process-wide sink first, which drops the only sender and lets the
    /// worker drain and stop. That was always the intent and was never true: the sink is a
    /// `'static` that nothing released, so the worker's channel never closed and this join could not
    /// return. A deadline is kept anyway, because "the sender is dropped first" is a property
    /// somebody can break again, and an unbounded wait on the exit path turns that mistake into an
    /// application that cannot be closed.
    pub(crate) fn shutdown(&self) {
        let Some(worker) = self.worker.lock().ok().and_then(|mut slot| slot.take()) else {
            return;
        };
        let deadline = Instant::now() + SHUTDOWN_GRACE;
        while !worker.is_finished() {
            if Instant::now() >= deadline {
                // Whatever is still queued is lost. A log index is a convenience over files that
                // are already written, so losing its tail is strictly better than refusing to
                // close.
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = worker.join();
    }

    /// How many receipts the bridge could not index, by reason.
    ///
    /// Read rather than logged: diagnosing a failed index write by writing a log record would
    /// produce another receipt, another failure, and another diagnosis.
    #[cfg(test)]
    pub(crate) fn counters(&self) -> Arc<LogIndexBridgeCounters> {
        self.counters.clone()
    }
}

/// Starts the bridge and returns both halves.
pub(crate) fn start_log_index_bridge(
    repository: Arc<dyn SessionLogIndexRepository>,
    notices: Arc<dyn PostCommitLogNoticePublisher>,
) -> (LogIndexBridge, Arc<LogIndexBridgeWorker>) {
    let (sender, receiver) = sync_channel(LOG_INDEX_QUEUE_CAPACITY);
    let counters = Arc::new(LogIndexBridgeCounters::default());
    let dropped: DroppedSources = Arc::new(Mutex::new(BTreeSet::new()));
    let worker = spawn_worker(
        receiver,
        repository,
        notices,
        counters.clone(),
        dropped.clone(),
    );
    (
        LogIndexBridge {
            sender,
            counters: counters.clone(),
            dropped,
        },
        Arc::new(LogIndexBridgeWorker {
            worker: Mutex::new(worker),
            counters,
        }),
    )
}

fn spawn_worker(
    receiver: Receiver<RedactedLogAppendReceipt>,
    repository: Arc<dyn SessionLogIndexRepository>,
    notices: Arc<dyn PostCommitLogNoticePublisher>,
    counters: Arc<LogIndexBridgeCounters>,
    dropped: DroppedSources,
) -> Option<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("vanehub-log-index".to_string())
        .spawn(move || {
            for receipt in receiver {
                // Whatever was dropped while the queue was full is written down here, on the
                // worker's thread, where reaching into SQLite is allowed.
                record_dropped_sources(&repository, &notices, &dropped);
                let record = to_record(receipt);
                match repository.insert(&record) {
                    // Announced after the write committed, and only for a row that was actually
                    // added. A notice for a row a subscriber then cannot find would read as a gap,
                    // which is a lie about lost data; a second notice for a re-indexed row would
                    // make one record look like two.
                    Ok(LogIndexInsertOutcome::Inserted { sequence }) => {
                        let coverage_state = repository
                            .coverage(record.correlation.session_id.as_deref())
                            .map(|coverage| coverage.state())
                            .unwrap_or(SessionLogCoverageState::Unavailable);
                        notices.publish(SessionLogNotice {
                            kind: SessionLogNoticeKind::Appended,
                            record_id: record.record_id,
                            sequence,
                            occurred_at: record.occurred_at,
                            level: record.level,
                            correlation: record.correlation,
                            coverage_state,
                            dropped_count: 0,
                            reason_code: None,
                        });
                    }
                    Ok(LogIndexInsertOutcome::AlreadyIndexed) => {}
                    Ok(LogIndexInsertOutcome::Conflicted) => {
                        counters.conflicts.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        counters.failed_inserts.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            // Once more on the way out. A burst that filled the queue and then went quiet would
            // otherwise leave its gaps unwritten until the next log line, and an application that
            // closed in between would come back reporting coverage it had not earned.
            record_dropped_sources(&repository, &notices, &dropped);
        })
        .ok()
}

/// Writes down the sources whose receipts were dropped.
///
/// Takes the whole set at once and clears it, so a source is recorded once per burst rather than
/// once per lost record: the gap is "this file has a hole", and a thousand rows of the same fact
/// would be a second unbounded structure inside the thing that exists to be bounded.
/// A gap is also announced, not only written down. Coverage tells a page it is incomplete, but a
/// subscriber watching the live stream never asks for a page — it accumulates notices — so without
/// a notice its view would stay short by exactly the records nobody could deliver, and look whole.
fn record_dropped_sources(
    repository: &Arc<dyn SessionLogIndexRepository>,
    notices: &Arc<dyn PostCommitLogNoticePublisher>,
    dropped: &DroppedSources,
) {
    let pending = match dropped.lock() {
        Ok(mut pending) if !pending.is_empty() => std::mem::take(&mut *pending),
        _ => return,
    };
    for source in pending {
        let _ = repository.record_gap(&source, "log_receipt_dropped", 1);
        // The current watermark, so the gap lands in order among the rows it sits between rather
        // than ahead of everything a subscriber has already seen.
        let sequence = repository.watermark().unwrap_or_default();
        notices.publish(SessionLogNotice {
            kind: SessionLogNoticeKind::Gap,
            // No row to fetch. An id here would be fetched, and the miss would be reported as an
            // error rather than as the loss it is.
            record_id: String::new(),
            sequence,
            occurred_at: String::new(),
            level: IndexedLogLevel::Warn,
            // No correlation either: the receipt that carried it is the thing that was dropped, so
            // attributing the gap to a session would be a guess presented as a fact.
            correlation: LogCorrelation::default(),
            coverage_state: repository
                .coverage(None)
                .map(|coverage| coverage.state())
                .unwrap_or(SessionLogCoverageState::Unavailable),
            dropped_count: 1,
            reason_code: Some("log_receipt_dropped".to_string()),
        });
    }
}

fn correlation(context: &BTreeMap<String, String>) -> LogCorrelation {
    let read = |key: &str| context.get(key).filter(|value| !value.is_empty()).cloned();
    LogCorrelation {
        session_id: read(SESSION_KEY),
        run_id: read(RUN_KEY),
        trace_id: read(TRACE_KEY),
        span_id: read(SPAN_KEY),
        operation_id: read(OPERATION_KEY),
        agent_id: read(AGENT_KEY),
        seat_id: read(SEAT_KEY),
    }
}

/// Milliseconds from the record's own timestamp.
///
/// Parsed rather than stamped on arrival: the ordering a reader sees has to be the order the
/// records happened in, and a receipt that waited in the queue would otherwise sort after one that
/// happened later and did not.
fn occurred_at_ms(timestamp: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|value| value.timestamp_millis())
        .unwrap_or_default()
}

fn to_record(receipt: RedactedLogAppendReceipt) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: receipt.record_id,
        source: LogSourceIdentity {
            directory_generation: receipt.source.directory_generation,
            file_id: receipt.source.file_id,
        },
        source_offset: receipt.source.offset,
        occurred_at_ms: occurred_at_ms(&receipt.timestamp),
        occurred_at: receipt.timestamp,
        level: IndexedLogLevel::parse(receipt.level).unwrap_or(IndexedLogLevel::Info),
        category: receipt.category,
        correlation: correlation(&receipt.context),
        message: receipt.message,
        context: receipt.context,
    }
}
