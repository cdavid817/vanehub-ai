//! Orthogonal facts about a CLI, and the single derived state the UI groups by.
//!
//! These are separate axes on purpose. "Installed", "runs", "logged in", "up to date", and
//! "VaneHub can change it" are five different questions, and the flat model that answered them
//! with one enum is why a healthy hand-installed CLI rendered as broken: the only thing wrong with
//! it was that VaneHub could not mutate it.
//!
//! `overall_state` exists for counting and sorting. It never replaces the axes -- the UI shows
//! both, so a tool summarised as `UpdateAvailable` can still display that it needs a login.

/// How many installations discovery found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliDiscoveryStatus {
    /// No scan has run yet. Distinct from `NotFound`: nothing has been ruled out.
    NotScanned,
    NotFound,
    FoundOne,
    FoundMultiple,
}

impl CliDiscoveryStatus {
    pub(crate) fn from_count(count: usize) -> Self {
        match count {
            0 => Self::NotFound,
            1 => Self::FoundOne,
            _ => Self::FoundMultiple,
        }
    }

    pub(crate) fn is_installed(self) -> bool {
        matches!(self, Self::FoundOne | Self::FoundMultiple)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotScanned => "not-scanned",
            Self::NotFound => "not-found",
            Self::FoundOne => "found-one",
            Self::FoundMultiple => "found-multiple",
        }
    }
}

/// Whether the active executable actually runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliExecutableStatus {
    /// Nothing is installed, so there is no executable to judge. Not a failure.
    NotApplicable,
    Healthy,
    /// Ran and failed, or is not a runnable image.
    Broken,
    TimedOut,
    PermissionDenied,
    UnsupportedArchitecture,
    /// Present but not probed -- for example a shadowed installation discovery listed without
    /// spending a probe on it.
    Unknown,
}

impl CliExecutableStatus {
    pub(crate) fn is_runnable(self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Whether this status is an actual fault. `Unknown` and `NotApplicable` are not: reporting
    /// "we did not check" as broken is the presentation bug this separation removes.
    pub(crate) fn is_faulty(self) -> bool {
        matches!(
            self,
            Self::Broken | Self::TimedOut | Self::PermissionDenied | Self::UnsupportedArchitecture
        )
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::Healthy => "healthy",
            Self::Broken => "broken",
            Self::TimedOut => "timeout",
            Self::PermissionDenied => "permission-denied",
            Self::UnsupportedArchitecture => "unsupported-architecture",
            Self::Unknown => "unknown",
        }
    }
}

/// Whether the provider considers this CLI logged in.
///
/// `Unknown` is the honest default. VaneHub never reads, stores, or proxies a credential, and a
/// CLI with no documented non-interactive probe stays `Unknown` rather than being assumed ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliAuthenticationStatus {
    Authenticated,
    Required,
    Expired,
    Unknown,
    /// The CLI has no account model at all.
    NotApplicable,
}

impl CliAuthenticationStatus {
    pub(crate) fn blocks_use(self) -> bool {
        matches!(self, Self::Required | Self::Expired)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Authenticated => "authenticated",
            Self::Required => "required",
            Self::Expired => "expired",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not-applicable",
        }
    }
}

/// Whether the CLI is usable right now, derived from the axes above plus dependency and Doctor
/// results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliReadinessStatus {
    Ready,
    NeedsAuth,
    MissingDependency,
    Misconfigured,
    Broken,
    Unknown,
}

impl CliReadinessStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsAuth => "needs-auth",
            Self::MissingDependency => "missing-dependency",
            Self::Misconfigured => "misconfigured",
            Self::Broken => "broken",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliCompatibilityStatus {
    Supported,
    UnsupportedVersion,
    UnsupportedPlatform,
    Unknown,
}

impl CliCompatibilityStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::UnsupportedVersion => "unsupported-version",
            Self::UnsupportedPlatform => "unsupported-platform",
            Self::Unknown => "unknown",
        }
    }
}

/// How the active version compares with the catalog of the source that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliUpdateStatus {
    /// No source can tell us about updates -- a detect-only installation, for instance. This is
    /// not a failure and must never render as one.
    NotApplicable,
    UpToDate,
    Available,
    /// Installed version is newer than the catalog's latest. Common on a prerelease channel; it is
    /// not an update prompt.
    Ahead,
    /// A catalog exists but could not be read. Distinct from `NotApplicable`: retrying may help.
    CatalogUnavailable,
    Unknown,
}

impl CliUpdateStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not-applicable",
            Self::UpToDate => "up-to-date",
            Self::Available => "available",
            Self::Ahead => "ahead",
            Self::CatalogUnavailable => "catalog-unavailable",
            Self::Unknown => "unknown",
        }
    }
}

/// How old the snapshot is. Cached data stays visible while refreshing; it is labelled, not hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliFreshness {
    Never,
    Fresh,
    Stale,
    Refreshing,
}

impl CliFreshness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Refreshing => "refreshing",
        }
    }
}

/// The single state the UI groups and counts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOverallState {
    Broken,
    Conflict,
    NeedsAuth,
    UpdateAvailable,
    Ready,
    Missing,
    Unknown,
}

impl CliOverallState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Broken => "broken",
            Self::Conflict => "conflict",
            Self::NeedsAuth => "needs-auth",
            Self::UpdateAvailable => "update-available",
            Self::Ready => "ready",
            Self::Missing => "missing",
            Self::Unknown => "unknown",
        }
    }
}

/// The axes `derive_overall_state` reads. Grouped into one struct so the precedence rule has a
/// single, testable signature rather than seven positional arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliStatusAxes {
    pub(crate) discovery: CliDiscoveryStatus,
    pub(crate) executable: CliExecutableStatus,
    pub(crate) authentication: CliAuthenticationStatus,
    pub(crate) compatibility: CliCompatibilityStatus,
    pub(crate) update: CliUpdateStatus,
    pub(crate) has_conflict: bool,
}

/// Documented precedence:
///
/// ```text
/// broken > conflict > needs-auth > update-available > ready > missing > unknown
/// ```
///
/// Missing ranks *below* ready because a tool that is simply not installed is a normal state, not
/// a problem to escalate above one that needs attention.
pub(crate) fn derive_overall_state(axes: CliStatusAxes) -> CliOverallState {
    if !axes.discovery.is_installed() {
        return match axes.discovery {
            CliDiscoveryStatus::NotFound => CliOverallState::Missing,
            // Never scanned: nothing has been established, so claiming "missing" would be a
            // finding VaneHub has not made.
            CliDiscoveryStatus::NotScanned => CliOverallState::Unknown,
            CliDiscoveryStatus::FoundOne | CliDiscoveryStatus::FoundMultiple => {
                CliOverallState::Unknown
            }
        };
    }
    if axes.executable.is_faulty()
        || axes.compatibility == CliCompatibilityStatus::UnsupportedPlatform
    {
        return CliOverallState::Broken;
    }
    if axes.has_conflict {
        return CliOverallState::Conflict;
    }
    if axes.authentication.blocks_use() {
        return CliOverallState::NeedsAuth;
    }
    if axes.update == CliUpdateStatus::Available {
        return CliOverallState::UpdateAvailable;
    }
    if axes.executable.is_runnable() {
        return CliOverallState::Ready;
    }
    // Installed, not faulty, but never probed.
    CliOverallState::Unknown
}

/// Readiness from the axes plus the two results only probes can supply.
pub(crate) fn derive_readiness(
    axes: CliStatusAxes,
    missing_dependency: bool,
    doctor_reported_problem: bool,
) -> CliReadinessStatus {
    if !axes.discovery.is_installed() || axes.executable.is_faulty() {
        return CliReadinessStatus::Broken;
    }
    if missing_dependency {
        return CliReadinessStatus::MissingDependency;
    }
    if axes.authentication.blocks_use() {
        return CliReadinessStatus::NeedsAuth;
    }
    if doctor_reported_problem || axes.compatibility == CliCompatibilityStatus::UnsupportedVersion {
        return CliReadinessStatus::Misconfigured;
    }
    if axes.executable.is_runnable() {
        return CliReadinessStatus::Ready;
    }
    CliReadinessStatus::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_axes() -> CliStatusAxes {
        CliStatusAxes {
            discovery: CliDiscoveryStatus::FoundOne,
            executable: CliExecutableStatus::Healthy,
            authentication: CliAuthenticationStatus::Authenticated,
            compatibility: CliCompatibilityStatus::Supported,
            update: CliUpdateStatus::UpToDate,
            has_conflict: false,
        }
    }

    #[test]
    fn discovery_counts_map_to_installed_state() {
        assert_eq!(
            CliDiscoveryStatus::from_count(0),
            CliDiscoveryStatus::NotFound
        );
        assert_eq!(
            CliDiscoveryStatus::from_count(1),
            CliDiscoveryStatus::FoundOne
        );
        assert_eq!(
            CliDiscoveryStatus::from_count(4),
            CliDiscoveryStatus::FoundMultiple
        );
        assert!(!CliDiscoveryStatus::NotScanned.is_installed());
        assert!(!CliDiscoveryStatus::NotFound.is_installed());
        assert!(CliDiscoveryStatus::FoundOne.is_installed());
    }

    #[test]
    fn not_checked_is_never_a_fault() {
        // The distinction the flat model lost: "we did not probe it" is not "it is broken".
        assert!(!CliExecutableStatus::Unknown.is_faulty());
        assert!(!CliExecutableStatus::NotApplicable.is_faulty());
        assert!(CliExecutableStatus::Broken.is_faulty());
        assert!(CliExecutableStatus::TimedOut.is_faulty());
        assert!(CliExecutableStatus::PermissionDenied.is_faulty());
        assert!(CliExecutableStatus::UnsupportedArchitecture.is_faulty());
        assert!(!CliExecutableStatus::Unknown.is_runnable());
    }

    #[test]
    fn unknown_authentication_does_not_block_use() {
        // A CLI VaneHub cannot probe is not thereby locked out; it is simply unknown.
        assert!(!CliAuthenticationStatus::Unknown.blocks_use());
        assert!(!CliAuthenticationStatus::NotApplicable.blocks_use());
        assert!(!CliAuthenticationStatus::Authenticated.blocks_use());
        assert!(CliAuthenticationStatus::Required.blocks_use());
        assert!(CliAuthenticationStatus::Expired.blocks_use());
    }

    #[test]
    fn overall_state_follows_the_documented_precedence() {
        // broken outranks everything else that is also true
        let broken = CliStatusAxes {
            executable: CliExecutableStatus::Broken,
            authentication: CliAuthenticationStatus::Required,
            update: CliUpdateStatus::Available,
            has_conflict: true,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(broken), CliOverallState::Broken);

        // conflict outranks needs-auth and update-available
        let conflict = CliStatusAxes {
            discovery: CliDiscoveryStatus::FoundMultiple,
            authentication: CliAuthenticationStatus::Required,
            update: CliUpdateStatus::Available,
            has_conflict: true,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(conflict), CliOverallState::Conflict);

        // needs-auth outranks update-available
        let needs_auth = CliStatusAxes {
            authentication: CliAuthenticationStatus::Expired,
            update: CliUpdateStatus::Available,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(needs_auth), CliOverallState::NeedsAuth);

        // update-available outranks ready
        let update = CliStatusAxes {
            update: CliUpdateStatus::Available,
            ..healthy_axes()
        };
        assert_eq!(
            derive_overall_state(update),
            CliOverallState::UpdateAvailable
        );

        assert_eq!(derive_overall_state(healthy_axes()), CliOverallState::Ready);
    }

    #[test]
    fn a_healthy_detect_only_installation_is_ready_not_broken() {
        // Nothing about this tool is wrong; VaneHub simply cannot mutate it. Manageability is not
        // an axis of `overall_state` at all, which is what stops the false "broken" rendering.
        let detect_only = CliStatusAxes {
            update: CliUpdateStatus::NotApplicable,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(detect_only), CliOverallState::Ready);

        let catalog_down = CliStatusAxes {
            update: CliUpdateStatus::CatalogUnavailable,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(catalog_down), CliOverallState::Ready);
    }

    #[test]
    fn a_version_ahead_of_the_catalog_is_not_an_update_prompt() {
        let ahead = CliStatusAxes {
            update: CliUpdateStatus::Ahead,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(ahead), CliOverallState::Ready);
    }

    #[test]
    fn a_missing_tool_ranks_below_ready_and_an_unscanned_one_is_unknown() {
        let missing = CliStatusAxes {
            discovery: CliDiscoveryStatus::NotFound,
            executable: CliExecutableStatus::NotApplicable,
            authentication: CliAuthenticationStatus::Unknown,
            update: CliUpdateStatus::NotApplicable,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(missing), CliOverallState::Missing);

        let unscanned = CliStatusAxes {
            discovery: CliDiscoveryStatus::NotScanned,
            executable: CliExecutableStatus::Unknown,
            authentication: CliAuthenticationStatus::Unknown,
            compatibility: CliCompatibilityStatus::Unknown,
            update: CliUpdateStatus::Unknown,
            has_conflict: false,
        };
        assert_eq!(derive_overall_state(unscanned), CliOverallState::Unknown);
    }

    #[test]
    fn an_unsupported_platform_is_broken_but_an_unsupported_version_is_not() {
        let wrong_platform = CliStatusAxes {
            compatibility: CliCompatibilityStatus::UnsupportedPlatform,
            ..healthy_axes()
        };
        assert_eq!(
            derive_overall_state(wrong_platform),
            CliOverallState::Broken
        );

        // An old-but-running CLI is usable; it surfaces through the compatibility axis and
        // readiness, not by being called broken.
        let old_version = CliStatusAxes {
            compatibility: CliCompatibilityStatus::UnsupportedVersion,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(old_version), CliOverallState::Ready);
        assert_eq!(
            derive_readiness(old_version, false, false),
            CliReadinessStatus::Misconfigured
        );
    }

    #[test]
    fn readiness_ranks_dependency_and_auth_ahead_of_doctor_findings() {
        assert_eq!(
            derive_readiness(healthy_axes(), false, false),
            CliReadinessStatus::Ready
        );
        assert_eq!(
            derive_readiness(healthy_axes(), true, true),
            CliReadinessStatus::MissingDependency
        );
        let needs_auth = CliStatusAxes {
            authentication: CliAuthenticationStatus::Required,
            ..healthy_axes()
        };
        assert_eq!(
            derive_readiness(needs_auth, false, true),
            CliReadinessStatus::NeedsAuth
        );
        assert_eq!(
            derive_readiness(healthy_axes(), false, true),
            CliReadinessStatus::Misconfigured
        );
    }

    #[test]
    fn readiness_is_broken_when_nothing_is_installed_or_the_binary_faults() {
        let missing = CliStatusAxes {
            discovery: CliDiscoveryStatus::NotFound,
            executable: CliExecutableStatus::NotApplicable,
            ..healthy_axes()
        };
        assert_eq!(
            derive_readiness(missing, false, false),
            CliReadinessStatus::Broken
        );

        let faulty = CliStatusAxes {
            executable: CliExecutableStatus::PermissionDenied,
            ..healthy_axes()
        };
        assert_eq!(
            derive_readiness(faulty, false, false),
            CliReadinessStatus::Broken
        );
    }

    #[test]
    fn installed_but_unprobed_is_unknown_on_both_derivations() {
        let unprobed = CliStatusAxes {
            executable: CliExecutableStatus::Unknown,
            authentication: CliAuthenticationStatus::Unknown,
            compatibility: CliCompatibilityStatus::Unknown,
            update: CliUpdateStatus::Unknown,
            ..healthy_axes()
        };
        assert_eq!(derive_overall_state(unprobed), CliOverallState::Unknown);
        assert_eq!(
            derive_readiness(unprobed, false, false),
            CliReadinessStatus::Unknown
        );
    }

    #[test]
    fn every_status_has_a_stable_wire_string() {
        assert_eq!(CliDiscoveryStatus::NotScanned.as_str(), "not-scanned");
        assert_eq!(CliExecutableStatus::TimedOut.as_str(), "timeout");
        assert_eq!(
            CliAuthenticationStatus::NotApplicable.as_str(),
            "not-applicable"
        );
        assert_eq!(CliReadinessStatus::NeedsAuth.as_str(), "needs-auth");
        assert_eq!(
            CliCompatibilityStatus::UnsupportedVersion.as_str(),
            "unsupported-version"
        );
        assert_eq!(
            CliUpdateStatus::CatalogUnavailable.as_str(),
            "catalog-unavailable"
        );
        // Four distinct freshness answers, because the UI does something different with each:
        // `Never` has nothing to show, `Stale` shows cached values with a badge, `Refreshing`
        // keeps them visible while work runs, and `Fresh` needs no qualifier.
        assert_eq!(CliFreshness::Never.as_str(), "never");
        assert_eq!(CliFreshness::Fresh.as_str(), "fresh");
        assert_eq!(CliFreshness::Stale.as_str(), "stale");
        assert_eq!(CliFreshness::Refreshing.as_str(), "refreshing");
        assert_ne!(CliFreshness::Never, CliFreshness::Stale);
        assert_eq!(
            CliOverallState::UpdateAvailable.as_str(),
            "update-available"
        );
    }
}
