use super::{
    AgentChatConfiguration, AgentEvent, AgentFileReference, AgentLog, AgentMessage, AgentOperation,
    AgentRuntimeApplicationError, AgentSession, CliProfileSnapshot, CompleteAgentMessage,
    EffectivePrompt, GenerationCancellation, GenerationLease, GenerationProcessEvent,
    GenerationProcessRequest, NewAgentMessage, StartedGenerationProcess, ToolUseBlock,
    WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, AgentLifecycle, AgentWorkflow, AvailabilityAssessment,
};
use serde_json::Value;
use std::collections::BTreeMap;

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

pub(crate) trait AgentClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait AgentEventPort: Send + Sync {
    fn publish(&self, event: AgentEvent) -> Result<(), AgentRuntimeApplicationError>;
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
