use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, RunnerCapabilities, RunnerDescriptor, RunnerDiscoveryPort,
    RunnerRecoveryMode, RunnerSelection,
};
use crate::contexts::sessions::api::{optional_session_metadata, SessionsApi};
use crate::contexts::ssh_connections::api::SshConnectionsApi;

#[derive(Clone)]
pub(crate) struct NativeRunnerDiscovery {
    sessions: SessionsApi,
    ssh: SshConnectionsApi,
}

impl NativeRunnerDiscovery {
    pub(crate) fn new(sessions: SessionsApi, ssh: SshConnectionsApi) -> Self {
        Self { sessions, ssh }
    }
}

impl RunnerDiscoveryPort for NativeRunnerDiscovery {
    fn list(
        &self,
        session_id: &str,
        _agent_id: &str,
    ) -> Result<Vec<RunnerDescriptor>, AgentRuntimeApplicationError> {
        let mut runners = builtin_runners();
        // Optional SSH metadata must not erase the independently usable Local descriptor.
        if let Some(target) =
            optional_session_metadata(self.sessions.current_runner_target(session_id))
                .map_err(|_| safe_discovery_error())?
        {
            let profile = self.ssh.execution_profile(&target.connection_id).ok();
            let available = profile.as_ref().is_some_and(|profile| {
                profile.revision == target.connection_revision
                    && profile.host == target.host
                    && profile.port == target.port
                    && profile.user == target.user
                    && profile.host_trusted
                    && profile.credential_configured
            });
            runners.push(descriptor(
                RunnerSelection::ssh(target.connection_id, target.connection_revision)
                    .map_err(|_| safe_discovery_error())?,
                &target.display_name,
                Some(target.host),
                available,
                (!available).then_some("ssh_authority_unavailable"),
                ssh_capabilities(),
            ));
        }
        for runner in &runners {
            runner
                .validate()
                .map_err(|error| AgentRuntimeApplicationError::Process(error.code().to_string()))?;
        }
        Ok(runners)
    }
}

fn builtin_runners() -> Vec<RunnerDescriptor> {
    vec![descriptor(
        RunnerSelection::local(),
        "Local",
        Some("This device".to_string()),
        true,
        None,
        local_capabilities(),
    )]
}

fn descriptor(
    selection: RunnerSelection,
    label: &str,
    host_label: Option<String>,
    available: bool,
    unavailable_reason: Option<&'static str>,
    capabilities: RunnerCapabilities,
) -> RunnerDescriptor {
    RunnerDescriptor {
        selection,
        label: label.to_string(),
        host_label,
        available,
        unavailable_reason,
        simulated: false,
        capabilities,
    }
}

fn local_capabilities() -> RunnerCapabilities {
    RunnerCapabilities {
        interactive_input: true,
        pty: false,
        cancellation: true,
        inspection: true,
        recovery: RunnerRecoveryMode::None,
    }
}

fn ssh_capabilities() -> RunnerCapabilities {
    RunnerCapabilities {
        interactive_input: true,
        pty: true,
        cancellation: true,
        inspection: true,
        recovery: RunnerRecoveryMode::InspectOnly,
    }
}

fn safe_discovery_error() -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Process("Runner discovery is unavailable.".to_string())
}

#[cfg(test)]
mod tests {
    use super::builtin_runners;

    #[test]
    fn builtin_catalog_contains_only_valid_available_runners() {
        let runners = builtin_runners();

        assert_eq!(runners.len(), 1);
        assert!(runners[0].available);
        for runner in runners {
            runner.validate().expect("built-in Runner is valid");
        }
    }
}
