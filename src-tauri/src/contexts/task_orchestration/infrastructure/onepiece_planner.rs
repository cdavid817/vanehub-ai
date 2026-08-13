use crate::contexts::agent_runtime::api::{
    AgentChatConfiguration, AgentFileReference, AgentMessageSource, AgentMessageTerminalOutcome,
    AgentRuntimeApi, ExecutionToolMode, InteractionMode, OrchestrationCorrelation,
    OrchestrationExecutionProfile, SendMessageRequest,
};
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
use crate::contexts::task_orchestration::application::{
    PlanApplicationError, PlanGenerationPort, PlanGenerationRequest, PlanGenerationResponse,
};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
const PLANNER_INSTRUCTION_VERSION: u32 = 2;
const DISCOVERY_TOOLS: &[&str] = &[
    "file",
    "grep",
    "glob",
    "search_code",
    "lsp_definition",
    "lsp_references",
    "lsp_hover",
    "lsp_symbols",
];

#[derive(Clone)]
pub(crate) struct OnePiecePlanGenerator {
    agents: AgentRuntimeApi,
    sessions: SessionsApi,
}

impl OnePiecePlanGenerator {
    pub(crate) fn new(agents: AgentRuntimeApi, sessions: SessionsApi) -> Self {
        Self { agents, sessions }
    }
}

impl PlanGenerationPort for OnePiecePlanGenerator {
    fn generate(
        &self,
        request: &PlanGenerationRequest,
    ) -> Result<PlanGenerationResponse, PlanApplicationError> {
        let profiles = self
            .agents
            .onepiece_provider_profiles()
            .map_err(runtime_error)?;
        let profile = profiles
            .profiles
            .into_iter()
            .find(|profile| profile.active && profile.credential_present)
            .ok_or_else(|| {
                PlanApplicationError::Validation(
                    "OnePiece planning requires an active Profile with a stored credential."
                        .to_string(),
                )
            })?;
        let prepared = self
            .sessions
            .prepare_creation(NewSessionRequest {
                agent_id: "onepiece".to_string(),
                seats: Vec::new(),
                interaction_mode: "api".to_string(),
                title: Some("Temporary Plan discovery".to_string()),
                workspace: NewSessionWorkspace {
                    folder: Some(request.project_path.clone()),
                    project_path: Some(request.project_path.clone()),
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
        let session_id = session.id().to_string();
        let started = self.agents.send_orchestration_message_with_completion(
            SendMessageRequest {
                source: AgentMessageSource::Desktop,
                session_id: session_id.clone(),
                content: format!(
                    "plannerInstructionVersion: {PLANNER_INSTRUCTION_VERSION}\n{}",
                    request.prompt
                ),
                configuration: AgentChatConfiguration {
                    agent_id: "onepiece".to_string(),
                    interaction_mode: InteractionMode::Api,
                    execution_mode: "plan".to_string(),
                    provider_id: Some("onepiece".to_string()),
                    model_id: Some(profile.model_id.clone()),
                    reasoning_depth: None,
                    streaming: true,
                    thinking: false,
                    long_context: false,
                },
                file_references: Vec::<AgentFileReference>::new(),
            },
            OrchestrationExecutionProfile {
                bounded_root: Some(request.project_path.clone()),
                tool_mode: ExecutionToolMode::Standard,
                permitted_tools: DISCOVERY_TOOLS
                    .iter()
                    .map(|tool| (*tool).to_string())
                    .collect(),
                tool_call_limit: Some(40),
                token_budget: Some(24_000),
                timeout_seconds: Some(120),
                correlation: OrchestrationCorrelation {
                    plan_run_id: None,
                    subtask_run_id: None,
                    attempt_id: None,
                },
            },
        );
        let result = match started {
            Ok(started) => match started.terminal.recv_timeout(Duration::from_secs(125)) {
                Ok(terminal) if terminal.outcome == AgentMessageTerminalOutcome::Completed => {
                    terminal.content.ok_or_else(|| {
                        PlanApplicationError::Storage(
                            "OnePiece discovery returned no Plan JSON.".to_string(),
                        )
                    })
                }
                Ok(_) => Err(PlanApplicationError::Storage(
                    "OnePiece discovery did not complete successfully.".to_string(),
                )),
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.agents.stop_generation(&session_id);
                    Err(PlanApplicationError::Storage(
                        "OnePiece discovery exceeded its bounded timeout.".to_string(),
                    ))
                }
                Err(RecvTimeoutError::Disconnected) => Err(PlanApplicationError::Storage(
                    "OnePiece discovery completion channel disconnected.".to_string(),
                )),
            },
            Err(error) => Err(runtime_error(error)),
        };
        let _ = self.sessions.delete(&session_id);
        Ok(PlanGenerationResponse {
            content: result?,
            active_profile_id: profile.id,
        })
    }
}

fn runtime_error(error: impl std::fmt::Display) -> PlanApplicationError {
    PlanApplicationError::Storage(error.to_string())
}
