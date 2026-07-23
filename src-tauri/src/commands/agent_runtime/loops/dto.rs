use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveLoopDefinitionInput {
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) project_path: String,
    pub(crate) base_branch: String,
    pub(crate) goal: String,
    pub(crate) acceptance_criteria: Vec<String>,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) worker_agent_id: String,
    pub(crate) verifier_agent_id: String,
    pub(crate) verification_commands: Vec<LoopVerificationCommand>,
    pub(crate) limits: LoopLimits,
    pub(crate) expected_version: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopVerificationCommand {
    pub(crate) id: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) timeout_seconds: u64,
    pub(crate) required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopLimits {
    pub(crate) max_iterations: u16,
    pub(crate) step_timeout_seconds: u64,
    pub(crate) total_timeout_seconds: u64,
    pub(crate) max_consecutive_runtime_errors: u16,
    pub(crate) max_consecutive_no_progress: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContinueLoopInput {
    pub(crate) run_id: String,
    pub(crate) feedback: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) project_path: String,
    pub(crate) base_branch: String,
    pub(crate) goal: String,
    pub(crate) acceptance_criteria: Vec<String>,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) worker_agent_id: String,
    pub(crate) verifier_agent_id: String,
    pub(crate) verification_commands: Vec<LoopVerificationCommand>,
    pub(crate) limits: LoopLimits,
    pub(crate) version: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopEvidence {
    pub(crate) id: String,
    pub(crate) run_id: String,
    pub(crate) iteration_id: Option<String>,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) summary: String,
    pub(crate) operation_id: Option<String>,
    pub(crate) command_id: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) details: Option<Value>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopIteration {
    pub(crate) id: String,
    pub(crate) run_id: String,
    pub(crate) sequence: u16,
    pub(crate) status: String,
    pub(crate) worker_session_id: Option<String>,
    pub(crate) verifier_session_id: Option<String>,
    pub(crate) worker_summary: Option<String>,
    pub(crate) verifier_recommendation: Option<String>,
    pub(crate) verifier_findings: Vec<String>,
    pub(crate) decision_reason: Option<String>,
    pub(crate) diff_fingerprint: Option<String>,
    pub(crate) check_failure_fingerprint: Option<String>,
    pub(crate) user_feedback: Option<String>,
    pub(crate) evidence: Vec<LoopEvidence>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopRun {
    pub(crate) id: String,
    pub(crate) definition_id: String,
    pub(crate) definition_snapshot: LoopDefinition,
    pub(crate) status: String,
    pub(crate) phase: String,
    pub(crate) terminal_reason: Option<String>,
    pub(crate) current_iteration: u16,
    pub(crate) consecutive_runtime_errors: u16,
    pub(crate) consecutive_no_progress: u16,
    pub(crate) pause_requested: bool,
    pub(crate) project_path: String,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) active_operation_id: Option<String>,
    pub(crate) iterations: Vec<LoopIteration>,
    pub(crate) simulated: bool,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartLoopResult {
    pub(crate) run: LoopRun,
    pub(crate) operation_id: String,
}
