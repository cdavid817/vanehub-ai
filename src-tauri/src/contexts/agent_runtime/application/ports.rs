//! Outbound ports consumed by Agent Runtime application services.
//!
//! Traits in this module describe behavior required from repositories, provider processes,
//! terminals, operations, clocks, logging, and event publication. Infrastructure implements these
//! contracts; application services do not depend on Tauri, SQLite, or concrete CLI libraries.

use super::{
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentMemory, AgentMessage,
    AgentOperation, AgentRuntimeApplicationError, AgentSession, AgentTerminalEvent,
    AgentTerminalInputRequest, AgentTerminalProcessRequest, AgentTerminalSession,
    AgentToolCallOutcome, ApiProviderConfig, BoundSkillPrompt, CliProfileSnapshot,
    CompleteAgentMessage, DurableAgentGenerationMessages, DurableAgentGenerationStart,
    EffectivePrompt, GenerationCancellation, GenerationLease, GenerationProcessEvent,
    GenerationProcessRequest, LoopChildRecoveryProjection, LoopEvidenceView, LoopGitStateView,
    LoopIterationView, LoopLog, LoopOperationContext, LoopOwnedRecoverySession,
    LoopRoleGenerationTerminal, LoopRoleSessionRequest, LoopRunView,
    LoopVerificationProcessRequest, LoopVerificationProcessResult, MemorySource, NewAgentMessage,
    OnePieceDiscoveredModel, OnePieceModelDiscoveryRequest, PersonalizationSettings,
    RegisterApiAgentInput, ResizeAgentTerminalRequest, SaveLoopVerifierResultRequest,
    StartedGenerationProcess, StopAgentTerminalRequest, ToolApprovalDecision, ToolDefinition,
    ToolUseBlock, UpdateApiAgentInput, WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, AgentLifecycle, AgentWorkflow, AvailabilityAssessment, LoopDefinition,
    LoopRun, LoopRunStatus,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Persistence contract for loop definitions, snapshots, and lifecycle transitions.
pub(crate) trait LoopRepository: Send + Sync {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError>;
    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError>;
    fn create_definition(
        &self,
        definition: &LoopDefinition,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn update_definition(
        &self,
        definition: &LoopDefinition,
        expected_version: u64,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn delete_definition(&self, definition_id: &str) -> Result<(), AgentRuntimeApplicationError>;
    fn create_run(
        &self,
        run: &LoopRun,
        definition_snapshot: &LoopDefinition,
        project_path: &str,
        created_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn has_active_run(&self, definition_id: &str) -> Result<bool, AgentRuntimeApplicationError>;
    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError>;
    fn list_run_views(
        &self,
        _definition_id: Option<&str>,
    ) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop run projection is unavailable.".to_string(),
        ))
    }
    fn find_run_view(
        &self,
        _run_id: &str,
    ) -> Result<Option<LoopRunView>, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop run projection is unavailable.".to_string(),
        ))
    }
    fn recovery_owned_sessions(
        &self,
        _run_id: &str,
    ) -> Result<Vec<LoopOwnedRecoverySession>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }
    fn attach_run_operation(
        &self,
        run_id: &str,
        operation_id: &str,
        expected_status: LoopRunStatus,
        updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn attach_run_worktree(
        &self,
        run_id: &str,
        path: &str,
        name: &str,
        branch: &str,
        expected_status: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn save_run_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        updated_at: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn save_pause_request(
        &self,
        _run: &LoopRun,
        _expected_status: LoopRunStatus,
        _expected_pause_requested: bool,
        _updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop pause persistence is unavailable.".to_string(),
        ))
    }

    fn find_run_definition_snapshot(
        &self,
        _run_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop run snapshot lookup is unavailable.".to_string(),
        ))
    }

    fn save_continue_transition(
        &self,
        _run: &LoopRun,
        _expected_status: LoopRunStatus,
        _feedback: &str,
        _updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop continuation persistence is unavailable.".to_string(),
        ))
    }

    fn list_recoverable_runs(&self) -> Result<Vec<LoopRun>, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop recovery lookup is unavailable.".to_string(),
        ))
    }

    fn save_recovery_transition(
        &self,
        _run: &LoopRun,
        _expected_status: LoopRunStatus,
        _evidence: &LoopEvidenceView,
        _updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop recovery persistence is unavailable.".to_string(),
        ))
    }
}

pub(crate) trait LoopExecutionControlPort: Send + Sync {
    fn request_cancellation(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait LoopExecutionLeasePort: Send + Sync {
    fn has_live_lease(&self, run_id: &str) -> Result<bool, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopSessionRecoveryPort: Send + Sync {
    fn recovery_projection(
        &self,
        session_id: &str,
    ) -> Result<LoopChildRecoveryProjection, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopIterationRepository: Send + Sync {
    fn insert_iteration(
        &self,
        iteration: &LoopIterationView,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn attach_worker_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn attach_verifier_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn save_verifier_result(
        &self,
        request: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn save_worker_summary(
        &self,
        _run_id: &str,
        _iteration_id: &str,
        _session_id: &str,
        _summary: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop Worker summary persistence is unavailable.".to_string(),
        ))
    }
    fn complete_iteration(
        &self,
        _run_id: &str,
        _iteration_id: &str,
        _status: LoopRunStatus,
        _decision_reason: &str,
        _completed_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop iteration completion persistence is unavailable.".to_string(),
        ))
    }
    fn save_iteration_fingerprints(
        &self,
        run_id: &str,
        iteration_id: &str,
        diff_fingerprint: &str,
        check_failure_fingerprint: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn append_evidence(
        &self,
        evidence: &LoopEvidenceView,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait LoopProjectPort: Send + Sync {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError>;
    fn prepare_loop_worktree(
        &self,
        _project_path: &str,
        _name: &str,
        _base_branch: &str,
    ) -> Result<super::PreparedLoopWorktree, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Loop(
            "Loop worktree preparation is unavailable.".to_string(),
        ))
    }
}

pub(crate) trait LoopGitStatePort: Send + Sync {
    fn snapshot(&self, session_id: &str) -> Result<LoopGitStateView, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopVerifierContextPort: Send + Sync {
    fn bounded_diff(&self, session_id: &str) -> Result<String, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopRoleSessionPort: Send + Sync {
    fn create_worker_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError>;
    fn create_verifier_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopWorkerGenerationPort: Send + Sync {
    fn start_worker_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopVerifierGenerationPort: Send + Sync {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopGenerationControlPort: Send + Sync {
    fn stop_loop_generation(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

/// Read boundary for stable Agent registry entries.
pub(crate) trait AgentRegistryRepository: Send + Sync {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError>;

    fn find(&self, agent_id: &str)
        -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentAvailabilityGateway: Send + Sync {
    fn assess(
        &self,
        managed_sdk_dependency_id: Option<&str>,
        executable_name: Option<&str>,
    ) -> Result<AvailabilityAssessment, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentWorkflowRepository: Send + Sync {
    fn load(&self) -> Result<AgentWorkflow, AgentRuntimeApplicationError>;

    fn save(&self, workflow: &AgentWorkflow) -> Result<(), AgentRuntimeApplicationError>;

    fn load_details(
        &self,
    ) -> Result<(String, BTreeMap<String, String>), AgentRuntimeApplicationError>;

    fn save_details(
        &self,
        adapter: &str,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentSessionGateway: Send + Sync {
    fn find_session(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentSession>, AgentRuntimeApplicationError>;

    fn validate_configuration(
        &self,
        session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError>;

    /// Normalizes against the Agent the configuration names, for a seat running its own Agent
    /// rather than the session's mirrored one.
    fn validate_seat_configuration(
        &self,
        session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError>;

    fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        file_references: &[AgentFileReference],
    ) -> Result<String, AgentRuntimeApplicationError>;

    fn create_message(
        &self,
        message: NewAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError>;

    fn start_generation(
        &self,
        _request: DurableAgentGenerationStart,
    ) -> Result<DurableAgentGenerationMessages, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Session(
            "durable generation start is not implemented".to_string(),
        ))
    }

    fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError>;

    fn find_terminal_usage_message(
        &self,
        _session_id: &str,
        _agent_id: &str,
    ) -> Result<Option<String>, AgentRuntimeApplicationError> {
        Ok(None)
    }

    fn append_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn append_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn append_tool_use(
        &self,
        message_id: &str,
        tool_use: ToolUseBlock,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn append_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn complete_message(
        &self,
        message: CompleteAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError>;

    fn fail_message(
        &self,
        message_id: &str,
        session_id: &str,
        error: &str,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError>;

    fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError>;

    fn update_lifecycle(
        &self,
        session_id: &str,
        lifecycle: AgentLifecycle,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentCliProfileGateway: Send + Sync {
    fn load(
        &self,
        agent_id: &str,
        configuration: &AgentChatConfiguration,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError>;

    fn load_interactive(
        &self,
        agent_id: &str,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::CliProfile(format!(
            "interactive CLI profile loading is not implemented for {agent_id}."
        )))
    }
}

pub(crate) trait EffectivePromptGateway: Send + Sync {
    fn assemble(
        &self,
        agent_id: &str,
        session_id: &str,
        user_prompt: &str,
    ) -> Result<EffectivePrompt, AgentRuntimeApplicationError>;

    fn record_execution(
        &self,
        report: super::PromptExecutionReport,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let _ = report;
        Ok(())
    }
}

/// Process boundary for provider-specific Agent generation.
pub(crate) trait AgentProcessGateway: Send + Sync {
    fn launch_workflow(
        &self,
        request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError>;

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError>;

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: std::sync::Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn stop_generation(
        &self,
        process_id: &str,
        initiator: super::ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentProcessEventSink: Send + Sync {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentTerminalGateway: Send + Sync {
    fn attach_retained(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentTerminalSession>, AgentRuntimeApplicationError>;

    fn open_or_attach(
        &self,
        request: AgentTerminalProcessRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError>;

    fn input(&self, request: AgentTerminalInputRequest)
        -> Result<(), AgentRuntimeApplicationError>;

    fn resize(
        &self,
        request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn stop(&self, request: StopAgentTerminalRequest)
        -> Result<bool, AgentRuntimeApplicationError>;

    fn cleanup_idle(
        &self,
        idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError>;

    fn shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentTerminalEventPort: Send + Sync {
    fn publish_terminal(
        &self,
        event: AgentTerminalEvent,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

/// Observable-operation boundary used by Agent and loop execution.
pub(crate) trait AgentTaskPort: Send + Sync {
    fn start_agent_launch(
        &self,
        agent_id: &str,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError>;

    fn start_agent_generation(
        &self,
        agent_id: &str,
        session_id: &str,
        message_id: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError>;

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::Operation(format!(
            "Loop {} operation is unavailable for run {}: {message}",
            context.kind.as_str(),
            context.run_id
        )))
    }

    fn append_log(
        &self,
        operation_id: &str,
        line: String,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn correlate_execution(
        &self,
        _operation_id: &str,
        _run_id: &str,
        _trace_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    fn fail(&self, operation_id: &str, error: String) -> Result<(), AgentRuntimeApplicationError>;

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

/// Unified redacted diagnostic logging boundary for Agent execution.
pub(crate) trait AgentLoggingPort: Send + Sync {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait LoopLoggingPort: Send + Sync {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentClockPort: Send + Sync {
    fn now(&self) -> String;
}

/// The native tool-use loop's boundary to the `permissions` context's Policy Decision Point
/// (`add-permissions-core`). `Action`/`Effect`/`Resource` are `permissions::domain`'s own types,
/// referenced directly here as a deliberately published cross-context contract rather than
/// duplicated locally — replacing `risk_tier_for`/`requires_approval` as the approval authority.
pub(crate) trait AgentPermissionPort: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        agent_id: &str,
        action: crate::contexts::permissions::api::Action,
        resource: crate::contexts::permissions::api::Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> crate::contexts::permissions::api::Effect;

    /// Registers a pending approval after `evaluate` resolves `Ask`, before the caller blocks on
    /// `await_approval`. `call_id` correlates back to that same wait.
    #[allow(clippy::too_many_arguments)]
    fn create_pending_approval(
        &self,
        agent_id: &str,
        action: crate::contexts::permissions::api::Action,
        resource: crate::contexts::permissions::api::Resource,
        session_id: &str,
        generation_id: &str,
        call_id: &str,
        project_key: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentEventPort: Send + Sync {
    fn publish(&self, event: AgentEvent) -> Result<(), AgentRuntimeApplicationError>;
}

/// Carries a completed seat turn from the generation sink to the turn coordinator.
///
/// The sink does not route the next seat itself: it holds ports rather than the application
/// service, and starting a generation from inside a terminal handler would nest generations in each
/// other's lifecycles. This mirrors how the Loop runtime hands its terminals off.
pub(crate) trait SeatTurnCompletionPort: Send + Sync {
    /// Returns false when this turn was already delivered, so a retried terminal cannot start the
    /// next seat twice.
    fn deliver(
        &self,
        terminal: super::SeatTurnTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError>;

    /// Taken by the turn coordinator; the sink only delivers.
    fn take_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<super::SeatTurnTerminal>, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopRoleGenerationCompletionPort: Send + Sync {
    fn deliver(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError>;

    fn take_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentMessageTerminalCompletionPort: Send + Sync {
    fn register(
        &self,
        session_id: &str,
    ) -> Result<super::AgentMessageTerminalReceiver, AgentRuntimeApplicationError>;

    fn deliver(
        &self,
        terminal: super::AgentMessageTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError>;

    fn remove(&self, session_id: &str) -> Result<bool, AgentRuntimeApplicationError>;
}

pub(crate) trait LoopVerificationProcessPort: Send + Sync {
    fn execute(
        &self,
        request: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentGenerationPort: Send + Sync {
    fn reserve(&self, session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError>;

    fn correlate(
        &self,
        lease: &GenerationLease,
        execution_context: &crate::contexts::execution_observability::api::ExecutionContext,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn correlate_prompt(
        &self,
        lease: &GenerationLease,
        execution: &super::PendingPromptExecution,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let _ = (lease, execution);
        Ok(())
    }

    fn attach(
        &self,
        lease: &GenerationLease,
        message_id: &str,
        process_id: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;

    fn release(&self, lease: &GenerationLease) -> Result<(), AgentRuntimeApplicationError>;

    fn cancel(
        &self,
        session_id: &str,
    ) -> Result<Option<GenerationCancellation>, AgentRuntimeApplicationError>;

    fn complete(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    fn fail(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    /// Non-mutating lookup of the process id currently attached to `session_id`'s active
    /// generation, if any — used to resolve a tool approval without cancelling the generation
    /// the way `cancel` would.
    fn active_process_id(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, AgentRuntimeApplicationError>;

    fn active_correlation(
        &self,
        _session_id: &str,
    ) -> Result<Option<super::ActiveGenerationCorrelation>, AgentRuntimeApplicationError> {
        Ok(None)
    }
}

/// Read boundary for recent conversation turns, used by API-based generation to assemble
/// provider-native message history without a local process transcript.
pub(crate) trait ConversationHistoryPort: Send + Sync {
    fn recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<AgentMessage>, AgentRuntimeApplicationError>;
}

/// Persistence boundary for API-based agent registration and per-agent provider configuration.
pub(crate) trait ApiAgentGateway: Send + Sync {
    fn register(
        &self,
        agent_id: &str,
        input: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError>;

    fn provider_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError>;

    /// Updates `display_name`/`model_id`/`base_url` in place (`add-agent-lifecycle-management`).
    /// `input.new_api_key` is ignored here — credential rotation goes through
    /// `ApiCredentialPort` at the application layer, exactly like `register`'s own `api_key`
    /// field is only ever read by the service before calling `register`, never by this gateway.
    fn update(
        &self,
        agent_id: &str,
        input: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError>;

    /// Deletes the agent and its `agent_modes`/`agent_capability_tags`/
    /// `skill_api_agent_bindings` rows, or fails with `AgentRuntimeApplicationError::Validation`
    /// naming what still references it — never partially applied (`add-agent-lifecycle-management`
    /// design.md Decision 2).
    fn delete(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    fn onepiece_provider_config(
        &self,
    ) -> Result<super::StoredOnePieceProviderConfig, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }

    fn save_onepiece_provider_config(
        &self,
        input: &super::StoredOnePieceProviderConfig,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let _ = input;
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }

    fn reset_onepiece_provider_config(
        &self,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }

    fn list_onepiece_provider_profiles(
        &self,
    ) -> Result<Vec<super::StoredOnePieceProviderProfile>, AgentRuntimeApplicationError> {
        Ok(Vec::new())
    }

    fn save_onepiece_provider_profile(
        &self,
        _profile: &super::StoredOnePieceProviderProfile,
    ) -> Result<super::StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }

    fn activate_onepiece_provider_profile(
        &self,
        _profile_id: &str,
    ) -> Result<super::StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }

    fn delete_onepiece_provider_profile(
        &self,
        _profile_id: &str,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        Err(AgentRuntimeApplicationError::AgentNotFound(
            "onepiece".to_string(),
        ))
    }
}

/// Secret storage boundary for API-based agent provider credentials.
pub(crate) trait ApiCredentialPort: Send + Sync {
    fn store(&self, agent_id: &str, api_key: &str) -> Result<(), AgentRuntimeApplicationError>;

    fn fetch(&self, agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError>;

    fn remove(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait OnePieceModelDiscoveryPort: Send + Sync {
    fn list_models(
        &self,
        request: OnePieceModelDiscoveryRequest,
    ) -> Result<Vec<OnePieceDiscoveredModel>, AgentRuntimeApplicationError>;

    fn validate_credential(
        &self,
        request: super::ProviderCredentialProbeRequest,
    ) -> Result<super::ProviderCredentialValidationResult, AgentRuntimeApplicationError>;
}

/// Resolution boundary for a native-agent tool call paused awaiting user approval. Only
/// `RuntimeAgentApiAdapter` implements this — CLI agents run their own approval flow internally
/// and never register a pending approval here (design.md Decision 4).
pub(crate) trait ToolApprovalPort: Send + Sync {
    /// Delivers `decision` for the pending approval identified by `process_id`/`call_id`.
    /// Returns `false` if no such pending approval exists (already resolved, the generation
    /// ended, or it was never registered) rather than treating that as an error.
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError>;
}

/// Read boundary for Skill content bound to an API agent, injected as that agent's generation
/// requests' system prompt (`add-agent-skill-support`). Implemented in
/// `tooling::skills::infrastructure` — `agent_runtime` depends on this port rather than the
/// Skill registry directly, matching every other cross-context dependency in this module.
pub(crate) trait AgentSkillPort: Send + Sync {
    fn bound_skill_prompts(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError>;

    fn execute_read(&self, request: super::AgentSkillReadRequest) -> AgentToolCallOutcome {
        let _ = request;
        AgentToolCallOutcome {
            output: "Skill discovery is unavailable in this runtime.".to_string(),
            is_error: true,
        }
    }
}

pub(crate) trait AgentCoreInstructionsPort: Send + Sync {
    fn instructions_for(
        &self,
        agent_id: &str,
    ) -> Result<Option<super::AgentCoreInstructions>, AgentRuntimeApplicationError>;
}

/// Bridges the native tool-use loop to MCP-sourced tools (`add-agent-mcp-tools`), through
/// `agent_runtime`'s own port rather than `tooling::mcp`'s types directly — mirrors
/// `AgentSkillPort`/`RuntimeAgentSkillAdapter`'s existing pattern for depending on another
/// context's API. Both methods are sync — the implementing adapter is responsible for bridging
/// to `tooling::mcp`'s async `McpApi` internally (`tauri::async_runtime::block_on`), matching how
/// this port is consumed from the tool-execution loop's synchronous call chain.
pub(crate) trait AgentMcpToolPort: Send + Sync {
    /// Every MCP-sourced tool visible and active for `project_path`, already named and shaped as
    /// `ToolDefinition`s ready to merge into the fixed catalog. Returns `Err` only when the
    /// lookup itself fails (not when there are simply no MCP tools) — callers are expected to
    /// degrade gracefully (log and continue with the fixed catalog alone) rather than fail the
    /// generation, matching `resolve_system_prompt`'s existing treatment of `AgentSkillPort`.
    fn catalog_entries(
        &self,
        project_path: &str,
    ) -> Result<Vec<ToolDefinition>, AgentRuntimeApplicationError>;

    /// Invokes an MCP-sourced tool by its full prefixed name (e.g. `mcp__<server>__<tool>`).
    /// Infallible by design — every failure mode (server no longer visible/active, connection
    /// failure, tool-level error reported by the remote server) resolves to
    /// `AgentToolCallOutcome{is_error: true, ..}`, matching `execute_shell`/`execute_file`'s
    /// existing infallible-signature convention. The shared cancellation flag belongs to the
    /// active generation and must remain connected to the MCP session until cleanup completes.
    fn call_tool(
        &self,
        project_path: &str,
        tool_name: &str,
        arguments: &Value,
        cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> AgentToolCallOutcome;
}

/// Persistence boundary for cross-session agent memory (`add-agent-cross-session-memory`,
/// `add-cli-memory-support`). Unlike `AgentSkillPort`, `agent_runtime` owns this concept outright
/// — no other context reads or writes it — so this port has a single, directly-implementing
/// SQLite adapter rather than a cross-context wrapper.
///
/// `add-cli-memory-support` turned this into a single host-level pool shared by every agent
/// (OnePiece and all CLI-wrapped agents) — `agent_id`/`folder` on `save` remain provenance-only,
/// no longer a filter, which is why `list`/`list_all_for_agent`/`delete_all_for_agent` collapsed
/// into unscoped `list_all`/`delete_all`.
pub(crate) trait AgentMemoryPort: Send + Sync {
    fn save(
        &self,
        agent_id: &str,
        folder: Option<&str>,
        content: &str,
        source: MemorySource,
    ) -> Result<(), AgentRuntimeApplicationError>;

    /// Lists every memory in the shared pool, regardless of which agent or folder produced it.
    fn list_all(&self) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError>;

    fn delete(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    /// Deletes every memory in the shared pool in one action (`add-personalization-settings`
    /// design.md D6, scope widened to the whole pool by `add-cli-memory-support`) — used by the
    /// "reset memory" management action, distinct from `delete`'s single-row removal.
    fn delete_all(&self) -> Result<(), AgentRuntimeApplicationError>;
}

/// Independent, on-demand memory extraction for CLI-wrapped agents (`add-cli-memory-support`
/// design.md D3). Unlike OnePiece's `extract_memories`/`maybe_compact` (which reuse credentials
/// already resolved for an in-flight OnePiece generation), a call through this port resolves
/// OnePiece's credentials itself, since CLI-wrapped agents have no generation-scoped credential to
/// reuse. Implementations SHALL return `AgentRuntimeApplicationError::Credential` when no usable
/// OnePiece credential is configured (an expected, common condition, not a call failure) so the
/// caller can log it distinctly from a genuine extraction-call failure
/// (`AgentRuntimeApplicationError::Memory`).
pub(crate) trait AgentMemoryExtractionPort: Send + Sync {
    /// `exchange` is the turn's plain-text content (the user's message and the agent's final
    /// response) — not provider wire-format turns, since CLI-wrapped agents never produce those.
    /// Returns `Ok(None)` when the call succeeds but finds nothing worth remembering, mirroring
    /// `extract_memories`'s existing empty-result semantics. Does not persist anything itself —
    /// the caller decides how to split and save the result (mirroring `extract_memories`'s own
    /// one-memory-per-line convention) via `AgentMemoryPort`, using whatever `agent_id`/`folder`
    /// it already has in scope.
    fn extract(&self, exchange: &str) -> Result<Option<String>, AgentRuntimeApplicationError>;
}

/// Host-level personalization settings read from `desktop` at generation time
/// (`add-personalization-settings`). `agent_runtime` does not own this data — `desktop` does, the
/// same way `tooling::skills` owns Skill content — so this port exists purely to bridge across
/// that context boundary, mirroring `AgentSkillPort`'s existing cross-context pattern rather than
/// `AgentMemoryPort`'s directly-owned one.
pub(crate) trait AgentPersonalizationPort: Send + Sync {
    fn settings(&self) -> Result<PersonalizationSettings, AgentRuntimeApplicationError>;
}

/// Projected retrieval hit surfaced to the model through the `recall` tool result
/// (`add-onepiece-vector-search` Task 13) — deliberately not `retrieval::domain::ScoredHit`:
/// `source_id` and `score` are internal to that context and give the model no decision value, so
/// they must not cross the context boundary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentRetrievalHit {
    pub(crate) content: String,
    pub(crate) created_at: String,
    pub(crate) matched_via: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentRetrievalOutcome {
    pub(crate) hits: Vec<AgentRetrievalHit>,
    pub(crate) degraded: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentCodeRetrievalHit {
    pub(crate) file_path: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) language: String,
    pub(crate) symbol_name: Option<String>,
    pub(crate) symbol_kind: Option<String>,
    pub(crate) snippet: String,
    pub(crate) matched_via: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentCodeRetrievalOutcome {
    pub(crate) hits: Vec<AgentCodeRetrievalHit>,
    pub(crate) degraded: Option<String>,
}

pub(crate) trait AgentCodeRetrievalPort: Send + Sync {
    fn is_available(&self, workspace_folder: &str) -> bool;
    fn search_code(
        &self,
        workspace_folder: &str,
        query: &str,
        limit: usize,
    ) -> Result<AgentCodeRetrievalOutcome, String>;
}

/// Outbound port to the `retrieval` context's hybrid memory search, consumed by the `recall` tool
/// (`add-onepiece-vector-search` Task 13). Implemented in `bootstrap` over
/// `retrieval::api::RetrievalApi` — mirrors `AgentSkillPort`/`AgentMcpToolPort`'s existing pattern
/// of depending on another context only through this context's own port, never that context's
/// infrastructure.
pub(crate) trait AgentRetrievalPort: Send + Sync {
    /// Called on every generation's tool-catalog resolution path, so it must never block, panic,
    /// or return an error — an unreadable configuration is indistinguishable from "not
    /// configured", exactly like `RetrievalApi::is_configured`'s own contract.
    fn is_configured(&self) -> bool;

    /// No scope arguments: memories are a single host-level pool shared by every agent
    /// (`agent-memory-shared-pool`), which is the same pool the recency injection draws from, so
    /// there is no per-agent or per-folder slice for a caller to name.
    fn search(&self, query: &str, limit: usize) -> Result<AgentRetrievalOutcome, String>;

    /// Best-effort wake signal for the background indexing worker after a memory changes —
    /// called by `execute_remember` after a successful save (Task 14): no write, no wait, and
    /// failure is harmless — mirrors `RetrievalApi::wake_worker`'s own contract.
    fn notify_source_changed(&self);

    fn code_retrieval(&self) -> Option<&dyn AgentCodeRetrievalPort> {
        None
    }
}

/// Session-owned scope for semantic code queries. Tool payloads never construct this value, so a
/// model can select only a relative document and position, not another workspace or server.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeIntelligenceContext {
    session_workspace: String,
}

#[cfg_attr(not(test), allow(dead_code))]
impl AgentCodeIntelligenceContext {
    pub(crate) fn from_session_workspace(session_workspace: impl Into<String>) -> Self {
        Self {
            session_workspace: session_workspace.into(),
        }
    }

    pub(crate) fn session_workspace(&self) -> &str {
        &self.session_workspace
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDocumentPositionInput {
    pub(crate) relative_path: String,
    pub(crate) line: u32,
    pub(crate) column: u32,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDocumentInput {
    pub(crate) relative_path: String,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentCodeIntelligenceStatus {
    Ready,
    Warming,
    Timeout,
    Unavailable,
    Failed,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeRange {
    pub(crate) start_line: u32,
    pub(crate) start_column: u32,
    pub(crate) end_line: u32,
    pub(crate) end_column: u32,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeLocation {
    pub(crate) file: String,
    pub(crate) range: AgentCodeRange,
    pub(crate) preview: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeHover {
    pub(crate) signature: Option<String>,
    pub(crate) documentation: Option<String>,
    pub(crate) range: Option<AgentCodeRange>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeDiagnostic {
    pub(crate) file: String,
    pub(crate) range: AgentCodeRange,
    pub(crate) severity: Option<String>,
    pub(crate) message: String,
    pub(crate) source: Option<String>,
    pub(crate) code: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeIntelligenceMetadata {
    pub(crate) status: AgentCodeIntelligenceStatus,
    pub(crate) server: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) document_version: Option<u64>,
    pub(crate) stale: bool,
    pub(crate) returned_count: usize,
    pub(crate) total: usize,
    pub(crate) truncated: bool,
    pub(crate) filtered_count: usize,
    pub(crate) reason_code: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCodeIntelligenceOutcome<T> {
    pub(crate) metadata: AgentCodeIntelligenceMetadata,
    pub(crate) value: Option<T>,
}

/// Synchronous consumer contract for the native Agent loop. Implementations bridge to the async
/// code-intelligence runtime and must observe the generation cancellation flag.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait AgentCodeIntelligencePort: Send + Sync {
    fn is_available(&self, context: &AgentCodeIntelligenceContext) -> bool;

    fn find_definition(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>>;

    fn find_references(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>>;

    fn get_hover(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Option<AgentCodeHover>>;

    fn get_diagnostics(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeDiagnostic>>;
}

pub(crate) struct AgentCodeIntelligencePending<T> {
    pub(crate) response: std::sync::mpsc::Receiver<AgentCodeIntelligenceOutcome<T>>,
    pub(crate) cancel: Arc<dyn Fn() + Send + Sync>,
}

/// Producer-side boundary used by the synchronous Agent adapter. Implementations enqueue work on
/// the asynchronous code-intelligence runtime and return immediately with a one-shot responder.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait AgentCodeIntelligenceResponderPort: Send + Sync {
    fn is_available(&self, context: &AgentCodeIntelligenceContext) -> bool;

    fn start_find_definition(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>>;

    fn start_find_references(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>>;

    fn start_get_hover(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Option<AgentCodeHover>>;

    fn start_get_diagnostics(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeDiagnostic>>;
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentWorkspaceMutation {
    pub(crate) canonical_workspace: PathBuf,
    pub(crate) relative_path: String,
}

/// Best-effort, non-blocking signal emitted only after a workspace mutation succeeds.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait AgentWorkspaceMutationPort: Send + Sync {
    fn publish(&self, mutation: AgentWorkspaceMutation);
}
