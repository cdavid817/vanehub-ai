use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRuntimeApplicationError,
};
use crate::contexts::agent_runtime::domain::{
    AvailabilityAssessment, AvailabilityProbe, ExecutableStatus, ManagedSdkStatus,
};
use crate::contexts::tooling::sdk::api::{SdkApi, SdkId};
use crate::platform::process;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct RuntimeAgentAvailabilityAdapter {
    sdk_api: SdkApi,
}

impl RuntimeAgentAvailabilityAdapter {
    pub(crate) fn new(sdk_api: SdkApi) -> Self {
        Self { sdk_api }
    }
}

impl AgentAvailabilityGateway for RuntimeAgentAvailabilityAdapter {
    fn assess(
        &self,
        managed_sdk_dependency_id: Option<&str>,
        executable_name: Option<&str>,
    ) -> Result<AvailabilityAssessment, AgentRuntimeApplicationError> {
        let managed_sdk = match managed_sdk_dependency_id {
            Some(id) => match SdkId::parse(id) {
                Some(sdk_id) if self.sdk_api.is_installed(sdk_id).unwrap_or(false) => {
                    ManagedSdkStatus::Available
                }
                Some(_) => ManagedSdkStatus::Missing(id.to_string()),
                None => ManagedSdkStatus::Unrecognized(id.to_string()),
            },
            None => ManagedSdkStatus::NotRequired,
        };
        let executable = match executable_name {
            Some(name) if process::command_exists(name, Duration::from_secs(2)) => {
                ExecutableStatus::Available
            }
            Some(name) => ExecutableStatus::Missing(name.to_string()),
            None => ExecutableStatus::NotDeclared,
        };
        Ok(AvailabilityAssessment::assess(AvailabilityProbe {
            managed_sdk,
            executable,
        }))
    }
}
