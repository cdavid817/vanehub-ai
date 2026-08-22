//! The WinGet distribution source.
//!
//! WinGet's exact-version support is not a static fact: whether `--version` is honoured depends on
//! the installed WinGet and the package. It is therefore confirmed by preflight and carried into
//! the plan, and an unconfirmed exact request is refused rather than run as a latest install under
//! an exact label.
//!
//! Downgrade and reinstall stay closed. The adapter has no verified argument form for either, and
//! an unverified one installs latest while claiming otherwise -- which is the defect this file
//! exists to avoid repeating.

use std::sync::Arc;
use std::time::Duration;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliDistributionPort, CliExecutionSpec, CliOutputSink, CliPlanRequest,
    CliProcessOutcome, CliSourcePreflight,
};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogStatus, CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::ids::{CliSourceId, CliToolId};
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliCommandPreview};
use crate::contexts::tooling::cli::domain::source::CliMutationKey;
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

use super::environment_gateway::{CliCommandGateway, CliCommandRequest};

const QUERY_TIMEOUT: Duration = Duration::from_secs(60);
const MUTATION_TIMEOUT: Duration = Duration::from_secs(900);
const CATALOG_TTL_SECONDS: i64 = 15 * 60;

/// Non-interactive agreement flags. Without them WinGet blocks on a prompt no desktop app can
/// answer, and the operation would hang until its timeout rather than failing usefully.
const AGREEMENT_FLAGS: [&str; 3] = [
    "--accept-package-agreements",
    "--accept-source-agreements",
    "--disable-interactivity",
];

pub(crate) struct WingetSource {
    gateway: Arc<dyn CliCommandGateway>,
}

impl WingetSource {
    pub(crate) fn new(gateway: Arc<dyn CliCommandGateway>) -> Self {
        Self { gateway }
    }

    fn source_id_value() -> CliSourceId {
        CliSourceId::new("winget").expect("static source id")
    }

    fn package_id(
        definition: &CliDistributionDefinition,
    ) -> Result<&'static str, CliEnvironmentError> {
        definition
            .package_reference
            .map(|reference| reference.identifier)
            .ok_or_else(|| CliEnvironmentError::UnsupportedSource {
                agent_id: String::new(),
                source_id: "winget".to_string(),
            })
    }
}

impl CliDistributionPort for WingetSource {
    fn source_id(&self) -> CliSourceId {
        Self::source_id_value()
    }

    fn mutation_key(&self, _agent_id: &CliToolId) -> CliMutationKey {
        CliMutationKey::winget()
    }

    fn preflight(
        &self,
        _definition: &CliDistributionDefinition,
        cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError> {
        let output = self.gateway.run(
            CliCommandRequest {
                program: "winget".to_string(),
                args: vec!["--version".to_string()],
                timeout: QUERY_TIMEOUT,
                audit_category: "cli.winget.preflight",
            },
            cancellation,
            None,
        )?;
        let version = output.joined().trim().to_string();
        Ok(CliSourcePreflight {
            available: output.succeeded(),
            supports_exact_version: output.succeeded() && supports_exact_version(&version),
            supports_repair: output.succeeded() && supports_repair(&version),
            source_version: (!version.is_empty()).then_some(version),
            // WinGet elevates per package rather than per invocation; the plan reports what the
            // preflight saw rather than assuming either way.
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
        let package = Self::package_id(definition)?;
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
                program: "winget".to_string(),
                args: vec![
                    "show".to_string(),
                    "--id".to_string(),
                    package.to_string(),
                    "--exact".to_string(),
                    "--versions".to_string(),
                    "--disable-interactivity".to_string(),
                ],
                timeout: QUERY_TIMEOUT,
                audit_category: "cli.winget.catalog",
            },
            cancellation,
            None,
        )?;
        if !output.succeeded() {
            return Ok(unavailable(CliCatalogUnavailableReason::QueryFailed));
        }

        let versions = parse_version_table(&output.lines);
        if versions.is_empty() {
            // Localized or reformatted output that yields nothing. Reporting no catalog is correct;
            // borrowing npm's would not be.
            return Ok(unavailable(CliCatalogUnavailableReason::UnparseableOutput));
        }
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
        let package = Self::package_id(definition)?;
        let unsupported = |action: &'static str| CliEnvironmentError::UnsupportedAction {
            agent_id: request.agent_id.as_str().to_string(),
            source_id: "winget".to_string(),
            action,
        };

        let verb = match request.action {
            CliActionKind::Install => "install",
            CliActionKind::Upgrade => "upgrade",
            CliActionKind::Uninstall => "uninstall",
            CliActionKind::Repair => "repair",
            // No verified argument form. Refusing is the whole point.
            CliActionKind::Downgrade => return Err(unsupported("downgrade")),
            CliActionKind::Reinstall => return Err(unsupported("reinstall")),
        };

        let mut args = vec![
            verb.to_string(),
            "--id".to_string(),
            package.to_string(),
            "--exact".to_string(),
        ];
        if matches!(
            request.action,
            CliActionKind::Install | CliActionKind::Upgrade
        ) {
            if let Some(target) = request.target_version {
                if !request.exact_version_confirmed {
                    // The caller asked for a specific version and preflight could not confirm
                    // WinGet honours one here. Running without `--version` would install latest
                    // and report it as the requested version.
                    return Err(CliEnvironmentError::InvalidVersion {
                        source_id: "winget".to_string(),
                        value: target.as_str().to_string(),
                    });
                }
                args.push("--version".to_string());
                args.push(target.as_str().to_string());
            }
        }
        // Uninstall and repair take no agreement flags beyond non-interactivity.
        if matches!(
            request.action,
            CliActionKind::Install | CliActionKind::Upgrade
        ) {
            args.extend(AGREEMENT_FLAGS.iter().map(|flag| (*flag).to_string()));
        } else {
            args.push("--disable-interactivity".to_string());
        }

        Ok(CliCommandPreview::new("winget", args))
    }

    fn build_execution(
        &self,
        plan: &CliActionPlan,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliExecutionSpec, CliEnvironmentError> {
        Ok(CliExecutionSpec {
            program: plan.command_preview.program.clone(),
            args: plan.command_preview.args.clone(),
            requires_network: true,
            requires_elevation: plan.requires_elevation,
        })
    }

    fn execute(
        &self,
        spec: CliExecutionSpec,
        cancellation: &CliCancellation,
        output: &dyn CliOutputSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError> {
        let result = self.gateway.run(
            CliCommandRequest {
                program: spec.program,
                args: spec.args,
                timeout: MUTATION_TIMEOUT,
                audit_category: "cli.winget.mutate",
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

/// `--version` on install/upgrade landed in WinGet 1.2.
fn supports_exact_version(reported: &str) -> bool {
    at_least(reported, 1, 2)
}

/// `winget repair` landed in 1.7.
fn supports_repair(reported: &str) -> bool {
    at_least(reported, 1, 7)
}

/// WinGet reports `v1.6.3482` or similar. An unrecognisable version withholds the capability
/// rather than assuming it -- assuming produces a command the local WinGet rejects.
fn at_least(reported: &str, major: u64, minor: u64) -> bool {
    let parsed = NormalizedCliVersion::parse(reported.trim());
    if !parsed.is_ordered() {
        return false;
    }
    let floor = NormalizedCliVersion::parse(format!("{major}.{minor}.0"));
    parsed
        .compare(&floor)
        .is_some_and(|ordering| ordering.is_ge())
}

/// Extracts versions from `winget show --versions`, whose output is a header, a dashed rule, and
/// then one version per line. Header wording is localized, so parsing keys off the shape -- a line
/// that is a version -- rather than off any translated label.
fn parse_version_table(lines: &[String]) -> Vec<NormalizedCliVersion> {
    lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('-'))
        .map(NormalizedCliVersion::parse)
        .filter(NormalizedCliVersion::is_ordered)
        .collect()
}

#[cfg(test)]
#[path = "winget_source_tests.rs"]
mod tests;
