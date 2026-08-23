use crate::contexts::tooling::cli::api::{compare_versions, CliApi};
use crate::contexts::tooling::cli::domain::ConflictState;
use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::ports::CliInstallationSnapshotPort;
use crate::contexts::tooling::cli_parameters::domain::compatibility::{
    CliInstallationSnapshot, CliVersionComparator,
};
use std::cmp::Ordering;

/// Reads the CLI lifecycle subdomain's cached detection state. It never triggers detection, so
/// rendering the settings page or previewing a draft cannot start an executable.
#[derive(Clone)]
pub(crate) struct CliLifecycleSnapshotAdapter {
    cli: CliApi,
}

impl CliLifecycleSnapshotAdapter {
    pub(crate) fn new(cli: CliApi) -> Self {
        Self { cli }
    }
}

impl CliInstallationSnapshotPort for CliLifecycleSnapshotAdapter {
    fn active_installation(
        &self,
        agent_id: &str,
    ) -> Result<CliInstallationSnapshot, CliParameterApplicationError> {
        let tools = self
            .cli
            .list_tools()
            .map_err(|error| CliParameterApplicationError::Repository(error.to_string()))?;
        let Some(status) = tools.into_iter().find(|tool| tool.agent_id == agent_id) else {
            return Ok(CliInstallationSnapshot::default());
        };
        let active = status
            .installations
            .iter()
            .find(|installation| installation.is_active);
        let installed = status.installed.unwrap_or(false);
        Ok(CliInstallationSnapshot {
            installed,
            runnable: active
                .map(|installation| installation.runnable)
                .unwrap_or(installed),
            active_path: status
                .active_installation_path
                .clone()
                .or_else(|| status.detected_path.clone()),
            version: active
                .and_then(|installation| installation.version.clone())
                .or(status.current_version),
            conflict: status.conflict_state != ConflictState::None,
        })
    }
}

/// Version ordering is owned by the CLI lifecycle subdomain. Reusing it here keeps one comparator
/// for detection and for parameter compatibility.
#[derive(Clone, Default)]
pub(crate) struct LifecycleVersionComparator;

impl CliVersionComparator for LifecycleVersionComparator {
    fn compare(&self, left: &str, right: &str) -> Option<Ordering> {
        compare_versions(left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_comparator_delegates_to_the_lifecycle_ordering() {
        let comparator = LifecycleVersionComparator;
        assert_eq!(
            comparator.compare("2.1.181", "2.1.181"),
            Some(Ordering::Equal)
        );
        assert_eq!(
            comparator.compare("2.1.182", "2.1.181"),
            Some(Ordering::Greater)
        );
        assert_eq!(
            comparator.compare("2.1.180", "2.1.181"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn an_unparseable_version_is_reported_as_unknown_rather_than_guessed() {
        let comparator = LifecycleVersionComparator;
        assert_eq!(comparator.compare("not-a-version", "2.1.181"), None);
    }
}
