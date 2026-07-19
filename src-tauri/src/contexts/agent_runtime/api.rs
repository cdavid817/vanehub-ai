use super::application::AgentRuntimeApplicationService;

pub(crate) use super::application::{
    AgentChatConfiguration, AgentFileReference, AgentMessage, AgentRuntimeApplicationError,
    AgentSessionDetails, AgentView, LaunchWorkflowResult, ReadinessView, SendMessageRequest,
    StopGenerationResult, WorkflowView,
};
#[cfg(test)]
pub(crate) use super::application::{AgentLaunchView, MessageTokenUsage};
pub(crate) use super::domain::{AgentAvailability, AgentLifecycle, InteractionMode};

#[derive(Clone)]
pub(crate) struct AgentRuntimeApi {
    service: AgentRuntimeApplicationService,
}

impl AgentRuntimeApi {
    pub(crate) fn new(service: AgentRuntimeApplicationService) -> Self {
        Self { service }
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
}
