//! The source-aware CLI use cases.
//!
//! Every method that can touch a process, a package manager, or the network is split into a
//! `prepare_*` that returns an operation id without doing any of that, and an `execute_*` the
//! caller runs on a background thread. Cached reads and persisted-plan reads stay direct.
//!
//! The service owns policy. Adapters own I/O. Nothing here names a concrete source: the registry
//! is asked for an adapter by the id the plan recorded, which is what makes "execute exactly the
//! disclosed source" a structural property rather than a convention.

use std::sync::Arc;

use super::environment_error::CliEnvironmentError;
use super::environment_ports::{
    CliClock, CliDiagnosticsPort, CliDiscoveryPort, CliEnvironmentRepository, CliIdFactory,
    CliMutationCoordinator, CliOperationsPort, CliProbePort, CliSourceRegistry,
};
use crate::contexts::tooling::cli::domain::ids::CliToolId;
use crate::contexts::tooling::cli::domain::registry::{definition, CLI_TOOL_DEFINITIONS};
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;

pub(crate) struct CliEnvironmentPorts {
    pub(crate) discovery: Arc<dyn CliDiscoveryPort>,
    pub(crate) probes: Arc<dyn CliProbePort>,
    pub(crate) sources: Arc<dyn CliSourceRegistry>,
    pub(crate) repository: Arc<dyn CliEnvironmentRepository>,
    pub(crate) operations: Arc<dyn CliOperationsPort>,
    pub(crate) coordinator: Arc<dyn CliMutationCoordinator>,
    pub(crate) diagnostics: Arc<dyn CliDiagnosticsPort>,
    pub(crate) clock: Arc<dyn CliClock>,
    pub(crate) ids: Arc<dyn CliIdFactory>,
}

#[derive(Clone)]
pub(crate) struct CliEnvironmentService {
    pub(super) ports: Arc<CliEnvironmentPorts>,
}

impl CliEnvironmentService {
    pub(crate) fn new(ports: CliEnvironmentPorts) -> Self {
        Self {
            ports: Arc::new(ports),
        }
    }

    /// Persisted snapshots for every registered CLI, in catalog order.
    ///
    /// Bounded by construction: it reads storage and computes the environment fingerprint, and
    /// starts no process, no network request, and no probe. A tool with no stored snapshot gets a
    /// never-scanned one -- which claims nothing, rather than claiming the tool is missing.
    pub(crate) fn list_cli_environments(
        &self,
    ) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError> {
        let stored = self.ports.repository.list_snapshots()?;
        let fingerprint = self.ports.discovery.environment_fingerprint()?;

        let mut snapshots = Vec::with_capacity(CLI_TOOL_DEFINITIONS.len());
        for tool in CLI_TOOL_DEFINITIONS {
            let agent_id = tool
                .tool_id()
                .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
            let existing = stored
                .iter()
                .find(|snapshot| snapshot.agent_id == agent_id)
                .cloned();
            snapshots.push(match existing {
                Some(mut snapshot) => {
                    // The environment moved since this was captured, so what is held describes a
                    // machine that no longer exists. It stays visible and gets labelled.
                    if snapshot.environment_fingerprint != fingerprint {
                        snapshot.mark_stale();
                    }
                    snapshot
                }
                None => CliEnvironmentSnapshot::never_scanned(agent_id, fingerprint.clone()),
            });
        }
        Ok(snapshots)
    }

    /// Resolves a wire `agentId` to a registered tool, or reports it as unknown.
    pub(super) fn resolve_tool(
        &self,
        agent_id: &str,
    ) -> Result<
        (
            CliToolId,
            &'static crate::contexts::tooling::cli::domain::definition::CliToolDefinition,
        ),
        CliEnvironmentError,
    > {
        let tool = definition(agent_id).ok_or_else(|| CliEnvironmentError::UnknownTool {
            agent_id: agent_id.to_string(),
        })?;
        let id = tool
            .tool_id()
            .map_err(|error| CliEnvironmentError::Validation(error.to_string()))?;
        Ok((id, tool))
    }

    /// The snapshot currently held for a tool, or a never-scanned one.
    pub(super) fn snapshot_or_never_scanned(
        &self,
        agent_id: &CliToolId,
        fingerprint: &str,
    ) -> Result<CliEnvironmentSnapshot, CliEnvironmentError> {
        Ok(self
            .ports
            .repository
            .load_snapshot(agent_id)?
            .unwrap_or_else(|| {
                CliEnvironmentSnapshot::never_scanned(agent_id.clone(), fingerprint.to_string())
            }))
    }
}

#[cfg(test)]
#[path = "environment_service_tests.rs"]
mod tests;
