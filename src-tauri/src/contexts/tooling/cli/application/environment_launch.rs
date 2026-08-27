//! Which executable the Agent Runtime launches for a tool.
//!
//! One authority. Before this, the runtime resolved a launch from the pre-change `cli_tool_status`
//! row while the CLI Management page reported the source-aware snapshot, so a machine with two
//! installations could show one on screen and run the other -- which is the PATH-precedence
//! problem the conflict contract exists to surface, reintroduced by the launch path itself.
//!
//! The answer is always an absolute path or nothing. A bare command name would re-enter PATH
//! resolution inside the child process, and PATH is exactly what is in dispute.

use super::environment_error::CliEnvironmentError;
use super::environment_ports::{CliCancellation, CliProbeBudget};
use super::environment_service::CliEnvironmentService;
use crate::contexts::tooling::cli::domain::installation::{
    deduplicate, group_launcher_families, CliInstallation,
};
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::contexts::tooling::cli::domain::status::CliDiscoveryStatus;

/// What another bounded context may know about one tool's installation.
///
/// Five facts, not a snapshot. The parameter context needs them to decide whether a flag is
/// supported by the installed version; handing it the snapshot would also hand it the plans, the
/// catalogs, and the actions, none of which are its business.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CliInstallationFacts {
    pub(crate) installed: bool,
    /// Whether the installation the runtime would launch can actually run.
    pub(crate) runnable: bool,
    /// The absolute path a launch would use, on the same rules as `resolve_launch_target`.
    pub(crate) active_path: Option<String>,
    pub(crate) version: Option<String>,
    /// Whether anything is reported about this tool's installation layout.
    pub(crate) conflict: bool,
}

/// What the environment can say about launching one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliLaunchTarget {
    /// The absolute path the runtime should start.
    Resolved(String),
    /// A snapshot exists and refuses: the installation cannot run, or a conflict blocks launching.
    ///
    /// Distinct from `NotScanned` on purpose. Falling back to a live lookup here would start the
    /// very installation the backend declined to pick.
    Refused,
    /// Nothing has been scanned yet, so the snapshot cannot answer.
    NotScanned,
}

impl CliEnvironmentService {
    /// The launch target for a tool, from the same snapshot the CLI Management page renders.
    pub(crate) fn resolve_launch_target(
        &self,
        agent_id: &str,
    ) -> Result<CliLaunchTarget, CliEnvironmentError> {
        let (tool_id, definition) = self.resolve_tool(agent_id)?;
        match self.ports.repository.load_snapshot(&tool_id)? {
            // Anything a scan concluded, including "not found". That is a finding, not an absence
            // of one, and reaching past it for a live candidate would let the launch path
            // contradict the page that says the tool is not installed.
            Some(snapshot) if snapshot.discovery != CliDiscoveryStatus::NotScanned => {
                Ok(launch_target_of(&snapshot))
            }
            // Either nothing stored, or stored and never scanned. A bounded live lookup answers
            // the first launch after install without waiting for a refresh to finish.
            _ => {
                let discovered = self.ports.discovery.discover(
                    &tool_id,
                    definition.executable_names,
                    CliProbeBudget::default(),
                    &CliCancellation::uncancelled(),
                )?;
                // Grouped and deduplicated the same way a refresh does, so one npm global install
                // on Windows is one installation rather than three competing launchers.
                let installations = group_launcher_families(deduplicate(discovered));
                Ok(first_launchable(&installations)
                    .map(|installation| {
                        CliLaunchTarget::Resolved(installation.executable_path.clone())
                    })
                    .unwrap_or(CliLaunchTarget::NotScanned))
            }
        }
    }
}

impl CliEnvironmentService {
    /// The installation facts another context may read. Bounded: one stored snapshot, no probe.
    ///
    /// A tool with no snapshot yet reports nothing rather than reaching for a live lookup: this is
    /// a compatibility question, and answering it from a scan the page has not run would let a
    /// parameter be judged against a version the user cannot see.
    pub(crate) fn installation_facts(
        &self,
        agent_id: &str,
    ) -> Result<CliInstallationFacts, CliEnvironmentError> {
        let (tool_id, _) = self.resolve_tool(agent_id)?;
        let Some(snapshot) = self.ports.repository.load_snapshot(&tool_id)? else {
            return Ok(CliInstallationFacts::default());
        };
        let chosen = snapshot
            .recommended_installation_id
            .as_ref()
            .or(snapshot.path_selected_installation_id.as_ref())
            .and_then(|id| {
                snapshot
                    .installations
                    .iter()
                    .find(|installation| &installation.id == id)
            });
        Ok(CliInstallationFacts {
            installed: snapshot.discovery.is_installed(),
            runnable: chosen
                .is_some_and(|installation| !installation.executable_status.is_faulty()),
            // The same answer the runtime would launch, so a parameter is judged against the binary
            // that will actually receive it.
            active_path: match launch_target_of(&snapshot) {
                CliLaunchTarget::Resolved(path) => Some(path),
                CliLaunchTarget::Refused | CliLaunchTarget::NotScanned => None,
            },
            version: chosen
                .and_then(|installation| installation.reported_version.as_ref())
                .map(|version| version.as_str().to_string()),
            conflict: !snapshot.conflicts.is_empty(),
        })
    }
}

/// The stored snapshot's own answer, using the identities it already computed.
fn launch_target_of(snapshot: &CliEnvironmentSnapshot) -> CliLaunchTarget {
    if snapshot
        .conflicts
        .iter()
        .any(|conflict| conflict.blocks_launch)
    {
        return CliLaunchTarget::Refused;
    }
    // Recommended first: it is what the backend would act on, and launching what it would not act
    // on puts the runtime and the management page on different installations again. PATH-selected
    // is the fallback, because it is what a shell would reach.
    let chosen = snapshot
        .recommended_installation_id
        .as_ref()
        .or(snapshot.path_selected_installation_id.as_ref())
        .and_then(|id| {
            snapshot
                .installations
                .iter()
                .find(|installation| &installation.id == id)
        });
    match chosen {
        Some(installation) if !installation.executable_status.is_faulty() => {
            CliLaunchTarget::Resolved(installation.executable_path.clone())
        }
        Some(_) => CliLaunchTarget::Refused,
        None => CliLaunchTarget::Refused,
    }
}

/// The first candidate a shell would reach that is not already known to be faulty.
fn first_launchable(installations: &[CliInstallation]) -> Option<&CliInstallation> {
    let mut ordered: Vec<&CliInstallation> = installations
        .iter()
        .filter(|installation| !installation.executable_status.is_faulty())
        .filter(|installation| !installation.target_missing)
        .collect();
    // PATH order first, in real order; known-location candidates only after PATH is exhausted.
    ordered.sort_by_key(|installation| {
        (
            installation.path_priority.is_none(),
            installation.path_priority,
        )
    });
    ordered.first().copied()
}

#[cfg(test)]
#[path = "environment_launch_tests.rs"]
mod tests;
