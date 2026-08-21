use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRuntimeApplicationError,
};
use crate::contexts::agent_runtime::domain::{
    AvailabilityAssessment, AvailabilityProbe, ExecutableStatus, ManagedSdkStatus,
};
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::sdk::api::{SdkApi, SdkId};

#[derive(Clone)]
pub(crate) struct RuntimeAgentAvailabilityAdapter {
    sdk_api: SdkApi,
    cli_api: CliApi,
}

impl RuntimeAgentAvailabilityAdapter {
    pub(crate) fn new(sdk_api: SdkApi, cli_api: CliApi) -> Self {
        Self { sdk_api, cli_api }
    }
}

impl AgentAvailabilityGateway for RuntimeAgentAvailabilityAdapter {
    fn assess(
        &self,
        agent_id: &str,
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
        // Resolved the way the launch path resolves it, not by asking `which` for the bare name.
        // CLI detection searches the directories the vendor installers actually write to
        // (tooling/cli/infrastructure/candidates.rs), and `cli_profile.rs` already launches from
        // that resolved path. Availability asking `PATH` instead is what reported an Agent
        // VaneHub had just installed itself -- opencode, into `~/.opencode/bin` -- as
        // "Command 'opencode' was not found on PATH", leaving it visible and installed on the CLI
        // Management page and unusable everywhere else.
        //
        // Falling back to the bare name keeps an Agent that is genuinely on `PATH` but unknown to
        // the CLI catalogue working, which is how anything not managed by that catalogue resolves.
        let executable = match executable_name {
            Some(name) => {
                let resolved = self
                    .cli_api
                    .resolve_executable(agent_id)
                    .ok()
                    .flatten()
                    .is_some();
                if resolved {
                    ExecutableStatus::Available
                } else {
                    ExecutableStatus::Missing(name.to_string())
                }
            }
            None => ExecutableStatus::NotDeclared,
        };
        Ok(AvailabilityAssessment::assess(AvailabilityProbe {
            managed_sdk,
            executable,
        }))
    }
}
