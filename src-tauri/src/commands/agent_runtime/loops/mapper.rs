use super::dto;
use crate::contexts::agent_runtime::api::{
    AgentRuntimeApplicationError, LoopDefinitionView, LoopLimits, LoopRunView,
    LoopVerificationCommand, SaveLoopDefinitionRequest,
};

pub(crate) fn save_request(
    input: dto::SaveLoopDefinitionInput,
) -> Result<SaveLoopDefinitionRequest, crate::commands::error::CommandError> {
    let commands = input
        .verification_commands
        .into_iter()
        .map(|command| {
            LoopVerificationCommand::new(
                command.id,
                command.program,
                command.args,
                command.working_directory,
                command.timeout_seconds,
                command.required,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(AgentRuntimeApplicationError::from)
        .map_err(crate::commands::error::map_command_error)?;
    let limits = LoopLimits::new(
        input.limits.max_iterations,
        input.limits.step_timeout_seconds,
        input.limits.total_timeout_seconds,
        input.limits.max_consecutive_runtime_errors,
        input.limits.max_consecutive_no_progress,
    )
    .map_err(AgentRuntimeApplicationError::from)
    .map_err(crate::commands::error::map_command_error)?;
    Ok(SaveLoopDefinitionRequest {
        name: input.name,
        enabled: input.enabled,
        project_path: input.project_path,
        base_branch: input.base_branch,
        goal: input.goal,
        acceptance_criteria: input.acceptance_criteria,
        allowed_paths: input.allowed_paths,
        protected_paths: input.protected_paths,
        worker_agent_id: input.worker_agent_id,
        verifier_agent_id: input.verifier_agent_id,
        verification_commands: commands,
        limits,
        expected_version: input.expected_version,
    })
}

pub(crate) fn definition(value: LoopDefinitionView) -> dto::LoopDefinition {
    dto::LoopDefinition {
        id: value.id,
        name: value.name,
        enabled: value.enabled,
        project_path: value.project_path,
        base_branch: value.base_branch,
        goal: value.goal,
        acceptance_criteria: value.acceptance_criteria,
        allowed_paths: value.allowed_paths,
        protected_paths: value.protected_paths,
        worker_agent_id: value.worker_agent_id,
        verifier_agent_id: value.verifier_agent_id,
        verification_commands: value
            .verification_commands
            .into_iter()
            .map(|command| dto::LoopVerificationCommand {
                id: command.id,
                program: command.program,
                args: command.args,
                working_directory: command.working_directory,
                timeout_seconds: command.timeout_seconds,
                required: command.required,
            })
            .collect(),
        limits: dto::LoopLimits {
            max_iterations: value.limits.max_iterations,
            step_timeout_seconds: value.limits.step_timeout_seconds,
            total_timeout_seconds: value.limits.total_timeout_seconds,
            max_consecutive_runtime_errors: value.limits.max_consecutive_runtime_errors,
            max_consecutive_no_progress: value.limits.max_consecutive_no_progress,
        },
        version: value.version,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

pub(crate) fn run(value: LoopRunView) -> dto::LoopRun {
    dto::LoopRun {
        id: value.id,
        definition_id: value.definition_id,
        definition_snapshot: definition(value.definition_snapshot),
        status: value.status.as_str().to_string(),
        phase: value.phase.as_str().to_string(),
        terminal_reason: value
            .terminal_reason
            .map(|reason| reason.as_str().to_string()),
        current_iteration: value.current_iteration,
        consecutive_runtime_errors: value.consecutive_runtime_errors,
        consecutive_no_progress: value.consecutive_no_progress,
        pause_requested: value.pause_requested,
        project_path: value.project_path,
        worktree_path: value.worktree_path,
        worktree_name: value.worktree_name,
        worktree_branch: value.worktree_branch,
        active_operation_id: value.active_operation_id,
        iterations: value
            .iterations
            .into_iter()
            .map(|iteration| dto::LoopIteration {
                id: iteration.id,
                run_id: iteration.run_id,
                sequence: iteration.sequence,
                status: iteration.status.as_str().to_string(),
                worker_session_id: iteration.worker_session_id,
                verifier_session_id: iteration.verifier_session_id,
                worker_summary: iteration.worker_summary,
                verifier_recommendation: iteration.verifier_recommendation,
                verifier_findings: iteration.verifier_findings,
                decision_reason: iteration.decision_reason,
                diff_fingerprint: iteration.diff_fingerprint,
                check_failure_fingerprint: iteration.check_failure_fingerprint,
                user_feedback: iteration.user_feedback,
                evidence: iteration
                    .evidence
                    .into_iter()
                    .map(|evidence| dto::LoopEvidence {
                        id: evidence.id,
                        run_id: evidence.run_id,
                        iteration_id: evidence.iteration_id,
                        kind: evidence.kind,
                        status: evidence.status,
                        summary: evidence.summary,
                        operation_id: evidence.operation_id,
                        command_id: evidence.command_id,
                        exit_code: evidence.exit_code,
                        duration_ms: evidence.duration_ms,
                        details: evidence.details,
                        created_at: evidence.created_at,
                    })
                    .collect(),
                started_at: iteration.started_at,
                completed_at: iteration.completed_at,
            })
            .collect(),
        simulated: value.simulated,
        created_at: value.created_at,
        started_at: value.started_at,
        updated_at: value.updated_at,
        completed_at: value.completed_at,
    }
}
