use crate::contexts::tooling::cli::api::{compare_versions, CliApi};
use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::ports::CliInstallationSnapshotPort;
use crate::contexts::tooling::cli_parameters::domain::compatibility::{
    CliInstallationSnapshot, CliVersionComparator,
};
use std::cmp::Ordering;

/// Reads the CLI context's cached environment snapshot. It never triggers detection, so rendering
/// the settings page or previewing a draft cannot start an executable.
///
/// It asks for five facts rather than the snapshot. The path it gets back is the one the runtime
/// would launch, so a parameter is judged against the binary that will actually receive it -- which
/// is the whole point of asking the CLI context instead of running a second detector here.
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
        let facts = self
            .cli
            .installation_facts(agent_id)
            .map_err(|error| CliParameterApplicationError::Repository(error.to_string()))?;
        Ok(CliInstallationSnapshot {
            installed: facts.installed,
            runnable: facts.runnable,
            active_path: facts.active_path,
            version: facts.version,
            conflict: facts.conflict,
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
