use super::AttemptDispatch;
use crate::contexts::agent_runtime::api::{
    AgentChatConfiguration, AgentFileReference, AgentMessageSource, AgentMessageTerminalOutcome,
    AgentRuntimeApi, ExecutionToolMode, InteractionMode, OrchestrationCorrelation,
    OrchestrationExecutionProfile, SendMessageRequest,
};
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
use crate::contexts::task_orchestration::application::PlanApplicationError;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

const DEFAULT_ATTEMPT_TIMEOUT_SECONDS: u64 = 900;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedAttemptSession {
    pub(crate) session_id: String,
    pub(crate) profile_id: String,
    pub(crate) model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptExecutionOutput {
    pub(crate) succeeded: bool,
    pub(crate) result_summary: Option<String>,
    pub(crate) error_class: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) execution_run_id: Option<String>,
}

#[derive(Clone)]
pub(crate) struct OnePieceAttemptExecutor {
    sessions: SessionsApi,
    agents: AgentRuntimeApi,
}

impl OnePieceAttemptExecutor {
    pub(crate) fn new(sessions: SessionsApi, agents: AgentRuntimeApi) -> Self {
        Self { sessions, agents }
    }

    pub(crate) fn create_session(
        &self,
        dispatch: &AttemptDispatch,
    ) -> Result<PreparedAttemptSession, PlanApplicationError> {
        let profiles = self
            .agents
            .onepiece_provider_profiles()
            .map_err(runtime_error)?;
        let profile = profiles
            .profiles
            .into_iter()
            .find(|profile| {
                profile.id == dispatch.profile_id && profile.active && profile.credential_present
            })
            .ok_or_else(|| {
                PlanApplicationError::Validation(
                    "captured OnePiece Profile is no longer active and ready".to_string(),
                )
            })?;
        let prepared = self
            .sessions
            .prepare_creation(NewSessionRequest {
                agent_id: "onepiece".to_string(),
                seats: Vec::new(),
                interaction_mode: "api".to_string(),
                title: Some(format!("Plan task {}", dispatch.task.ordinal + 1)),
                workspace: NewSessionWorkspace {
                    folder: Some(dispatch.worktree_path.clone()),
                    project_path: Some(dispatch.project_path.clone()),
                    remote_workspace: None,
                    worktree: None,
                },
                owner: SessionOwner::desktop(),
                activation: SessionActivation::PreserveActive,
            })
            .map_err(runtime_error)?;
        let session = self
            .sessions
            .execute_creation(prepared)
            .map_err(runtime_error)?;
        Ok(PreparedAttemptSession {
            session_id: session.id().to_string(),
            profile_id: profile.id,
            model_id: profile.model_id,
        })
    }

    pub(crate) fn execute(
        &self,
        dispatch: &AttemptDispatch,
        attempt_id: &str,
        session: &PreparedAttemptSession,
        prompt: String,
    ) -> Result<AttemptExecutionOutput, PlanApplicationError> {
        let timeout_seconds = dispatch
            .task
            .limits
            .timeout_seconds
            .unwrap_or(DEFAULT_ATTEMPT_TIMEOUT_SECONDS);
        let started = self
            .agents
            .send_orchestration_message_with_completion(
                SendMessageRequest {
                    source: AgentMessageSource::Desktop,
                    session_id: session.session_id.clone(),
                    content: prompt,
                    configuration: AgentChatConfiguration {
                        agent_id: "onepiece".to_string(),
                        interaction_mode: InteractionMode::Api,
                        permission_mode: "agent".to_string(),
                        provider_id: Some("onepiece".to_string()),
                        model_id: Some(session.model_id.clone()),
                        reasoning_depth: None,
                        streaming: true,
                        thinking: false,
                        long_context: false,
                    },
                    file_references: Vec::<AgentFileReference>::new(),
                },
                OrchestrationExecutionProfile {
                    bounded_root: Some(dispatch.worktree_path.clone()),
                    tool_mode: ExecutionToolMode::Standard,
                    permitted_tools: vec![
                        "shell".into(),
                        "file".into(),
                        "grep".into(),
                        "glob".into(),
                        "edit".into(),
                        "remember".into(),
                        "recall".into(),
                    ],
                    tool_call_limit: dispatch.task.limits.tool_call_limit,
                    token_budget: dispatch.task.limits.token_budget,
                    timeout_seconds: Some(timeout_seconds),
                    correlation: OrchestrationCorrelation {
                        plan_run_id: Some(dispatch.plan_run_id.clone()),
                        subtask_run_id: Some(dispatch.subtask_run_id.clone()),
                        attempt_id: Some(attempt_id.to_string()),
                    },
                },
            )
            .map_err(runtime_error)?;
        let correlation = self
            .agents
            .active_generation_correlation(&session.session_id)
            .map_err(runtime_error)?;
        let operation_id = correlation
            .as_ref()
            .and_then(|value| value.operation_id.clone());
        let execution_run_id = correlation.and_then(|value| value.execution_run_id);
        let terminal = match started
            .terminal
            .recv_timeout(Duration::from_secs(timeout_seconds.saturating_add(5)))
        {
            Ok(terminal) => terminal,
            Err(RecvTimeoutError::Timeout) => {
                let _ = self.agents.stop_generation(&session.session_id);
                return Ok(AttemptExecutionOutput {
                    succeeded: false,
                    result_summary: None,
                    error_class: Some("timeout".to_string()),
                    operation_id,
                    execution_run_id,
                });
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(PlanApplicationError::Storage(
                    "OnePiece attempt terminal channel disconnected".to_string(),
                ));
            }
        };
        let succeeded = terminal.outcome == AgentMessageTerminalOutcome::Completed;
        let error_class = (!succeeded)
            .then(|| self.classify_terminal_failure(&session.session_id, terminal.outcome));
        Ok(AttemptExecutionOutput {
            succeeded,
            result_summary: terminal
                .content
                .map(|content| bounded_text(&content, 4_000)),
            error_class,
            operation_id,
            execution_run_id,
        })
    }

    pub(crate) fn usage(&self, session_id: &str) -> Result<(u32, u32), PlanApplicationError> {
        let summary = self
            .sessions
            .session_usage_summary(session_id)
            .map_err(runtime_error)?;
        let token_usage = u32::try_from(summary.reported.total_tokens.max(0)).unwrap_or(u32::MAX);
        let tool_call_count = self
            .sessions
            .list_messages(session_id, Some(200), None)
            .map_err(runtime_error)?
            .into_iter()
            .flat_map(|message| message.tool_use.unwrap_or_default())
            .count();
        Ok((
            token_usage,
            u32::try_from(tool_call_count).unwrap_or(u32::MAX),
        ))
    }

    pub(crate) fn stop_session(&self, session_id: &str) -> Result<(), PlanApplicationError> {
        self.agents
            .stop_generation(session_id)
            .map(|_| ())
            .map_err(runtime_error)
    }

    fn classify_terminal_failure(
        &self,
        session_id: &str,
        outcome: AgentMessageTerminalOutcome,
    ) -> String {
        if outcome == AgentMessageTerminalOutcome::Cancelled {
            return "cancelled".to_string();
        }
        let diagnostic = self
            .sessions
            .list_messages(session_id, Some(20), None)
            .ok()
            .and_then(|messages| messages.into_iter().rev().find_map(|message| message.error))
            .unwrap_or_default();
        if diagnostic.contains("orchestration timeout") {
            "timeout".to_string()
        } else if diagnostic.contains("orchestration tool-call limit") {
            "tool_call_limit".to_string()
        } else if diagnostic.contains("orchestration token") {
            "token_budget".to_string()
        } else {
            "provider_failed".to_string()
        }
    }
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn runtime_error(error: impl std::fmt::Display) -> PlanApplicationError {
    PlanApplicationError::Storage(error.to_string())
}
