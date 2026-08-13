use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanRunSummaryView {
    pub(crate) id: String,
    pub(crate) plan_id: String,
    pub(crate) status: String,
    pub(crate) completed_tasks: u32,
    pub(crate) total_tasks: u32,
    pub(crate) simulated: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanRunPageView {
    pub(crate) items: Vec<PlanRunSummaryView>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanRunDetailView {
    #[serde(flatten)]
    pub(crate) summary: PlanRunSummaryView,
    pub(crate) project_path: String,
    pub(crate) base_ref: String,
    pub(crate) base_oid: Option<String>,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) originating_session_id: Option<String>,
    pub(crate) tasks: Vec<PlanSubTaskRunView>,
    pub(crate) finalization: Option<PlanFinalizationView>,
    pub(crate) available_controls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanFinalizationView {
    pub(crate) id: String,
    pub(crate) sequence: u32,
    pub(crate) status: String,
    pub(crate) evidence: Vec<PlanAttemptEvidenceView>,
    pub(crate) repair_attempts: Vec<PlanFinalRepairView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanFinalRepairView {
    pub(crate) id: String,
    pub(crate) sequence: u32,
    pub(crate) status: String,
    pub(crate) session_id: Option<String>,
    pub(crate) token_usage: u32,
    pub(crate) tool_call_count: u32,
    pub(crate) error_class: Option<String>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanSubTaskRunView {
    pub(crate) id: String,
    pub(crate) subtask_id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) topological_rank: u16,
    pub(crate) ordinal: u16,
    pub(crate) result_summary: Option<String>,
    pub(crate) changed_files: Vec<String>,
    pub(crate) verification_summary: Option<String>,
    pub(crate) attempts: Vec<PlanSubTaskAttemptView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanSubTaskAttemptView {
    pub(crate) id: String,
    pub(crate) sequence: u32,
    pub(crate) status: String,
    pub(crate) session_id: Option<String>,
    pub(crate) profile_id: Option<String>,
    pub(crate) execution_run_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) token_usage: u32,
    pub(crate) tool_call_count: u32,
    pub(crate) error_class: Option<String>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanAttemptEvidenceView {
    pub(crate) id: String,
    pub(crate) command_id: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) output_summary: Option<String>,
    pub(crate) created_at: String,
}
