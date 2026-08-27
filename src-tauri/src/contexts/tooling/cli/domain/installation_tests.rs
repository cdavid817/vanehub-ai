// Included through `#[path]` from installation.rs.
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
        alias_paths: Vec::new(),
        target_missing: false,
        reported_version: version.map(NormalizedCliVersion::parse),
        source_id: Some(CliSourceId::new("npm").expect("source id")),
        source_kind: CliSourceKind::Npm,
        source_confidence: CliSourceConfidence::Inferred,
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
        &format!("/path{priority}/{id}"),
        CliEnvironmentOrigin::Path,
        Some(priority),
        version,
        status,
    )
}

fn known_install(id: &str, version: Option<&str>, status: CliExecutableStatus) -> CliInstallation {
    installation(
        id,
        &format!("/known/{id}"),
        CliEnvironmentOrigin::KnownLocation,
        None,
        version,
        status,
    )
}

fn kinds(conflicts: &[CliConflict]) -> Vec<CliConflictKind> {
    conflicts.iter().map(|conflict| conflict.kind).collect()
}

fn find(conflicts: &[CliConflict], kind: CliConflictKind) -> Option<&CliConflict> {
    conflicts.iter().find(|conflict| conflict.kind == kind)
}

#[test]
fn a_healthy_first_path_entry_is_both_selected_and_recommended() {
    let installations = [
        path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        path_install("b", 1, Some("1.0.0"), CliExecutableStatus::Healthy),
    ];

    let selection = select_active(&installations);

    assert_eq!(selection.path_selected, Some(0));
    assert_eq!(selection.recommended, Some(0));
    assert!(!selection.diverges());
}

#[test]
fn a_broken_first_launcher_stays_path_selected_while_a_healthy_one_is_recommended() {
    // The whole reason the two identities exist. Calling the healthy copy "active" would be a
    // false statement about what the user's terminal runs.
    let installations = [
        path_install("broken-front", 0, None, CliExecutableStatus::Broken),
        path_install(
            "working-back",
            1,
            Some("1.0.0"),
            CliExecutableStatus::Healthy,
        ),
    ];

    let selection = select_active(&installations);

    assert_eq!(
        selection.path_selected,
        Some(0),
        "PATH order alone decides this"
    );
    assert_eq!(selection.recommended, Some(1), "probe results decide this");
    assert!(selection.diverges());

    let conflicts = derive_conflicts(&installations, selection);
    let conflict = find(&conflicts, CliConflictKind::BrokenPathPrecedence).expect("conflict");
    assert_eq!(conflict.severity, CliConflictSeverity::Error);
    assert!(conflict.blocks_mutation);
    assert!(conflict.blocks_launch);
    assert_eq!(conflict.reason_code, "broken-path-precedence");
    assert_eq!(
        conflict
            .installations
            .iter()
            .map(CliInstallationId::as_str)
            .collect::<Vec<_>>(),
        vec!["broken-front", "working-back"]
    );
}

#[test]
fn path_selection_ignores_health_entirely() {
    // Every PATH entry is broken. The first one is still what the shell runs.
    let installations = [
        path_install("second", 5, None, CliExecutableStatus::TimedOut),
        path_install("first", 1, None, CliExecutableStatus::Broken),
    ];

    let selection = select_active(&installations);

    assert_eq!(
        selection.path_selected,
        Some(1),
        "lowest priority, not list order"
    );
    assert_eq!(selection.recommended, Some(1));
    assert!(!selection.diverges());
}

#[test]
fn nothing_on_path_reports_no_path_selection_but_may_still_recommend() {
    let installations = [known_install(
        "only",
        Some("1.0.0"),
        CliExecutableStatus::Healthy,
    )];

    let selection = select_active(&installations);

    // A real answer: the command would resolve to nothing.
    assert_eq!(selection.path_selected, None);
    assert_eq!(selection.recommended, Some(0));
    assert!(selection.diverges());
}

#[test]
fn nothing_discovered_selects_nothing() {
    let selection = select_active(&[]);
    assert_eq!(selection.path_selected, None);
    assert_eq!(selection.recommended, None);
    assert!(!selection.diverges());
    assert!(derive_conflicts(&[], selection).is_empty());
}

#[test]
fn a_more_manageable_known_location_never_becomes_path_selected() {
    let mut npm_copy = known_install("npm", Some("9.9.9"), CliExecutableStatus::Healthy);
    npm_copy.source_confidence = CliSourceConfidence::Verified;
    let installations = [
        path_install("manual", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        npm_copy,
    ];

    let selection = select_active(&installations);

    assert_eq!(selection.path_selected, Some(0));
    assert_eq!(selection.recommended, Some(0));
}

#[test]
fn windows_launcher_aliases_collapse_into_one_logical_installation() {
    // One npm global install produces three files. Reporting three competing installations makes
    // every Windows npm install look conflicted.
    let mut cmd = path_install("cmd", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
    cmd.executable_path = r"C:\Users\a\AppData\Roaming\npm\claude.cmd".to_string();
    let mut ps1 = path_install("ps1", 0, None, CliExecutableStatus::Unknown);
    ps1.executable_path = r"C:\Users\a\AppData\Roaming\npm\claude.ps1".to_string();
    let mut bare = path_install("bare", 0, None, CliExecutableStatus::Unknown);
    bare.executable_path = r"C:\Users\a\AppData\Roaming\npm\claude".to_string();

    let grouped = group_launcher_families(vec![cmd, ps1, bare]);

    if cfg!(target_os = "windows") {
        assert_eq!(grouped.len(), 1);
        assert_eq!(
            grouped[0].id.as_str(),
            "cmd",
            "first in PATH order survives"
        );
        assert_eq!(grouped[0].alias_paths.len(), 2);
        assert!(grouped[0].is_runnable());
    } else {
        // On Unix there is no extension convention to fold, and stem-grouping would merge
        // genuinely different programs.
        assert_eq!(grouped.len(), 3);
    }
}

#[test]
fn launcher_family_keys_fold_case_and_extension_only_on_windows() {
    let mut upper = path_install("upper", 0, None, CliExecutableStatus::Unknown);
    upper.executable_path = r"C:\Tools\Claude.CMD".to_string();
    let mut lower = path_install("lower", 1, None, CliExecutableStatus::Unknown);
    lower.executable_path = r"C:\tools\claude.ps1".to_string();

    if cfg!(target_os = "windows") {
        assert_eq!(upper.launcher_family_key(), lower.launcher_family_key());
    } else {
        assert_ne!(upper.launcher_family_key(), lower.launcher_family_key());
    }

    // Different directories are always different families, on every platform.
    let mut elsewhere = path_install("elsewhere", 2, None, CliExecutableStatus::Unknown);
    elsewhere.executable_path = r"C:\Other\claude.cmd".to_string();
    assert_ne!(upper.launcher_family_key(), elsewhere.launcher_family_key());
}

#[test]
fn a_collapsed_family_reports_a_duplicate_alias_conflict_that_blocks_nothing() {
    let mut kept = path_install("kept", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
    kept.alias_paths = vec!["/path0/kept.cmd".to_string()];
    let installations = [kept];

    let selection = select_active(&installations);
    let conflicts = derive_conflicts(&installations, selection);

    let conflict = find(&conflicts, CliConflictKind::DuplicateLauncherAlias).expect("conflict");
    assert_eq!(conflict.severity, CliConflictSeverity::Info);
    // Expected on Windows; it changes nothing about what runs.
    assert!(!conflict.blocks_mutation);
    assert!(!conflict.blocks_launch);
}

#[test]
fn a_symlink_pointing_at_a_missing_target_is_a_stale_launcher() {
    let mut dangling = path_install("dangling", 0, None, CliExecutableStatus::Broken);
    dangling.target_missing = true;
    let installations = [dangling];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict = find(&conflicts, CliConflictKind::StaleLauncherTarget).expect("conflict");
    assert_eq!(conflict.severity, CliConflictSeverity::Error);
    assert!(conflict.blocks_mutation);
    assert!(conflict.blocks_launch);
}

#[test]
fn an_architecture_mismatch_blocks_both_mutation_and_launch() {
    let installations = [path_install(
        "wrong-arch",
        0,
        None,
        CliExecutableStatus::UnsupportedArchitecture,
    )];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict = find(&conflicts, CliConflictKind::ArchitectureMismatch).expect("conflict");
    assert!(conflict.blocks_mutation);
    assert!(conflict.blocks_launch);
}

#[test]
fn a_runnable_installation_off_path_is_environment_path_divergence() {
    // Reachable from a login shell, absent from the PATH this process inherited. Neither missing
    // nor what runs.
    let installations = [
        path_install("on-path", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        known_install(
            "login-shell-only",
            Some("2.0.0"),
            CliExecutableStatus::Healthy,
        ),
    ];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict = find(&conflicts, CliConflictKind::EnvironmentPathDivergence).expect("conflict");
    assert_eq!(conflict.severity, CliConflictSeverity::Warning);
    assert!(!conflict.blocks_mutation);
    assert!(conflict.blocks_launch);
}

#[test]
fn distinct_sources_block_mutation_because_the_target_is_ambiguous() {
    let mut winget = path_install("winget", 1, Some("1.0.0"), CliExecutableStatus::Healthy);
    winget.source_kind = CliSourceKind::Winget;
    winget.source_id = Some(CliSourceId::new("winget").expect("id"));
    let installations = [
        path_install("npm", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        winget,
    ];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict =
        find(&conflicts, CliConflictKind::MultipleInstallationSources).expect("conflict");
    assert!(conflict.blocks_mutation);
    // Which one runs is unambiguous -- PATH decides -- so launch is not blocked.
    assert!(!conflict.blocks_launch);
    assert!(conflicts_block_mutation(&conflicts));
}

#[test]
fn differing_versions_block_launch_but_not_mutation() {
    let installations = [
        path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        path_install("b", 1, Some("2.0.0"), CliExecutableStatus::Healthy),
    ];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict = find(&conflicts, CliConflictKind::VersionDivergence).expect("conflict");
    assert!(!conflict.blocks_mutation);
    assert!(conflict.blocks_launch);
}

#[test]
fn an_unprobed_installation_does_not_create_a_version_conflict() {
    let installations = [
        path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        path_install("b", 1, None, CliExecutableStatus::Unknown),
    ];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    assert!(find(&conflicts, CliConflictKind::VersionDivergence).is_none());
}

#[test]
fn an_unestablishable_source_blocks_mutation() {
    let mut unknown = path_install("unknown", 0, Some("1.0.0"), CliExecutableStatus::Healthy);
    unknown.source_id = None;
    unknown.source_kind = CliSourceKind::Unknown;
    unknown.source_confidence = CliSourceConfidence::Unknown;
    let installations = [unknown];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    let conflict = find(&conflicts, CliConflictKind::AmbiguousSourceOwnership).expect("conflict");
    assert!(conflict.blocks_mutation);
    assert!(!conflict.blocks_launch);
}

#[test]
fn a_single_clean_installation_has_no_conflicts() {
    let installations = [path_install(
        "only",
        0,
        Some("1.0.0"),
        CliExecutableStatus::Healthy,
    )];

    assert!(derive_conflicts(&installations, select_active(&installations)).is_empty());
    assert!(!conflicts_block_mutation(&[]));
}

#[test]
fn duplicate_installations_at_one_version_and_source_only_shadow() {
    let installations = [
        path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy),
        path_install("b", 1, Some("1.0.0"), CliExecutableStatus::Healthy),
    ];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    // Same source, same version, healthy first entry: nothing is wrong.
    assert!(conflicts.is_empty(), "{:?}", kinds(&conflicts));
}

#[test]
fn conflicts_are_ordered_most_severe_first() {
    let mut winget = path_install("winget", 1, Some("2.0.0"), CliExecutableStatus::Healthy);
    winget.source_kind = CliSourceKind::Winget;
    let mut broken = path_install("broken", 0, None, CliExecutableStatus::Broken);
    broken.alias_paths = vec!["/path0/broken.cmd".to_string()];
    let installations = [broken, winget];

    let conflicts = derive_conflicts(&installations, select_active(&installations));

    assert!(conflicts.len() >= 3);
    assert_eq!(conflicts[0].severity, CliConflictSeverity::Error);
    let last = conflicts.last().expect("conflict");
    assert!(last.severity <= conflicts[0].severity);
    assert_eq!(CliConflictSeverity::Info.as_str(), "info");
    assert_eq!(CliConflictSeverity::Warning.as_str(), "warning");
    assert_eq!(CliConflictSeverity::Error.as_str(), "error");
}

#[test]
fn every_conflict_kind_has_a_stable_reason_code() {
    let all = [
        (
            CliConflictKind::DuplicateLauncherAlias,
            "duplicate-launcher-alias",
        ),
        (CliConflictKind::PathShadowing, "path-shadowing"),
        (
            CliConflictKind::BrokenPathPrecedence,
            "broken-path-precedence",
        ),
        (
            CliConflictKind::MultipleInstallationSources,
            "multiple-installation-sources",
        ),
        (CliConflictKind::VersionDivergence, "version-divergence"),
        (
            CliConflictKind::AmbiguousSourceOwnership,
            "ambiguous-source-ownership",
        ),
        (
            CliConflictKind::EnvironmentPathDivergence,
            "environment-path-divergence",
        ),
        (
            CliConflictKind::ArchitectureMismatch,
            "architecture-mismatch",
        ),
        (
            CliConflictKind::StaleLauncherTarget,
            "stale-launcher-target",
        ),
    ];
    assert_eq!(all.len(), 9, "all nine kinds are covered");
    for (kind, wire) in all {
        assert_eq!(kind.as_str(), wire);
        // A conflict built from a kind always carries that kind's own policy.
        let conflict = CliConflict::of(kind, Vec::new());
        assert_eq!(conflict.reason_code, wire);
        assert_eq!(conflict.blocks_mutation, kind.blocks_mutation());
        assert_eq!(conflict.blocks_launch, kind.blocks_launch());
        assert_eq!(conflict.severity, kind.severity());
    }
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
    assert_eq!(unique[0].id.as_str(), "link");
    assert_eq!(unique[1].id.as_str(), "other");
}

#[test]
fn a_candidate_that_could_not_be_canonicalized_is_kept_under_its_literal_path() {
    let a = path_install("a", 0, None, CliExecutableStatus::Healthy);
    let b = path_install("b", 1, None, CliExecutableStatus::Healthy);
    assert_eq!(a.dedup_key(), "/path0/a");

    let unique = deduplicate(vec![a, b]);
    assert_eq!(unique.len(), 2);
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
    assert!(!elsewhere.is_runnable());
    assert_eq!(elsewhere.environment_origin.as_str(), "known-location");
    assert_eq!(elsewhere.path_priority, None);
}

#[test]
fn a_path_heuristic_records_inferred_confidence_never_verified() {
    let inferred = path_install("a", 0, Some("1.0.0"), CliExecutableStatus::Healthy);

    assert_eq!(inferred.source_confidence, CliSourceConfidence::Inferred);
    assert!(inferred.source_confidence < CliSourceConfidence::Verified);
    assert_eq!(
        inferred.source_id.as_ref().map(CliSourceId::as_str),
        Some("npm")
    );
}
