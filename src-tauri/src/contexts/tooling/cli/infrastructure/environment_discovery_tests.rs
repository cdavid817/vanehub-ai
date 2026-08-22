// Included through `#[path]` from environment_discovery.rs.
//
// These exercise the pure parts -- classification, identity, filename expansion, fingerprinting.
// PATH enumeration itself is covered by the desktop suite against a temporary PATH, because
// asserting it here would mean depending on whatever the developer happens to have installed.
use super::*;

#[test]
fn a_path_shape_yields_inferred_confidence_never_verified() {
    // The distinction that gates automatic mutation: a path is evidence of ownership, not proof.
    let cases = [
        (
            "C:\\Users\\a\\AppData\\Roaming\\npm\\codex.cmd",
            CliSourceKind::Npm,
        ),
        ("/home/a/.npm-global/bin/gemini", CliSourceKind::Npm),
        (
            "/home/a/.nvm/versions/node/v20.1.0/bin/claude",
            CliSourceKind::Npm,
        ),
        (
            "C:\\Users\\a\\Microsoft\\WinGet\\Links\\claude.exe",
            CliSourceKind::Winget,
        ),
        ("/opt/homebrew/bin/claude", CliSourceKind::Homebrew),
        ("/home/a/.volta/bin/claude", CliSourceKind::Volta),
        ("/home/a/.bun/bin/opencode", CliSourceKind::Bun),
        ("/home/a/.local/bin/agy", CliSourceKind::VendorInstaller),
        ("/usr/local/bin/claude", CliSourceKind::System),
    ];
    for (path, expected_kind) in cases {
        let (kind, confidence) = classify_source(path);
        assert_eq!(kind, expected_kind, "{path}");
        assert_eq!(
            confidence,
            CliSourceConfidence::Inferred,
            "{path} must never be Verified from a path alone"
        );
    }
}

#[test]
fn an_unrecognised_path_is_unknown_with_unknown_confidence() {
    let (kind, confidence) = classify_source("/opt/somewhere/custom/claude");
    assert_eq!(kind, CliSourceKind::Unknown);
    // Not Inferred: nothing was inferred.
    assert_eq!(confidence, CliSourceConfidence::Unknown);
}

#[test]
fn only_the_three_managed_kinds_carry_a_source_id_to_plan_against() {
    assert_eq!(
        source_id_for(CliSourceKind::Npm).map(|id| id.as_str().to_string()),
        Some("npm".to_string())
    );
    assert_eq!(
        source_id_for(CliSourceKind::Winget).map(|id| id.as_str().to_string()),
        Some("winget".to_string())
    );
    assert_eq!(
        source_id_for(CliSourceKind::VendorInstaller).map(|id| id.as_str().to_string()),
        Some("vendor".to_string())
    );
    // A detect-only kind has no source id, which is what keeps planning from reaching it.
    for kind in [
        CliSourceKind::Homebrew,
        CliSourceKind::Bun,
        CliSourceKind::Volta,
        CliSourceKind::Desktop,
        CliSourceKind::System,
        CliSourceKind::Manual,
        CliSourceKind::Unknown,
    ] {
        assert!(source_id_for(kind).is_none(), "{}", kind.as_str());
    }
}

#[test]
fn windows_expands_a_bare_name_into_the_shim_extensions() {
    let filenames = executable_filenames("claude");
    if cfg!(target_os = "windows") {
        assert!(filenames.contains(&"claude.cmd".to_string()));
        assert!(filenames.contains(&"claude.exe".to_string()));
        // The bare name stays last so an extension-bearing shim wins, as a shell would resolve it.
        assert_eq!(filenames.last().map(String::as_str), Some("claude"));
    } else {
        assert_eq!(filenames, vec!["claude".to_string()]);
    }
}

#[test]
fn installation_ids_are_stable_per_path_and_bounded() {
    let first = installation_id("/usr/local/bin/claude");
    let second = installation_id("/usr/local/bin/claude");
    assert_eq!(first, second);
    assert_ne!(first, installation_id("/usr/bin/claude"));

    // A pathological path still yields a valid identifier rather than being rejected.
    let deep = format!("/{}/claude", "segment/".repeat(60));
    let id = installation_id(&deep);
    assert!(id.as_str().len() <= 128);
    assert!(id.as_str().starts_with("i-"));
}

#[test]
fn a_candidate_that_cannot_be_canonicalized_is_still_reported() {
    // Nothing exists at this path, so canonicalization fails. Dropping the candidate would hide a
    // real installation whenever a symlink cannot be resolved.
    let installation = build_installation(
        Path::new("/definitely/not/here/claude"),
        CliEnvironmentOrigin::Path,
        Some(3),
    );
    assert_eq!(installation.canonical_path, None);
    assert_eq!(installation.executable_path, "/definitely/not/here/claude");
    assert_eq!(installation.path_priority, Some(3));
    assert!(installation.is_on_path());
    // Not yet probed, which is not a fault.
    assert_eq!(installation.executable_status, CliExecutableStatus::Unknown);
    assert!(!installation.is_runnable());
    // Dedup falls back to the literal path when there is no canonical target.
    assert_eq!(installation.dedup_key(), "/definitely/not/here/claude");
}

#[test]
fn a_known_location_candidate_carries_no_path_priority() {
    let installation = build_installation(
        Path::new("/home/a/.local/bin/agy"),
        CliEnvironmentOrigin::KnownLocation,
        None,
    );
    assert!(!installation.is_on_path());
    assert_eq!(installation.path_priority, None);
    assert_eq!(installation.source_kind, CliSourceKind::VendorInstaller);
}

#[test]
fn the_fingerprint_changes_with_path_and_never_contains_the_home_directory() {
    let fingerprint = environment_fingerprint();

    assert!(fingerprint.contains(std::env::consts::OS));
    assert!(fingerprint.contains(std::env::consts::ARCH));
    assert!(fingerprint.contains("local-desktop"));

    // The home path is hashed, not embedded.
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = home.to_string_lossy().to_string();
        if !home.is_empty() {
            assert!(
                !fingerprint.contains(&home),
                "fingerprint must not embed the home directory"
            );
        }
    }
    // Stable across calls within one environment.
    assert_eq!(fingerprint, environment_fingerprint());
}

#[test]
fn the_hash_is_stable_and_does_not_reveal_its_input() {
    assert_eq!(stable_hash("/home/alice"), stable_hash("/home/alice"));
    assert_ne!(stable_hash("/home/alice"), stable_hash("/home/bob"));
    assert!(!stable_hash("/home/alice").contains("alice"));
    assert_eq!(stable_hash("").len(), 16);
}

#[test]
fn distinct_source_kinds_are_deduplicated_and_ordered() {
    let installations = vec![
        build_installation(
            Path::new("/usr/local/bin/claude"),
            CliEnvironmentOrigin::Path,
            Some(0),
        ),
        build_installation(
            Path::new("/home/a/.npm-global/bin/claude"),
            CliEnvironmentOrigin::Path,
            Some(1),
        ),
        build_installation(
            Path::new("/home/a/.npm-global/bin/other"),
            CliEnvironmentOrigin::Path,
            Some(2),
        ),
    ];
    let kinds = distinct_source_kinds(&installations);
    assert_eq!(kinds.len(), 2);
    assert!(kinds.contains(&CliSourceKind::Npm));
    assert!(kinds.contains(&CliSourceKind::System));
}

#[test]
fn discovery_is_bounded_by_the_supplied_budget() {
    let discovery = SystemCliDiscovery;
    let budget = CliProbeBudget {
        max_candidates: 0,
        ..CliProbeBudget::default()
    };
    let found = discovery
        .discover(
            &CliToolId::new("claude-code").expect("tool id"),
            &["claude"],
            budget,
            &CliCancellation::never(),
        )
        .expect("discovery succeeds");
    // A zero budget finds nothing rather than scanning anyway.
    assert!(found.is_empty());
}

#[test]
fn a_cancelled_discovery_returns_what_it_already_has() {
    let discovery = SystemCliDiscovery;
    let cancellation = CliCancellation::new(std::sync::Arc::new(
        std::sync::atomic::AtomicBool::new(true),
    ));
    let found = discovery
        .discover(
            &CliToolId::new("claude-code").expect("tool id"),
            &["claude"],
            CliProbeBudget::default(),
            &cancellation,
        )
        .expect("discovery succeeds");
    assert!(found.is_empty());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn version_managed_directories_are_ordered_by_version_not_by_name() {
    // The bug lexical ordering causes: as text, "v10.0.0" sorts before "v9.0.0", so the newest
    // install was offered last and an older node's copy won.
    let mut versions = [
        NormalizedCliVersion::parse("v9.0.0"),
        NormalizedCliVersion::parse("v10.0.0"),
        NormalizedCliVersion::parse("v20.11.1"),
    ];
    versions.sort_by(|left, right| right.display_order(left));
    assert_eq!(
        versions
            .iter()
            .map(NormalizedCliVersion::as_str)
            .collect::<Vec<_>>(),
        vec!["v20.11.1", "v10.0.0", "v9.0.0"]
    );

    // And a directory that does not name a version sorts last rather than being dropped.
    let mut mixed = [
        NormalizedCliVersion::parse("system"),
        NormalizedCliVersion::parse("v18.0.0"),
    ];
    mixed.sort_by(|left, right| right.display_order(left));
    assert_eq!(mixed[0].as_str(), "v18.0.0");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn a_missing_nvm_root_yields_no_candidates_rather_than_an_error() {
    let paths = version_managed_bin_paths(Path::new("/definitely/not/a/home"), "claude");
    assert!(paths.is_empty());
}
