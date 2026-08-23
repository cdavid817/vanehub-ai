#[path = "api_environment.rs"]
mod environment;

pub(crate) use crate::contexts::tooling::cli::domain::compare_versions;
pub(crate) use environment::{CliEnvironmentApi, CliEnvironmentError};

use crate::contexts::tooling::cli::application::environment_launch::CliLaunchTarget;

/// Launch resolution for everything outside this context.
///
/// Deliberately narrow. The Agent Runtime and the delegation tools need one thing from the CLI
/// context -- which executable to start -- and handing them `CliEnvironmentApi` would also hand
/// them the ability to prepare and execute machine changes, which is not theirs to do.
///
/// It reads the same snapshot the CLI Management page renders. Before this it read the pre-change
/// `cli_tool_status` row instead, so a host with two installations could show one on the page and
/// launch the other.
#[derive(Clone)]
pub(crate) struct CliApi {
    environment: CliEnvironmentApi,
}

impl CliApi {
    pub(crate) fn new(environment: CliEnvironmentApi) -> Self {
        Self { environment }
    }

    /// The absolute executable to launch, or `None` when the environment declines to name one.
    ///
    /// `None` covers two different situations on purpose -- nothing scanned yet and found nothing,
    /// or scanned and refused -- because the caller's move is the same either way: report the tool
    /// as unavailable rather than guess. What it must never do is fall back to a bare command
    /// name, which would re-enter PATH resolution inside the child process.
    pub(crate) fn resolve_executable(
        &self,
        agent_id: &str,
    ) -> Result<Option<String>, CliEnvironmentError> {
        Ok(match self.environment.resolve_launch_target(agent_id)? {
            CliLaunchTarget::Resolved(path) => Some(path),
            CliLaunchTarget::Refused | CliLaunchTarget::NotScanned => None,
        })
    }
}
