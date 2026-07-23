use super::loop_worker_prompt::{truncate_utf8, worker_prompt};
use super::{
    AgentClockPort, AgentRuntimeApplicationError, LoopGitStatePort, LoopIterationRepository,
    LoopIterationView, LoopRoleGenerationOutcome, LoopRoleGenerationTerminal, LoopRoleSessionPort,
    LoopRoleSessionRequest, LoopWorkerGenerationPort, StartLoopWorkerRequest,
    StartedLoopWorkerView,
};
use crate::contexts::agent_runtime::domain::LoopRunStatus;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopWorkerApplicationPorts {
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) roles: Arc<dyn LoopRoleSessionPort>,
    pub(crate) git: Arc<dyn LoopGitStatePort>,
    pub(crate) generations: Arc<dyn LoopWorkerGenerationPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopWorkerApplicationService {
    ports: LoopWorkerApplicationPorts,
}

impl LoopWorkerApplicationService {
    pub(crate) fn new(ports: LoopWorkerApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn start_iteration(
        &self,
        request: StartLoopWorkerRequest,
    ) -> Result<StartedLoopWorkerView, AgentRuntimeApplicationError> {
        validate_request(&request)?;
        let iteration_id = format!("loop-iteration-{}", Uuid::new_v4());
        let started_at = self.ports.clock.now();
        let feedback = request
            .user_feedback
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        self.ports.iterations.insert_iteration(&LoopIterationView {
            id: iteration_id.clone(),
            run_id: request.run_id.clone(),
            sequence: request.sequence,
            status: LoopRunStatus::Running,
            worker_session_id: None,
            verifier_session_id: None,
            worker_summary: None,
            verifier_recommendation: None,
            verifier_findings: Vec::new(),
            decision_reason: None,
            diff_fingerprint: None,
            check_failure_fingerprint: None,
            user_feedback: feedback.clone(),
            evidence: Vec::new(),
            started_at,
            completed_at: None,
        })?;

        self.launch_iteration(&iteration_id, &request, feedback.as_deref())
    }

    pub(crate) fn resume_iteration(
        &self,
        iteration_id: &str,
        request: StartLoopWorkerRequest,
    ) -> Result<StartedLoopWorkerView, AgentRuntimeApplicationError> {
        validate_request(&request)?;
        let feedback = request
            .user_feedback
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        self.launch_iteration(iteration_id, &request, feedback)
    }

    fn launch_iteration(
        &self,
        iteration_id: &str,
        request: &StartLoopWorkerRequest,
        feedback: Option<&str>,
    ) -> Result<StartedLoopWorkerView, AgentRuntimeApplicationError> {
        let session_id = self
            .ports
            .roles
            .create_worker_session(LoopRoleSessionRequest {
                run_id: request.run_id.clone(),
                iteration_id: iteration_id.to_string(),
                agent_id: request.definition_snapshot.worker_agent_id.clone(),
                project_path: request.project_path.clone(),
                worktree_path: request.worktree_path.clone(),
                worktree_name: request.worktree_name.clone(),
                worktree_branch: request.worktree_branch.clone(),
            })?;
        self.ports
            .iterations
            .attach_worker_session(iteration_id, &session_id)?;

        let git = self.ports.git.snapshot(&session_id)?;
        let prompt = worker_prompt(request, &git, feedback);
        let context_bytes = prompt.len();
        let message_id = self
            .ports
            .generations
            .start_worker_generation(&session_id, &prompt)?;
        Ok(StartedLoopWorkerView {
            iteration_id: iteration_id.to_string(),
            session_id,
            message_id,
            context_bytes,
        })
    }

    pub(crate) fn complete(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<String, AgentRuntimeApplicationError> {
        if terminal.role != "worker" {
            return Err(validation("Loop role result is not from a Worker."));
        }
        match terminal.outcome {
            LoopRoleGenerationOutcome::Completed => {}
            LoopRoleGenerationOutcome::Failed => {
                return Err(AgentRuntimeApplicationError::Loop(
                    terminal
                        .error
                        .unwrap_or_else(|| "Loop Worker generation failed.".to_string()),
                ))
            }
            LoopRoleGenerationOutcome::Cancelled => {
                return Err(AgentRuntimeApplicationError::Loop(
                    "Loop Worker generation was cancelled.".to_string(),
                ))
            }
        }
        let summary = terminal.content.unwrap_or_default();
        let summary = truncate_utf8(summary.trim(), 8 * 1024).to_string();
        self.ports.iterations.save_worker_summary(
            &terminal.run_id,
            &terminal.iteration_id,
            &terminal.session_id,
            &summary,
        )?;
        Ok(summary)
    }
}

fn validate_request(request: &StartLoopWorkerRequest) -> Result<(), AgentRuntimeApplicationError> {
    for (value, label) in [
        (&request.run_id, "Loop run id"),
        (&request.project_path, "Loop project path"),
        (&request.worktree_path, "Loop worktree path"),
        (&request.worktree_name, "Loop worktree name"),
        (&request.worktree_branch, "Loop worktree branch"),
    ] {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(validation(format!("{label} is required.")));
        }
    }
    if request.sequence == 0 || request.sequence > request.definition_snapshot.limits.max_iterations
    {
        return Err(validation("Loop iteration is outside configured limits."));
    }
    if request
        .prior_evidence
        .iter()
        .any(|evidence| evidence.run_id != request.run_id)
    {
        return Err(validation("Loop evidence belongs to another run."));
    }
    Ok(())
}

fn validation(message: impl Into<String>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.into())
}
