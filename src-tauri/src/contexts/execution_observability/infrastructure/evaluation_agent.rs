use crate::contexts::agent_runtime::api::{
    AgentAvailability, AgentChatConfiguration, AgentFileReference, AgentMessageSource,
    AgentMessageTerminalOutcome, AgentRuntimeApi, InteractionMode, SendMessageRequest,
};
use crate::contexts::execution_observability::application::AgentExecutionEvidence;
use crate::contexts::execution_observability::domain::{
    DISPATCH_AGENT_UNAVAILABLE, DISPATCH_AGENT_UNSUPPORTED, DISPATCH_TERMINAL_DISCONNECTED,
};
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EvaluationDispatchRequest {
    pub(crate) task_id: String,
    pub(crate) attempt_id: String,
    pub(crate) agent_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) prompt: String,
    pub(crate) workspace: String,
    pub(crate) timeout_seconds: u64,
    pub(crate) canonical_run_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EvaluationDispatchResult {
    pub(crate) evidence: AgentExecutionEvidence,
    pub(crate) session_id: String,
    pub(crate) operation_id: Option<String>,
    pub(crate) execution_run_id: Option<String>,
    pub(crate) output_summary: Option<String>,
}

#[derive(Clone)]
pub(crate) struct NativeEvaluationAgentAdapter {
    sessions: SessionsApi,
    agents: AgentRuntimeApi,
}

impl NativeEvaluationAgentAdapter {
    pub(crate) fn new(sessions: SessionsApi, agents: AgentRuntimeApi) -> Self {
        Self { sessions, agents }
    }

    pub(crate) fn dispatch(
        &self,
        request: &EvaluationDispatchRequest,
    ) -> Result<EvaluationDispatchResult, String> {
        let mode = self.eligible_mode(&request.agent_id)?;
        let prepared = self
            .sessions
            .prepare_creation(NewSessionRequest {
                // An evaluation session is machine-driven; it takes the default like any other
                // caller that does not choose.
                personalization_mode: None,
                agent_id: request.agent_id.clone(),
                seats: Vec::new(),
                interaction_mode: mode.as_str().to_string(),
                title: Some(format!("Evaluation {}", request.task_id)),
                workspace: NewSessionWorkspace {
                    folder: Some(request.workspace.clone()),
                    project_path: Some(request.workspace.clone()),
                    remote_workspace: None,
                    worktree: None,
                },
                owner: SessionOwner::desktop(),
                activation: SessionActivation::PreserveActive,
            })
            .map_err(|error| error.to_string())?;
        let session = self
            .sessions
            .execute_creation(prepared)
            .map_err(|error| error.to_string())?;
        let session_id = session.id().to_string();
        let timeout_seconds = request.timeout_seconds.max(1);
        let started_at = Instant::now();
        let started = self
            .agents
            .send_evaluation_message_with_completion(SendMessageRequest {
                source: AgentMessageSource::Desktop,
                session_id: session_id.clone(),
                content: request.prompt.clone(),
                configuration: AgentChatConfiguration {
                    agent_id: request.agent_id.clone(),
                    interaction_mode: mode,
                    execution_mode: "execute".to_string(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    reasoning_depth: None,
                    streaming: true,
                    thinking: false,
                    long_context: false,
                },
                file_references: Vec::<AgentFileReference>::new(),
            })
            .map_err(|error| error.to_string())?;
        let correlation = self
            .agents
            .active_generation_correlation(&session_id)
            .map_err(|error| error.to_string())?;
        let terminal = started
            .terminal
            .recv_timeout(Duration::from_secs(timeout_seconds.saturating_add(5)));
        let (completed, timed_out, cancelled, output_summary) = match terminal {
            Ok(terminal) => (
                terminal.outcome == AgentMessageTerminalOutcome::Completed,
                false,
                terminal.outcome == AgentMessageTerminalOutcome::Cancelled,
                terminal.content.map(|value| bounded_text(&value, 4_000)),
            ),
            Err(RecvTimeoutError::Timeout) => {
                let _ = self.agents.stop_generation(&session_id);
                (false, true, false, None)
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(DISPATCH_TERMINAL_DISCONNECTED.to_string());
            }
        };
        let summary = self
            .sessions
            .session_usage_summary(&session_id)
            .map_err(|error| error.to_string())?;
        let tool_calls = self
            .sessions
            .list_messages(&session_id, Some(200), None)
            .map_err(|error| error.to_string())?
            .into_iter()
            .flat_map(|message| message.tool_use.unwrap_or_default())
            .count();
        Ok(EvaluationDispatchResult {
            evidence: AgentExecutionEvidence {
                completed,
                timed_out,
                stuck: false,
                cancelled,
                tool_calls: u32::try_from(tool_calls).unwrap_or(u32::MAX),
                duration_ms: u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
                retries: 0,
                replans: 0,
                recoveries: 0,
                interventions: 0,
                reported_input_tokens: reported_count(
                    summary.coverage.reported_responses,
                    summary.reported.input_tokens,
                ),
                reported_output_tokens: reported_count(
                    summary.coverage.reported_responses,
                    summary.reported.output_tokens,
                ),
                context_evidence_manifest_id: None,
                pricing: None,
            },
            session_id,
            operation_id: correlation
                .as_ref()
                .and_then(|value| value.operation_id.clone()),
            execution_run_id: correlation.and_then(|value| value.execution_run_id),
            output_summary,
        })
    }

    fn eligible_mode(&self, agent_id: &str) -> Result<InteractionMode, String> {
        let agent = self
            .agents
            .get_agent(agent_id)
            .map_err(|error| error.to_string())?;
        eligible_mode(
            &agent.id,
            agent.availability,
            agent.managed_sdk_dependency_id.as_deref(),
            &agent.supported_interaction_modes,
        )
    }
}

fn eligible_mode(
    agent_id: &str,
    availability: AgentAvailability,
    managed_dependency: Option<&str>,
    supported_modes: &[InteractionMode],
) -> Result<InteractionMode, String> {
    if availability != AgentAvailability::Available {
        return Err(DISPATCH_AGENT_UNAVAILABLE.to_string());
    }
    if agent_id == "onepiece" && supported_modes.contains(&InteractionMode::Api) {
        return Ok(InteractionMode::Api);
    }
    if managed_dependency.is_some() && supported_modes.contains(&InteractionMode::Cli) {
        return Ok(InteractionMode::Cli);
    }
    Err(DISPATCH_AGENT_UNSUPPORTED.to_string())
}

fn non_negative(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn reported_count(observations: i64, value: i64) -> Option<u64> {
    (observations > 0).then(|| non_negative(value)).flatten()
}

fn bounded_text(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_allows_onepiece_and_managed_cli_only() {
        assert_eq!(
            eligible_mode(
                "onepiece",
                AgentAvailability::Available,
                None,
                &[InteractionMode::Api]
            ),
            Ok(InteractionMode::Api)
        );
        assert_eq!(
            eligible_mode(
                "codex-cli",
                AgentAvailability::Available,
                Some("codex"),
                &[InteractionMode::Cli]
            ),
            Ok(InteractionMode::Cli)
        );
        assert!(eligible_mode(
            "browser-agent",
            AgentAvailability::Available,
            None,
            &[InteractionMode::Browser]
        )
        .is_err());
    }

    #[test]
    fn unavailable_agents_and_negative_usage_are_not_fabricated() {
        assert!(eligible_mode(
            "codex-cli",
            AgentAvailability::Unavailable,
            Some("codex"),
            &[InteractionMode::Cli]
        )
        .is_err());
        assert_eq!(non_negative(-1), None);
        assert_eq!(non_negative(7), Some(7));
        assert_eq!(reported_count(0, 0), None);
    }
}
