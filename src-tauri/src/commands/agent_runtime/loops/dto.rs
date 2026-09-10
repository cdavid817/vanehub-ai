use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopBranchChoice {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) available: bool,
    pub(crate) simulated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopReadinessCheck {
    pub(crate) code: String,
    pub(crate) category: String,
    pub(crate) status: String,
    pub(crate) blocking: bool,
    pub(crate) detail: Option<String>,
    pub(crate) remediation_target: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopSurfaceAssessment {
    pub(crate) surface: String,
    pub(crate) coverage: String,
    pub(crate) detail: String,
    pub(crate) blocking: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopExecutionAssessment {
    pub(crate) requested_mode: String,
    pub(crate) surfaces: Vec<LoopSurfaceAssessment>,
    pub(crate) blockers: Vec<String>,
    pub(crate) limitations: Vec<String>,
    pub(crate) satisfies_requested_mode: bool,
    pub(crate) acknowledgement_required: bool,
    pub(crate) witness_digest: String,
    pub(crate) assessed_at: String,
    pub(crate) simulated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopReadinessReport {
    pub(crate) definition_id: String,
    pub(crate) ready: bool,
    pub(crate) simulated: bool,
    pub(crate) checks: Vec<LoopReadinessCheck>,
    pub(crate) checked_at: String,
    pub(crate) requested_mode: Option<String>,
    pub(crate) scope_state: String,
    pub(crate) assessment: Option<LoopExecutionAssessment>,
    pub(crate) acknowledgement_required: bool,
    pub(crate) definition_revision: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopControlEnvelope {
    #[serde(default)]
    pub(crate) expected_revision: Option<u64>,
    #[serde(default)]
    pub(crate) idempotency_key: Option<String>,
    #[serde(default)]
    pub(crate) audit_acknowledgement_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareLoopAdmissionInput {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) definition_id: Option<String>,
    #[serde(default)]
    pub(crate) run_id: Option<String>,
    pub(crate) expected_revision: u64,
    #[serde(default)]
    pub(crate) client_context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopAdmission {
    pub(crate) action: String,
    pub(crate) target_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) requested_mode: String,
    pub(crate) scope_digest: String,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) assessment: LoopExecutionAssessment,
    pub(crate) acknowledgement_required: bool,
    pub(crate) challenge_id: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) simulated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopAuditAcknowledgement {
    pub(crate) acknowledgement_id: String,
    pub(crate) action: String,
    pub(crate) target_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestLoopAcceptanceInput {
    pub(crate) run_id: String,
    #[serde(default)]
    pub(crate) expected_revision: Option<u64>,
    #[serde(default)]
    pub(crate) expected_scope_digest: Option<String>,
    #[serde(default)]
    pub(crate) expected_evidence_id: Option<String>,
    #[serde(default)]
    pub(crate) idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopAcceptanceResult {
    pub(crate) run: LoopRun,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopRunScope {
    pub(crate) requested_mode: Option<String>,
    pub(crate) scope_digest: Option<String>,
    pub(crate) binding_status: String,
    pub(crate) assessment: Option<LoopExecutionAssessment>,
    pub(crate) sealed_evidence_id: Option<String>,
    pub(crate) acceptance_operation_id: Option<String>,
    pub(crate) baseline_digest: Option<String>,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
}

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
    /// Omitted by older clients: the definition is then saved as legacy-unverified and cannot
    /// start until it is saved with the current scope version.
    #[serde(default)]
    pub(crate) scope_schema_version: Option<u32>,
    #[serde(default)]
    pub(crate) requested_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoopVerificationCommand {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) kind: Option<String>,
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
    #[serde(default)]
    pub(crate) envelope: Option<LoopControlEnvelope>,
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
    pub(crate) scope_schema_version: Option<u32>,
    pub(crate) requested_mode: Option<String>,
    pub(crate) scope_state: String,
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
    pub(crate) revision: u64,
    pub(crate) scope: Option<LoopRunScope>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartLoopResult {
    pub(crate) run: LoopRun,
    pub(crate) operation_id: String,
}
