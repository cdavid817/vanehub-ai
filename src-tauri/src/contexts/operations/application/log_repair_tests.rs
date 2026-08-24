//! What a repair pass promises, and what it refuses to claim.
//!
//! Driven through in-memory doubles rather than a database, because everything under test here is a
//! decision the pass makes — when to commit, when to stop, what it is allowed to delete — and a real
//! SQLite would only make those decisions slower to observe. The transaction boundary those
//! decisions rely on is proved against a real database in `log_index_repair_store_tests`.

use super::log_index::{
    IndexedSessionLogPage, IndexedSessionLogQuery, IndexedSessionLogRecord, OperationsLogError,
    SessionLogBackfillState, SessionLogBackfillStatus, SessionLogCoverage, SessionLogCoverageState,
};
use super::log_index_ports::{
    BackfillOperationPublisher, LineRejections, LogBatchCommit, LogIndexClock, LogIndexDiagnostics,
    LogIndexIdGenerator, LogIndexInsertOutcome, LogSourceIdentity, LogSourceSnapshot,
    RedactedLogBatch, RedactedLogRecord, RedactedLogSourceReader, SessionLogIndexRepository,
};
use super::log_query_service::{SessionLogQueryService, REPAIR_BATCH_RECORDS};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------------------------
// Doubles
// ---------------------------------------------------------------------------------------------

/// What a commit did to the store, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Committed {
        source: String,
        records: usize,
        rejections: Vec<(String, u32)>,
        next_offset: u64,
    },
    Gap {
        source: String,
        reason: String,
    },
    Forgot,
    ClearedGaps {
        through_id: i64,
    },
    Pruned {
        source: String,
    },
}

#[derive(Default)]
struct RecordingIndex {
    events: Mutex<Vec<Event>>,
    checkpoints: Mutex<BTreeMap<String, u64>>,
    repair_state: Mutex<Option<SessionLogBackfillStatus>>,
    /// Commits that must fail before any succeed. Stands in for a store that is unavailable.
    failing_commits: Mutex<usize>,
    conflicts: AtomicUsize,
    gap_watermark: Mutex<i64>,
    /// Rows left per source, so a bounded prune has something to drain.
    prunable: Mutex<BTreeMap<String, u32>>,
}

impl RecordingIndex {
    fn events(&self) -> Vec<Event> {
        self.events.lock().expect("events").clone()
    }

    fn push(&self, event: Event) {
        self.events.lock().expect("events").push(event);
    }

    fn checkpoint_of(&self, key: &str) -> Option<u64> {
        self.checkpoints
            .lock()
            .expect("checkpoints")
            .get(key)
            .copied()
    }
}

impl SessionLogIndexRepository for RecordingIndex {
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
        Ok(IndexedSessionLogPage {
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
            coverage: SessionLogCoverage::with_state(SessionLogCoverageState::Complete),
        })
    }

    fn record(&self, _record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError> {
        Err(OperationsLogError::RecordNotFound)
    }

    fn coverage(
        &self,
        _session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        Ok(SessionLogCoverage::with_state(
            SessionLogCoverageState::Complete,
        ))
    }

    fn watermark(&self) -> Result<i64, OperationsLogError> {
        Ok(0)
    }

    fn error_count(&self, _session_id: &str) -> Result<u32, OperationsLogError> {
        Ok(0)
    }

    fn checkpoint(&self, source: &LogSourceIdentity) -> Result<Option<u64>, OperationsLogError> {
        Ok(self.checkpoint_of(&source.as_key()))
    }

    fn record_gap(
        &self,
        source: &LogSourceIdentity,
        reason_code: &str,
        _dropped: u32,
    ) -> Result<(), OperationsLogError> {
        self.push(Event::Gap {
            source: source.as_key(),
            reason: reason_code.to_string(),
        });
        Ok(())
    }

    fn forget_sources(&self, _retained: &[LogSourceIdentity]) -> Result<u32, OperationsLogError> {
        self.push(Event::Forgot);
        Ok(0)
    }

    /// The whole batch or none of it, including the checkpoint. A failing commit leaves the
    /// checkpoint exactly where it was, which is what the next pass resumes from.
    fn commit_batch(
        &self,
        source: &LogSourceIdentity,
        records: &[RedactedLogRecord],
        rejections: &LineRejections,
        next_offset: u64,
    ) -> Result<LogBatchCommit, OperationsLogError> {
        {
            let mut failing = self.failing_commits.lock().expect("failing");
            if *failing > 0 {
                *failing -= 1;
                return Err(OperationsLogError::IndexUnavailable(
                    "log_index_storage_failed",
                ));
            }
        }
        self.push(Event::Committed {
            source: source.as_key(),
            records: records.len(),
            rejections: rejections
                .iter()
                .map(|(reason, count)| ((*reason).to_string(), *count))
                .collect(),
            next_offset,
        });
        self.checkpoints
            .lock()
            .expect("checkpoints")
            .insert(source.as_key(), next_offset);
        Ok(LogBatchCommit {
            inserted: u32::try_from(records.len()).unwrap_or(u32::MAX),
            ..LogBatchCommit::default()
        })
    }

    fn load_repair_state(&self) -> Result<Option<SessionLogBackfillStatus>, OperationsLogError> {
        Ok(self.repair_state.lock().expect("state").clone())
    }

    fn save_repair_state(
        &self,
        status: &SessionLogBackfillStatus,
    ) -> Result<(), OperationsLogError> {
        *self.repair_state.lock().expect("state") = Some(status.clone());
        Ok(())
    }

    fn gap_watermark(&self) -> Result<i64, OperationsLogError> {
        Ok(*self.gap_watermark.lock().expect("watermark"))
    }

    fn clear_gaps_through(
        &self,
        _sources: &[LogSourceIdentity],
        through_id: i64,
    ) -> Result<u32, OperationsLogError> {
        self.push(Event::ClearedGaps { through_id });
        Ok(0)
    }

    fn conflict_count(&self, _sources: &[LogSourceIdentity]) -> Result<u32, OperationsLogError> {
        Ok(u32::try_from(self.conflicts.load(Ordering::SeqCst)).unwrap_or(u32::MAX))
    }

    fn prune_source_generation(
        &self,
        source: &LogSourceIdentity,
        limit: u32,
    ) -> Result<u32, OperationsLogError> {
        self.push(Event::Pruned {
            source: source.as_key(),
        });
        let mut prunable = self.prunable.lock().expect("prunable");
        let remaining = prunable.entry(source.as_key()).or_default();
        let removed = (*remaining).min(limit);
        *remaining -= removed;
        Ok(removed)
    }
}

/// A corpus described entirely by numbers: how many records each file holds, at what offsets.
#[derive(Clone)]
struct ScriptedSource {
    file_id: String,
    /// Records per read, in order. An empty entry is a batch that found nothing.
    batches: Vec<usize>,
    rejections: LineRejections,
    end_offset: u64,
}

#[derive(Default)]
struct ScriptedReader {
    generation: Mutex<String>,
    sources: Mutex<Vec<ScriptedSource>>,
    /// Set to fail the listing, which is how a temporary IO error is staged.
    list_error: Mutex<Option<&'static str>>,
    /// Cancels the running pass once this many reads have happened. A cancel staged before the
    /// pass begins would be cleared by the pass itself, and a cancel from another thread would be
    /// a race — so the reader, which the pass calls, is where a deterministic one belongs.
    cancel_after_reads: Mutex<Option<(usize, std::sync::Weak<SessionLogQueryService>)>>,
    reads: AtomicUsize,
    /// Read calls that fail before any succeed.
    failing_reads: Mutex<usize>,
}

impl ScriptedReader {
    fn with(sources: Vec<ScriptedSource>) -> Self {
        Self {
            generation: Mutex::new("generation-1".to_string()),
            sources: Mutex::new(sources),
            ..Self::default()
        }
    }
}

/// Every batch is 100 bytes per record, so an offset is a count and a count is an offset.
const BYTES_PER_RECORD: u64 = 100;

impl RedactedLogSourceReader for ScriptedReader {
    fn sources(&self) -> Result<Vec<LogSourceSnapshot>, OperationsLogError> {
        if let Some(code) = *self.list_error.lock().expect("list error") {
            return Err(OperationsLogError::RepairFailed(code));
        }
        let generation = self.generation.lock().expect("generation").clone();
        Ok(self
            .sources
            .lock()
            .expect("sources")
            .iter()
            .map(|source| LogSourceSnapshot {
                identity: LogSourceIdentity {
                    directory_generation: generation.clone(),
                    file_id: source.file_id.clone(),
                },
                end_offset: source.end_offset,
            })
            .collect())
    }

    fn read_batch(
        &self,
        source: &LogSourceIdentity,
        from_offset: u64,
        max_records: usize,
        _max_bytes: u64,
    ) -> Result<RedactedLogBatch, OperationsLogError> {
        let reads = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some((after, service)) = self.cancel_after_reads.lock().expect("cancel").as_ref() {
            if reads >= *after {
                if let Some(service) = service.upgrade() {
                    service.cancel_repair();
                }
            }
        }
        {
            let mut failing = self.failing_reads.lock().expect("failing");
            if *failing > 0 {
                *failing -= 1;
                return Err(OperationsLogError::RepairFailed("log_source_read_failed"));
            }
        }
        let sources = self.sources.lock().expect("sources");
        let Some(scripted) = sources
            .iter()
            .find(|scripted| scripted.file_id == source.file_id)
        else {
            return Ok(RedactedLogBatch {
                next_offset: from_offset,
                reached_end: true,
                ..RedactedLogBatch::default()
            });
        };
        let index = (from_offset / (BYTES_PER_RECORD * 10)) as usize;
        let count = scripted.batches.get(index).copied().unwrap_or(0);
        let taken = count.min(max_records);
        let records = (0..taken)
            .map(|entry| RedactedLogRecord {
                record_id: format!("{}-{}-{}", scripted.file_id, index, entry),
                source: source.clone(),
                source_offset: from_offset + entry as u64 * BYTES_PER_RECORD,
                occurred_at: "2026-08-25T10:00:00Z".to_string(),
                occurred_at_ms: 1_787_911_200_000,
                level: super::log_index::IndexedLogLevel::Info,
                category: "test".to_string(),
                message: "hello".to_string(),
                context: BTreeMap::new(),
                correlation: super::log_index::LogCorrelation::default(),
            })
            .collect();
        // One "page" of the scripted file is ten records wide, so a batch always lands on a page
        // boundary and an offset maps back to an index.
        let next_offset = from_offset + BYTES_PER_RECORD * 10;
        let reached_end = index + 1 >= scripted.batches.len();
        Ok(RedactedLogBatch {
            records,
            next_offset,
            rejections: if index == 0 {
                scripted.rejections.clone()
            } else {
                LineRejections::new()
            },
            reached_end,
        })
    }

    fn export_sources(&self) -> Result<Vec<String>, OperationsLogError> {
        Ok(Vec::new())
    }
}

struct FixedClock;

impl LogIndexClock for FixedClock {
    fn now(&self) -> String {
        "2026-08-25T10:00:00Z".to_string()
    }
}

#[derive(Default)]
struct CountingIds(AtomicUsize);

impl LogIndexIdGenerator for CountingIds {
    fn next_operation_id(&self) -> String {
        format!("log-repair-{}", self.0.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Default)]
struct RecordingDiagnostics(Mutex<Vec<(String, BTreeMap<String, String>)>>);

impl LogIndexDiagnostics for RecordingDiagnostics {
    fn report(&self, reason_code: &str, context: BTreeMap<String, String>) {
        self.0
            .lock()
            .expect("diagnostics")
            .push((reason_code.to_string(), context));
    }
}

#[derive(Default)]
struct RecordingBackfill(Mutex<Vec<SessionLogBackfillStatus>>);

impl BackfillOperationPublisher for RecordingBackfill {
    fn publish(&self, status: SessionLogBackfillStatus) {
        self.0.lock().expect("backfill").push(status);
    }
}

struct Harness {
    /// Held behind an `Arc` so the reader can call back into the pass that is calling it, which is
    /// the only place a mid-pass cancel can be staged without depending on a scheduler.
    service: Arc<SessionLogQueryService>,
    index: Arc<RecordingIndex>,
    reader: Arc<ScriptedReader>,
    diagnostics: Arc<RecordingDiagnostics>,
    published: Arc<RecordingBackfill>,
}

impl Harness {
    fn states(&self) -> Vec<SessionLogBackfillState> {
        self.published
            .0
            .lock()
            .expect("published")
            .iter()
            .map(|status| status.state)
            .collect()
    }

    fn diagnostic_codes(&self) -> Vec<String> {
        self.diagnostics
            .0
            .lock()
            .expect("diagnostics")
            .iter()
            .map(|(code, _)| code.clone())
            .collect()
    }
}

fn source(file_id: &str, batches: Vec<usize>) -> ScriptedSource {
    let end_offset = batches.len() as u64 * BYTES_PER_RECORD * 10;
    ScriptedSource {
        file_id: file_id.to_string(),
        batches,
        rejections: LineRejections::new(),
        end_offset,
    }
}

fn harness(sources: Vec<ScriptedSource>) -> Harness {
    let index = Arc::new(RecordingIndex::default());
    let reader = Arc::new(ScriptedReader::with(sources));
    let diagnostics = Arc::new(RecordingDiagnostics::default());
    let published = Arc::new(RecordingBackfill::default());
    let service = Arc::new(SessionLogQueryService::new(
        index.clone(),
        reader.clone(),
        Arc::new(FixedClock),
        Arc::new(CountingIds::default()),
        diagnostics.clone(),
        published.clone(),
    ));
    Harness {
        service,
        index,
        reader,
        diagnostics,
        published,
    }
}

/// A harness whose reader cancels the pass after a given number of reads.
///
/// Deterministic by construction: the cancel happens on the pass's own thread, between two batches,
/// at a point the test names. A cancel raised from another thread would be a race, and one raised
/// before `repair()` is a different case entirely — the pass clears the flag when it starts.
fn cancelling_harness(sources: Vec<ScriptedSource>, after_reads: usize) -> Harness {
    let harness = harness(sources);
    *harness
        .reader
        .cancel_after_reads
        .lock()
        .expect("cancel after reads") = Some((after_reads, Arc::downgrade(&harness.service)));
    harness
}

// ---------------------------------------------------------------------------------------------
// State machine, single-flight, progress
// ---------------------------------------------------------------------------------------------

/// The pass moves through every state it advertises, in order.
///
/// The three working states are separated because they can make different claims: discovering knows
/// nothing yet, indexing has read something, and reconciling is the only one allowed to delete. A
/// pass that jumped straight to a state it had not earned would be reporting a claim it cannot back.
#[test]
fn a_pass_announces_queued_discovering_indexing_reconciling_and_completed_in_order() {
    let harness = harness(vec![source("file-1", vec![3])]);

    let terminal = harness.service.repair();

    assert_eq!(terminal.state, SessionLogBackfillState::Completed);
    assert!(
        !terminal.operation_id.is_empty(),
        "the operation is unnamed"
    );
    let states = harness.states();
    assert_eq!(states.first(), Some(&SessionLogBackfillState::Queued));
    assert!(states.contains(&SessionLogBackfillState::Discovering));
    assert!(states.contains(&SessionLogBackfillState::Indexing));
    assert!(states.contains(&SessionLogBackfillState::Reconciling));
    assert_eq!(states.last(), Some(&SessionLogBackfillState::Completed));
    // Queued is announced before anything is read, so a caller is never told "nothing is
    // happening" about a request that was already accepted.
    let queued_at = states
        .iter()
        .position(|state| *state == SessionLogBackfillState::Queued)
        .expect("queued");
    let indexing_at = states
        .iter()
        .position(|state| *state == SessionLogBackfillState::Indexing)
        .expect("indexing");
    assert!(queued_at < indexing_at);
}

/// A second request joins the running pass rather than starting a competing one.
///
/// Two passes over one corpus race to the same checkpoints, and each undoes the other's progress:
/// one commits offset 400 while the other is mid-batch from 200, and the loser resumes from a
/// position it never read to.
#[test]
fn a_second_request_for_the_same_generation_joins_the_running_pass() {
    let harness = harness(vec![source("file-1", vec![3])]);
    let first = harness.service.repair();

    // The claim is released when a pass finishes, so a repair after one completes is a new pass —
    // which is why this asserts against a claim staged as still active.
    harness
        .index
        .save_repair_state(&SessionLogBackfillStatus {
            state: SessionLogBackfillState::Indexing,
            ..first.clone()
        })
        .expect("stage an interrupted pass");

    let running = SessionLogBackfillStatus {
        state: SessionLogBackfillState::Indexing,
        ..first.clone()
    };
    harness
        .service
        .stage_active_repair("generation-1", &running);

    let second = harness.service.repair();

    assert_eq!(second.operation_id, first.operation_id, "a second pass ran");
    assert_eq!(second.state, SessionLogBackfillState::Indexing);
}

/// A claim held against a corpus that no longer exists is taken over rather than waited on.
///
/// The directory moved while a pass was running. That pass is reading files nobody will query, and
/// treating its claim as live would block every future repair on work that cannot matter.
#[test]
fn a_claim_for_a_vanished_generation_does_not_block_the_current_one() {
    let harness = harness(vec![source("file-1", vec![2])]);
    harness.service.stage_active_repair(
        "generation-that-is-gone",
        &SessionLogBackfillStatus {
            operation_id: "stranded".to_string(),
            state: SessionLogBackfillState::Indexing,
            files_completed: 0,
            files_total: 1,
            records_indexed: 0,
            started_at: None,
            updated_at: None,
            reason_code: None,
        },
    );

    let terminal = harness.service.repair();

    assert_ne!(terminal.operation_id, "stranded");
    assert_eq!(terminal.state, SessionLogBackfillState::Completed);
}

/// While a pass runs, a page that would otherwise read `complete` reads `indexing`.
///
/// The rows are real and the set is not final. `complete` is the one answer that lets a reader
/// conclude something from an absence, so it is the one answer a running repair may not give.
#[test]
fn a_query_during_a_repair_reports_indexing_rather_than_complete() {
    let harness = harness(vec![source("file-1", vec![1])]);
    harness.service.stage_active_repair(
        "generation-1",
        &SessionLogBackfillStatus {
            operation_id: "running".to_string(),
            state: SessionLogBackfillState::Indexing,
            files_completed: 0,
            files_total: 1,
            records_indexed: 0,
            started_at: None,
            updated_at: None,
            reason_code: None,
        },
    );

    let coverage = harness.service.coverage(None).expect("coverage");

    assert_eq!(coverage.state(), SessionLogCoverageState::Indexing);
}

// ---------------------------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------------------------

/// One pass touches at most the file bound, however many files exist.
///
/// The rest are not lost: the next pass starts where this one stopped, because a checkpoint is per
/// file and a file nobody reached simply has none yet.
#[test]
fn one_pass_touches_no_more_files_than_its_bound() {
    let files: Vec<ScriptedSource> = (0..(super::log_query_service::REPAIR_FILES_PER_PASS + 20))
        .map(|index| source(&format!("file-{index}"), vec![1]))
        .collect();
    let total = files.len();
    let harness = harness(files);

    let terminal = harness.service.repair();

    assert!(total > super::log_query_service::REPAIR_FILES_PER_PASS);
    assert_eq!(
        terminal.files_total as usize,
        super::log_query_service::REPAIR_FILES_PER_PASS
    );
    let committed = harness
        .index
        .events()
        .into_iter()
        .filter(|event| matches!(event, Event::Committed { .. }))
        .count();
    assert!(committed <= super::log_query_service::REPAIR_FILES_PER_PASS);
}

/// One file gets a bounded number of batches before the pass moves on.
///
/// Without it a single enormous file is indexed to its end before any other file is looked at once,
/// and a reader querying the newest logs waits on the oldest.
#[test]
fn one_file_gets_no_more_batches_than_its_bound() {
    let endless = super::log_query_service::REPAIR_BATCHES_PER_FILE + 50;
    let harness = harness(vec![
        source("file-1", vec![1; endless]),
        source("file-2", vec![1]),
    ]);

    harness.service.repair();

    let first_file_commits = harness
        .index
        .events()
        .into_iter()
        .filter(
            |event| matches!(event, Event::Committed { source, .. } if source.ends_with("file-1")),
        )
        .count();
    assert_eq!(
        first_file_commits,
        super::log_query_service::REPAIR_BATCHES_PER_FILE
    );
    // And the second file was still reached, which is the point of the bound.
    assert!(harness.index.events().into_iter().any(
        |event| matches!(event, Event::Committed { source, .. } if source.ends_with("file-2"))
    ));
}

/// A batch never asks for more records than the bound.
#[test]
fn a_batch_never_requests_more_records_than_the_bound() {
    let harness = harness(vec![source("file-1", vec![REPAIR_BATCH_RECORDS * 4])]);

    harness.service.repair();

    for event in harness.index.events() {
        if let Event::Committed { records, .. } = event {
            assert!(
                records <= REPAIR_BATCH_RECORDS,
                "{records} records in one batch exceeds the bound"
            );
        }
    }
}

/// Pruning a superseded generation is many bounded transactions, never one long one.
#[test]
fn pruning_a_superseded_generation_is_bounded_per_transaction() {
    let harness = harness(vec![source("file-1", vec![1])]);
    // The file is now shorter than the checkpoint says we already read: those bytes are gone.
    harness
        .index
        .checkpoints
        .lock()
        .expect("checkpoints")
        .insert("generation-1::file-1".to_string(), 100_000);
    harness
        .index
        .prunable
        .lock()
        .expect("prunable")
        .insert("generation-1::file-1".to_string(), 1_200);

    harness.service.repair();

    let prunes = harness
        .index
        .events()
        .into_iter()
        .filter(|event| matches!(event, Event::Pruned { .. }))
        .count();
    // 1200 rows at 500 per transaction is three calls that remove rows plus one that finds none.
    assert!(prunes >= 3, "the prune ran in {prunes} transactions");
}

// ---------------------------------------------------------------------------------------------
// Checkpoint atomicity
// ---------------------------------------------------------------------------------------------

/// A commit that failed moves nothing, including the checkpoint.
///
/// This is the asymmetry the whole design rests on. Rows without a checkpoint cost a re-read; a
/// checkpoint without its rows loses records permanently, because the offset says they were read.
#[test]
fn a_failed_commit_leaves_the_checkpoint_where_it_was() {
    let harness = harness(vec![source("file-1", vec![3, 3])]);
    *harness.index.failing_commits.lock().expect("failing") = 1;

    harness.service.repair();

    assert_eq!(
        harness.index.checkpoint_of("generation-1::file-1"),
        None,
        "the checkpoint advanced past records that were never committed"
    );
    assert!(harness
        .diagnostic_codes()
        .contains(&"log_index_unavailable".to_string()));
}

/// After a failed commit the next pass reads the same bytes again, and indexing them is a no-op the
/// second time. That is what makes an interrupted repair safe to simply run again.
#[test]
fn a_pass_after_a_failed_commit_resumes_from_the_last_committed_checkpoint() {
    let harness = harness(vec![source("file-1", vec![3, 4])]);
    *harness.index.failing_commits.lock().expect("failing") = 1;
    harness.service.repair();
    let after_failure = harness.index.checkpoint_of("generation-1::file-1");

    let second = harness.service.repair();

    assert_eq!(after_failure, None);
    assert_eq!(second.state, SessionLogBackfillState::Completed);
    let offsets: Vec<u64> = harness
        .index
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Committed { next_offset, .. } => Some(next_offset),
            _ => None,
        })
        .collect();
    // The second pass started at zero — the first batch's records again — rather than at the
    // offset the failed batch would have reached.
    assert_eq!(offsets.first().copied(), Some(1_000));
}

/// Rows, rejections and the checkpoint arrive in one call, so a store cannot commit one without
/// the others. A pass that wrote them separately would have to decide which order is safe, and
/// there is no safe order.
#[test]
fn rows_rejections_and_the_checkpoint_travel_in_one_commit() {
    let mut rejections = LineRejections::new();
    rejections.insert("log_record_rejected", 2);
    let harness = harness(vec![ScriptedSource {
        rejections,
        ..source("file-1", vec![3])
    }]);

    harness.service.repair();

    let committed = harness
        .index
        .events()
        .into_iter()
        .find_map(|event| match event {
            Event::Committed {
                records,
                rejections,
                next_offset,
                ..
            } => Some((records, rejections, next_offset)),
            _ => None,
        })
        .expect("a batch was committed");
    assert_eq!(committed.0, 3);
    assert_eq!(
        committed.1,
        vec![("log_record_rejected".to_string(), 2)],
        "the rejections were not part of the same commit"
    );
    assert!(committed.2 > 0, "the checkpoint was not part of the commit");
}

// ---------------------------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------------------------

/// A cancelled pass keeps every checkpoint it committed.
///
/// A cancel that rolled back would make cancelling more expensive than finishing, which is the
/// opposite of what a cancel is for.
#[test]
fn cancelling_mid_pass_keeps_committed_checkpoints_and_reports_cancelled() {
    let harness = cancelling_harness(vec![source("file-1", vec![2, 2, 2, 2])], 2);

    let terminal = harness.service.repair();

    assert_eq!(terminal.state, SessionLogBackfillState::Cancelled);
    // Cancellation is observed at a batch boundary, so the pass stops between transactions and
    // never inside one. What committed before the cancel stays committed: rolling it back would
    // make cancelling more expensive than finishing.
    let committed: Vec<u64> = harness
        .index
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Committed { next_offset, .. } => Some(next_offset),
            _ => None,
        })
        .collect();
    assert!(!committed.is_empty(), "nothing committed before the cancel");
    assert_eq!(
        harness.index.checkpoint_of("generation-1::file-1"),
        committed.last().copied(),
        "the checkpoint does not match the last committed batch"
    );
    assert!(
        harness
            .index
            .events()
            .into_iter()
            .all(|event| !matches!(event, Event::ClearedGaps { .. })),
        "a cancelled pass cleared gaps it had not proven"
    );
}

/// A cancel that arrived before any pass began does not kill the next one.
///
/// The flag names no operation, so treating it as durable would mean one stray cancel silently
/// disabling repair for the rest of the process — and nothing would ever say why the index stopped
/// catching up.
#[test]
fn a_cancel_from_before_a_pass_started_does_not_cancel_it() {
    let harness = harness(vec![source("file-1", vec![2])]);
    harness.service.cancel_repair();

    let terminal = harness.service.repair();

    assert_eq!(terminal.state, SessionLogBackfillState::Completed);
}

// ---------------------------------------------------------------------------------------------
// Gap snapshot and coverage recovery
// ---------------------------------------------------------------------------------------------

/// A clean pass clears only the gaps that existed when it started.
///
/// A drop recorded while the pass ran describes records it never read. Clearing that one would put
/// coverage back to `complete` on the strength of work that did not cover it.
#[test]
fn a_clean_pass_clears_gaps_only_up_to_the_snapshot_it_took() {
    let harness = harness(vec![source("file-1", vec![2])]);
    *harness.index.gap_watermark.lock().expect("watermark") = 7;

    harness.service.repair();

    let cleared = harness
        .index
        .events()
        .into_iter()
        .find_map(|event| match event {
            Event::ClearedGaps { through_id } => Some(through_id),
            _ => None,
        });
    assert_eq!(
        cleared,
        Some(7),
        "the clear was not bounded by the snapshot"
    );
}

/// An unresolved conflict blocks the clearing entirely.
///
/// A conflict means two different records claimed one id and the index kept the first. Until that
/// is understood the corpus is not provably whole, whatever the offsets say.
#[test]
fn an_unresolved_conflict_stops_a_pass_from_clearing_any_gap() {
    let harness = harness(vec![source("file-1", vec![2])]);
    harness.index.conflicts.store(1, Ordering::SeqCst);

    harness.service.repair();

    assert!(
        harness
            .index
            .events()
            .into_iter()
            .all(|event| !matches!(event, Event::ClearedGaps { .. })),
        "gaps were cleared while a conflict was unresolved"
    );
}

/// A pass that did not reach every source's captured target clears nothing.
#[test]
fn a_pass_that_stopped_short_of_its_targets_clears_nothing() {
    // The file claims to be far longer than the scripted batches can reach, so the pass runs out of
    // content before it reaches the captured end offset.
    let harness = harness(vec![ScriptedSource {
        end_offset: 10_000_000,
        ..source("file-1", vec![1])
    }]);

    harness.service.repair();

    assert!(
        harness
            .index
            .events()
            .into_iter()
            .all(|event| !matches!(event, Event::ClearedGaps { .. })),
        "a pass cleared gaps for a corpus it had not finished reading"
    );
}

/// A directory that cannot be listed fails the pass and deletes nothing.
///
/// This is the difference between "the corpus is empty" and "I could not see the corpus". Reading
/// the second as the first would delete every indexed row on a disk hiccup.
#[test]
fn a_listing_failure_fails_the_pass_and_removes_nothing() {
    let harness = harness(vec![source("file-1", vec![2])]);
    *harness.reader.list_error.lock().expect("list error") = Some("log_directory_unreadable");

    let terminal = harness.service.repair();

    assert_eq!(terminal.state, SessionLogBackfillState::Failed);
    assert_eq!(
        terminal.reason_code.as_deref(),
        Some("log_repair_failed"),
        "the failure is reported as a stable code"
    );
    assert!(
        harness.index.events().is_empty(),
        "a failed listing still touched the store: {:?}",
        harness.index.events()
    );
}

/// A read that fails costs its file, not the pass. Everything else still gets indexed.
#[test]
fn a_read_failure_on_one_file_does_not_stop_the_others() {
    let harness = harness(vec![source("file-1", vec![2]), source("file-2", vec![2])]);
    *harness.reader.failing_reads.lock().expect("failing") = 1;

    let terminal = harness.service.repair();

    assert_eq!(terminal.state, SessionLogBackfillState::Completed);
    assert!(harness.index.events().into_iter().any(
        |event| matches!(event, Event::Committed { source, .. } if source.ends_with("file-2"))
    ));
    assert!(harness
        .diagnostic_codes()
        .contains(&"log_repair_failed".to_string()));
}

// ---------------------------------------------------------------------------------------------
// Restart
// ---------------------------------------------------------------------------------------------

/// After a restart the status comes from what was persisted, not from an empty in-memory slot.
///
/// Otherwise a process that died mid-pass comes back reporting `idle`, which reads as "no repair
/// has ever been needed" — and the index is behind.
#[test]
fn a_restarted_service_reports_the_interrupted_pass_rather_than_idle() {
    let harness = harness(vec![source("file-1", vec![1])]);
    harness
        .index
        .save_repair_state(&SessionLogBackfillStatus {
            operation_id: "interrupted".to_string(),
            state: SessionLogBackfillState::Indexing,
            files_completed: 2,
            files_total: 9,
            records_indexed: 41,
            started_at: Some("2026-08-25T09:00:00Z".to_string()),
            updated_at: Some("2026-08-25T09:05:00Z".to_string()),
            reason_code: None,
        })
        .expect("persist");

    // A fresh service over the same store is what a restart is.
    let restarted = SessionLogQueryService::new(
        harness.index.clone(),
        harness.reader.clone(),
        Arc::new(FixedClock),
        Arc::new(CountingIds::default()),
        harness.diagnostics.clone(),
        harness.published.clone(),
    );
    let status = restarted.backfill_status();

    assert_eq!(status.operation_id, "interrupted");
    assert_eq!(status.state, SessionLogBackfillState::Indexing);
    assert_eq!(status.records_indexed, 41);
}

/// A resumed pass starts from the committed checkpoint, not from the beginning.
#[test]
fn a_resumed_pass_starts_from_the_committed_checkpoint() {
    let harness = harness(vec![source("file-1", vec![2, 2, 2])]);
    harness
        .index
        .checkpoints
        .lock()
        .expect("checkpoints")
        .insert("generation-1::file-1".to_string(), 2_000);

    harness.service.repair();

    let first_commit = harness
        .index
        .events()
        .into_iter()
        .find_map(|event| match event {
            Event::Committed { next_offset, .. } => Some(next_offset),
            _ => None,
        })
        .expect("a batch was committed");
    // Resumed at the third page rather than the first: the two pages before it are already indexed.
    assert!(
        first_commit > 2_000,
        "the resumed pass re-read from {first_commit}"
    );
}
