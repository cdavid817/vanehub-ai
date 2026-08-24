//! What an indexed session-log query looks like, and what it is allowed to claim.
//!
//! Two stores hold log data and only one of them is authoritative. The redacted JSONL files are the
//! durable record; everything here describes a projection built from them that can be deleted and
//! rebuilt without losing anything. That asymmetry is why the coverage type below exists at all: a
//! projection can be behind, and a reader who cannot tell "nothing happened" from "I have not
//! indexed that yet" will believe the first one.

use std::collections::BTreeMap;

/// The default and maximum page sizes.
///
/// A page is what crosses the IPC boundary and lands in a list, so the ceiling is the product's,
/// not the caller's: a client asking for everything would be asking the renderer to hold the corpus.
pub(crate) const DEFAULT_LOG_PAGE_SIZE: usize = 100;
pub(crate) const MAX_LOG_PAGE_SIZE: usize = 500;

/// How many indexed rows a text search may examine before it stops.
///
/// Reaching this bound makes the answer partial, never "complete, no match": having looked at the
/// first N candidates and found nothing does not establish that nothing matches.
pub(crate) const MAX_LOG_SEARCH_CANDIDATES: usize = 5_000;

/// The severity vocabulary, mirroring the durable record's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum IndexedLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl IndexedLogLevel {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
}

/// The correlations a log record can carry.
///
/// Every field is optional because a record emitted outside a session, a run, or a trace has none
/// of them, and inventing one would attribute work to something that did not do it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LogCorrelation {
    pub(crate) session_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) seat_id: Option<String>,
}

/// One indexed record, as a reader sees it.
///
/// `message` and `context` are already redacted: they are copied from the durable line, which was
/// redacted before it was written. The index never sees an unredacted value, which is what makes
/// "the index leaked a secret" a contradiction rather than a risk to manage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedSessionLogRecord {
    pub(crate) record_id: String,
    /// Monotonic within the index. Paired with `occurred_at_ms` it makes a total order that two
    /// records written inside one millisecond cannot tie on.
    pub(crate) sequence: i64,
    pub(crate) occurred_at: String,
    pub(crate) level: IndexedLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
    pub(crate) correlation: LogCorrelation,
}

/// Which records a query is about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionLogQueryScope {
    pub(crate) session_id: Option<String>,
    pub(crate) seat_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
}

/// How a query narrows within its scope, and in what order it reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionLogFilters {
    /// Empty means every level. A caller that meant "none" would be asking for nothing, and
    /// answering that with everything is the friendlier of two wrong answers to refuse.
    pub(crate) levels: Vec<IndexedLogLevel>,
    /// Matched against the already-redacted message, category, and safe context only.
    pub(crate) search: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    /// Which end of the corpus a page starts from. Part of what a cursor is issued against, because
    /// it decides which rows are "after" the cursor's position.
    pub(crate) sort: super::log_cursor::LogSortDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedSessionLogQuery {
    pub(crate) scope: SessionLogQueryScope,
    pub(crate) filters: SessionLogFilters,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
}

/// What the index can honestly say about how much of the corpus it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionLogCoverageState {
    /// Every retained source record in scope is indexed and no known gap applies.
    Complete,
    /// A bounded repair is running. The rows returned are real; the set is not final.
    Indexing,
    /// Something is known to be missing — retention, a dropped notice, a rejected line, a
    /// truncated search.
    Partial,
    /// The index cannot answer safely at all.
    Unavailable,
}

impl SessionLogCoverageState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Indexing => "indexing",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Coverage travels with every page and every summary.
///
/// A count without one of these is a number a reader will take as definitive. `dropped_count` and
/// the boundaries exist so "partial" can say how partial rather than only that it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SessionLogCoverage {
    pub(crate) state: SessionLogCoverageStateHolder,
    pub(crate) oldest_available_at: Option<String>,
    pub(crate) newest_available_at: Option<String>,
    /// The newest record the index is caught up to. Behind `newest_available_at` while indexing.
    pub(crate) indexed_through: Option<String>,
    pub(crate) dropped_count: u32,
    /// Set when a bound stopped the answer short rather than the data running out.
    pub(crate) truncated: bool,
    /// Stable codes, never prose: a reader groups by them and free text does not group.
    pub(crate) reason_codes: Vec<String>,
}

/// A newtype only so `SessionLogCoverage` can derive `Default` with a meaningful default.
///
/// The default is `Unavailable`, not `Complete`. A coverage value that was never filled in must not
/// read as a confident answer — that is the exact failure this whole type exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionLogCoverageStateHolder(pub(crate) SessionLogCoverageState);

impl Default for SessionLogCoverageStateHolder {
    fn default() -> Self {
        Self(SessionLogCoverageState::Unavailable)
    }
}

impl SessionLogCoverage {
    pub(crate) fn state(&self) -> SessionLogCoverageState {
        self.state.0
    }

    pub(crate) fn with_state(state: SessionLogCoverageState) -> Self {
        Self {
            state: SessionLogCoverageStateHolder(state),
            ..Self::default()
        }
    }

    /// Records a bound having been hit. Degrades the state rather than overwriting it, because a
    /// truncated search inside an already-indexing corpus is still indexing.
    pub(crate) fn mark_truncated(mut self, reason: &str) -> Self {
        self.truncated = true;
        if self.state() == SessionLogCoverageState::Complete {
            self.state = SessionLogCoverageStateHolder(SessionLogCoverageState::Partial);
        }
        if !self.reason_codes.iter().any(|code| code == reason) {
            self.reason_codes.push(reason.to_string());
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedSessionLogPage {
    pub(crate) items: Vec<IndexedSessionLogRecord>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) truncated: bool,
    pub(crate) coverage: SessionLogCoverage,
}

/// One authoritative row, fetched by id.
///
/// This is how a live notice becomes a rendered row: the notice says which record, and this says
/// what it is. One shape rather than two that can disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedSessionLogDetail {
    pub(crate) record: IndexedSessionLogRecord,
    pub(crate) coverage: SessionLogCoverage,
}

/// The narrow figure the workspace summary badge needs.
///
/// Carries its coverage for the same reason every other count does: a zero next to `partial` means
/// "not observed", and a badge that dropped the coverage would render it as "none happened".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogSummary {
    pub(crate) session_id: String,
    pub(crate) new_errors: u32,
    pub(crate) coverage: SessionLogCoverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionLogBackfillState {
    Idle,
    Running,
    Cancelled,
    Failed,
    Completed,
}

impl SessionLogBackfillState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Completed => "completed",
        }
    }
}

/// What a repair operation is doing, as a reader sees it.
///
/// The operation id is backend-managed. A client-supplied one would let two clients name the same
/// repair differently, or one client name two repairs the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogBackfillStatus {
    pub(crate) operation_id: String,
    pub(crate) state: SessionLogBackfillState,
    pub(crate) files_completed: u32,
    pub(crate) files_total: u32,
    pub(crate) records_indexed: u64,
    pub(crate) started_at: Option<String>,
    pub(crate) updated_at: Option<String>,
    /// Stable code, never the message a parse failure produced: that message is source text.
    pub(crate) reason_code: Option<String>,
}

/// Where a subscriber resumes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogSubscriptionBootstrap {
    pub(crate) watermark_sequence: i64,
    pub(crate) coverage: SessionLogCoverage,
}

/// What a notice announces.
///
/// A subscriber does two different things with these, so they cannot share a shape and be told
/// apart by whether a field happens to be set. An `Appended` notice names a row that can be
/// fetched; a `Gap` names rows that never arrived and never will. Treating the second as the first
/// would send the subscriber looking for a record id that does not exist, and reporting a lookup
/// failure as an error rather than as the loss it actually is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SessionLogNoticeKind {
    #[default]
    Appended,
    Gap,
}

impl SessionLogNoticeKind {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Appended => "appended",
            Self::Gap => "gap",
        }
    }
}

/// One bounded notice per indexed record, or per hole where records should have been.
///
/// Identifiers, ordering, correlation, coverage. Not the line: a view that wants the row fetches it
/// by id, which keeps the event bus from carrying the corpus and keeps one authoritative shape for
/// a row instead of two.
///
/// A gap notice carries the same envelope deliberately. It has to travel in sequence with the rows
/// around it — a subscriber that learned about a hole out of order would apply it to the wrong part
/// of its view — and travelling in sequence means carrying a sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogNotice {
    pub(crate) kind: SessionLogNoticeKind,
    /// The row this announces. Empty for a gap: there is no row, and an id that looked fetchable
    /// would be fetched.
    pub(crate) record_id: String,
    pub(crate) sequence: i64,
    pub(crate) occurred_at: String,
    pub(crate) level: IndexedLogLevel,
    pub(crate) correlation: LogCorrelation,
    pub(crate) coverage_state: SessionLogCoverageState,
    /// Gap only: how many records were lost, and the stable code saying why. Never what they said —
    /// a gap notice is the one message published about records nobody was able to redact.
    pub(crate) dropped_count: u32,
    pub(crate) reason_code: Option<String>,
}

/// What an export is allowed to read.
///
/// A list of redacted source files, never an index handle. An export served from the projection
/// would hand the user whatever it happened to hold — a subset during repair, a stale set after a
/// directory change — under a name that promises the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SafeLogExportPreparation {
    pub(crate) source_files: Vec<String>,
    pub(crate) oldest_available_at: Option<String>,
    pub(crate) newest_available_at: Option<String>,
    /// Always true by construction. Present so a reader of the DTO can see the guarantee rather
    /// than having to know it.
    pub(crate) redacted: bool,
}

/// Every way an indexed log operation can fail, as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperationsLogError {
    /// The cursor did not decode, or its version is not one this build issues.
    InvalidCursor,
    /// The cursor decoded but was issued for different filters. Continuing would silently mix two
    /// result sets, and the boundary between them would look like ordinary pagination.
    CursorFilterMismatch,
    InvalidQuery(&'static str),
    RecordNotFound,
    /// The index cannot answer. Distinct from an empty result, which is an answer.
    IndexUnavailable(&'static str),
    /// A repair could not proceed. Carries a code, never a parse message.
    RepairFailed(&'static str),
}

impl OperationsLogError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidCursor => "log_invalid_cursor",
            Self::CursorFilterMismatch => "log_cursor_filter_mismatch",
            Self::InvalidQuery(_) => "log_invalid_query",
            Self::RecordNotFound => "log_record_not_found",
            Self::IndexUnavailable(_) => "log_index_unavailable",
            Self::RepairFailed(_) => "log_repair_failed",
        }
    }
}

impl std::fmt::Display for OperationsLogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_coverage_value_that_was_never_filled_in_is_not_a_confident_answer() {
        assert_eq!(
            SessionLogCoverage::default().state(),
            SessionLogCoverageState::Unavailable
        );
    }

    #[test]
    fn hitting_a_bound_degrades_complete_but_leaves_indexing_alone() {
        let truncated = SessionLogCoverage::with_state(SessionLogCoverageState::Complete)
            .mark_truncated("log_search_candidates_exhausted");
        assert_eq!(truncated.state(), SessionLogCoverageState::Partial);
        assert!(truncated.truncated);
        assert_eq!(truncated.reason_codes, ["log_search_candidates_exhausted"]);

        // Already-degraded coverage keeps its state: a truncated search inside a corpus that is
        // still indexing is still indexing, and reporting it as merely partial would say the
        // repair had finished.
        let indexing = SessionLogCoverage::with_state(SessionLogCoverageState::Indexing)
            .mark_truncated("log_search_candidates_exhausted");
        assert_eq!(indexing.state(), SessionLogCoverageState::Indexing);
    }

    #[test]
    fn levels_round_trip_through_their_stable_tokens() {
        for level in [
            IndexedLogLevel::Error,
            IndexedLogLevel::Warn,
            IndexedLogLevel::Info,
            IndexedLogLevel::Debug,
        ] {
            assert_eq!(IndexedLogLevel::parse(level.token()), Some(level));
        }
        assert_eq!(IndexedLogLevel::parse("trace"), None);
    }

    #[test]
    fn every_log_error_has_a_stable_code() {
        assert_eq!(
            OperationsLogError::InvalidCursor.code(),
            "log_invalid_cursor"
        );
        assert_eq!(
            OperationsLogError::CursorFilterMismatch.code(),
            "log_cursor_filter_mismatch"
        );
        assert_eq!(
            OperationsLogError::IndexUnavailable("closed").to_string(),
            "log_index_unavailable"
        );
    }
}
