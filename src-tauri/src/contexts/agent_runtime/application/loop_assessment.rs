//! Gathers the witnesses `assess_execution` needs from the registry, the API provider
//! configuration, the reviewed CLI catalog and the host, so readiness, authoritative start,
//! preparation, resume and continuation all judge the same facts the same way.

use super::{
    assess_execution, AgentClockPort, AgentRegistryRepository, AgentRuntimeApplicationError,
    ApiAgentGateway, LoopAssessmentInput, LoopCliCapabilityPort, LoopExecutionAssessment,
    LoopRoleAgentFacts, LoopScopePlatformPort,
};
use crate::contexts::agent_runtime::domain::{InteractionMode, LoopDefinition};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct LoopAssessmentService {
    registry: Arc<dyn AgentRegistryRepository>,
    api_agents: Arc<dyn ApiAgentGateway>,
    cli: Arc<dyn LoopCliCapabilityPort>,
    platform: Arc<dyn LoopScopePlatformPort>,
    clock: Arc<dyn AgentClockPort>,
}

impl LoopAssessmentService {
    pub(crate) fn new(
        registry: Arc<dyn AgentRegistryRepository>,
        api_agents: Arc<dyn ApiAgentGateway>,
        cli: Arc<dyn LoopCliCapabilityPort>,
        platform: Arc<dyn LoopScopePlatformPort>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self {
            registry,
            api_agents,
            cli,
            platform,
            clock,
        }
    }

    pub(crate) fn platform(&self) -> &Arc<dyn LoopScopePlatformPort> {
        &self.platform
    }

    pub(crate) fn assess(
        &self,
        definition: &LoopDefinition,
    ) -> Result<LoopExecutionAssessment, AgentRuntimeApplicationError> {
        let values = definition.values();
        Ok(assess_execution(LoopAssessmentInput {
            definition,
            worker: self.role_facts(&values.worker_agent_id)?,
            verifier: self.role_facts(&values.verifier_agent_id)?,
            platform: self.platform.witness(),
            assessed_at: self.clock.now(),
        }))
    }

    fn role_facts(
        &self,
        agent_id: &str,
    ) -> Result<Option<LoopRoleAgentFacts>, AgentRuntimeApplicationError> {
        let Some(agent) = self.registry.find(agent_id)? else {
            return Ok(None);
        };
        let trusted = if agent.supports(InteractionMode::Cli) {
            false
        } else {
            self.api_agents
                .provider_config(agent_id)?
                .map(|config| config.auto_approve_tools)
                .unwrap_or(false)
        };
        let cli = if agent.supports(InteractionMode::Cli) {
            self.cli.cli_witness(agent_id)
        } else {
            None
        };
        Ok(Some(LoopRoleAgentFacts::from_definition(
            &agent, trusted, cli,
        )))
    }
}
