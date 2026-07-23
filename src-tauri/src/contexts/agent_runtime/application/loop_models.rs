use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopLimits, LoopRunPhase, LoopRunStatus, LoopTerminalReason,
    LoopVerificationCommand, LoopVerifierRecommendation,
};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopVerificationCommandView {
    pub(crate) id: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) timeout_seconds: u64,
    pub(crate) required: bool,
}

impl From<&LoopVerificationCommand> for LoopVerificationCommandView {
    fn from(command: &LoopVerificationCommand) -> Self {
        Self {
            id: command.id().to_string(),
            program: command.program().to_string(),
            args: command.args().to_vec(),
            working_directory: command.working_directory().map(str::to_string),
            timeout_seconds: command.timeout_seconds(),
            required: command.required(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopLimitsView {
    pub(crate) max_iterations: u16,
    pub(crate) step_timeout_seconds: u64,
    pub(crate) total_timeout_seconds: u64,
    pub(crate) max_consecutive_runtime_errors: u16,
    pub(crate) max_consecutive_no_progress: u16,
}

impl From<&LoopLimits> for LoopLimitsView {
    fn from(limits: &LoopLimits) -> Self {
        Self {
            max_iterations: limits.max_iterations(),
            step_timeout_seconds: limits.step_timeout_seconds(),
            total_timeout_seconds: limits.total_timeout_seconds(),
            max_consecutive_runtime_errors: limits.max_consecutive_runtime_errors(),
            max_consecutive_no_progress: limits.max_consecutive_no_progress(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopDefinitionView {
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
    pub(crate) verification_commands: Vec<LoopVerificationCommandView>,
    pub(crate) limits: LoopLimitsView,
    pub(crate) version: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<&LoopDefinition> for LoopDefinitionView {
    fn from(definition: &LoopDefinition) -> Self {
        let values = definition.values();
        Self {
            id: values.id.clone(),
            name: values.name.clone(),
            enabled: values.enabled,
            project_path: values.project_path.clone(),
            base_branch: values.base_branch.clone(),
            goal: values.goal.clone(),
            acceptance_criteria: values.acceptance_criteria.clone(),
            allowed_paths: values.allowed_paths.clone(),
            protected_paths: values.protected_paths.clone(),
            worker_agent_id: values.worker_agent_id.clone(),
            verifier_agent_id: values.verifier_agent_id.clone(),
            verification_commands: values
                .verification_commands
                .iter()
                .map(LoopVerificationCommandView::from)
                .collect(),
            limits: LoopLimitsView::from(&values.limits),
            version: values.version,
            created_at: values.created_at.clone(),
            updated_at: values.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoopEvidenceView {
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoopIterationView {
    pub(crate) id: String,
    pub(crate) run_id: String,
    pub(crate) sequence: u16,
    pub(crate) status: LoopRunStatus,
    pub(crate) worker_session_id: Option<String>,
    pub(crate) verifier_session_id: Option<String>,
    pub(crate) worker_summary: Option<String>,
    pub(crate) verifier_recommendation: Option<String>,
    pub(crate) verifier_findings: Vec<String>,
    pub(crate) decision_reason: Option<String>,
    pub(crate) diff_fingerprint: Option<String>,
    pub(crate) check_failure_fingerprint: Option<String>,
    pub(crate) user_feedback: Option<String>,
    pub(crate) evidence: Vec<LoopEvidenceView>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoopRunView {
    pub(crate) id: String,
    pub(crate) definition_id: String,
    pub(crate) definition_snapshot: LoopDefinitionView,
    pub(crate) status: LoopRunStatus,
    pub(crate) phase: LoopRunPhase,
    pub(crate) terminal_reason: Option<LoopTerminalReason>,
    pub(crate) current_iteration: u16,
    pub(crate) consecutive_runtime_errors: u16,
    pub(crate) consecutive_no_progress: u16,
    pub(crate) pause_requested: bool,
    pub(crate) project_path: String,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_name: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) active_operation_id: Option<String>,
    pub(crate) iterations: Vec<LoopIterationView>,
    pub(crate) simulated: bool,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartLoopResultView {
    pub(crate) run_id: String,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinueLoopRequest {
    pub(crate) run_id: String,
    pub(crate) feedback: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveLoopDefinitionRequest {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopGitStateEntryView {
    pub(crate) path: String,
    pub(crate) index_status: String,
    pub(crate) worktree_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopGitStateView {
    pub(crate) branch: Option<String>,
    pub(crate) entries: Vec<LoopGitStateEntryView>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedLoopWorktree {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) branch: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StartLoopWorkerRequest {
    pub(crate) run_id: String,
    pub(crate) sequence: u16,
    pub(crate) definition_snapshot: LoopDefinitionView,
    pub(crate) project_path: String,
    pub(crate) worktree_path: String,
    pub(crate) worktree_name: String,
    pub(crate) worktree_branch: String,
    pub(crate) prior_evidence: Vec<LoopEvidenceView>,
    pub(crate) user_feedback: Option<String>,
    pub(crate) elapsed_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopRoleSessionRequest {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) agent_id: String,
    pub(crate) project_path: String,
    pub(crate) worktree_path: String,
    pub(crate) worktree_name: String,
    pub(crate) worktree_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedLoopWorkerView {
    pub(crate) iteration_id: String,
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) context_bytes: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RunLoopVerificationRequest {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) worktree_root: String,
    pub(crate) commands: Vec<LoopVerificationCommandView>,
    pub(crate) cancellation: super::LoopVerificationCancellation,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoopVerificationBatchResult {
    pub(crate) evidence: Vec<LoopEvidenceView>,
    pub(crate) required_checks_passed: bool,
    pub(crate) cancelled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StartLoopVerifierRequest {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) definition_snapshot: LoopDefinitionView,
    pub(crate) project_path: String,
    pub(crate) worktree_path: String,
    pub(crate) worktree_name: String,
    pub(crate) worktree_branch: String,
    pub(crate) check_evidence: Vec<LoopEvidenceView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedLoopVerifierView {
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) context_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopVerifierResult {
    pub(crate) recommendation: LoopVerifierRecommendation,
    pub(crate) findings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveLoopVerifierResultRequest {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) session_id: String,
    pub(crate) result: LoopVerifierResult,
}
