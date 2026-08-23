//! What a distribution source *is* and what it can actually do.
//!
//! The model this replaces had one `LifecycleEligibility` enum whose variants mixed a package
//! manager (`Npm`, `Winget`), a download transport (`Wget`), and an absence of both (`Manual`,
//! `Unavailable`). That shape cannot answer "can this source install an exact version" without a
//! conditional somewhere in the UI, and it is why npm catalog data ended up deciding the update
//! state of a WinGet installation.
//!
//! Capabilities here are data carried by the source definition, never a UI conditional.

use super::ids::CliSourceId;
use super::version::NormalizedCliVersion;

/// The distribution mechanism that owns, or most likely owns, an installation.
///
/// `wget`, `curl`, and `Invoke-WebRequest` are deliberately absent: they are transports used *by*
/// a vendor installer, not sources that own a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CliSourceKind {
    Npm,
    Winget,
    /// An audited, platform-specific installer published by the CLI's vendor.
    VendorInstaller,
    Homebrew,
    Bun,
    Volta,
    /// Bundled inside a desktop application's install tree.
    Desktop,
    /// A system package manager or a system path such as `/usr/bin`.
    System,
    /// Present and runnable, but placed by hand. Healthy, just not ours to mutate.
    Manual,
    Unknown,
}

impl CliSourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Winget => "winget",
            Self::VendorInstaller => "vendor-installer",
            Self::Homebrew => "homebrew",
            Self::Bun => "bun",
            Self::Volta => "volta",
            Self::Desktop => "desktop",
            Self::System => "system",
            Self::Manual => "manual",
            Self::Unknown => "unknown",
        }
    }

    /// Whether this change ships lifecycle management for the source at all. Detect-only is a
    /// statement about VaneHub's capability, never about the installation's health -- a manually
    /// installed CLI that runs fine is healthy and detect-only at the same time.
    pub(crate) fn is_detect_only(self) -> bool {
        !matches!(self, Self::Npm | Self::Winget | Self::VendorInstaller)
    }

    /// What to tell the user about a source VaneHub can see but not drive.
    ///
    /// A stable code, localized by the UI. `None` for the three managed sources: there is nothing
    /// to explain when the action is simply offered.
    ///
    /// Each one names the tool that *does* own the installation, because the useful answer to
    /// "why is there no upgrade button" is "run `brew upgrade`", not "unsupported".
    pub(crate) fn guidance_code(self) -> Option<&'static str> {
        match self {
            Self::Npm | Self::Winget | Self::VendorInstaller => None,
            Self::Homebrew => Some("cli.guidance.homebrew"),
            Self::Bun => Some("cli.guidance.bun"),
            Self::Volta => Some("cli.guidance.volta"),
            Self::Desktop => Some("cli.guidance.desktop"),
            Self::System => Some("cli.guidance.system"),
            Self::Manual => Some("cli.guidance.manual"),
            Self::Unknown => Some("cli.guidance.unknown"),
        }
    }
}

/// Whether VaneHub can drive a source, or only report what it sees.
///
/// Carried on the summary so no UI infers it from the source name. A source added later would
/// otherwise be classified by whichever component remembered to update its own list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliSourceManagement {
    Managed,
    DetectOnly,
}

impl CliSourceManagement {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::DetectOnly => "detect-only",
        }
    }

    pub(crate) fn of(kind: CliSourceKind) -> Self {
        if kind.is_detect_only() {
            Self::DetectOnly
        } else {
            Self::Managed
        }
    }
}

/// How certain the backend is that an installation came from a given source.
///
/// A path heuristic is never `Verified`. Recording that distinction is what stops
/// "`/usr/local/bin` looks npm-ish" from authorizing an npm mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CliSourceConfidence {
    Unknown,
    /// Derived from a path shape or a neighbouring file.
    Inferred,
    /// The source itself confirmed ownership, e.g. the package manager lists this package.
    Verified,
}

impl CliSourceConfidence {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Inferred => "inferred",
            Self::Verified => "verified",
        }
    }
}

/// Whether a source can aim an action at a specific version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliTargetVersionMode {
    /// The source cannot perform this action at all.
    Unsupported,
    /// The action runs, but only at whatever the source considers latest. A caller must not label
    /// the result with a requested version it did not honour.
    LatestOnly,
    /// The action accepts and honours an exact version.
    Exact,
}

impl CliTargetVersionMode {
    pub(crate) fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    pub(crate) fn accepts_exact_target(self) -> bool {
        matches!(self, Self::Exact)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::LatestOnly => "latest-only",
            Self::Exact => "exact",
        }
    }
}

/// A capability whose availability cannot be known from static data.
///
/// WinGet `repair` is the case that motivates it: whether a given installed package supports
/// repair depends on the package and the local WinGet build, so it is resolved during planning and
/// recorded in the plan rather than advertised up front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliDynamicCapability {
    Unsupported,
    RequiresPreflight,
}

impl CliDynamicCapability {
    pub(crate) fn needs_preflight(self) -> bool {
        matches!(self, Self::RequiresPreflight)
    }

    /// `requires-preflight` is not `supported`: whether the action can run here is only known
    /// after asking the source, so the UI must not present it as available up front.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::RequiresPreflight => "requires-preflight",
        }
    }
}

/// Which lifecycle actions a source supports, and at what version granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliSourceCapabilities {
    pub(crate) install: CliTargetVersionMode,
    pub(crate) upgrade: CliTargetVersionMode,
    pub(crate) downgrade: CliTargetVersionMode,
    pub(crate) reinstall: CliTargetVersionMode,
    pub(crate) uninstall: bool,
    pub(crate) repair: CliDynamicCapability,
}

impl CliSourceCapabilities {
    /// Every detect-only source. Spelled out once so a new detect-only source cannot accidentally
    /// inherit a mutating capability from a copied literal.
    pub(crate) const DETECT_ONLY: Self = Self {
        install: CliTargetVersionMode::Unsupported,
        upgrade: CliTargetVersionMode::Unsupported,
        downgrade: CliTargetVersionMode::Unsupported,
        reinstall: CliTargetVersionMode::Unsupported,
        uninstall: false,
        repair: CliDynamicCapability::Unsupported,
    };

    pub(crate) fn manages_anything(&self) -> bool {
        self.install.is_supported()
            || self.upgrade.is_supported()
            || self.downgrade.is_supported()
            || self.reinstall.is_supported()
            || self.uninstall
            || self.repair.needs_preflight()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CliPlatform {
    Windows,
    Macos,
    Linux,
}

impl CliPlatform {
    /// The platform this build runs on, or `None` on a target the CLI domain does not model.
    /// `None` means "no platform-specific behaviour is authorized", never "assume Linux".
    pub(crate) fn current() -> Option<Self> {
        if cfg!(target_os = "windows") {
            Some(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }
}

/// The platforms a distribution is declared to support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlatformSet(&'static [CliPlatform]);

impl PlatformSet {
    pub(crate) const ALL: Self =
        Self(&[CliPlatform::Windows, CliPlatform::Macos, CliPlatform::Linux]);
    pub(crate) const WINDOWS_ONLY: Self = Self(&[CliPlatform::Windows]);
    pub(crate) const UNIX: Self = Self(&[CliPlatform::Macos, CliPlatform::Linux]);

    pub(crate) const fn of(platforms: &'static [CliPlatform]) -> Self {
        Self(platforms)
    }

    pub(crate) fn contains(self, platform: CliPlatform) -> bool {
        // `contains` over a 3-element slice; a set type would cost more than it saves.
        let mut index = 0;
        while index < self.0.len() {
            if matches!(
                (self.0[index], platform),
                (CliPlatform::Windows, CliPlatform::Windows)
                    | (CliPlatform::Macos, CliPlatform::Macos)
                    | (CliPlatform::Linux, CliPlatform::Linux)
            ) {
                return true;
            }
            index += 1;
        }
        false
    }

    /// Whether this distribution is usable on the platform the process is running on. An unmodelled
    /// target is not supported, so nothing is offered there.
    pub(crate) fn supports_current_platform(self) -> bool {
        CliPlatform::current().is_some_and(|platform| self.contains(platform))
    }

    pub(crate) fn as_slice(self) -> &'static [CliPlatform] {
        self.0
    }
}

/// The resource a mutation serializes against.
///
/// Two upgrades that both write the global npm prefix must not interleave even though they target
/// different CLIs, so the key is the *resource*, not the tool.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CliMutationKey(String);

impl CliMutationKey {
    pub(crate) const NPM_GLOBAL: &'static str = "npm-global";
    pub(crate) const WINGET: &'static str = "winget";

    pub(crate) fn npm_global() -> Self {
        Self(Self::NPM_GLOBAL.to_string())
    }

    pub(crate) fn winget() -> Self {
        Self(Self::WINGET.to_string())
    }

    /// A vendor installer writes only its own tool's install tree, so it serializes per tool
    /// rather than globally.
    pub(crate) fn vendor(agent_id: &str) -> Self {
        Self(format!("vendor:{agent_id}"))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A release channel a source exposes. `stable` is the default wherever channels exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliReleaseChannel {
    pub(crate) id: &'static str,
    pub(crate) is_default: bool,
}

impl CliReleaseChannel {
    pub(crate) const STABLE: Self = Self {
        id: "stable",
        is_default: true,
    };
}

/// A source summary as it appears on a snapshot: what this source is and what it could do for
/// this tool right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliSourceSummary {
    pub(crate) source_id: CliSourceId,
    pub(crate) kind: CliSourceKind,
    pub(crate) capabilities: CliSourceCapabilities,
    pub(crate) supported_on_this_platform: bool,
    /// Present only when the source itself was reachable and reported one.
    pub(crate) available_version_count: Option<usize>,
    /// Whether VaneHub drives this source or only reports it.
    pub(crate) management: CliSourceManagement,
    /// What to tell the user when it does not. `None` for a managed source.
    pub(crate) guidance_code: Option<&'static str>,
    /// The versions this source offers, newest first, as its own catalog reported them.
    ///
    /// Carried on the summary so a target selector reads the source's list rather than a frontend
    /// reconstruction. Empty when the catalog was unavailable, which is distinct from a source
    /// that genuinely offers nothing -- `available_version_count` tells the two apart.
    pub(crate) available_versions: Vec<NormalizedCliVersion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transports_are_not_modelled_as_sources() {
        // The removed `LifecycleEligibility::Wget` named a download tool. The source that owns a
        // script-installed CLI is its vendor; wget and curl are how the file arrives.
        let names = [
            CliSourceKind::Npm,
            CliSourceKind::Winget,
            CliSourceKind::VendorInstaller,
            CliSourceKind::Homebrew,
            CliSourceKind::Bun,
            CliSourceKind::Volta,
            CliSourceKind::Desktop,
            CliSourceKind::System,
            CliSourceKind::Manual,
            CliSourceKind::Unknown,
        ]
        .map(CliSourceKind::as_str);

        assert!(!names.contains(&"wget"));
        assert!(!names.contains(&"curl"));
        assert!(!names.contains(&"powershell"));
        assert!(names.contains(&"vendor-installer"));
    }

    #[test]
    fn only_the_three_managed_sources_are_not_detect_only() {
        for kind in [
            CliSourceKind::Npm,
            CliSourceKind::Winget,
            CliSourceKind::VendorInstaller,
        ] {
            assert!(
                !kind.is_detect_only(),
                "{} must be manageable",
                kind.as_str()
            );
        }
        for kind in [
            CliSourceKind::Homebrew,
            CliSourceKind::Bun,
            CliSourceKind::Volta,
            CliSourceKind::Desktop,
            CliSourceKind::System,
            CliSourceKind::Manual,
            CliSourceKind::Unknown,
        ] {
            assert!(
                kind.is_detect_only(),
                "{} must be detect-only",
                kind.as_str()
            );
        }
    }

    #[test]
    fn detect_only_capabilities_grant_nothing() {
        let capabilities = CliSourceCapabilities::DETECT_ONLY;
        assert!(!capabilities.manages_anything());
        assert!(!capabilities.install.is_supported());
        assert!(!capabilities.uninstall);
        assert!(!capabilities.repair.needs_preflight());
    }

    #[test]
    fn latest_only_is_supported_but_refuses_an_exact_target() {
        // The distinction that matters: a vendor script runs, but it installs whatever it defaults
        // to. Reporting success under a requested version it did not honour is the defect.
        assert!(CliTargetVersionMode::LatestOnly.is_supported());
        assert!(!CliTargetVersionMode::LatestOnly.accepts_exact_target());
        assert!(CliTargetVersionMode::Exact.accepts_exact_target());
        assert!(!CliTargetVersionMode::Unsupported.is_supported());
        assert!(!CliTargetVersionMode::Unsupported.accepts_exact_target());
    }

    #[test]
    fn confidence_orders_from_unknown_to_verified() {
        assert!(CliSourceConfidence::Verified > CliSourceConfidence::Inferred);
        assert!(CliSourceConfidence::Inferred > CliSourceConfidence::Unknown);
    }

    #[test]
    fn platform_sets_answer_membership_without_assuming_a_default() {
        assert!(PlatformSet::WINDOWS_ONLY.contains(CliPlatform::Windows));
        assert!(!PlatformSet::WINDOWS_ONLY.contains(CliPlatform::Linux));
        assert!(!PlatformSet::WINDOWS_ONLY.contains(CliPlatform::Macos));
        assert!(PlatformSet::UNIX.contains(CliPlatform::Macos));
        assert!(PlatformSet::UNIX.contains(CliPlatform::Linux));
        assert!(!PlatformSet::UNIX.contains(CliPlatform::Windows));
        assert!(PlatformSet::ALL.contains(CliPlatform::Windows));

        let empty = PlatformSet::of(&[]);
        assert!(!empty.supports_current_platform());
    }

    #[test]
    fn the_current_platform_is_supported_by_all_and_not_by_the_empty_set() {
        // Whichever host runs the suite, ALL covers it and the empty set does not.
        assert!(PlatformSet::ALL.supports_current_platform());
        assert!(!PlatformSet::of(&[]).supports_current_platform());
        // WINDOWS_ONLY agrees with the build target rather than with a hardcoded expectation.
        assert_eq!(
            PlatformSet::WINDOWS_ONLY.supports_current_platform(),
            cfg!(target_os = "windows")
        );
    }

    #[test]
    fn mutation_keys_separate_shared_resources_from_per_tool_ones() {
        // Two npm installs contend even for different CLIs: they write one global prefix.
        assert_eq!(CliMutationKey::npm_global(), CliMutationKey::npm_global());
        assert_ne!(CliMutationKey::npm_global(), CliMutationKey::winget());
        // Two vendor installs for different CLIs touch different trees and may run concurrently.
        assert_ne!(
            CliMutationKey::vendor("claude-code"),
            CliMutationKey::vendor("opencode")
        );
        assert_eq!(
            CliMutationKey::vendor("claude-code").as_str(),
            "vendor:claude-code"
        );
    }

    #[test]
    fn every_source_value_has_a_stable_wire_string() {
        assert_eq!(CliSourceConfidence::Unknown.as_str(), "unknown");
        assert_eq!(CliSourceConfidence::Inferred.as_str(), "inferred");
        assert_eq!(CliSourceConfidence::Verified.as_str(), "verified");
        assert_eq!(CliTargetVersionMode::Unsupported.as_str(), "unsupported");
        assert_eq!(CliTargetVersionMode::LatestOnly.as_str(), "latest-only");
        assert_eq!(CliTargetVersionMode::Exact.as_str(), "exact");
        assert_eq!(CliPlatform::Windows.as_str(), "windows");
        assert_eq!(CliPlatform::Macos.as_str(), "macos");
        assert_eq!(CliPlatform::Linux.as_str(), "linux");
    }

    #[test]
    fn a_platform_set_can_be_enumerated_for_display() {
        assert_eq!(
            PlatformSet::ALL.as_slice(),
            &[CliPlatform::Windows, CliPlatform::Macos, CliPlatform::Linux]
        );
        assert_eq!(
            PlatformSet::WINDOWS_ONLY.as_slice(),
            &[CliPlatform::Windows]
        );
        assert!(PlatformSet::of(&[]).as_slice().is_empty());
    }

    #[test]
    fn a_source_summary_reports_capability_and_reach_separately_from_health() {
        let summary = CliSourceSummary {
            source_id: CliSourceId::new("npm").expect("source id"),
            kind: CliSourceKind::Npm,
            capabilities: CliSourceCapabilities {
                install: CliTargetVersionMode::Exact,
                upgrade: CliTargetVersionMode::Exact,
                downgrade: CliTargetVersionMode::Exact,
                reinstall: CliTargetVersionMode::Exact,
                uninstall: true,
                repair: CliDynamicCapability::Unsupported,
            },
            supported_on_this_platform: true,
            available_version_count: Some(42),
            management: CliSourceManagement::Managed,
            guidance_code: None,
            available_versions: Vec::new(),
        };
        assert!(summary.capabilities.manages_anything());
        assert_eq!(summary.available_version_count, Some(42));

        // A source that was never reached reports no count rather than zero: "zero versions
        // available" and "we could not ask" are different answers.
        let unreached = CliSourceSummary {
            capabilities: CliSourceCapabilities::DETECT_ONLY,
            supported_on_this_platform: false,
            available_version_count: None,
            ..summary
        };
        assert!(!unreached.capabilities.manages_anything());
        assert_eq!(unreached.available_version_count, None);
        assert_eq!(unreached.kind, CliSourceKind::Npm);
    }

    #[test]
    fn stable_is_the_default_channel() {
        // Resolved the way a distribution resolves it, so the test breaks if `STABLE` ever stops
        // being the one a channel list defaults to.
        let channels = [CliReleaseChannel::STABLE];
        let default = channels.iter().find(|channel| channel.is_default);
        assert_eq!(default.map(|channel| channel.id), Some("stable"));
    }
}
