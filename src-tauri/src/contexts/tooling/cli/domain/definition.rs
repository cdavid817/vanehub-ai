//! What a CLI tool is and which sources can distribute it.
//!
//! A tool has *many* distributions, each with its own catalog, capabilities, platforms, and trust
//! policy. That plurality is the point: `claude-code` is reachable through npm, WinGet, and a
//! vendor installer, and those three disagree about what versions exist and which actions are
//! possible. Collapsing them into one `latestVersion` and one eligibility enum is what let npm
//! registry data decide the update state of a WinGet installation.

use super::ids::{CliIdError, CliSourceId, CliToolId};
use super::probe::CliProbeDefinition;
use super::source::{
    CliDynamicCapability, CliMutationKey, CliPlatform, CliReleaseChannel, CliSourceCapabilities,
    CliSourceKind, CliTargetVersionMode, PlatformSet,
};
use super::trust::CliSourceTrustPolicy;
use super::version::NormalizedCliVersion;

/// How a source names this package. Never accepted from the frontend -- a package identifier
/// arriving over the wire is a request to install something the backend never audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliPackageReference {
    pub(crate) identifier: &'static str,
}

/// Which versions of a CLI this product is known to work with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliCompatibilityPolicy {
    /// Below this, compatibility is `unsupported-version`. `None` means no floor has been
    /// established, which yields `unknown` -- not `supported`.
    pub(crate) minimum_supported: Option<&'static str>,
    pub(crate) platforms: PlatformSet,
}

impl CliCompatibilityPolicy {
    pub(crate) const fn any_desktop() -> Self {
        Self {
            minimum_supported: None,
            platforms: PlatformSet::ALL,
        }
    }

    /// `None` when no floor is declared or the reported version is opaque: an unordered version
    /// cannot be shown to be below a floor, and guessing would flag healthy installs as outdated.
    pub(crate) fn is_below_floor(&self, version: &NormalizedCliVersion) -> Option<bool> {
        let floor = NormalizedCliVersion::parse(self.minimum_supported?);
        version.compare(&floor).map(|ordering| ordering.is_lt())
    }
}

/// One way to obtain a CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliDistributionDefinition {
    pub(crate) source_id: &'static str,
    pub(crate) kind: CliSourceKind,
    pub(crate) package_reference: Option<CliPackageReference>,
    pub(crate) platforms: PlatformSet,
    pub(crate) capabilities: CliSourceCapabilities,
    pub(crate) channels: &'static [CliReleaseChannel],
    pub(crate) trust: CliSourceTrustPolicy,
}

impl CliDistributionDefinition {
    pub(crate) fn source_id(&self) -> Result<CliSourceId, CliIdError> {
        CliSourceId::new(self.source_id)
    }

    /// The resource this distribution serializes against while mutating.
    pub(crate) fn mutation_key(&self, agent_id: &str) -> CliMutationKey {
        match self.kind {
            CliSourceKind::Npm => CliMutationKey::npm_global(),
            CliSourceKind::Winget => CliMutationKey::winget(),
            // A vendor installer writes only this tool's tree, so two different CLIs may install
            // concurrently without contending.
            _ => CliMutationKey::vendor(agent_id),
        }
    }

    pub(crate) fn default_channel(&self) -> Option<CliReleaseChannel> {
        self.channels
            .iter()
            .copied()
            .find(|channel| channel.is_default)
            .or_else(|| self.channels.first().copied())
    }

    /// Whether this distribution can act on the given platform *at all*.
    ///
    /// For an audited installer this additionally requires a template for that exact platform:
    /// declaring Windows support without a Windows template is precisely the state that used to
    /// produce a `bash -lc` plan on Windows.
    pub(crate) fn is_actionable_on(&self, platform: CliPlatform) -> bool {
        if !self.platforms.contains(platform) {
            return false;
        }
        match self.trust.installer() {
            Some(trust) => trust.template_for(platform).is_some(),
            None => true,
        }
    }

    pub(crate) fn is_actionable_here(&self) -> bool {
        CliPlatform::current().is_some_and(|platform| self.is_actionable_on(platform))
    }

    /// The version granularity this distribution honours for `action` on `platform`.
    ///
    /// An installer template may be narrower than the distribution's declared capability, and the
    /// narrower of the two wins. Returning the declared capability when the template cannot honour
    /// it is how a "install 1.2.3" request becomes a latest install reported as 1.2.3.
    pub(crate) fn target_mode_on(
        &self,
        action: CliDistributionAction,
        platform: CliPlatform,
    ) -> CliTargetVersionMode {
        if !self.is_actionable_on(platform) {
            return CliTargetVersionMode::Unsupported;
        }
        let declared = match action {
            CliDistributionAction::Install => self.capabilities.install,
            CliDistributionAction::Upgrade => self.capabilities.upgrade,
            CliDistributionAction::Downgrade => self.capabilities.downgrade,
            CliDistributionAction::Reinstall => self.capabilities.reinstall,
        };
        match self
            .trust
            .installer()
            .and_then(|trust| trust.template_for(platform))
        {
            Some(template) => narrower(declared, template.target_version),
            None => declared,
        }
    }
}

/// The four version-bearing actions. `uninstall` and `repair` carry no target version, so they are
/// modelled by their own capability fields rather than by a `CliTargetVersionMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliDistributionAction {
    Install,
    Upgrade,
    Downgrade,
    Reinstall,
}

fn narrower(left: CliTargetVersionMode, right: CliTargetVersionMode) -> CliTargetVersionMode {
    use CliTargetVersionMode::{Exact, LatestOnly, Unsupported};
    match (left, right) {
        (Unsupported, _) | (_, Unsupported) => Unsupported,
        (LatestOnly, _) | (_, LatestOnly) => LatestOnly,
        (Exact, Exact) => Exact,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliToolDefinition {
    pub(crate) agent_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) provider: &'static str,
    /// Every name the executable may carry, in preference order. Windows shims mean one CLI can
    /// appear as `claude`, `claude.cmd`, and `claude.exe` in the same PATH.
    pub(crate) executable_names: &'static [&'static str],
    pub(crate) distributions: &'static [CliDistributionDefinition],
    pub(crate) probes: CliProbeDefinition,
    pub(crate) compatibility: CliCompatibilityPolicy,
}

impl CliToolDefinition {
    pub(crate) fn tool_id(&self) -> Result<CliToolId, CliIdError> {
        CliToolId::new(self.agent_id)
    }

    pub(crate) fn distribution(&self, source_id: &str) -> Option<&CliDistributionDefinition> {
        self.distributions
            .iter()
            .find(|distribution| distribution.source_id == source_id)
    }

    pub(crate) fn distribution_of_kind(
        &self,
        kind: CliSourceKind,
    ) -> Option<&CliDistributionDefinition> {
        self.distributions
            .iter()
            .find(|distribution| distribution.kind == kind)
    }

    /// Distributions that can actually do something on this host. A distribution declared for a
    /// platform VaneHub is not running on contributes nothing but noise.
    pub(crate) fn actionable_distributions(
        &self,
    ) -> impl Iterator<Item = &CliDistributionDefinition> {
        self.distributions
            .iter()
            .filter(|distribution| distribution.is_actionable_here())
    }
}

/// npm honours an exact version for every version-bearing action and can uninstall.
pub(crate) const NPM_CAPABILITIES: CliSourceCapabilities = CliSourceCapabilities {
    install: CliTargetVersionMode::Exact,
    upgrade: CliTargetVersionMode::Exact,
    downgrade: CliTargetVersionMode::Exact,
    reinstall: CliTargetVersionMode::Exact,
    uninstall: true,
    repair: CliDynamicCapability::Unsupported,
};

/// WinGet install and upgrade accept `--version` when the local WinGet and the source support it,
/// which the adapter confirms during preflight. Downgrade and reinstall stay closed until a
/// separate verified capability is added -- an unverified downgrade is a silent latest install.
pub(crate) const WINGET_CAPABILITIES: CliSourceCapabilities = CliSourceCapabilities {
    install: CliTargetVersionMode::Exact,
    upgrade: CliTargetVersionMode::Exact,
    downgrade: CliTargetVersionMode::Unsupported,
    reinstall: CliTargetVersionMode::Unsupported,
    uninstall: true,
    repair: CliDynamicCapability::RequiresPreflight,
};

/// An audited installer runs at whatever version it defaults to. Nothing here is exact unless a
/// template raises it, and no installer uninstalls.
pub(crate) const VENDOR_CAPABILITIES: CliSourceCapabilities = CliSourceCapabilities {
    install: CliTargetVersionMode::LatestOnly,
    upgrade: CliTargetVersionMode::LatestOnly,
    downgrade: CliTargetVersionMode::Unsupported,
    reinstall: CliTargetVersionMode::Unsupported,
    uninstall: false,
    repair: CliDynamicCapability::Unsupported,
};

pub(crate) const STABLE_CHANNEL_ONLY: &[CliReleaseChannel] = &[CliReleaseChannel::STABLE];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::trust::{
        CliInstallerRuntime, CliInstallerTemplate, CliInstallerTrust,
    };
    use crate::contexts::tooling::managed_install::api::{ArtifactIntegrity, RetrievalPolicy};

    const UNIX_ONLY_INSTALLER: CliInstallerTrust = CliInstallerTrust {
        policy: RetrievalPolicy {
            allowed_hosts: &["example.test"],
            max_download_bytes: 1024,
            download_timeout_seconds: 30,
        },
        templates: &[CliInstallerTemplate {
            platform: CliPlatform::Linux,
            runtime: CliInstallerRuntime::ShellFile {
                interpreter: "bash",
            },
            url: "https://example.test/install.sh",
            target_version: CliTargetVersionMode::LatestOnly,
            version_argument: None,
            integrity: ArtifactIntegrity::Unverified,
        }],
    };

    fn npm_distribution() -> CliDistributionDefinition {
        CliDistributionDefinition {
            source_id: "npm",
            kind: CliSourceKind::Npm,
            package_reference: Some(CliPackageReference {
                identifier: "@openai/codex",
            }),
            platforms: PlatformSet::ALL,
            capabilities: NPM_CAPABILITIES,
            channels: STABLE_CHANNEL_ONLY,
            trust: CliSourceTrustPolicy::PackageManager,
        }
    }

    fn vendor_distribution() -> CliDistributionDefinition {
        CliDistributionDefinition {
            source_id: "vendor",
            kind: CliSourceKind::VendorInstaller,
            package_reference: None,
            // Declared for every platform on purpose: the templates, not this field, are what
            // decide whether Windows is actionable.
            platforms: PlatformSet::ALL,
            capabilities: VENDOR_CAPABILITIES,
            channels: STABLE_CHANNEL_ONLY,
            trust: CliSourceTrustPolicy::AuditedInstaller(UNIX_ONLY_INSTALLER),
        }
    }

    #[test]
    fn a_declared_platform_without_a_template_is_not_actionable() {
        let vendor = vendor_distribution();
        // The distribution claims all platforms, but only Linux has an audited template.
        assert!(vendor.platforms.contains(CliPlatform::Windows));
        assert!(!vendor.is_actionable_on(CliPlatform::Windows));
        assert!(!vendor.is_actionable_on(CliPlatform::Macos));
        assert!(vendor.is_actionable_on(CliPlatform::Linux));
    }

    #[test]
    fn a_package_manager_distribution_needs_no_template() {
        let npm = npm_distribution();
        assert!(npm.is_actionable_on(CliPlatform::Windows));
        assert!(npm.is_actionable_on(CliPlatform::Macos));
        assert!(npm.is_actionable_on(CliPlatform::Linux));
    }

    #[test]
    fn an_unactionable_platform_supports_no_action_at_any_granularity() {
        let vendor = vendor_distribution();
        for action in [
            CliDistributionAction::Install,
            CliDistributionAction::Upgrade,
            CliDistributionAction::Downgrade,
            CliDistributionAction::Reinstall,
        ] {
            assert_eq!(
                vendor.target_mode_on(action, CliPlatform::Windows),
                CliTargetVersionMode::Unsupported
            );
        }
    }

    #[test]
    fn the_narrower_of_capability_and_template_wins() {
        let vendor = vendor_distribution();
        // Capability says LatestOnly, template says LatestOnly.
        assert_eq!(
            vendor.target_mode_on(CliDistributionAction::Install, CliPlatform::Linux),
            CliTargetVersionMode::LatestOnly
        );
        assert_eq!(
            vendor.target_mode_on(CliDistributionAction::Downgrade, CliPlatform::Linux),
            CliTargetVersionMode::Unsupported
        );

        assert_eq!(
            narrower(
                CliTargetVersionMode::Exact,
                CliTargetVersionMode::LatestOnly
            ),
            CliTargetVersionMode::LatestOnly
        );
        assert_eq!(
            narrower(CliTargetVersionMode::Exact, CliTargetVersionMode::Exact),
            CliTargetVersionMode::Exact
        );
        assert_eq!(
            narrower(
                CliTargetVersionMode::LatestOnly,
                CliTargetVersionMode::Unsupported
            ),
            CliTargetVersionMode::Unsupported
        );
    }

    #[test]
    fn npm_reaches_every_action_and_winget_withholds_the_unverified_ones() {
        let npm = npm_distribution();
        assert_eq!(
            npm.target_mode_on(CliDistributionAction::Downgrade, CliPlatform::Linux),
            CliTargetVersionMode::Exact
        );
        assert!(npm.capabilities.uninstall);
        assert!(!npm.capabilities.repair.needs_preflight());
        assert!(npm.capabilities.manages_anything());

        assert_eq!(
            WINGET_CAPABILITIES.downgrade,
            CliTargetVersionMode::Unsupported
        );
        assert_eq!(
            WINGET_CAPABILITIES.reinstall,
            CliTargetVersionMode::Unsupported
        );
        assert_eq!(WINGET_CAPABILITIES.upgrade, CliTargetVersionMode::Exact);
        assert!(WINGET_CAPABILITIES.repair.needs_preflight());
    }

    #[test]
    fn mutation_keys_follow_the_resource_the_source_writes() {
        assert_eq!(
            npm_distribution().mutation_key("codex-cli"),
            CliMutationKey::npm_global()
        );
        // Two CLIs installed through npm contend even though they are different tools.
        assert_eq!(
            npm_distribution().mutation_key("gemini-cli"),
            npm_distribution().mutation_key("codex-cli")
        );
        assert_eq!(
            vendor_distribution().mutation_key("opencode"),
            CliMutationKey::vendor("opencode")
        );
    }

    #[test]
    fn stable_is_the_default_channel_and_a_channelless_source_has_none() {
        assert_eq!(
            npm_distribution().default_channel(),
            Some(CliReleaseChannel::STABLE)
        );
        let channelless = CliDistributionDefinition {
            channels: &[],
            ..npm_distribution()
        };
        assert_eq!(channelless.default_channel(), None);
    }

    #[test]
    fn actionability_here_follows_the_build_target() {
        let vendor = vendor_distribution();
        // Only Linux has a template, so this is true on exactly one platform.
        assert_eq!(vendor.is_actionable_here(), cfg!(target_os = "linux"));
        // A package manager needs no template and is actionable on every modelled platform.
        assert_eq!(
            npm_distribution().is_actionable_here(),
            CliPlatform::current().is_some()
        );
    }

    #[test]
    fn actionable_distributions_hide_the_ones_this_host_cannot_use() {
        const DISTRIBUTIONS: &[CliDistributionDefinition] = &[
            CliDistributionDefinition {
                source_id: "npm",
                kind: CliSourceKind::Npm,
                package_reference: Some(CliPackageReference {
                    identifier: "@openai/codex",
                }),
                platforms: PlatformSet::ALL,
                capabilities: NPM_CAPABILITIES,
                channels: STABLE_CHANNEL_ONLY,
                trust: CliSourceTrustPolicy::PackageManager,
            },
            CliDistributionDefinition {
                source_id: "winget",
                kind: CliSourceKind::Winget,
                package_reference: Some(CliPackageReference {
                    identifier: "Fixture.Tool",
                }),
                platforms: PlatformSet::WINDOWS_ONLY,
                capabilities: WINGET_CAPABILITIES,
                channels: STABLE_CHANNEL_ONLY,
                trust: CliSourceTrustPolicy::PackageManager,
            },
        ];
        let tool = CliToolDefinition {
            agent_id: "fixture-cli",
            display_name: "Fixture CLI",
            provider: "Fixture",
            executable_names: &["fixture"],
            distributions: DISTRIBUTIONS,
            probes: CliProbeDefinition::version_only(),
            compatibility: CliCompatibilityPolicy::any_desktop(),
        };

        let actionable = tool
            .actionable_distributions()
            .map(|distribution| distribution.source_id)
            .collect::<Vec<_>>();
        if cfg!(target_os = "windows") {
            assert_eq!(actionable, vec!["npm", "winget"]);
        } else {
            // WinGet is declared but unreachable here, so it is not offered as an action.
            assert_eq!(actionable, vec!["npm"]);
        }

        assert_eq!(
            tool.tool_id().map(|id| id.to_string()).as_deref(),
            Ok("fixture-cli")
        );
        assert!(tool.distribution("missing").is_none());
        assert_eq!(
            tool.distribution_of_kind(CliSourceKind::Winget)
                .map(|distribution| distribution.source_id),
            Some("winget")
        );
        assert!(tool.distribution_of_kind(CliSourceKind::Homebrew).is_none());
    }

    #[test]
    fn a_compatibility_floor_never_judges_an_opaque_version() {
        let policy = CliCompatibilityPolicy {
            minimum_supported: Some("1.2.0"),
            platforms: PlatformSet::ALL,
        };
        assert_eq!(
            policy.is_below_floor(&NormalizedCliVersion::parse("1.1.9")),
            Some(true)
        );
        assert_eq!(
            policy.is_below_floor(&NormalizedCliVersion::parse("1.2.0")),
            Some(false)
        );
        assert_eq!(
            policy.is_below_floor(&NormalizedCliVersion::parse("2.0.0")),
            Some(false)
        );
        // Opaque: cannot tell, so it must not be flagged as outdated.
        assert_eq!(
            policy.is_below_floor(&NormalizedCliVersion::parse("nightly")),
            None
        );
        // No declared floor: also cannot tell.
        assert_eq!(
            CliCompatibilityPolicy::any_desktop()
                .is_below_floor(&NormalizedCliVersion::parse("0.0.1")),
            None
        );
    }
}
