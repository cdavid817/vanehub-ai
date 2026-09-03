//! What the evidence store can say about a whole session's worth of work.
//!
//! Separate from the page and summary models because these answer a different kind of question. A
//! page shows records; these count them. Putting the counting here rather than letting a consumer
//! page through records and tally them is the point of the whole file: a consumer that paged would
//! either read everything (unbounded) or read one page and report a total that is really a page
//! total, and nothing in the answer would say which.
//!
//! Every total that cannot be derived from complete data is absent rather than zero. A sum over
//! durations where some records have no duration is not a smaller sum — it is a different quantity,
//! and reporting it as the total would understate the session by however much was never observed.

use crate::contexts::execution_observability::domain::{EvidenceSeatId, EvidenceSessionId};

/// How many runs or seats one report query may name.
///
/// The list arrives from a caller, and it becomes an `IN` clause. SQLite's own parameter ceiling is
/// far higher, but a query naming a thousand runs is not a report anybody reads.
pub(crate) const MAX_EVIDENCE_REPORT_FILTERS: usize = 200;

/// How many tool rows one aggregate returns, newest-heaviest first.
///
/// A session can touch a long tail of tools used once each. The tail is not what a report is for,
/// and carrying it would make the answer grow with the session rather than stay a summary.
pub(crate) const MAX_EVIDENCE_TOOL_ROWS: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceReportQuery {
    pub(crate) session_id: EvidenceSessionId,
    /// Empty means every run in the session.
    pub(crate) run_ids: Vec<String>,
    pub(crate) seat_ids: Vec<EvidenceSeatId>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
}

impl EvidenceReportQuery {
    pub(crate) fn new(session_id: EvidenceSessionId) -> Self {
        Self {
            session_id,
            run_ids: Vec::new(),
            seat_ids: Vec::new(),
            from: None,
            to: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceToolAggregate {
    pub(crate) tool_name: String,
    pub(crate) invocations: u32,
    pub(crate) failures: u32,
    /// Absent unless every invocation in the group was observed start to end.
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceCommandAggregate {
    pub(crate) total: u32,
    pub(crate) failed: u32,
    pub(crate) running: u32,
    /// Absent unless every command that finished was observed start to end. A session with a
    /// command still running therefore has no total, which is correct: it is not over.
    pub(crate) duration_ms: Option<u64>,
}

/// Checks, except for `skipped`.
///
/// `passed` and `failed` sum the per-record check counts, because a verification record carries how
/// many assertions it ran. A skipped verification ran none by construction and reports no counts at
/// all, so the only unit available for it is the record — which is why this one figure counts
/// records while the other two count checks. Naming it `skipped_records` would be more precise and
/// less readable; the asymmetry is recorded here instead.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceVerificationAggregate {
    pub(crate) passed: u32,
    pub(crate) failed: u32,
    pub(crate) skipped: u32,
}

/// A failure count under a stable code.
///
/// Derived from the projection's own columns — never from a message, a command line, or a tool
/// name. A report is quoted, and producer text quoted out of one is unredacted content in a
/// document nobody reviewed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceFailureAggregate {
    pub(crate) reason_code: String,
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceReportAggregate {
    pub(crate) tools: Vec<EvidenceToolAggregate>,
    pub(crate) commands: EvidenceCommandAggregate,
    pub(crate) verification: EvidenceVerificationAggregate,
    pub(crate) failures: Vec<EvidenceFailureAggregate>,
    /// True when this session's coverage is anything but complete, or when the tool tail was cut.
    /// A consumer turns it into "partial" rather than deciding for itself what the shortfall means.
    pub(crate) incomplete: bool,
}

/// Percentiles over observed record durations.
///
/// Absent rather than zero when nothing finished. A p50 of zero would report a session where every
/// call returned instantly, which is a specific and false claim rather than a missing one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceLatencyAggregate {
    pub(crate) p50_ms: Option<u64>,
    pub(crate) p95_ms: Option<u64>,
    pub(crate) slowest_record_duration_ms: Option<u64>,
    pub(crate) incomplete: bool,
}

/// The stable codes a failure row can carry.
///
/// Named here rather than inlined in SQL so the set a consumer must translate is enumerable, and so
/// a new one cannot appear without appearing in this list.
pub(crate) mod failure_codes {
    /// The program returned non-zero: its own verdict on itself.
    pub(crate) const COMMAND_EXIT: &str = "command_failed_exit";
    /// The platform killed it. Distinct from an exit because a reader acts differently — a signal
    /// is usually a timeout, an out-of-memory kill, or a cancel, none of which the program chose.
    pub(crate) const COMMAND_SIGNAL: &str = "command_failed_signal";
    /// Failed with neither an exit code nor a signal observed.
    pub(crate) const COMMAND_UNKNOWN: &str = "command_failed_unknown";
    pub(crate) const TOOL: &str = "tool_failed";
    pub(crate) const DELEGATION: &str = "delegation_failed";
    pub(crate) const VERIFICATION: &str = "verification_failed";
}
