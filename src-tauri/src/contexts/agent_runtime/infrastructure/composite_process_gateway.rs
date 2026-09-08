use super::providers::acp::ACP_PROCESS_PREFIX;
use crate::contexts::agent_runtime::application::{
    AgentProcessEventSink, AgentProcessGateway, AgentRuntimeApplicationError,
    GenerationProcessRequest, ProcessStopInitiator, ProviderRegistry, StartedGenerationProcess,
    ToolApprovalDecision, ToolApprovalPort, WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::ProviderTransport;
use std::sync::Arc;

const API_PROCESS_PREFIX: &str = "agent-api-process-";

/// Routes `AgentProcessGateway` calls to the CLI subprocess adapter, the ACP stdio adapter, or
/// the direct-API adapter. Launch kind separates API from CLI; the provider's declared managed
/// transport separates ACP from headless. All three implement this one port, so session creation
/// and the application service need no per-transport branching of their own -- this is the only
/// place that knows three implementations exist.
#[derive(Clone)]
pub(crate) struct CompositeAgentProcessGateway {
    cli: Arc<dyn AgentProcessGateway>,
    acp: Arc<dyn AgentProcessGateway>,
    api: Arc<dyn AgentProcessGateway>,
    providers: Arc<ProviderRegistry>,
}

impl CompositeAgentProcessGateway {
    pub(crate) fn new(
        cli: Arc<dyn AgentProcessGateway>,
        acp: Arc<dyn AgentProcessGateway>,
        api: Arc<dyn AgentProcessGateway>,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            cli,
            acp,
            api,
            providers,
        }
    }

    fn for_process(&self, process_id: &str) -> &Arc<dyn AgentProcessGateway> {
        if process_id.starts_with(API_PROCESS_PREFIX) {
            &self.api
        } else if process_id.starts_with(ACP_PROCESS_PREFIX) {
            &self.acp
        } else {
            &self.cli
        }
    }

    fn uses_acp(&self, agent_id: &str) -> bool {
        self.providers
            .get(agent_id)
            .map(|provider| {
                provider.capabilities().managed_transport() == Some(ProviderTransport::AcpStdio)
            })
            .unwrap_or(false)
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
        } else if self.uses_acp(&request.agent.id) {
            self.acp.start_generation(request)
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

/// Delivers an approval or answer to whichever adapter owns the process. The native tool loop
/// and the ACP bridge both wait on the same `call_id` channel, so the caller never has to know
/// which one raised the request.
pub(crate) struct CompositeToolApprovalPort {
    api: Arc<dyn ToolApprovalPort>,
    acp: Arc<dyn ToolApprovalPort>,
}

impl CompositeToolApprovalPort {
    pub(crate) fn new(api: Arc<dyn ToolApprovalPort>, acp: Arc<dyn ToolApprovalPort>) -> Self {
        Self { api, acp }
    }
}

impl ToolApprovalPort for CompositeToolApprovalPort {
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        if process_id.starts_with(ACP_PROCESS_PREFIX) {
            self.acp.resolve(process_id, call_id, decision)
        } else {
            self.api.resolve(process_id, call_id, decision)
        }
    }
}
