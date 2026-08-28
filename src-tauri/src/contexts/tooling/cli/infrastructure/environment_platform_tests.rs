//! Platform-specific discovery and conflict contracts.
//!
//! Classification and grouping are pure string logic, so these run identically on every host and
//! assert the platform rule rather than the developer's machine. The parts that genuinely need a
//! real PATH -- enumeration order, the executable bit, a `noexec` mount -- belong to the temporary
//! PATH fixtures in task 12.10, and are named in each module below so the gap stays visible.

use std::path::Path;

use super::environment_discovery::{build_installation, classify_source};
use crate::contexts::tooling::cli::domain::installation::{
    conflicts_block_mutation, deduplicate, derive_conflicts, group_launcher_families,
    select_active, CliConflictKind, CliEnvironmentOrigin, CliInstallation,
};
use crate::contexts::tooling::cli::domain::source::CliSourceKind;
use crate::contexts::tooling::cli::domain::status::CliExecutableStatus;

fn on_path(path: &str, priority: u32) -> CliInstallation {
    build_installation(Path::new(path), CliEnvironmentOrigin::Path, Some(priority))
}

fn healthy_on_path(path: &str, priority: u32) -> CliInstallation {
    let mut installation = on_path(path, priority);
    installation.executable_status = CliExecutableStatus::Healthy;
    installation
}

fn conflict_kinds(installations: &[CliInstallation]) -> Vec<CliConflictKind> {
    derive_conflicts(installations, select_active(installations))
        .into_iter()
        .map(|conflict| conflict.kind)
        .collect()
}

mod windows {
    use super::*;

    #[test]
    fn pathext_variants_of_one_install_form_one_launcher_family() {
        let base = r"C:\Users\a\AppData\Roaming\npm";
        let grouped = group_launcher_families(vec![
            on_path(&format!(r"{base}\claude.cmd"), 0),
            on_path(&format!(r"{base}\claude.ps1"), 0),
            on_path(&format!(r"{base}\claude"), 0),
        ]);

        if cfg!(target_os = "windows") {
            // One npm global install, not three competing ones.
            assert_eq!(grouped.len(), 1);
            assert_eq!(grouped[0].alias_paths.len(), 2);
            assert!(conflict_kinds(&grouped).contains(&CliConflictKind::DuplicateLauncherAlias));
        } else {
            // Unix has no extension convention, so folding stems there would merge distinct files.
            assert_eq!(grouped.len(), 3);
        }
    }

    #[test]
    fn launcher_matching_folds_case_only_on_windows() {
        let upper = on_path(r"C:\Program Files\Tool\CLAUDE.CMD", 0);
        let lower = on_path(r"c:\program files\tool\claude.cmd", 1);

        if cfg!(target_os = "windows") {
            assert_eq!(upper.launcher_family_key(), lower.launcher_family_key());
        } else {
            assert_ne!(upper.launcher_family_key(), lower.launcher_family_key());
        }
    }

    #[test]
    fn a_junction_or_symlink_collapses_through_its_canonical_target() {
        let mut junction = on_path(r"C:\Tools\claude.exe", 0);
        junction.canonical_path = Some(r"C:\Real\claude.exe".to_string());
        let mut real = on_path(r"C:\Real\claude.exe", 1);
        real.canonical_path = Some(r"C:\Real\claude.exe".to_string());

        let unique = deduplicate(vec![junction, real]);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].path_priority, Some(0), "PATH order is preserved");
    }

    #[test]
    fn npm_and_winget_installs_stay_two_sources_and_block_mutation() {
        let installations = [
            healthy_on_path(r"C:\Users\a\AppData\Roaming\npm\claude.cmd", 0),
            healthy_on_path(r"C:\Users\a\Microsoft\WinGet\Links\claude.exe", 1),
        ];

        assert_eq!(installations[0].source_kind, CliSourceKind::Npm);
        assert_eq!(installations[1].source_kind, CliSourceKind::Winget);
        assert_ne!(
            installations[0].launcher_family_key(),
            installations[1].launcher_family_key()
        );
        assert!(
            conflict_kinds(&installations).contains(&CliConflictKind::MultipleInstallationSources)
        );
        assert!(conflicts_block_mutation(&derive_conflicts(
            &installations,
            select_active(&installations)
        )));
    }

    #[test]
    fn an_nvm_style_versioned_path_is_still_npm_owned() {
        // nvm-windows keeps each node under its own version directory; the global package inside is
        // still npm's.
        assert_eq!(
            classify_source(
                r"C:\Users\a\AppData\Roaming\nvm\v20.11.1\node_modules\.bin\claude.cmd"
            )
            .0,
            CliSourceKind::Npm
        );
    }

    #[test]
    fn user_and_machine_path_entries_keep_the_priorities_discovery_saw() {
        // A shell reaches the lowest-priority entry first, whichever list order discovery returned.
        let installations = [
            healthy_on_path(r"C:\Program Files\nodejs\claude.cmd", 9),
            healthy_on_path(r"C:\Users\a\AppData\Roaming\npm\claude.cmd", 0),
        ];

        let selection = select_active(&installations);

        assert_eq!(selection.path_selected, Some(1));
        assert_eq!(installations[1].path_priority, Some(0));
    }

    #[test]
    fn a_broken_first_shim_is_reported_rather_than_stepped_over() {
        let mut broken_shim = on_path(r"C:\Users\a\AppData\Roaming\npm\claude.cmd", 0);
        broken_shim.executable_status = CliExecutableStatus::Broken;
        let installations = [
            broken_shim,
            healthy_on_path(r"C:\Users\a\Microsoft\WinGet\Links\claude.exe", 1),
        ];

        let selection = select_active(&installations);

        assert_eq!(selection.path_selected, Some(0), "the shim is what runs");
        assert_eq!(selection.recommended, Some(1));
        assert!(conflict_kinds(&installations).contains(&CliConflictKind::BrokenPathPrecedence));
    }
}

mod macos {
    use super::*;

    #[test]
    fn homebrew_prefixes_are_distinguished_from_usr_local() {
        // Apple silicon Homebrew lives under /opt/homebrew; a bare /usr/local/bin entry is a system
        // path unless something marks it otherwise.
        assert_eq!(
            classify_source("/opt/homebrew/bin/claude").0,
            CliSourceKind::Homebrew
        );
        assert_eq!(
            classify_source("/opt/homebrew/Cellar/claude/1.2.0/bin/claude").0,
            CliSourceKind::Homebrew
        );
        assert_eq!(
            classify_source("/usr/local/bin/claude").0,
            CliSourceKind::System
        );
    }

    #[test]
    fn both_homebrew_prefixes_present_at_once_is_a_source_and_version_conflict() {
        let mut intel = healthy_on_path("/usr/local/Homebrew/bin/claude", 1);
        intel.reported_version = Some(
            crate::contexts::tooling::cli::domain::version::NormalizedCliVersion::parse("1.0.0"),
        );
        let mut arm = healthy_on_path("/opt/homebrew/bin/claude", 0);
        arm.reported_version = Some(
            crate::contexts::tooling::cli::domain::version::NormalizedCliVersion::parse("2.0.0"),
        );
        let installations = [arm, intel];

        let kinds = conflict_kinds(&installations);

        assert!(kinds.contains(&CliConflictKind::VersionDivergence));
    }

    #[test]
    fn a_cellar_symlink_retarget_moves_the_dedup_identity_not_the_launcher_path() {
        // A Homebrew upgrade repoints /opt/homebrew/bin/claude at a new Cellar version.
        let mut before = on_path("/opt/homebrew/bin/claude", 0);
        before.canonical_path = Some("/opt/homebrew/Cellar/claude/1.2.0/bin/claude".to_string());
        let mut after = before.clone();
        after.canonical_path = Some("/opt/homebrew/Cellar/claude/1.3.0/bin/claude".to_string());

        assert_ne!(before.dedup_key(), after.dedup_key());
        assert_eq!(before.executable_path, after.executable_path);
    }

    #[test]
    fn an_architecture_mismatch_blocks_mutation_and_launch() {
        // An x86_64 binary on arm64 without Rosetta fails with its own status, so the conflict can
        // name the cause instead of reporting a generic breakage.
        let mut rosetta = on_path("/usr/local/bin/claude", 0);
        rosetta.executable_status = CliExecutableStatus::UnsupportedArchitecture;
        let installations = [rosetta];

        let conflicts = derive_conflicts(&installations, select_active(&installations));
        let conflict = conflicts
            .iter()
            .find(|conflict| conflict.kind == CliConflictKind::ArchitectureMismatch)
            .expect("architecture conflict");

        assert!(conflict.blocks_mutation);
        assert!(conflict.blocks_launch);
    }

    #[test]
    fn an_install_only_a_login_shell_can_see_is_divergence_not_absence() {
        // A Finder-launched app inherits no shell profile, so a Homebrew install can be missing from
        // this process's PATH while a terminal finds it immediately.
        let mut login_only = build_installation(
            Path::new("/opt/homebrew/bin/claude"),
            CliEnvironmentOrigin::KnownLocation,
            None,
        );
        login_only.executable_status = CliExecutableStatus::Healthy;
        let installations = [healthy_on_path("/usr/bin/claude", 0), login_only];

        let kinds = conflict_kinds(&installations);

        assert!(kinds.contains(&CliConflictKind::EnvironmentPathDivergence));
        assert!(!installations[1].is_on_path());
        assert_eq!(installations[1].path_priority, None);
    }
}

mod linux {
    use super::*;

    #[test]
    fn each_standard_location_classifies_to_its_own_source() {
        let cases = [
            ("/usr/bin/claude", CliSourceKind::System),
            ("/usr/local/bin/claude", CliSourceKind::System),
            ("/home/a/.local/bin/agy", CliSourceKind::VendorInstaller),
            ("/home/a/.npm-global/bin/claude", CliSourceKind::Npm),
            (
                "/home/a/.nvm/versions/node/v20.1.0/bin/claude",
                CliSourceKind::Npm,
            ),
            ("/home/a/.volta/bin/claude", CliSourceKind::Volta),
            ("/home/a/.bun/bin/opencode", CliSourceKind::Bun),
        ];
        for (path, expected) in cases {
            assert_eq!(classify_source(path).0, expected, "{path}");
        }
    }

    #[test]
    fn an_update_alternatives_chain_resolves_to_one_installation() {
        // /usr/bin/claude -> /etc/alternatives/claude -> /opt/claude/bin/claude. Dedup follows the
        // canonical target, so the indirection does not invent a second installation.
        let mut via_alternatives = healthy_on_path("/usr/bin/claude", 0);
        via_alternatives.canonical_path = Some("/opt/claude/bin/claude".to_string());
        let mut direct = build_installation(
            Path::new("/opt/claude/bin/claude"),
            CliEnvironmentOrigin::KnownLocation,
            None,
        );
        direct.canonical_path = Some("/opt/claude/bin/claude".to_string());

        let unique = deduplicate(vec![via_alternatives, direct]);

        assert_eq!(unique.len(), 1);
        assert!(unique[0].is_on_path(), "the PATH-visible entry survives");
    }

    #[test]
    fn a_missing_executable_bit_or_noexec_mount_surfaces_as_a_probe_fault() {
        // Neither is a missing installation: the file is there and the user can see it.
        for status in [
            CliExecutableStatus::PermissionDenied,
            CliExecutableStatus::Broken,
        ] {
            let mut installation = on_path("/mnt/noexec/claude", 0);
            installation.executable_status = status;
            assert!(!installation.is_runnable());
            assert!(status.is_faulty(), "{}", status.as_str());
        }
    }

    #[test]
    fn a_user_install_and_a_system_install_block_mutation() {
        let installations = [
            healthy_on_path("/home/a/.npm-global/bin/claude", 0),
            healthy_on_path("/usr/bin/claude", 1),
        ];

        let conflicts = derive_conflicts(&installations, select_active(&installations));

        assert!(conflicts_block_mutation(&conflicts));
        assert!(
            conflict_kinds(&installations).contains(&CliConflictKind::MultipleInstallationSources)
        );
    }

    #[test]
    fn a_version_manager_install_absent_from_this_path_is_divergence() {
        // nvm and Volta put their bin on PATH from a shell profile, which a desktop process started
        // from an icon never reads.
        let mut version_managed = build_installation(
            Path::new("/home/a/.nvm/versions/node/v20.1.0/bin/claude"),
            CliEnvironmentOrigin::KnownLocation,
            None,
        );
        version_managed.executable_status = CliExecutableStatus::Healthy;
        let installations = [healthy_on_path("/usr/bin/claude", 0), version_managed];

        assert!(
            conflict_kinds(&installations).contains(&CliConflictKind::EnvironmentPathDivergence)
        );
    }

    #[test]
    fn a_dangling_launcher_is_a_stale_target_not_a_healthy_install() {
        let mut dangling = on_path("/usr/local/bin/claude", 0);
        dangling.target_missing = true;
        dangling.executable_status = CliExecutableStatus::Broken;
        let installations = [dangling];

        let conflicts = derive_conflicts(&installations, select_active(&installations));
        let conflict = conflicts
            .iter()
            .find(|conflict| conflict.kind == CliConflictKind::StaleLauncherTarget)
            .expect("stale launcher conflict");

        assert!(conflict.blocks_mutation);
        assert!(conflict.blocks_launch);
    }
}
