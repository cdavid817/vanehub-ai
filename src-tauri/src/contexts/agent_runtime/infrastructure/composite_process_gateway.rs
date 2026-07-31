use crate::contexts::agent_runtime::application::{
    AgentProcessEventSink, AgentProcessGateway, AgentRuntimeApplicationError,
    GenerationProcessRequest, ProcessStopInitiator, StartedGenerationProcess,
    WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use std::sync::Arc;

const API_PROCESS_PREFIX: &str = "agent-api-process-";

/// Routes `AgentProcessGateway` calls to the CLI subprocess adapter or the direct-API adapter
/// based on the target agent's `launch_kind`. Both adapters implement this one port, so session
/// creation, multi-agent coordination, and the application service need no per-launch-kind
/// branching of their own — this is the only place that knows two implementations exist.
#[derive(Clone)]
pub(crate) struct CompositeAgentProcessGateway {
    cli: Arc<dyn AgentProcessGateway>,
    api: Arc<dyn AgentProcessGateway>,
}

impl CompositeAgentProcessGateway {
    pub(crate) fn new(
        cli: Arc<dyn AgentProcessGateway>,
        api: Arc<dyn AgentProcessGateway>,
    ) -> Self {
        Self { cli, api }
    }

    fn for_process(&self, process_id: &str) -> &Arc<dyn AgentProcessGateway> {
        if process_id.starts_with(API_PROCESS_PREFIX) {
            &self.api
        } else {
            &self.cli
        }
    }
}

impl AgentProcessGateway for CompositeAgentProcessGateway {
    fn launch_workflow(
        &self,
        request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        if request.agent.launch.kind == "api" {
            self.api.launch_workflow(request)
        } else {
            self.cli.launch_workflow(request)
        }
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        if request.agent.launch.kind == "api" {
            self.api.start_generation(request)
        } else {
            self.cli.start_generation(request)
        }
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.for_process(process_id)
            .monitor_generation(process_id, sink)
    }

    fn stop_generation(
        &self,
        process_id: &str,
        initiator: ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.for_process(process_id)
            .stop_generation(process_id, initiator)
    }
}
