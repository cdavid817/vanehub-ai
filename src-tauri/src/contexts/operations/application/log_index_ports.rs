//! What the log-index application needs from the outside, and nothing more.
//!
//! Every port here is named for a job rather than for a technology. The application never learns
//! that the index is SQLite, that the source is JSONL, or that a notice travels over Tauri — which
//! is what keeps a query's semantics testable without any of those.

use super::log_index::{
    IndexedLogLevel, IndexedSessionLogPage, IndexedSessionLogQuery, IndexedSessionLogRecord,
    LogCorrelation, OperationsLogError, SessionLogBackfillStatus, SessionLogCoverage,
    SessionLogNotice,
};
use std::collections::BTreeMap;

/// A source file's identity, as the index tracks it.
///
/// A path is deliberately not enough. Rotation renames a file whose records are the same records;
/// truncation reuses a path for unrelated bytes; a directory change replaces the corpus. A
/// checkpoint resumed against the wrong one of those reads from an offset that means nothing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LogSourceIdentity {
    /// Which configured directory this file was found under. Changing the directory starts a new
    /// generation, so old checkpoints cannot attach to new sources.
    pub(crate) directory_generation: String,
    /// Stable per file across a rename, and different after a truncate-and-recreate.
    pub(crate) file_id: String,
}

impl LogSourceIdentity {
    pub(crate) fn as_key(&self) -> String {
        format!("{}::{}", self.directory_generation, self.file_id)
    }
}

/// One already-redacted record, as the index receives it.
///
/// `record_id` is present for records written since ids existed and absent for older lines, which
/// the reader derives deterministically from identity, offset, and a fingerprint of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactedLogRecord {
    pub(crate) record_id: String,
    pub(crate) source: LogSourceIdentity,
    /// Byte offset of this line's first byte. Half of the witness that makes a retry idempotent.
    pub(crate) source_offset: u64,
    pub(crate) occurred_at: String,
    pub(crate) occurred_at_ms: i64,
    pub(crate) level: IndexedLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
    pub(crate) correlation: LogCorrelation,
}

/// One source file as a repair pass found it.
///
/// The end offset is captured at discovery and used twice. It is the *target* the pass is trying to
/// reach, so "caught up" means a definite thing rather than "the file stopped growing while I was
/// looking"; and it is the *witness* for truncation, because a file now shorter than the offset we
/// already read past cannot be the same content we read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogSourceSnapshot {
    pub(crate) identity: LogSourceIdentity,
    pub(crate) end_offset: u64,
}

/// Why a complete line was skipped, and how many.
///
/// Per reason rather than one total, because the three reasons need different responses: a
/// malformed line is a producer bug, an oversized one is a bound doing its job, and invalid UTF-8
/// means the file itself is damaged. A single count would present them as one condition.
pub(crate) type LineRejections = BTreeMap<&'static str, u32>;

/// One repair batch's worth of source reading.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RedactedLogBatch {
    pub(crate) records: Vec<RedactedLogRecord>,
    /// Where the next batch starts. Advanced past complete lines only: a partial trailing line is
    /// a line the writer has not finished, and consuming it would index half a record and then
    /// never see the other half.
    pub(crate) next_offset: u64,
    /// Complete lines that could not be indexed, by stable reason code. Counted, never quoted:
    /// the bytes that failed are source content, and a diagnostic that echoed them would put
    /// unredactable text somewhere redaction never ran.
    pub(crate) rejections: LineRejections,
    pub(crate) reached_end: bool,
}

impl RedactedLogBatch {
    #[cfg(test)]
    pub(crate) fn rejected_total(&self) -> u32 {
        self.rejections.values().copied().sum()
    }
}

/// What one committed batch did.
///
/// Rows, gaps and the checkpoint move together or not at all, so there is one outcome for the whole
/// batch rather than one per record. A caller that had to reconcile per-record outcomes against a
/// single checkpoint would be reimplementing the transaction it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LogBatchCommit {
    pub(crate) inserted: u32,
    pub(crate) already_indexed: u32,
    pub(crate) conflicted: u32,
}

/// What an insert did. A retry has to be able to tell "already there" from "written now" without
/// the caller inspecting the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogIndexInsertOutcome {
    Inserted {
        sequence: i64,
    },
    /// The same record id with the same witness. Success, and no second notice.
    AlreadyIndexed,
    /// The same record id with a different witness. The original row is kept and coverage drops.
    Conflicted,
}

/// Where indexed rows live.
///
/// Every method is expected to be cheap enough for an interactive query, which is the reason the
/// index exists: the alternative is scanning files, and a scan cannot be bounded by a page.
pub(crate) trait SessionLogIndexRepository: Send + Sync {
    fn insert(
        &self,
        record: &RedactedLogRecord,
    ) -> Result<LogIndexInsertOutcome, OperationsLogError>;

    fn query(
        &self,
        query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError>;

    fn record(&self, record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError>;

    fn coverage(&self, session_id: Option<&str>) -> Result<SessionLogCoverage, OperationsLogError>;

    /// The newest indexed sequence, for a subscriber deciding where to resume.
    fn watermark(&self) -> Result<i64, OperationsLogError>;

    fn error_count(&self, session_id: &str) -> Result<u32, OperationsLogError>;

    fn checkpoint(&self, source: &LogSourceIdentity) -> Result<Option<u64>, OperationsLogError>;

    /// Records that a range could not be indexed, so coverage can say so instead of a count
    /// quietly being short.
    fn record_gap(
        &self,
        source: &LogSourceIdentity,
        reason_code: &str,
        dropped: u32,
    ) -> Result<(), OperationsLogError>;

    /// Drops rows whose source is gone, and moves the oldest queryable boundary with them.
    fn forget_sources(&self, retained: &[LogSourceIdentity]) -> Result<u32, OperationsLogError>;

    /// Writes one batch's rows, its gaps and its checkpoint in a single transaction.
    ///
    /// The three have to move together. A checkpoint committed ahead of its rows would make the
    /// next pass resume past records that were never indexed, and nothing afterwards could tell:
    /// the offset says they were read, and read is the only claim a checkpoint makes. Rows
    /// committed without their checkpoint are merely re-derived on the next pass, which is why the
    /// asymmetry is safe in exactly one direction and this method exists to enforce it.
    fn commit_batch(
        &self,
        source: &LogSourceIdentity,
        records: &[RedactedLogRecord],
        rejections: &LineRejections,
        next_offset: u64,
    ) -> Result<LogBatchCommit, OperationsLogError>;

    /// The persisted repair, so progress survives a restart and a resumed pass can say so.
    fn load_repair_state(&self) -> Result<Option<SessionLogBackfillStatus>, OperationsLogError>;

    fn save_repair_state(
        &self,
        status: &SessionLogBackfillStatus,
    ) -> Result<(), OperationsLogError>;

    /// The newest gap id at the moment a repair started.
    ///
    /// A repair may only clear what it can prove it fixed, and it cannot prove anything about a
    /// gap that appeared while it ran — that one describes a loss it never read. So the snapshot
    /// is taken first and clearing is bounded by it.
    fn gap_watermark(&self) -> Result<i64, OperationsLogError>;

    /// Clears gaps for the given sources up to and including `through_id`.
    ///
    /// Bounded by the snapshot rather than "all gaps for this source", because a drop recorded
    /// during the pass is a hole this pass did not fill.
    fn clear_gaps_through(
        &self,
        sources: &[LogSourceIdentity],
        through_id: i64,
    ) -> Result<u32, OperationsLogError>;

    /// How many unresolved identity conflicts exist for these sources.
    ///
    /// A conflict means two different records claimed one id, and the index kept the first. Until
    /// that is understood the corpus is not provably whole, so it blocks the clearing above.
    fn conflict_count(&self, sources: &[LogSourceIdentity]) -> Result<u32, OperationsLogError>;

    /// Deletes rows and the checkpoint for one superseded source generation, in bounded batches.
    ///
    /// Returns how many rows went. The caller loops until it returns zero, so a corpus larger than
    /// one transaction can hold is pruned without one transaction spanning all of it.
    fn prune_source_generation(
        &self,
        source: &LogSourceIdentity,
        limit: u32,
    ) -> Result<u32, OperationsLogError>;
}

/// Where already-redacted source records are read from.
///
/// Reading and parsing happen here, outside any database transaction. A transaction held across
/// file IO would hold the write lock for as long as the disk took, which is what turns a repair
/// into an application-wide stall.
pub(crate) trait RedactedLogSourceReader: Send + Sync {
    /// Every retained source, oldest first, under the current directory generation.
    ///
    /// This is the authoritative inventory, and the distinction matters: an error here means the
    /// listing failed, never that the corpus is empty. A repair that read a temporary IO failure as
    /// "no sources retained" would delete every indexed row on a disk hiccup.
    fn sources(&self) -> Result<Vec<LogSourceSnapshot>, OperationsLogError>;

    fn read_batch(
        &self,
        source: &LogSourceIdentity,
        from_offset: u64,
        max_records: usize,
        max_bytes: u64,
    ) -> Result<RedactedLogBatch, OperationsLogError>;

    /// The files an export may read. Never an index handle.
    fn export_sources(&self) -> Result<Vec<String>, OperationsLogError>;
}

pub(crate) trait LogIndexClock: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait LogIndexIdGenerator: Send + Sync {
    fn next_operation_id(&self) -> String;
}

/// Where a notice goes after its transaction commits.
///
/// Published after the commit and never before: a subscriber that heard about a row and then could
/// not find it would read that as a gap, which is a lie about lost data.
pub(crate) trait PostCommitLogNoticePublisher: Send + Sync {
    fn publish(&self, notice: SessionLogNotice);
}

/// Where the index reports its own trouble.
///
/// Separate from the unified logging port on purpose: the index is what a log write feeds, and
/// diagnosing a failed index write by logging through it is how one failure becomes a loop.
pub(crate) trait LogIndexDiagnostics: Send + Sync {
    /// Codes and identifiers only. The text that failed to parse is source content.
    fn report(&self, reason_code: &str, context: BTreeMap<String, String>);
}

/// Where repair progress is published so a client can watch it.
pub(crate) trait BackfillOperationPublisher: Send + Sync {
    fn publish(&self, status: SessionLogBackfillStatus);
}
