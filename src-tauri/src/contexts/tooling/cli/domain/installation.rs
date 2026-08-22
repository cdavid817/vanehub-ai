//! One discovered executable, and the rule that decides which one is actually in charge.
//!
//! "Active" means the one a shell would run, which is decided by PATH order and nothing else. A
//! known-location candidate never outranks an earlier PATH entry because its source is more
//! convenient for VaneHub to manage -- doing that would report a version the user's terminal does
//! not run.

use super::ids::{CliInstallationId, CliSourceId};
use super::source::{CliSourceConfidence, CliSourceKind};
use super::status::CliExecutableStatus;
use super::version::NormalizedCliVersion;

/// Where discovery found a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliEnvironmentOrigin {
    /// Resolved by walking PATH. This is what a shell would actually execute.
    Path,
    /// Found in a bounded, platform-specific location that is not on PATH. Real, useful for
    /// diagnosis, and *not* what runs when the user types the command.
    KnownLocation,
}

impl CliEnvironmentOrigin {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::KnownLocation => "known-location",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliInstallation {
    pub(crate) id: CliInstallationId,
    pub(crate) executable_path: String,
    /// Resolved target when symlinks could be followed. `None` when canonicalization failed, which
    /// is a diagnostic, not a reason to drop the candidate.
    pub(crate) canonical_path: Option<String>,
    pub(crate) reported_version: Option<NormalizedCliVersion>,
    pub(crate) source_id: Option<CliSourceId>,
    pub(crate) source_kind: CliSourceKind,
    pub(crate) source_confidence: CliSourceConfidence,
    /// Position in PATH, lowest first. `None` for a known-location candidate.
    pub(crate) path_priority: Option<u32>,
    pub(crate) environment_origin: CliEnvironmentOrigin,
    pub(crate) executable_status: CliExecutableStatus,
}

impl CliInstallation {
    pub(crate) fn is_runnable(&self) -> bool {
        self.executable_status.is_runnable()
    }

    pub(crate) fn is_on_path(&self) -> bool {
        self.environment_origin == CliEnvironmentOrigin::Path
    }

    /// The identity two candidates are deduplicated by: the canonical target when available, else
    /// the literal path. Two PATH entries that symlink to one binary are one installation.
    pub(crate) fn dedup_key(&self) -> &str {
        self.canonical_path
            .as_deref()
            .unwrap_or(&self.executable_path)
    }
}

/// Which installation is in charge, and whether that is bad news.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveSelection {
    pub(crate) index: usize,
    /// True when the selected installation does not run. It is still the active one -- that is the
    /// diagnosis -- but nothing should be planned against it without saying so.
    pub(crate) is_broken: bool,
}

/// Selects the active installation.
///
/// PATH order decides. The first runnable PATH entry wins; if no PATH entry runs, the first PATH
/// entry is kept as active-but-broken so the user can see *what* is shadowing their working
/// install. Only when nothing is on PATH at all does a known location become active.
pub(crate) fn select_active(installations: &[CliInstallation]) -> Option<ActiveSelection> {
    let path_entries = || {
        installations
            .iter()
            .enumerate()
            .filter(|(_, installation)| installation.is_on_path())
    };

    if let Some((index, _)) = path_entries().find(|(_, installation)| installation.is_runnable()) {
        return Some(ActiveSelection {
            index,
            is_broken: false,
        });
    }
    if let Some((index, _)) = path_entries().min_by_key(|(_, installation)| {
        // Lowest PATH priority is the entry a shell reaches first. `u32::MAX` for a PATH entry
        // with no recorded priority keeps it behind entries that do have one.
        installation.path_priority.unwrap_or(u32::MAX)
    }) {
        return Some(ActiveSelection {
            index,
            is_broken: true,
        });
    }

    // Nothing on PATH. A known location is not what a shell runs, but it is the only thing
    // installed, so it is reported as active rather than leaving the tool with none.
    let runnable_known = installations
        .iter()
        .position(CliInstallation::is_runnable)
        .map(|index| ActiveSelection {
            index,
            is_broken: false,
        });
    runnable_known.or_else(|| {
        (!installations.is_empty()).then_some(ActiveSelection {
            index: 0,
            is_broken: true,
        })
    })
}

/// Removes candidates that resolve to the same binary, keeping the first occurrence so PATH order
/// is preserved.
pub(crate) fn deduplicate(installations: Vec<CliInstallation>) -> Vec<CliInstallation> {
    let mut seen = Vec::new();
    let mut unique = Vec::new();
    for installation in installations {
        let key = installation.dedup_key().to_string();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        unique.push(installation);
    }
    unique
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliConflictKind {
    /// More than one installation exists. On its own this is informational.
    MultipleInstallations,
    /// Installations report different versions, so which one runs changes behaviour.
    VersionMismatch,
    /// Some installations run and some do not.
    RunnableMismatch,
    /// The one PATH would run is broken while a working installation exists further down. This is
    /// the one that actually breaks the user's terminal.
    ActiveIsShadowingWorkingInstall,
}

impl CliConflictKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MultipleInstallations => "multiple-installations",
            Self::VersionMismatch => "version-mismatch",
            Self::RunnableMismatch => "runnable-mismatch",
            Self::ActiveIsShadowingWorkingInstall => "active-shadows-working-install",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliConflict {
    pub(crate) kind: CliConflictKind,
    /// The installations involved, so the details drawer can point at them.
    pub(crate) installations: Vec<CliInstallationId>,
}

/// Every conflict present, most severe first. Empty means a clean environment.
pub(crate) fn derive_conflicts(
    installations: &[CliInstallation],
    active: Option<ActiveSelection>,
) -> Vec<CliConflict> {
    if installations.len() <= 1 {
        return Vec::new();
    }
    let ids = || {
        installations
            .iter()
            .map(|installation| installation.id.clone())
            .collect::<Vec<_>>()
    };
    let mut conflicts = Vec::new();

    let has_working_alternative = installations
        .iter()
        .enumerate()
        .any(|(index, installation)| {
            installation.is_runnable() && active.is_some_and(|selection| selection.index != index)
        });
    if active.is_some_and(|selection| selection.is_broken) && has_working_alternative {
        conflicts.push(CliConflict {
            kind: CliConflictKind::ActiveIsShadowingWorkingInstall,
            installations: ids(),
        });
    }

    let runnable = installations
        .iter()
        .filter(|installation| installation.is_runnable())
        .count();
    if runnable > 0 && runnable < installations.len() {
        conflicts.push(CliConflict {
            kind: CliConflictKind::RunnableMismatch,
            installations: ids(),
        });
    }

    let mut versions = installations
        .iter()
        .filter_map(|installation| installation.reported_version.as_ref())
        .map(NormalizedCliVersion::as_str)
        .collect::<Vec<_>>();
    versions.sort_unstable();
    versions.dedup();
    if versions.len() > 1 {
        conflicts.push(CliConflict {
            kind: CliConflictKind::VersionMismatch,
            installations: ids(),
        });
    }

    conflicts.push(CliConflict {
        kind: CliConflictKind::MultipleInstallations,
        installations: ids(),
    });
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installation(
        id: &str,
        path: &str,
        origin: CliEnvironmentOrigin,
        priority: Option<u32>,
        version: Option<&str>,
        status: CliExecutableStatus,
    ) -> CliInstallation {
        CliInstallation {
            id: CliInstallationId::new(id).expect("installation id"),
            executable_path: path.to_string(),
            canonical_path: None,
            reported_version: version.map(NormalizedCliVersion::parse),
            source_id: None,
            source_kind: CliSourceKind::Unknown,
            source_confidence: CliSourceConfidence::Unknown,
            path_priority: priority,
            environment_origin: origin,
            executable_status: status,
        }
    }

    fn path_install(
        id: &str,
        priority: u32,
        version: Option<&str>,
        status: CliExecutableStatus,
    ) -> CliInstallation {
        installation(
            id,
            &format!("/path/{id}"),
            CliEnvironmentOrigin::Path,
            Some(priority),
            version,
            status,
        )
    }

    fn known_install(
        id: &str,
        version: Option<&str>,
        status: CliExecutableStatus,
    ) -> CliInstallation {
        installation(
            id,
            &format!("/known/{id}"),
            CliEnvironmentOrigin::KnownLocation,
            None,
            version,
            status,
        )
    }

    #[test]
    fn the_first_runnable_path_entry_is_active() {
        let installations = [
            path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            path_install("b", 1, Some("2.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 0);
        assert!(!active.is_broken);
    }

    #[test]
    fn a_broken_first_entry_yields_to_the_next_runnable_path_entry() {
        let installations = [
            path_install("broken", 0, None, CliExecutableStatus::Broken),
            path_install("working", 1, Some("2.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 1);
        assert!(!active.is_broken);
    }

    #[test]
    fn when_no_path_entry_runs_the_first_one_stays_active_but_broken() {
        // Reporting "no active installation" here would hide the thing the user needs to see: the
        // broken binary at the front of their PATH is what their shell runs.
        let installations = [
            path_install("second", 5, None, CliExecutableStatus::TimedOut),
            path_install("first", 1, None, CliExecutableStatus::Broken),
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 1, "lowest PATH priority wins, not list order");
        assert!(active.is_broken);
    }

    #[test]
    fn a_more_manageable_known_location_never_outranks_a_working_path_entry() {
        // The npm-managed copy would be far more convenient to upgrade. It is still not what the
        // terminal runs, so it is not active.
        let mut npm_copy = known_install("npm", Some("9.9.9"), CliExecutableStatus::Healthy);
        npm_copy.source_kind = CliSourceKind::Npm;
        npm_copy.source_confidence = CliSourceConfidence::Verified;

        let installations = [
            path_install("manual", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            npm_copy,
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 0);
        assert_eq!(
            installations[active.index].source_kind,
            CliSourceKind::Unknown
        );
    }

    #[test]
    fn a_known_location_becomes_active_only_when_nothing_is_on_path() {
        let installations = [
            known_install("broken", None, CliExecutableStatus::Broken),
            known_install("working", Some("1.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 1);
        assert!(!active.is_broken);

        let all_broken = [known_install("only", None, CliExecutableStatus::Broken)];
        let active = select_active(&all_broken).expect("active");
        assert_eq!(active.index, 0);
        assert!(active.is_broken);
    }

    #[test]
    fn nothing_discovered_means_no_active_installation() {
        assert_eq!(select_active(&[]), None);
    }

    #[test]
    fn candidates_resolving_to_one_binary_collapse_while_keeping_path_order() {
        let mut symlink = path_install("link", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
        symlink.canonical_path = Some("/real/claude".to_string());
        let mut real = path_install("real", 1, Some("1.0.0"), CliExecutableStatus::Healthy);
        real.canonical_path = Some("/real/claude".to_string());
        let other = path_install("other", 2, Some("2.0.0"), CliExecutableStatus::Healthy);

        let unique = deduplicate(vec![symlink, real, other]);

        assert_eq!(unique.len(), 2);
        // The first occurrence survives, so PATH order is not disturbed by deduplication.
        assert_eq!(unique[0].id.as_str(), "link");
        assert_eq!(unique[1].id.as_str(), "other");
    }

    #[test]
    fn a_candidate_that_could_not_be_canonicalized_is_kept_under_its_literal_path() {
        let a = path_install("a", 0, None, CliExecutableStatus::Healthy);
        let b = path_install("b", 1, None, CliExecutableStatus::Healthy);
        assert_eq!(a.dedup_key(), "/path/a");

        // Neither has a canonical target, so neither is dropped -- failing to resolve a symlink
        // must not silently discard a real installation.
        let unique = deduplicate(vec![a, b]);
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn a_single_installation_has_no_conflicts() {
        let installations = [path_install(
            "only",
            0,
            Some("1.0.0"),
            CliExecutableStatus::Healthy,
        )];
        let active = select_active(&installations);
        assert!(derive_conflicts(&installations, active).is_empty());
        assert!(derive_conflicts(&[], None).is_empty());
    }

    #[test]
    fn duplicate_installations_at_the_same_version_are_only_informational() {
        let installations = [
            path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            path_install("b", 1, Some("1.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations);
        let kinds = derive_conflicts(&installations, active)
            .into_iter()
            .map(|conflict| conflict.kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds, vec![CliConflictKind::MultipleInstallations]);
    }

    #[test]
    fn a_broken_active_shadowing_a_working_install_is_the_most_severe_conflict() {
        let installations = [
            path_install("broken-front", 0, None, CliExecutableStatus::Broken),
            path_install(
                "working-back",
                1,
                Some("1.0.0"),
                CliExecutableStatus::Broken,
            ),
        ];
        // Both broken: nothing is being shadowed, so the severe conflict is absent.
        let active = select_active(&installations);
        let kinds = derive_conflicts(&installations, active)
            .into_iter()
            .map(|conflict| conflict.kind)
            .collect::<Vec<_>>();
        assert!(!kinds.contains(&CliConflictKind::ActiveIsShadowingWorkingInstall));

        // Now the second one works. PATH still runs the broken one, and that is the finding.
        let installations = [
            path_install("broken-front", 0, None, CliExecutableStatus::Broken),
            known_install("working-back", Some("1.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations).expect("active");
        assert_eq!(active.index, 0);
        assert!(active.is_broken);
        let kinds = derive_conflicts(&installations, Some(active))
            .into_iter()
            .map(|conflict| conflict.kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds[0], CliConflictKind::ActiveIsShadowingWorkingInstall);
        assert!(kinds.contains(&CliConflictKind::RunnableMismatch));
    }

    #[test]
    fn differing_versions_and_runnability_are_reported_separately() {
        let installations = [
            path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            path_install("b", 1, Some("2.0.0"), CliExecutableStatus::Broken),
        ];
        let active = select_active(&installations);
        let kinds = derive_conflicts(&installations, active)
            .into_iter()
            .map(|conflict| conflict.kind)
            .collect::<Vec<_>>();

        assert!(kinds.contains(&CliConflictKind::VersionMismatch));
        assert!(kinds.contains(&CliConflictKind::RunnableMismatch));
        assert!(kinds.contains(&CliConflictKind::MultipleInstallations));
        // Most severe first.
        assert_eq!(
            *kinds.last().expect("last"),
            CliConflictKind::MultipleInstallations
        );
    }

    #[test]
    fn an_unprobed_installation_does_not_create_a_version_conflict() {
        // One reported a version, the other was never probed. That is not disagreement.
        let installations = [
            path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            path_install("b", 1, None, CliExecutableStatus::Unknown),
        ];
        let active = select_active(&installations);
        let kinds = derive_conflicts(&installations, active)
            .into_iter()
            .map(|conflict| conflict.kind)
            .collect::<Vec<_>>();
        assert!(!kinds.contains(&CliConflictKind::VersionMismatch));
    }

    #[test]
    fn conflicts_name_the_installations_involved() {
        let installations = [
            path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
            path_install("b", 1, Some("2.0.0"), CliExecutableStatus::Healthy),
        ];
        let active = select_active(&installations);
        let conflict = derive_conflicts(&installations, active)
            .into_iter()
            .next()
            .expect("conflict");
        assert_eq!(
            conflict
                .installations
                .iter()
                .map(CliInstallationId::as_str)
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn origin_and_runnability_are_reported_per_installation() {
        let on_path = path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
        let elsewhere = known_install("b", Some("1.0.0"), CliExecutableStatus::Unknown);

        assert!(on_path.is_on_path());
        assert!(on_path.is_runnable());
        assert_eq!(on_path.environment_origin.as_str(), "path");
        assert_eq!(on_path.path_priority, Some(0));

        assert!(!elsewhere.is_on_path());
        // Unknown is not runnable and not a fault; it simply was not probed.
        assert!(!elsewhere.is_runnable());
        assert_eq!(elsewhere.environment_origin.as_str(), "known-location");
        assert_eq!(elsewhere.path_priority, None);

        assert_eq!(
            CliConflictKind::ActiveIsShadowingWorkingInstall.as_str(),
            "active-shadows-working-install"
        );
        assert_eq!(
            CliConflictKind::VersionMismatch.as_str(),
            "version-mismatch"
        );
        assert_eq!(
            CliConflictKind::RunnableMismatch.as_str(),
            "runnable-mismatch"
        );
        assert_eq!(
            CliConflictKind::MultipleInstallations.as_str(),
            "multiple-installations"
        );
    }

    #[test]
    fn a_path_heuristic_records_inferred_confidence_never_verified() {
        let mut inferred = path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
        inferred.source_kind = CliSourceKind::Npm;
        inferred.source_confidence = CliSourceConfidence::Inferred;
        inferred.source_id = Some(CliSourceId::new("npm").expect("source id"));

        // The distinction that gates automatic mutation: a path that looks npm-ish is not proof
        // that npm owns it.
        assert_eq!(inferred.source_confidence, CliSourceConfidence::Inferred);
        assert!(inferred.source_confidence < CliSourceConfidence::Verified);
        assert_eq!(
            inferred.source_id.as_ref().map(CliSourceId::as_str),
            Some("npm")
        );
    }
}
