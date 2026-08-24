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

/// One repair batch's worth of source reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactedLogBatch {
    pub(crate) records: Vec<RedactedLogRecord>,
    /// Where the next batch starts. Advanced past complete lines only: a partial trailing line is
    /// a line the writer has not finished, and consuming it would index half a record and then
    /// never see the other half.
    pub(crate) next_offset: u64,
    /// Complete lines that could not satisfy the safe schema. Counted rather than quoted.
    pub(crate) rejected: u32,
    pub(crate) reached_end: bool,
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

    fn save_checkpoint(
        &self,
        source: &LogSourceIdentity,
        offset: u64,
    ) -> Result<(), OperationsLogError>;

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
}

/// Where already-redacted source records are read from.
///
/// Reading and parsing happen here, outside any database transaction. A transaction held across
/// file IO would hold the write lock for as long as the disk took, which is what turns a repair
/// into an application-wide stall.
pub(crate) trait RedactedLogSourceReader: Send + Sync {
    /// Every retained source, oldest first, under the current directory generation.
    fn sources(&self) -> Result<Vec<LogSourceIdentity>, OperationsLogError>;

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
