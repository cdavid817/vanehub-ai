use super::application::{AgentRuntimeApplicationService, AgentTerminalApplicationService};

pub(crate) use super::application::{
    AgentChatConfiguration, AgentFileReference, AgentMessage, AgentRuntimeApplicationError,
    AgentSessionDetails, AgentTerminalInputRequest, AgentTerminalSession, AgentTerminalSize,
    AgentView, LaunchWorkflowResult, OpenAgentTerminalRequest, ReadinessView,
    ResizeAgentTerminalRequest, SendMessageRequest, StopAgentTerminalRequest, StopGenerationResult,
    WorkflowView,
};
#[cfg(test)]
pub(crate) use super::application::{AgentLaunchView, MessageTokenUsage};
pub(crate) use super::domain::{AgentAvailability, AgentLifecycle, InteractionMode};

#[derive(Clone)]
pub(crate) struct AgentRuntimeApi {
    service: AgentRuntimeApplicationService,
    terminal_service: AgentTerminalApplicationService,
}

impl AgentRuntimeApi {
    pub(crate) fn new(
        service: AgentRuntimeApplicationService,
        terminal_service: AgentTerminalApplicationService,
    ) -> Self {
        Self {
            service,
            terminal_service,
        }
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
        self.service.send_message(request)
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
