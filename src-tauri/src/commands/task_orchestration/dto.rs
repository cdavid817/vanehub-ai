use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApprovePlanResult {
    pub(crate) run_id: String,
    pub(crate) summary: ApprovalTransitionSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApprovalTransitionSummary {
    pub(crate) project_path: String,
    pub(crate) task_count: usize,
    pub(crate) retained_worktree: bool,
    pub(crate) automatic_git_operations: bool,
}
