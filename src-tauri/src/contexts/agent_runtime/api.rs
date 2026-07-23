use super::application::{
    AgentRuntimeApplicationService, AgentTerminalApplicationService, LoopApplicationService,
    LoopControlApplicationService, LoopRecoveryApplicationService,
};
use super::infrastructure::NativeLoopScheduler;

pub(crate) use super::application::{
    AgentChatConfiguration, AgentFileReference, AgentMessage, AgentRuntimeApplicationError,
    AgentSessionDetails, AgentTerminalInputRequest, AgentTerminalSession, AgentTerminalSize,
    AgentView, ContinueLoopRequest, LaunchWorkflowResult, LoopDefinitionView, LoopRunView,
    OpenAgentTerminalRequest, ReadinessView, ResizeAgentTerminalRequest, SaveLoopDefinitionRequest,
    SendMessageRequest, StartLoopResultView, StopAgentTerminalRequest, StopGenerationResult,
    WorkflowView,
};
#[cfg(test)]
pub(crate) use super::application::{AgentLaunchView, MessageTokenUsage};
pub(crate) use super::domain::{
    AgentAvailability, AgentLifecycle, InteractionMode, LoopLimits, LoopVerificationCommand,
};

#[derive(Clone)]
pub(crate) struct AgentRuntimeApi {
    service: AgentRuntimeApplicationService,
    terminal_service: AgentTerminalApplicationService,
    loops: LoopApplicationService,
    loop_controls: LoopControlApplicationService,
    loop_recovery: LoopRecoveryApplicationService,
    loop_scheduler: NativeLoopScheduler,
}

impl AgentRuntimeApi {
    pub(crate) fn new(
        service: AgentRuntimeApplicationService,
        terminal_service: AgentTerminalApplicationService,
        loops: LoopApplicationService,
        loop_controls: LoopControlApplicationService,
        loop_recovery: LoopRecoveryApplicationService,
        loop_scheduler: NativeLoopScheduler,
    ) -> Self {
        Self {
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
        }
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
