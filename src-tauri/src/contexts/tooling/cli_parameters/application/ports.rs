use super::error::CliParameterApplicationError;
use super::models::{PersistedCliParameterProfile, ReplaceCliParameterProfile};
use crate::contexts::tooling::cli_parameters::domain::catalog::CliParameterCatalog;
use crate::contexts::tooling::cli_parameters::domain::compatibility::CliInstallationSnapshot;
use crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnostic;
use crate::contexts::tooling::cli_parameters::domain::profile::StoredCliParameterProfile;
use std::sync::Arc;

pub(crate) trait CliParameterCatalogPort: Send + Sync {
    fn catalog(&self) -> Result<Arc<CliParameterCatalog>, CliParameterApplicationError>;
}

/// The repository owns one transaction per mutation and never exposes a connection.
pub(crate) trait CliParameterProfileRepository: Send + Sync {
    fn load(
        &self,
        agent_id: &str,
    ) -> Result<StoredCliParameterProfile, CliParameterApplicationError>;

    /// Validates the current revision, replaces every row for the agent, and increments the
    /// revision exactly once, atomically.
    fn replace_if_revision(
        &self,
        mutation: ReplaceCliParameterProfile,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError>;

    fn reset_if_revision(
        &self,
        agent_id: &str,
        expected_revision: i64,
        catalog_version: &str,
    ) -> Result<PersistedCliParameterProfile, CliParameterApplicationError>;
}

/// Read-only projection of the CLI lifecycle detection state. Implementations must not spawn a
/// process; they return whatever the owning subdomain last cached.
pub(crate) trait CliInstallationSnapshotPort: Send + Sync {
    fn active_installation(
        &self,
        agent_id: &str,
    ) -> Result<CliInstallationSnapshot, CliParameterApplicationError>;
}

/// Existence-only directory probe for path-list parameters. Implementations must not recurse.
pub(crate) trait CliParameterDirectoryPort: Send + Sync {
    fn directory_exists(&self, path: &str) -> bool;
}

pub(crate) trait CliParameterDiagnosticsPort: Send + Sync {
    fn emit(&self, diagnostic: &CliParameterDiagnostic);
}
