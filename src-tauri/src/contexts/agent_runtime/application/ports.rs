use super::{
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentMessage, AgentOperation,
    AgentRuntimeApplicationError, AgentSession, AgentTerminalEvent, AgentTerminalInputRequest,
    AgentTerminalProcessRequest, AgentTerminalSession, CliProfileSnapshot, CompleteAgentMessage,
    EffectivePrompt, GenerationCancellation, GenerationLease, GenerationProcessEvent,
    GenerationProcessRequest, LoopEvidenceView, LoopGitStateView, LoopIterationView, LoopLog,
    LoopOperationContext, LoopRoleGenerationTerminal, LoopRoleSessionRequest, LoopRunView,
    LoopVerificationProcessRequest, LoopVerificationProcessResult, NewAgentMessage,
    ResizeAgentTerminalRequest, SaveLoopVerifierResultRequest, StartedGenerationProcess,
    StopAgentTerminalRequest, ToolUseBlock, WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, AgentLifecycle, AgentWorkflow, AvailabilityAssessment, LoopDefinition,
    LoopRun, LoopRunStatus,
};
use serde_json::Value;
use std::collections::BTreeMap;

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

    fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError>;

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
}

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

    fn stop_generation(&self, process_id: &str) -> Result<bool, AgentRuntimeApplicationError>;
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

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;

    fn fail(&self, operation_id: &str, error: String) -> Result<(), AgentRuntimeApplicationError>;

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentLoggingPort: Send + Sync {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait LoopLoggingPort: Send + Sync {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait AgentClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait AgentEventPort: Send + Sync {
    fn publish(&self, event: AgentEvent) -> Result<(), AgentRuntimeApplicationError>;
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

pub(crate) trait LoopVerificationProcessPort: Send + Sync {
    fn execute(
        &self,
        request: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError>;
}

pub(crate) trait AgentGenerationPort: Send + Sync {
    fn reserve(&self, session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError>;

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
}
