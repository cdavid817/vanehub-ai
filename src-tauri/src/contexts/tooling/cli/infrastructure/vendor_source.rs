//! The audited vendor installer source.
//!
//! This is the highest-risk adapter in the product: VaneHub fetches a program and runs it. Every
//! constraint is therefore explicit rather than conventional.
//!
//! Two execution shapes are removed here and cannot be reintroduced by editing this file alone,
//! because the domain refuses to produce a plan for them:
//!
//! - `curl URL | bash` / `wget -qO- URL | sh`. The installer is downloaded to a bounded temporary
//!   file and that file is executed. A pipeline has nowhere to put an argument, nothing to
//!   checksum, and no way to distinguish a truncated download from a complete one.
//! - `irm URL | iex`. Same reasons, plus PowerShell's `-Command` swallows the URL into a string
//!   the shell interprets. `-File` takes a path.
//!
//! Platform selection has no fallback arm: a vendor that publishes only a shell installer yields
//! nothing on Windows, and the caller's only correct response is to withhold the action.

use std::sync::Arc;
use std::time::Duration;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliDistributionPort, CliExecutionSpec, CliOutputSink, CliPhaseSink,
    CliPlanRequest, CliProcessOutcome, CliSourcePreflight,
};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::ids::{CliSourceId, CliToolId};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliCommandPreview};
use crate::contexts::tooling::cli::domain::source::{CliMutationKey, CliPlatform};
use crate::contexts::tooling::cli::domain::trust::{
    CliInstallerRuntime, CliInstallerTemplate, CliInstallerTrust,
};

use crate::contexts::tooling::managed_install::api::{
    ArtifactRequest, ManagedArtifactRetriever, ManagedInstallError,
};

use super::environment_gateway::{CliCommandGateway, CliCommandRequest};

const MUTATION_TIMEOUT: Duration = Duration::from_secs(900);

/// Converts at the boundary rather than sharing an error type. `managed_install` stays free of
/// CLI vocabulary, and the mapping is the one place that decides how a refused download reads to
/// a CLI caller.
fn from_managed(error: ManagedInstallError) -> CliEnvironmentError {
    match error {
        // A digest mismatch and an allowlist refusal are both "we will not run this", which is
        // what `Validation` means to the CLI surface.
        ManagedInstallError::Refused(message) => CliEnvironmentError::Validation(message),
        ManagedInstallError::ChecksumMismatch => CliEnvironmentError::Validation(
            "the installer does not match its published checksum".to_string(),
        ),
        other => CliEnvironmentError::Process(other.to_string()),
    }
}

/// The file name the installer lands under, chosen by interpreter rather than by URL.
///
/// A vendor-controlled path segment must not decide what lands on disk, and on Windows the
/// extension is what picks an interpreter.
const fn installer_file_name(runtime: CliInstallerRuntime) -> &'static str {
    match runtime {
        CliInstallerRuntime::PowerShellFile => "installer.ps1",
        CliInstallerRuntime::ShellFile { .. } => "installer.sh",
    }
}

pub(crate) struct VendorSource {
    gateway: Arc<dyn CliCommandGateway>,
    downloader: Arc<dyn ManagedArtifactRetriever>,
}

impl VendorSource {
    pub(crate) fn new(
        gateway: Arc<dyn CliCommandGateway>,
        downloader: Arc<dyn ManagedArtifactRetriever>,
    ) -> Self {
        Self {
            gateway,
            downloader,
        }
    }

    fn source_id_value() -> CliSourceId {
        CliSourceId::trusted("vendor")
    }

    /// The template for this host, or an error naming the platform.
    ///
    /// There is no fallback: `template_for` matches the platform exactly, so a shell-only vendor
    /// returns `None` on Windows and this refuses rather than reaching for the `.sh`.
    fn template_for_here(
        trust: &CliInstallerTrust,
    ) -> Result<&CliInstallerTemplate, CliEnvironmentError> {
        let platform = CliPlatform::current().ok_or(CliEnvironmentError::RuntimeUnsupported)?;
        trust
            .template_for(platform)
            .ok_or(CliEnvironmentError::RuntimeUnsupported)
    }

    fn trust_of(
        definition: &CliDistributionDefinition,
    ) -> Result<&CliInstallerTrust, CliEnvironmentError> {
        definition
            .trust
            .installer()
            .ok_or(CliEnvironmentError::RuntimeUnsupported)
    }
}

impl CliDistributionPort for VendorSource {
    fn source_id(&self) -> CliSourceId {
        Self::source_id_value()
    }

    fn mutation_key(&self, agent_id: &CliToolId) -> CliMutationKey {
        // A vendor installer writes only this tool's own tree, so two different CLIs may install
        // at the same time without contending.
        CliMutationKey::vendor(agent_id.as_str())
    }

    fn preflight(
        &self,
        definition: &CliDistributionDefinition,
        _cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError> {
        let trust = Self::trust_of(definition)?;
        let template = Self::template_for_here(trust);
        Ok(CliSourcePreflight {
            available: template.is_ok(),
            source_version: None,
            // Only a template that declares a verified version convention may be aimed.
            supports_exact_version: template
                .map(|template| template.target_version.accepts_exact_target())
                .unwrap_or(false),
            supports_repair: false,
            // The installer decides; VaneHub does not elevate on its behalf.
            requires_elevation: false,
        })
    }

    fn list_versions(
        &self,
        agent_id: &CliToolId,
        _definition: &CliDistributionDefinition,
        channel: Option<&str>,
        _cancellation: &CliCancellation,
    ) -> Result<CliVersionCatalog, CliEnvironmentError> {
        let now = chrono::Utc::now();
        // A vendor installer publishes no queryable version list. Saying so is the honest answer;
        // returning npm's list for a vendor-installed tool is the defect being removed.
        Ok(CliVersionCatalog::unavailable(
            agent_id.clone(),
            Self::source_id_value(),
            channel.map(str::to_string),
            CliCatalogUnavailableReason::NotApplicable,
            now,
            now + chrono::Duration::seconds(15 * 60),
        ))
    }

    fn build_command_preview(
        &self,
        request: &CliPlanRequest<'_>,
        definition: &CliDistributionDefinition,
    ) -> Result<CliCommandPreview, CliEnvironmentError> {
        let trust = Self::trust_of(definition)?;
        let template = Self::template_for_here(trust)?;

        if !matches!(
            request.action,
            CliActionKind::Install | CliActionKind::Upgrade
        ) {
            // An installer installs. It does not uninstall, downgrade, or repair, and inventing an
            // invocation for those would be guessing at a vendor's calling convention.
            return Err(CliEnvironmentError::UnsupportedAction {
                agent_id: request.agent_id.as_str().to_string(),
                source_id: "vendor".to_string(),
                action: request.action.as_str(),
            });
        }
        if request.target_version.is_some() && !template.target_version.accepts_exact_target() {
            return Err(CliEnvironmentError::InvalidVersion {
                source_id: "vendor".to_string(),
                value: request
                    .target_version
                    .map(|version| version.as_str().to_string())
                    .unwrap_or_default(),
            });
        }

        // The preview names the interpreter and the audited URL. The real path is only known at
        // execution time and is a temporary path, which has no business being persisted in a plan
        // or shown in a dialog -- the URL is what the user is actually approving.
        //
        // The placeholder deliberately avoids angle brackets: `is_shell_free` treats `<` and `>` as
        // redirection, and a preview that trips that check is indistinguishable from an adapter
        // that really did build a shell string.
        Ok(match template.runtime {
            CliInstallerRuntime::PowerShellFile => CliCommandPreview::new(
                "powershell",
                vec![
                    "-NoProfile".to_string(),
                    "-ExecutionPolicy".to_string(),
                    "Bypass".to_string(),
                    "-File".to_string(),
                    format!("installer:{}", template.url),
                ],
            ),
            CliInstallerRuntime::ShellFile { interpreter } => {
                CliCommandPreview::new(interpreter, vec![format!("installer:{}", template.url)])
            }
        })
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
        phases: &dyn CliPhaseSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError> {
        // The URL travels in the preview placeholder; the definition is the authority for what may
        // actually be fetched, so it is re-resolved here rather than parsed back out of a string.
        let (trust, template) = resolve_template_for_spec(&spec)?;
        if !trust.permits_url(template.url) {
            return Err(CliEnvironmentError::Validation(
                "the installer URL is not on this source's allowlist".to_string(),
            ));
        }

        // Downloading is genuinely cancellable: the file lands in a temporary directory and nothing
        // on the machine has been touched yet.
        phases.enter(CliOperationPhase::Downloading, true);
        let installer = self
            .downloader
            .retrieve(
                ArtifactRequest {
                    url: template.url,
                    policy: &trust.policy,
                    // The selected template's digest, not whichever template happened to declare
                    // one. Every shipped template is currently `Unverified`, so this is the same
                    // behavior today and the right behavior once one is not.
                    integrity: template.integrity,
                    file_name: installer_file_name(template.runtime),
                    executable: true,
                },
                &cancellation.signal(),
            )
            .map_err(from_managed)?;
        let args = match template.runtime {
            CliInstallerRuntime::PowerShellFile => vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                // `-File`, never `-Command`: a command string is interpreted, a path is not.
                "-File".to_string(),
                installer.path.to_string_lossy().to_string(),
            ],
            CliInstallerRuntime::ShellFile { .. } => {
                vec![installer.path.to_string_lossy().to_string()]
            }
        };

        // The installer is about to run. From here a cancellation cannot undo what it has already
        // written, so cancel stops being offered at exactly this point rather than a phase earlier
        // or later.
        phases.enter(CliOperationPhase::Mutating, false);
        let result = self.gateway.run(
            CliCommandRequest {
                program: spec.program,
                args,
                timeout: MUTATION_TIMEOUT,
                audit_category: "cli.vendor.install",
            },
            cancellation,
            Some(output),
        );
        // `installer` drops here on every path, removing the temporary file after success,
        // failure, timeout, and cancellation alike.
        let result = result?;
        Ok(CliProcessOutcome {
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            cancelled: result.cancelled,
            truncated: result.truncated,
        })
    }
}

/// Recovers the audited template a spec refers to, by matching its interpreter against the
/// registry rather than trusting anything embedded in the spec.
fn resolve_template_for_spec(
    spec: &CliExecutionSpec,
) -> Result<(CliInstallerTrust, CliInstallerTemplate), CliEnvironmentError> {
    let platform = CliPlatform::current().ok_or(CliEnvironmentError::RuntimeUnsupported)?;
    for tool in crate::contexts::tooling::cli::domain::registry::CLI_TOOL_DEFINITIONS {
        let Some(distribution) = tool.distribution("vendor") else {
            continue;
        };
        let Some(trust) = distribution.trust.installer() else {
            continue;
        };
        let Some(template) = trust.template_for(platform) else {
            continue;
        };
        let matches = spec.args.iter().any(|arg| arg.contains(template.url));
        if matches {
            return Ok((*trust, *template));
        }
    }
    Err(CliEnvironmentError::Validation(
        "no audited installer template matches this plan".to_string(),
    ))
}

#[cfg(test)]
#[path = "vendor_source_tests.rs"]
mod tests;
