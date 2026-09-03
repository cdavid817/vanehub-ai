use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The evidence wire contract.
///
/// Field names and shapes match the Zod schemas the frontend committed to in Task Group 2, because
/// activation is proven by re-running those conformance cases against these payloads rather than
/// by new tests written to fit whatever this layer happens to emit.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryCoverageDto {
    pub(crate) state: String,
    pub(crate) reason_codes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) oldest_available_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) newest_available_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) indexed_through_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dropped_count: Option<u32>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceRunStateDto {
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceChangesDto {
    pub(crate) changed_files: u32,
    pub(crate) unviewed_files: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidencePairDto {
    pub(crate) running: u32,
    pub(crate) failed: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceShellsDto {
    pub(crate) live: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceLogsDto {
    pub(crate) new_errors: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceVerificationDto {
    pub(crate) passed: u32,
    pub(crate) failed: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceUsageDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reported_tokens: Option<u32>,
    pub(crate) coverage: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceEvidenceSummaryDto {
    pub(crate) session_id: String,
    pub(crate) generated_at: String,
    pub(crate) coverage: QueryCoverageDto,
    pub(crate) run_state: EvidenceRunStateDto,
    /// Owned by other contexts. These read as zero only next to a `coverage` that says this
    /// context cannot see them, never as a standalone figure.
    pub(crate) changes: EvidenceChangesDto,
    pub(crate) execution_records: EvidencePairDto,
    pub(crate) shells: EvidenceShellsDto,
    pub(crate) logs: EvidenceLogsDto,
    pub(crate) traces: EvidencePairDto,
    pub(crate) verification: EvidenceVerificationDto,
    pub(crate) usage: EvidenceUsageDto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRecordDto {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seat_id: Option<String>,
    /// Omitted when only a completion was observed. `skip_serializing_if` is what makes the field
    /// absent from the JSON rather than an empty string or a time nobody measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ended_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
    pub(crate) status: String,
    pub(crate) fidelity: String,
    pub(crate) coverage: QueryCoverageDto,
    #[serde(flatten)]
    pub(crate) detail: ExecutionRecordDetailDto,
}

/// `rename_all` on an untagged enum renames the *variants*, not the fields inside them, so each
/// variant carries its own. Without it the kind-specific half of every record would serialize as
/// snake_case while the shared half stayed camelCase, and the frontend's discriminated union would
/// reject the row.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum ExecutionRecordDetailDto {
    #[serde(rename_all = "camelCase")]
    Command {
        command_id: String,
        runtime_kind: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        redacted_display: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd_display: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        signal: Option<String>,
        output_availability: String,
        output_truncated: bool,
    },
    #[serde(rename_all = "camelCase")]
    Tool {
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        tool_name: String,
        source: String,
    },
    #[serde(rename_all = "camelCase")]
    Verification {
        verification_name: String,
        outcome: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        passed_count: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        failed_count: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    Delegation {
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        child_agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        attempt: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRecordPageDto {
    pub(crate) items: Vec<ExecutionRecordDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
    pub(crate) coverage: QueryCoverageDto,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceRelatedCountsDto {
    pub(crate) logs: u32,
    pub(crate) commands: u32,
    pub(crate) files: u32,
    pub(crate) findings: u32,
    pub(crate) usage_observations: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRecordDetailViewDto {
    pub(crate) record: ExecutionRecordDto,
    pub(crate) related_counts: EvidenceRelatedCountsDto,
    pub(crate) safe_attributes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceSubscriptionBootstrapDto {
    pub(crate) session_id: String,
    pub(crate) watermark_sequence: i64,
    pub(crate) coverage: QueryCoverageDto,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceScopeDto {
    pub(crate) session_id: Option<String>,
    pub(crate) seat_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) span_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) command_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceFiltersDto {
    #[serde(default)]
    pub(crate) kinds: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) statuses: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) fidelities: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) search: Option<String>,
}

/// A refusal the frontend can localize.
///
/// Reason codes only. A backend message would be rendered verbatim in a panel, and the failures
/// worth showing a user already have translations; echoing the input, the cursor, a path, SQL, or
/// a payload would also put the thing that failed into a second place.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidenceCommandErrorDto {
    pub(crate) reason_code: String,
}
