use super::providers::apply_configuration_overrides;
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentCliProfileGateway, AgentRuntimeApplicationError,
    CliProfileSnapshot,
};
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::{CliParameterLaunchScope, CliParametersApi};

#[derive(Clone)]
pub(crate) struct RuntimeAgentCliProfileAdapter {
    parameters: CliParametersApi,
    cli: CliApi,
}

impl RuntimeAgentCliProfileAdapter {
    pub(crate) fn new(parameters: CliParametersApi, cli: CliApi) -> Self {
        Self { parameters, cli }
    }
}

impl AgentCliProfileGateway for RuntimeAgentCliProfileAdapter {
    fn load(
        &self,
        agent_id: &str,
        configuration: &AgentChatConfiguration,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let selections = self
            .parameters
            .load_selections(agent_id)
            .map_err(cli_profile_error)?;
        let selections = apply_configuration_overrides(agent_id, selections, configuration);
        let selections = self
            .parameters
            .normalize_selections(agent_id, &selections)
            .map_err(cli_profile_error)?;
        let managed_args = self
            .parameters
            .preview_args(agent_id, &selections, CliParameterLaunchScope::Chat)
            .map_err(cli_profile_error)?;
        let executable = self
            .cli
            .resolve_executable(agent_id)
            .map_err(cli_profile_error)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::CliProfile(format!(
                    "Agent executable could not be resolved for {agent_id}."
                ))
            })?;
        Ok(CliProfileSnapshot {
            executable,
            selections,
            managed_args,
        })
    }

    fn load_interactive(
        &self,
        agent_id: &str,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let selections = self
            .parameters
            .load_selections(agent_id)
            .map_err(cli_profile_error)?;
        let selections = self
            .parameters
            .normalize_selections(agent_id, &selections)
            .map_err(cli_profile_error)?;
        let managed_args = self
            .parameters
            .preview_args(agent_id, &selections, CliParameterLaunchScope::Interactive)
            .map_err(cli_profile_error)?;
        let executable = self
            .cli
            .resolve_executable(agent_id)
            .map_err(cli_profile_error)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::CliProfile(format!(
                    "Agent executable could not be resolved for {agent_id}."
                ))
            })?;
        Ok(CliProfileSnapshot {
            executable,
            selections,
            managed_args,
        })
    }
}

fn cli_profile_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::CliProfile(error.to_string())
}
