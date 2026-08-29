use crate::contexts::execution_observability::domain::{
    CommandRuntimeKind, EvidenceCommandId, EvidenceCoverageState, EvidenceKind, EvidenceSeatId,
    EvidenceSessionId, EvidenceToolCallId, ExecutionFidelity, ExecutionStatus, OutputAvailability,
    QueryCoverage, VerificationOutcome,
};

/// Default and maximum page sizes, matching the frontend contract so a client cannot ask for more
/// than the service will serve and then treat the shortfall as an empty tail.
pub(crate) const DEFAULT_EVIDENCE_PAGE_SIZE: usize = 100;
pub(crate) const MAX_EVIDENCE_PAGE_SIZE: usize = 500;

/// What a projected execution record looks like to the application layer.
///
/// Deliberately not the journal event: a record is the accumulated state of a lifecycle, which is
/// what a list has to show, whereas an event is one observation about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionRecordProjection {
    pub(crate) record_id: String,
    pub(crate) kind: ExecutionRecordKind,
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) seat_id: Option<EvidenceSeatId>,
    /// Absent when only a completion was observed. A projection never invents a start time; the
    /// record is `incomplete` and the field stays missing.
    pub(crate) started_at: Option<String>,
    pub(crate) ended_at: Option<String>,
    /// Only ever derived from an observed start and an observed end.
    pub(crate) duration_ms: Option<u64>,
    pub(crate) status: ExecutionStatus,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) detail: ExecutionRecordDetailFields,
    pub(crate) last_sequence: i64,
    pub(crate) occurred_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionRecordKind {
    Command,
    Tool,
    Delegation,
    Verification,
}

impl ExecutionRecordKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Tool => "tool",
            Self::Delegation => "delegation",
            Self::Verification => "verification",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "command" => Self::Command,
            "tool" => Self::Tool,
            "delegation" => Self::Delegation,
            "verification" => Self::Verification,
            _ => return None,
        })
    }

    pub(crate) fn for_kind(kind: EvidenceKind) -> Option<Self> {
        Some(match kind {
            EvidenceKind::CommandStarted | EvidenceKind::CommandCompleted => Self::Command,
            EvidenceKind::ToolStarted | EvidenceKind::ToolCompleted => Self::Tool,
            EvidenceKind::AgentDelegated | EvidenceKind::AgentCompleted => Self::Delegation,
            EvidenceKind::VerificationCompleted => Self::Verification,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutionRecordDetailFields {
    Command {
        command_id: EvidenceCommandId,
        runtime_kind: CommandRuntimeKind,
        redacted_display: Option<String>,
        cwd_display: Option<String>,
        exit_code: Option<i32>,
        signal: Option<String>,
        output_availability: OutputAvailability,
        output_truncated: bool,
    },
    Tool {
        tool_call_id: Option<EvidenceToolCallId>,
        tool_name: String,
    },
    Delegation {
        parent_agent_id: Option<String>,
        child_agent_id: Option<String>,
        attempt: Option<u32>,
    },
    Verification {
        name: String,
        outcome: VerificationOutcome,
        passed_count: Option<u32>,
        failed_count: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceRecordPage {
    pub(crate) items: Vec<ExecutionRecordProjection>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) coverage: QueryCoverage,
}

/// Filters a record query supports. Every field participates in the cursor fingerprint, so
/// adding one here without adding it there would let a cursor survive a filter change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExecutionRecordFilters {
    pub(crate) kinds: Vec<ExecutionRecordKind>,
    pub(crate) statuses: Vec<ExecutionStatus>,
    pub(crate) fidelities: Vec<ExecutionFidelity>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceQueryScope {
    pub(crate) session_id: Option<EvidenceSessionId>,
    pub(crate) seat_id: Option<EvidenceSeatId>,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) command_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionRecordQuery {
    pub(crate) scope: EvidenceQueryScope,
    pub(crate) filters: ExecutionRecordFilters,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceEvidenceSummaryQuery {
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) seat_id: Option<EvidenceSeatId>,
}

/// Only the parts execution_observability actually owns.
///
/// Logs, Shells, changes, review progress, and usage belong to other contexts. Group 3 has no
/// port to them, so those figures are absent and the coverage says so rather than returning a
/// definitive zero that would render as "nothing happened".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceEvidenceSummary {
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) generated_at: String,
    pub(crate) coverage: QueryCoverage,
    pub(crate) run_status: Option<ExecutionStatus>,
    pub(crate) run_id: Option<String>,
    pub(crate) run_started_at: Option<String>,
    pub(crate) running_records: u32,
    pub(crate) failed_records: u32,
    pub(crate) verification_passed: u32,
    pub(crate) verification_failed: u32,
    /// Sources this context does not own, each with the coverage that says why it is absent.
    pub(crate) unowned_sources: Vec<UnownedSummarySource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnownedSummarySource {
    pub(crate) source: &'static str,
    pub(crate) coverage_state: EvidenceCoverageState,
    pub(crate) reason_code: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EvidenceCorrelationCounts {
    pub(crate) commands: u32,
    pub(crate) tools: u32,
    pub(crate) delegations: u32,
    pub(crate) verifications: u32,
    pub(crate) file_mutations: u32,
    pub(crate) usage_observations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionRecordDetailQuery {
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionRecordDetailView {
    pub(crate) record: ExecutionRecordProjection,
    pub(crate) counts: EvidenceCorrelationCounts,
    pub(crate) error_reason_code: Option<String>,
}

/// What happened to one ingestion attempt. `Duplicate` is a success: the producer's retry did the
/// right thing and the journal already holds the assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordEvidenceOutcome {
    Recorded { sequence: i64 },
    Duplicate { sequence: i64 },
    Conflict,
}

/// Identifiers and counts only. This crosses the Tauri event channel, where redaction cannot be
/// re-applied, so nothing that is not an id, a sequence, or a classification may appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceNotice {
    pub(crate) kind: EvidenceNoticeKind,
    pub(crate) sequence: i64,
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) occurred_at: String,
    pub(crate) record_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) command_id: Option<String>,
    pub(crate) seat_id: Option<EvidenceSeatId>,
    pub(crate) dropped_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceNoticeKind {
    RecordAppended,
    RecordUpdated,
    SummaryChanged,
    CoverageGap,
}

impl EvidenceNoticeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RecordAppended => "record-appended",
            Self::RecordUpdated => "record-updated",
            Self::SummaryChanged => "summary-changed",
            Self::CoverageGap => "coverage-gap",
        }
    }
}

/// What a subscriber needs before it starts applying live notices: the sequence the store has
/// already committed through. Anything at or below it is already in the page the client fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceSubscriptionBootstrap {
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) watermark_sequence: i64,
    pub(crate) coverage: QueryCoverage,
}
