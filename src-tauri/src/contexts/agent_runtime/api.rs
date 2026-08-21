//! Published in-process facade for Agent Runtime use cases.
//!
//! Command adapters and other native contexts use this facade instead of reaching into Agent
//! Runtime repositories or provider infrastructure. It coordinates interactive execution, Agent
//! terminals, loop engineering, and durable Multi-Agent runs.

use super::application::{
    AgentRuntimeApplicationService, AgentTerminalApplicationService, BrowserHandoffControlPort,
    ContextManifestQueryService, ContextQualityQueryService, ExpertRoleApplicationService,
    LocalModelDiscoveryService, LoopApplicationService, LoopControlApplicationService,
    LoopRecoveryApplicationService, LoopVerificationCancellation, LoopVerificationCommandView,
    LoopVerificationProcessPort, LoopVerificationProcessRequest, LoopVerificationProcessStatus,
};
use super::infrastructure::{
    background_shell_registry, task_list_store, ManualNativeToolControl, NativeLoopScheduler,
    NativeSeatTurnCoordinator,
};
use std::sync::Arc;

pub(crate) use super::application::{
    ActiveGenerationCorrelation, AgentChatConfiguration, AgentFileReference, AgentMemory,
    AgentMessage, AgentMessageSource, AgentMessageTerminalOutcome, AgentRuntimeApplicationError,
    AgentSessionDetails, AgentTerminalInputRequest, AgentTerminalSession, AgentTerminalSize,
    AgentView, ApiProviderConfig, ChangeSetApplyPort, ChangeSetApplyRecord, ChangeSetFileRecord,
    ChangeSetRecord, ChangeSetStatus, CliDelegationPort, ContinueLoopRequest,
    DelegationAttemptRecord, DelegationMode, DelegationRecord, DelegationStatus, DelegationTarget,
    DiscoverOnePieceProviderModelsInput, EmbeddingEndpointView, FileChangeKind, HybridRoutePreview,
    HybridRoutePreviewInput, LaunchWorkflowResult, LocalEndpointVerificationRequest,
    LocalModelDiscoveryResult, LoopDefinitionView, LoopReadinessReportView, LoopRunView,
    ManualApplyDelegationRequest, ManualStartDelegationRequest, NativeToolErrorCode,
    NativeToolPersistencePort, NativeToolPortRequest, NativeToolRegistry, NativeToolResultEnvelope,
    NativeToolResultStatus, OnePieceProviderConfig, OnePieceProviderModelDiscoveryResult,
    OnePieceProviderModelOption, OnePieceProviderPreset, OnePieceProviderProfiles,
    OpenAgentTerminalRequest, ProviderCredentialValidationResult, ReadinessView, RecoveryRecord,
    RecoveryStatus, RegisterApiAgentInput, ResizeAgentTerminalRequest, RunnerDescriptor,
    RunnerSelection, SaveCustomOnePieceProviderProfileInput, SaveLoopDefinitionRequest,
    SaveOnePieceProviderConfigInput, SaveOnePieceProviderProfileInput, SendMessageRequest,
    StartLoopResultView, StartedAgentMessage, StopAgentTerminalRequest, StopGenerationResult,
    StoredEndpointProfileMetadata, StoredHybridRoutingRule, ToolApprovalDecision,
    UpdateApiAgentInput, ValidateOnePieceProviderCredentialInput, WorkflowView,
};

#[cfg(test)]
pub(crate) use super::application::LoopReadinessCheckView;

const GUARDED_VALIDATION_OUTPUT_LIMIT: usize = 4_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardedValidationRequest {
    pub(crate) worktree_root: String,
    pub(crate) command: LoopVerificationCommand,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GuardedValidationCancellation {
    inner: LoopVerificationCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuardedValidationStatus {
    Passed,
    Failed,
    TimedOut,
    Cancelled,
}

impl GuardedValidationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardedValidationResult {
    pub(crate) status: GuardedValidationStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: u64,
    pub(crate) output_summary: Option<String>,
    pub(crate) output_truncated: bool,
}
#[cfg(test)]
pub(crate) use super::application::{AgentLaunchView, MessageTokenUsage};
pub(crate) use super::domain::{
    AgentAvailability, AgentLifecycle, ContextEvidenceManifest, ContextEvidenceManifestPage,
    ContextQualityAssessmentPage, ContextQualitySummary, ExpertRole, ExpertRoleInput,
    InteractionMode, LoopLimits, LoopVerificationCommand,
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
    pub(crate) guarded_validation: Arc<dyn LoopVerificationProcessPort>,
    pub(crate) context_quality: ContextQualityQueryService,
    pub(crate) context_manifests: ContextManifestQueryService,
    pub(crate) native_tools: NativeToolRegistry,
    pub(crate) browser_handoff: Option<std::sync::Arc<dyn BrowserHandoffControlPort>>,
    pub(crate) manual_native_tools: ManualNativeToolControl,
    pub(crate) local_discovery: LocalModelDiscoveryService,
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
    guarded_validation: Arc<dyn LoopVerificationProcessPort>,
    context_quality: ContextQualityQueryService,
    context_manifests: ContextManifestQueryService,
    native_tools: NativeToolRegistry,
    browser_handoff: Option<std::sync::Arc<dyn BrowserHandoffControlPort>>,
    manual_native_tools: ManualNativeToolControl,
    local_discovery: LocalModelDiscoveryService,
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
            guarded_validation,
            context_quality,
            context_manifests,
            native_tools,
            browser_handoff,
            manual_native_tools,
            local_discovery,
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
            guarded_validation,
            context_quality,
            context_manifests,
            native_tools,
            browser_handoff,
            manual_native_tools,
            local_discovery,
        }
    }

    pub(crate) fn list_context_quality_history(
        &self,
        range_days: u32,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ContextQualityAssessmentPage, AgentRuntimeApplicationError> {
        self.context_quality.list(range_days, cursor, limit)
    }

    pub(crate) fn context_quality_summary(
        &self,
        range_days: u32,
    ) -> Result<ContextQualitySummary, AgentRuntimeApplicationError> {
        self.context_quality.summarize(range_days)
    }

    pub(crate) fn list_context_evidence_manifests(
        &self,
        session_id: Option<&str>,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ContextEvidenceManifestPage, AgentRuntimeApplicationError> {
        self.context_manifests
            .list(session_id, cursor, limit)
            .map_err(AgentRuntimeApplicationError::ContextQuality)
    }

    pub(crate) fn get_context_evidence_manifest(
        &self,
        generation_id: &str,
    ) -> Result<Option<ContextEvidenceManifest>, AgentRuntimeApplicationError> {
        self.context_manifests
            .get(generation_id)
            .map_err(AgentRuntimeApplicationError::ContextQuality)
    }

    pub(crate) fn run_guarded_validation_cancellable(
        &self,
        request: GuardedValidationRequest,
        cancellation: GuardedValidationCancellation,
    ) -> Result<GuardedValidationResult, AgentRuntimeApplicationError> {
        let result = self
            .guarded_validation
            .execute(LoopVerificationProcessRequest {
                worktree_root: request.worktree_root,
                command: LoopVerificationCommandView::from(&request.command),
                cancellation: cancellation.inner,
            })?;
        let (output_summary, bounded_truncated) =
            bounded_validation_output(&result.stdout, &result.stderr);
        Ok(GuardedValidationResult {
            status: match result.status {
                LoopVerificationProcessStatus::Passed => GuardedValidationStatus::Passed,
                LoopVerificationProcessStatus::Failed => GuardedValidationStatus::Failed,
                LoopVerificationProcessStatus::TimedOut => GuardedValidationStatus::TimedOut,
                LoopVerificationProcessStatus::Cancelled => GuardedValidationStatus::Cancelled,
            },
            exit_code: result.exit_code,
            duration_ms: result.duration_ms,
            output_summary,
            output_truncated: result.output_truncated || bounded_truncated,
        })
    }

    pub(crate) fn get_browser_handoff(&self, operation_id: &str) -> Result<serde_json::Value, ()> {
        self.browser_handoff
            .as_ref()
            .ok_or(())?
            .get_handoff(operation_id)
    }

    pub(crate) fn begin_browser_handoff(
        &self,
        operation_id: &str,
    ) -> Result<serde_json::Value, ()> {
        self.browser_handoff
            .as_ref()
            .ok_or(())?
            .begin_handoff(operation_id)
    }

    pub(crate) fn resume_browser_automation(
        &self,
        operation_id: &str,
        ownership_token: &str,
    ) -> Result<serde_json::Value, ()> {
        self.browser_handoff
            .as_ref()
            .ok_or(())?
            .resume_automation(operation_id, ownership_token)
    }

    pub(crate) fn is_native_tool_registered(&self, name: &str) -> bool {
        self.native_tools.is_registered(name)
    }

    /// Terminates every background command the session owns (`add-background-shell-execution`).
    /// Called when a session ends, alongside the equivalent workspace-shell cleanup -- not on
    /// generation cancellation, which would kill the dev server a user deliberately left running.
    pub(crate) fn reap_background_commands(&self, session_id: &str) {
        background_shell_registry().reap_session(session_id);
        // The task list is session-scoped runtime state with the same lifetime, so it is
        // discarded on the same edge (`add-agent-task-list`).
        task_list_store().clear_session(session_id);
    }

    /// Terminates every remaining background command on desktop shutdown. Windows' job object
    /// would reap these when the process handle closed, but an orphaned Unix process group would
    /// survive, so this is the portable guarantee rather than a convenience.
    pub(crate) fn reap_all_background_commands(&self) {
        background_shell_registry().reap_all();
    }

    pub(crate) async fn start_manual_delegation(
        &self,
        request: ManualStartDelegationRequest,
    ) -> Result<serde_json::Value, String> {
        let control = self.manual_native_tools.clone();
        tauri::async_runtime::spawn_blocking(move || control.start_delegation(request))
            .await
            .map_err(|_| "manual_dispatch_failed".to_owned())?
    }

    pub(crate) async fn apply_manual_delegation_changes(
        &self,
        request: ManualApplyDelegationRequest,
    ) -> Result<serde_json::Value, String> {
        let control = self.manual_native_tools.clone();
        tauri::async_runtime::spawn_blocking(move || control.apply_delegation_changes(request))
            .await
            .map_err(|_| "manual_dispatch_failed".to_owned())?
    }

    pub(crate) fn cancel_manual_native_tool(&self, operation_id: &str) -> bool {
        self.manual_native_tools.cancel(operation_id)
    }

    pub(crate) fn native_tool_readiness_reason(&self, name: &str) -> Option<&'static str> {
        self.native_tools
            .readiness_reason(name)
            .map(|reason| reason.as_str())
    }

    pub(crate) fn is_native_tool_backend_ready(&self, name: &str) -> bool {
        self.is_native_tool_registered(name) && self.native_tool_readiness_reason(name).is_none()
    }

    pub(crate) fn is_native_tool_feature_enabled(&self, capability: &str, mode: &str) -> bool {
        self.native_tools.is_feature_enabled(capability, mode)
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

    pub(crate) fn check_loop_readiness(
        &self,
        definition_id: &str,
    ) -> Result<LoopReadinessReportView, AgentRuntimeApplicationError> {
        self.loops.readiness(definition_id)
    }

    pub(crate) async fn check_loop_readiness_blocking(
        &self,
        definition_id: String,
    ) -> Result<LoopReadinessReportView, AgentRuntimeApplicationError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.check_loop_readiness(&definition_id))
            .await
            .map_err(|_| {
                AgentRuntimeApplicationError::Loop(
                    "Loop readiness task failed before completion.".to_string(),
                )
            })?
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

    pub(crate) fn save_custom_onepiece_provider_profile(
        &self,
        input: SaveCustomOnePieceProviderProfileInput,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        self.service.save_custom_onepiece_provider_profile(input)
    }

    pub(crate) fn endpoint_profile_metadata(
        &self,
        profile_id: &str,
    ) -> Result<Option<StoredEndpointProfileMetadata>, AgentRuntimeApplicationError> {
        self.service.endpoint_profile_metadata(profile_id)
    }

    pub(crate) fn hybrid_routing_rules(
        &self,
    ) -> Result<Vec<StoredHybridRoutingRule>, AgentRuntimeApplicationError> {
        self.service.hybrid_routing_rules()
    }

    pub(crate) fn replace_hybrid_routing_rules(
        &self,
        rules: Vec<StoredHybridRoutingRule>,
    ) -> Result<Vec<StoredHybridRoutingRule>, AgentRuntimeApplicationError> {
        self.service.replace_hybrid_routing_rules(rules)
    }

    pub(crate) fn preview_hybrid_route(
        &self,
        input: HybridRoutePreviewInput,
    ) -> Result<HybridRoutePreview, AgentRuntimeApplicationError> {
        self.service.preview_hybrid_route(input)
    }

    pub(crate) async fn discover_local_model_endpoints(
        &self,
    ) -> Result<LocalModelDiscoveryResult, AgentRuntimeApplicationError> {
        let service = self.local_discovery.clone();
        tauri::async_runtime::spawn_blocking(move || service.discover_loopback())
            .await
            .map_err(|error| {
                AgentRuntimeApplicationError::Operation(format!(
                    "Local endpoint discovery task failed: {error}"
                ))
            })?
    }

    pub(crate) async fn verify_local_model_endpoint(
        &self,
        input: LocalEndpointVerificationRequest,
    ) -> Result<LocalModelDiscoveryResult, AgentRuntimeApplicationError> {
        let service = self.local_discovery.clone();
        let (operation_id, endpoint) =
            tauri::async_runtime::spawn_blocking(move || service.verify_endpoint(input))
                .await
                .map_err(|error| {
                    AgentRuntimeApplicationError::Operation(format!(
                        "Local endpoint verification task failed: {error}"
                    ))
                })??;
        Ok(LocalModelDiscoveryResult {
            operation_id,
            endpoints: vec![endpoint],
        })
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

    /// Resolves a blocked `exit_plan_mode` request (`add-agent-plan-exit-request`). The decision
    /// arrives from the UI as "approved or not", and turning that into a `ToolApprovalDecision`
    /// belongs here rather than in the Tauri command: `tests/architecture.rs` holds command
    /// adapters to zero control-flow decisions, and this mapping is one.
    pub(crate) fn resolve_plan_exit(
        &self,
        session_id: &str,
        call_id: &str,
        approved: bool,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let decision = if approved {
            ToolApprovalDecision::Approved
        } else {
            ToolApprovalDecision::Denied
        };
        self.resolve_tool_approval(session_id, call_id, decision)
    }

    pub(crate) fn resolve_tool_approval(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let generation =
            self.service
                .resolve_tool_approval(session_id, call_id, decision.clone())?;
        let manual = self
            .manual_native_tools
            .resolve_approval(session_id, call_id, decision);
        Ok(generation || manual)
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

    pub(crate) fn send_message_with_runner(
        &self,
        request: SendMessageRequest,
        runner: RunnerSelection,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let session_id = request.session_id.clone();
        let message = self.service.send_message_with_runner(request, runner)?;
        if self.service.is_multi_seat_session(&session_id) {
            self.seat_turns.schedule(&session_id)?;
        }
        Ok(message)
    }

    pub(crate) fn list_runners(
        &self,
        session_id: &str,
        agent_id: &str,
    ) -> Result<Vec<RunnerDescriptor>, AgentRuntimeApplicationError> {
        self.service.list_runners(session_id, agent_id)
    }

    pub(crate) fn send_message_with_completion(
        &self,
        request: SendMessageRequest,
    ) -> Result<StartedAgentMessage, AgentRuntimeApplicationError> {
        self.service.send_message_with_completion(request)
    }

    pub(crate) fn send_evaluation_message_with_completion(
        &self,
        request: SendMessageRequest,
    ) -> Result<StartedAgentMessage, AgentRuntimeApplicationError> {
        self.service
            .send_non_interactive_message_with_completion(request)
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

    pub(crate) fn shutdown_generations(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        self.service.shutdown_generations()
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

fn bounded_validation_output(stdout: &str, stderr: &str) -> (Option<String>, bool) {
    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => return (None, false),
        (false, true) => format!("stdout:\n{stdout}"),
        (true, false) => format!("stderr:\n{stderr}"),
        (false, false) => format!("stdout:\n{stdout}\nstderr:\n{stderr}"),
    };
    let truncated = combined.chars().count() > GUARDED_VALIDATION_OUTPUT_LIMIT;
    (
        Some(
            combined
                .chars()
                .take(GUARDED_VALIDATION_OUTPUT_LIMIT)
                .collect(),
        ),
        truncated,
    )
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
