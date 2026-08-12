//! Published in-process facade for Agent Runtime use cases.
//!
//! Command adapters and other native contexts use this facade instead of reaching into Agent
//! Runtime repositories or provider infrastructure. It coordinates interactive execution, Agent
//! terminals, loop engineering, and durable Multi-Agent runs.

use super::application::{
    AgentRuntimeApplicationService, AgentTerminalApplicationService, ExpertRoleApplicationService,
    LoopApplicationService, LoopControlApplicationService, LoopRecoveryApplicationService,
};
use super::infrastructure::{NativeLoopScheduler, NativeSeatTurnCoordinator};

pub(crate) use super::application::{
    ActiveGenerationCorrelation, AgentChatConfiguration, AgentFileReference, AgentMemory,
    AgentMessage, AgentMessageTerminalOutcome, AgentRuntimeApplicationError, AgentSessionDetails,
    AgentTerminalInputRequest, AgentTerminalSession, AgentTerminalSize, AgentView,
    ApiProviderConfig, ContinueLoopRequest, DiscoverOnePieceProviderModelsInput,
    EmbeddingEndpointView, LaunchWorkflowResult, LoopDefinitionView, LoopRunView,
    OnePieceProviderConfig, OnePieceProviderModelDiscoveryResult, OnePieceProviderModelOption,
    OnePieceProviderPreset, OnePieceProviderProfiles, OpenAgentTerminalRequest,
    ProviderCredentialValidationResult, ReadinessView, RegisterApiAgentInput,
    ResizeAgentTerminalRequest, SaveLoopDefinitionRequest, SaveOnePieceProviderConfigInput,
    SaveOnePieceProviderProfileInput, SendMessageRequest, StartLoopResultView, StartedAgentMessage,
    StopAgentTerminalRequest, StopGenerationResult, ToolApprovalDecision, UpdateApiAgentInput,
    ValidateOnePieceProviderCredentialInput, WorkflowView,
};
#[cfg(test)]
pub(crate) use super::application::{AgentLaunchView, MessageTokenUsage};
pub(crate) use super::domain::{
    AgentAvailability, AgentLifecycle, ExpertRole, ExpertRoleInput, InteractionMode, LoopLimits,
    LoopVerificationCommand,
};

/// Assembled in bootstrap and handed over whole, so adding a service does not lengthen a
/// positional argument list nobody can read.
pub(crate) struct AgentRuntimeApiServices {
    pub(crate) service: AgentRuntimeApplicationService,
    pub(crate) terminal_service: AgentTerminalApplicationService,
    pub(crate) loops: LoopApplicationService,
    pub(crate) loop_controls: LoopControlApplicationService,
    pub(crate) loop_recovery: LoopRecoveryApplicationService,
    pub(crate) loop_scheduler: NativeLoopScheduler,
    pub(crate) expert_roles: ExpertRoleApplicationService,
    pub(crate) seat_turns: NativeSeatTurnCoordinator,
}

#[derive(Clone)]
/// In-process Agent Runtime boundary assembled by bootstrap.
///
/// The facade schedules variable-duration work after the application service has persisted its
/// initial state, keeping command handlers thin and preventing infrastructure leakage.
pub(crate) struct AgentRuntimeApi {
    service: AgentRuntimeApplicationService,
    terminal_service: AgentTerminalApplicationService,
    loops: LoopApplicationService,
    loop_controls: LoopControlApplicationService,
    loop_recovery: LoopRecoveryApplicationService,
    loop_scheduler: NativeLoopScheduler,
    seat_turns: NativeSeatTurnCoordinator,
    expert_roles: ExpertRoleApplicationService,
}

impl AgentRuntimeApi {
    pub(crate) fn new(services: AgentRuntimeApiServices) -> Self {
        let AgentRuntimeApiServices {
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
            expert_roles,
            seat_turns,
        } = services;
        Self {
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
            expert_roles,
            seat_turns,
        }
    }

    pub(crate) fn list_expert_roles(
        &self,
    ) -> Result<Vec<ExpertRole>, AgentRuntimeApplicationError> {
        self.expert_roles.list()
    }

    pub(crate) fn save_expert_role(
        &self,
        role_id: Option<String>,
        input: ExpertRoleInput,
    ) -> Result<ExpertRole, AgentRuntimeApplicationError> {
        self.expert_roles.save(role_id, input)
    }

    pub(crate) fn delete_expert_role(
        &self,
        role_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.expert_roles.delete(role_id)
    }

    pub(crate) fn list_loop_definitions(
        &self,
    ) -> Result<Vec<LoopDefinitionView>, AgentRuntimeApplicationError> {
        self.loops.list_definitions()
    }

    pub(crate) fn create_loop_definition(
        &self,
        request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        self.loops.create_definition(request)
    }

    pub(crate) fn update_loop_definition(
        &self,
        definition_id: &str,
        request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        self.loops.update_definition(definition_id, request)
    }

    pub(crate) fn delete_loop_definition(
        &self,
        definition_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.loops.delete_definition(definition_id)
    }

    pub(crate) fn list_loop_runs(
        &self,
        definition_id: Option<&str>,
    ) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
        self.loops.list_runs(definition_id)
    }

    pub(crate) fn get_loop_run(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loops.get_run(run_id)
    }

    pub(crate) fn start_loop(
        &self,
        definition_id: &str,
    ) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
        let result = self.loops.start_manual(definition_id)?;
        self.loop_scheduler.schedule(&result.run_id)?;
        Ok(result)
    }

    pub(crate) fn pause_loop(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loop_controls.request_pause(run_id)?;
        self.loops.get_run(run_id)
    }

    pub(crate) fn resume_loop(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loop_controls.resume(run_id)?;
        self.loop_scheduler.schedule(run_id)?;
        self.loops.get_run(run_id)
    }

    pub(crate) fn cancel_loop(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loop_controls.cancel(run_id)?;
        self.loops.get_run(run_id)
    }

    pub(crate) fn accept_loop(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loop_controls.accept(run_id)?;
        self.loops.get_run(run_id)
    }

    pub(crate) fn continue_loop(
        &self,
        request: ContinueLoopRequest,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        let run_id = request.run_id.clone();
        self.loop_controls.continue_with_feedback(request)?;
        self.loop_scheduler.schedule(&run_id)?;
        self.loops.get_run(&run_id)
    }

    pub(crate) fn reject_loop(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.loop_controls.reject(run_id)?;
        self.loops.get_run(run_id)
    }

    pub(crate) fn reconcile_loop_startup(
        &self,
    ) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
        let recovered = self.loop_recovery.reconcile_startup()?;
        recovered
            .iter()
            .map(|run| self.loops.get_run(run.id()))
            .collect()
    }

    pub(crate) fn list_agents(
        &self,
        capability_tag: Option<&str>,
    ) -> Result<Vec<AgentView>, AgentRuntimeApplicationError> {
        self.service.list_agents(capability_tag)
    }

    pub(crate) fn get_agent(
        &self,
        agent_id: &str,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        self.service.get_agent(agent_id)
    }

    pub(crate) fn register_api_agent(
        &self,
        request: RegisterApiAgentInput,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        self.service.register_api_agent(request)
    }

    pub(crate) fn api_agent_provider_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        self.service.api_agent_provider_config(agent_id)
    }

    pub(crate) fn onepiece_provider_config(
        &self,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        self.service.onepiece_provider_config()
    }

    pub(crate) fn save_onepiece_provider_config(
        &self,
        input: SaveOnePieceProviderConfigInput,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        self.service.save_onepiece_provider_config(input)
    }

    pub(crate) fn reset_onepiece_provider_config(
        &self,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        self.service.reset_onepiece_provider_config()
    }

    pub(crate) fn onepiece_provider_profiles(
        &self,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        self.service.onepiece_provider_profiles()
    }

    pub(crate) fn onepiece_provider_presets(&self) -> Vec<OnePieceProviderPreset> {
        self.service.onepiece_provider_presets()
    }

    pub(crate) async fn discover_onepiece_provider_models(
        &self,
        input: DiscoverOnePieceProviderModelsInput,
    ) -> Result<OnePieceProviderModelDiscoveryResult, AgentRuntimeApplicationError> {
        let service = self.service.clone();
        tauri::async_runtime::spawn_blocking(move || {
            service.discover_onepiece_provider_models(input)
        })
        .await
        .map_err(|error| {
            AgentRuntimeApplicationError::Validation(format!(
                "OnePiece model discovery task failed: {error}"
            ))
        })?
    }

    pub(crate) async fn validate_onepiece_provider_credential(
        &self,
        input: ValidateOnePieceProviderCredentialInput,
    ) -> Result<ProviderCredentialValidationResult, AgentRuntimeApplicationError> {
        let service = self.service.clone();
        tauri::async_runtime::spawn_blocking(move || {
            service.validate_onepiece_provider_credential(input)
        })
        .await
        .map_err(|error| {
            AgentRuntimeApplicationError::Validation(format!(
                "OnePiece credential validation task failed: {error}"
            ))
        })?
    }

    pub(crate) fn save_onepiece_provider_profile(
        &self,
        input: SaveOnePieceProviderProfileInput,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        self.service.save_onepiece_provider_profile(input)
    }

    pub(crate) fn activate_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        self.service.activate_onepiece_provider_profile(profile_id)
    }

    pub(crate) fn delete_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        self.service.delete_onepiece_provider_profile(profile_id)
    }

    pub(crate) fn resolve_embedding_endpoint(
        &self,
        profile_id: &str,
    ) -> Result<EmbeddingEndpointView, AgentRuntimeApplicationError> {
        self.service.resolve_embedding_endpoint(profile_id)
    }

    // 与 discover_onepiece_provider_models / validate_onepiece_provider_credential 一样用
    // spawn_blocking 包裹：底层复用同一个阻塞式 HTTP 客户端
    // （HttpOnePieceModelDiscoveryAdapter），直接在异步上下文里调用会占住 tokio 工作线程。
    pub(crate) async fn list_embedding_models(
        &self,
        profile_id: &str,
        transient_credential: Option<&str>,
    ) -> Result<Vec<OnePieceProviderModelOption>, AgentRuntimeApplicationError> {
        let service = self.service.clone();
        let profile_id = profile_id.to_string();
        let transient_credential = transient_credential.map(str::to_string);
        tauri::async_runtime::spawn_blocking(move || {
            service.list_embedding_models(&profile_id, transient_credential.as_deref())
        })
        .await
        .map_err(|error| {
            AgentRuntimeApplicationError::Validation(format!(
                "OnePiece embedding model discovery task failed: {error}"
            ))
        })?
    }

    pub(crate) fn update_api_agent(
        &self,
        agent_id: &str,
        input: UpdateApiAgentInput,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        self.service.update_api_agent(agent_id, input)
    }

    pub(crate) fn delete_api_agent(
        &self,
        agent_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.service.delete_api_agent(agent_id)
    }

    pub(crate) fn resolve_tool_approval(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.service
            .resolve_tool_approval(session_id, call_id, decision)
    }

    pub(crate) fn list_all_memories(
        &self,
    ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        self.service.list_all_memories()
    }

    pub(crate) fn delete_agent_memory(
        &self,
        memory_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.service.delete_agent_memory(memory_id)
    }

    pub(crate) fn reset_all_memories(&self) -> Result<(), AgentRuntimeApplicationError> {
        self.service.reset_all_memories()
    }

    pub(crate) fn workflow(&self) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        self.service.workflow()
    }

    pub(crate) fn select_agent(
        &self,
        agent_id: &str,
        interaction_mode: InteractionMode,
    ) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        self.service.select_agent(agent_id, interaction_mode)
    }

    pub(crate) fn browser_readiness(
        &self,
        agent_id: &str,
    ) -> Result<ReadinessView, AgentRuntimeApplicationError> {
        self.service.browser_readiness(agent_id)
    }

    pub(crate) fn launch_active_workflow(
        &self,
    ) -> Result<LaunchWorkflowResult, AgentRuntimeApplicationError> {
        self.service.launch_active_workflow()
    }

    pub(crate) fn session_details(
        &self,
    ) -> Result<AgentSessionDetails, AgentRuntimeApplicationError> {
        self.service.session_details()
    }

    pub(crate) fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let session_id = request.session_id.clone();
        let message = self.service.send_message(request)?;
        // A single-seat session has nobody to hand off to, so it never pays for a coordinator.
        if self.service.is_multi_seat_session(&session_id) {
            self.seat_turns.schedule(&session_id)?;
        }
        Ok(message)
    }

    pub(crate) fn send_message_with_completion(
        &self,
        request: SendMessageRequest,
    ) -> Result<StartedAgentMessage, AgentRuntimeApplicationError> {
        self.service.send_message_with_completion(request)
    }

    pub(crate) fn active_generation_correlation(
        &self,
        session_id: &str,
    ) -> Result<Option<ActiveGenerationCorrelation>, AgentRuntimeApplicationError> {
        self.service.active_generation_correlation(session_id)
    }

    pub(crate) fn stop_generation(
        &self,
        session_id: &str,
    ) -> Result<StopGenerationResult, AgentRuntimeApplicationError> {
        self.service.stop_generation(session_id)
    }

    pub(crate) fn open_agent_terminal(
        &self,
        request: OpenAgentTerminalRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        self.terminal_service.open_or_attach(request)
    }

    pub(crate) fn write_agent_terminal_input(
        &self,
        request: AgentTerminalInputRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.terminal_service.input(request)
    }

    pub(crate) fn resize_agent_terminal(
        &self,
        request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.terminal_service.resize(request)
    }

    pub(crate) fn stop_agent_terminal(
        &self,
        request: StopAgentTerminalRequest,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.terminal_service.stop(request)
    }

    pub(crate) fn cleanup_idle_agent_terminals(
        &self,
        idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        self.terminal_service.cleanup_idle(idle_after_seconds)
    }

    pub(crate) fn shutdown_agent_terminals(
        &self,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        self.terminal_service.shutdown()
    }
}

/// Boundary `commands::agent_runtime::delete_agent_memory` needs from this facade to delete one
/// stored memory (`add-onepiece-vector-search` Task 14). A trait — rather than that command
/// calling the inherent `delete_agent_memory` method directly — so the command's own tests can
/// substitute a fake instead of constructing a full `AgentRuntimeApi`, which would otherwise
/// require every one of this facade's concrete application services just to delete one row.
pub(crate) trait AgentMemoryDeletionGateway: Send + Sync {
    fn delete_agent_memory(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

impl AgentMemoryDeletionGateway for AgentRuntimeApi {
    fn delete_agent_memory(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        // Calls the inherent method above, not this trait method — method resolution always
        // prefers an inherent impl over a trait impl for the same receiver type, so this cannot
        // recurse.
        self.delete_agent_memory(memory_id)
    }
}
