//! The npm distribution source.
//!
//! npm is the only source that honours an exact version for every version-bearing action, so it is
//! also the source where dropping the requested version is most visible: `npm install --global
//! pkg@1.1.0` and `pkg@latest` are one argument apart.
//!
//! The package name comes from the backend registry. A package identifier arriving over the wire
//! is a request to install something nobody audited, so `build_command_preview` reads it from the
//! distribution definition and ignores everything else.

use std::sync::Arc;
use std::time::Duration;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliDistributionPort, CliExecutionSpec, CliOutputSink, CliPhaseSink,
    CliPlanRequest, CliProcessOutcome, CliSourcePreflight,
};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogStatus, CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::ids::{CliSourceId, CliToolId};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliCommandPreview};
use crate::contexts::tooling::cli::domain::source::CliMutationKey;
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

use super::environment_gateway::{CliCommandGateway, CliCommandRequest};

const CATALOG_TIMEOUT: Duration = Duration::from_secs(30);
const MUTATION_TIMEOUT: Duration = Duration::from_secs(600);
/// Fifteen minutes, matching the documented catalog freshness.
const CATALOG_TTL_SECONDS: i64 = 15 * 60;

pub(crate) struct NpmSource {
    gateway: Arc<dyn CliCommandGateway>,
}

impl NpmSource {
    pub(crate) fn new(gateway: Arc<dyn CliCommandGateway>) -> Self {
        Self { gateway }
    }

    fn source_id_value() -> CliSourceId {
        CliSourceId::new("npm").expect("static source id")
    }

    /// npm is invoked by name so the platform layer resolves it, rather than by a path this module
    /// guesses. On Windows the shim is `npm.cmd`.
    fn program() -> String {
        if cfg!(target_os = "windows") {
            "npm.cmd".to_string()
        } else {
            "npm".to_string()
        }
    }

    fn package_of(
        definition: &CliDistributionDefinition,
    ) -> Result<&'static str, CliEnvironmentError> {
        definition
            .package_reference
            .map(|reference| reference.identifier)
            .ok_or_else(|| CliEnvironmentError::UnsupportedSource {
                agent_id: String::new(),
                source_id: "npm".to_string(),
            })
    }
}

impl CliDistributionPort for NpmSource {
    fn source_id(&self) -> CliSourceId {
        Self::source_id_value()
    }

    fn mutation_key(&self, _agent_id: &CliToolId) -> CliMutationKey {
        // The global prefix is one resource. Two upgrades for different CLIs still contend.
        CliMutationKey::npm_global()
    }

    fn preflight(
        &self,
        _definition: &CliDistributionDefinition,
        cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError> {
        let output = self.gateway.run(
            CliCommandRequest {
                program: Self::program(),
                args: vec!["--version".to_string()],
                timeout: CATALOG_TIMEOUT,
                audit_category: "cli.npm.preflight",
            },
            cancellation,
            None,
        )?;
        Ok(CliSourcePreflight {
            available: output.succeeded(),
            source_version: output
                .succeeded()
                .then(|| output.joined().trim().to_string())
                .filter(|value| !value.is_empty()),
            // npm pins by construction: `package@version` cannot be misread, so this needs no
            // dynamic confirmation the way WinGet does.
            supports_exact_version: true,
            supports_repair: false,
            requires_elevation: false,
        })
    }

    fn list_versions(
        &self,
        agent_id: &CliToolId,
        definition: &CliDistributionDefinition,
        channel: Option<&str>,
        cancellation: &CliCancellation,
    ) -> Result<CliVersionCatalog, CliEnvironmentError> {
        let package = Self::package_of(definition)?;
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::seconds(CATALOG_TTL_SECONDS);
        let unavailable = |reason| {
            CliVersionCatalog::unavailable(
                agent_id.clone(),
                Self::source_id_value(),
                channel.map(str::to_string),
                reason,
                now,
                expires,
            )
        };

        let output = self.gateway.run(
            CliCommandRequest {
                program: Self::program(),
                args: vec![
                    "view".to_string(),
                    package.to_string(),
                    "versions".to_string(),
                    "--json".to_string(),
                ],
                timeout: CATALOG_TIMEOUT,
                audit_category: "cli.npm.catalog",
            },
            cancellation,
            None,
        )?;
        if !output.succeeded() {
            return Ok(unavailable(CliCatalogUnavailableReason::QueryFailed));
        }

        let Some(versions) = parse_versions(&output.joined()) else {
            // Registry output that does not parse contributes nothing. Borrowing another source's
            // data instead is the defect this whole change removes.
            return Ok(unavailable(CliCatalogUnavailableReason::UnparseableOutput));
        };
        if versions.is_empty() {
            return Ok(unavailable(CliCatalogUnavailableReason::UnparseableOutput));
        }

        // Newest first, and the latest *stable* release is what an unspecified target resolves to.
        let mut ordered = versions;
        ordered.sort_by(|left, right| right.display_order(left));
        let latest = ordered
            .iter()
            .find(|version| version.is_stable())
            .or_else(|| ordered.first())
            .cloned();

        Ok(CliVersionCatalog {
            agent_id: agent_id.clone(),
            source_id: Self::source_id_value(),
            channel: channel.map(str::to_string),
            versions: ordered,
            latest,
            fetched_at: now,
            expires_at: expires,
            status: CliCatalogStatus::Available,
        })
    }

    fn build_command_preview(
        &self,
        request: &CliPlanRequest<'_>,
        definition: &CliDistributionDefinition,
    ) -> Result<CliCommandPreview, CliEnvironmentError> {
        let package = Self::package_of(definition)?;
        let args = match request.action {
            CliActionKind::Install
            | CliActionKind::Upgrade
            | CliActionKind::Downgrade
            | CliActionKind::Reinstall => {
                // The requested version lands in the package spec verbatim. `latest` only when the
                // caller genuinely asked for no specific version.
                let spec = match request.target_version {
                    Some(version) => format!("{package}@{}", version.as_str()),
                    None => format!("{package}@latest"),
                };
                vec!["install".to_string(), "--global".to_string(), spec]
            }
            CliActionKind::Uninstall => vec![
                "uninstall".to_string(),
                "--global".to_string(),
                package.to_string(),
            ],
            CliActionKind::Repair => {
                return Err(CliEnvironmentError::UnsupportedAction {
                    agent_id: request.agent_id.as_str().to_string(),
                    source_id: "npm".to_string(),
                    action: "repair",
                })
            }
        };
        Ok(CliCommandPreview::new(Self::program(), args))
    }

    fn build_execution(
        &self,
        plan: &CliActionPlan,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliExecutionSpec, CliEnvironmentError> {
        // Derived from the reviewed preview rather than rebuilt, so the command the user confirmed
        // and the command that runs are the same object.
        Ok(CliExecutionSpec {
            program: plan.command_preview.program.clone(),
            args: plan.command_preview.args.clone(),
            requires_network: true,
            requires_elevation: false,
        })
    }

    fn execute(
        &self,
        spec: CliExecutionSpec,
        cancellation: &CliCancellation,
        output: &dyn CliOutputSink,
        phases: &dyn CliPhaseSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError> {
        // npm fetches and writes inside one command, so there is no observable download boundary
        // to report. The whole call is treated as the irreversible part, which is the safe
        // direction to be wrong in: cancel is never offered while npm may be writing.
        phases.enter(CliOperationPhase::Mutating, false);
        let result = self.gateway.run(
            CliCommandRequest {
                program: spec.program,
                args: spec.args,
                timeout: MUTATION_TIMEOUT,
                audit_category: "cli.npm.mutate",
            },
            cancellation,
            Some(output),
        )?;
        Ok(CliProcessOutcome {
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            cancelled: result.cancelled,
            truncated: result.truncated,
        })
    }
}

/// Parses `npm view <pkg> versions --json`, which is a JSON array -- or a bare string when the
/// package has exactly one published version.
fn parse_versions(raw: &str) -> Option<Vec<NormalizedCliVersion>> {
    let value: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    match value {
        serde_json::Value::Array(entries) => Some(
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .map(NormalizedCliVersion::parse)
                .collect(),
        ),
        serde_json::Value::String(single) => Some(vec![NormalizedCliVersion::parse(single)]),
        _ => None,
    }
}

#[cfg(test)]
#[path = "npm_source_tests.rs"]
mod tests;
