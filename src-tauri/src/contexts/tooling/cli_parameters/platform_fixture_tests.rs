//! Deterministic fixtures that must behave identically on Windows, macOS and Linux.
//!
//! These exist because the ways this subdomain can differ by platform are all invisible on a
//! developer's own machine: a separator rule that only rejects backslashes on Windows, an
//! executable that only resolves with `PATHEXT`, a symlink that a directory probe mistakes for a
//! directory, a SQLite file under a home directory the runner does not have. Every assertion here
//! is either platform-independent by design — and says so — or explicitly conditional.
//!
//! Nothing here spawns a provider, reads a credential, or reaches the network. The only executable
//! involved is the repository's own fixture stub, and it is inspected rather than run.

use super::api::{
    CliParameterApplicationError, CliParameterSettingsApi, ResetCliParameterProfileInput,
    SaveCliParameterProfileInput,
};
use super::application::models::ReplaceCliParameterProfile;
use super::application::ports::{
    CliInstallationSnapshotPort, CliParameterDiagnosticsPort, CliParameterProfileRepository,
};
use super::application::service::CliParameterApplicationService;
use super::domain::compatibility::{CliInstallationSnapshot, CliParameterSupport};
use super::domain::definition::{CliParameterDefinition, CliParameterPlatform};
use super::domain::rendering::CliArgumentSlot;
use super::domain::selection::{CliParameterSelection, CliParameterSelectionMap};
use super::domain::validation::normalize_selection;
use super::infrastructure::{EmbeddedCliParameterCatalog, LifecycleVersionComparator};
use super::infrastructure::{FilesystemDirectoryProbe, SqliteCliParameterProfileRepository};
use crate::contexts::tooling::cli_parameters::application::ports::CliParameterDirectoryPort;
use crate::contexts::tooling::cli_parameters::domain::catalog::CliParameterCatalog;
use crate::platform::database::NativeDatabase;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CATALOG_VERSION: &str = "2.0.0";

fn catalog() -> CliParameterCatalog {
    CliParameterCatalog::parse(include_str!("catalog/catalog.v2.json")).expect("canonical catalog")
}

fn definition(agent_id: &str, parameter_id: &str) -> CliParameterDefinition {
    catalog()
        .definitions(agent_id)
        .expect("agent")
        .iter()
        .find(|entry| entry.id == parameter_id)
        .unwrap_or_else(|| panic!("{agent_id}:{parameter_id} is missing from the registry"))
        .clone()
}

fn list(entries: &[&str]) -> CliParameterSelection {
    CliParameterSelection::Value {
        value: super::domain::selection::CliParameterValue::TextList(
            entries.iter().map(|entry| (*entry).to_string()).collect(),
        ),
    }
}

fn normalized_list(parameter: &CliParameterDefinition, entries: &[&str]) -> Vec<String> {
    match normalize_selection(parameter, &list(entries)).expect("normalize") {
        CliParameterSelection::Value {
            value: super::domain::selection::CliParameterValue::TextList(values),
        } => values,
        other => panic!("expected a text list, got {other:?}"),
    }
}

// ---------------------------------------------------------------- path-list normalization

#[test]
fn a_path_list_trims_both_separators_on_every_platform() {
    // A profile written on Windows and read on Linux must normalize the same way. Trimming only
    // the platform's own separator would leave `C:\dir\` intact on POSIX and hand the CLI a token
    // ending in a separator.
    let parameter = definition("gemini-cli", "includeDirectories");

    assert_eq!(
        normalized_list(&parameter, &["/home/user/project/", "C:\\work\\repo\\"]),
        vec![
            "/home/user/project".to_string(),
            "C:\\work\\repo".to_string()
        ],
    );
}

#[test]
fn a_path_with_spaces_and_unicode_stays_one_entry() {
    let parameter = definition("gemini-cli", "includeDirectories");

    let normalized = normalized_list(
        &parameter,
        &[
            "C:/Program Files/app",
            "/srv/项目 目录",
            "/srv/proje\u{301}t",
        ],
    );

    assert_eq!(normalized.len(), 3);
    assert!(normalized.contains(&"C:/Program Files/app".to_string()));
    assert!(normalized.contains(&"/srv/项目 目录".to_string()));
}

#[test]
fn windows_specific_path_forms_are_accepted_verbatim() {
    // Drive-qualified, UNC and extended-length forms all reach the CLI unchanged. The registry has
    // no business rewriting them, and a rule that silently rewrote one would produce a path the CLI
    // cannot open.
    let parameter = definition("gemini-cli", "includeDirectories");

    let normalized = normalized_list(
        &parameter,
        &[
            "C:\\Users\\dev\\repo",
            "\\\\server\\share\\repo",
            "\\\\?\\C:\\very\\long\\path",
        ],
    );

    assert_eq!(
        normalized,
        vec![
            "C:\\Users\\dev\\repo".to_string(),
            "\\\\server\\share\\repo".to_string(),
            "\\\\?\\C:\\very\\long\\path".to_string(),
        ],
    );
}

#[test]
fn a_control_character_is_rejected_on_every_platform() {
    // A newline inside a path token would split one argv entry into two once anything writes it to
    // a script file, which is a platform-independent hazard.
    let parameter = definition("gemini-cli", "includeDirectories");

    assert!(normalize_selection(&parameter, &list(&["/srv/ok\nrm -rf /"])).is_err());
    assert!(normalize_selection(&parameter, &list(&["/srv/ok\u{0}"])).is_err());
}

// ---------------------------------------------------------------- directory probe

fn probe() -> FilesystemDirectoryProbe {
    FilesystemDirectoryProbe
}

#[test]
fn the_directory_probe_separates_directories_from_files_and_absences() {
    let root = tempfile::tempdir().expect("tempdir");
    let directory = root.path().join("workspace");
    fs::create_dir(&directory).expect("create directory");
    let file = root.path().join("not-a-directory.txt");
    fs::write(&file, b"x").expect("write file");

    assert!(probe().directory_exists(&directory.to_string_lossy()));
    assert!(!probe().directory_exists(&file.to_string_lossy()));
    assert!(!probe().directory_exists(&root.path().join("missing").to_string_lossy()));
    assert!(!probe().directory_exists(""));
}

#[cfg(unix)]
#[test]
fn a_symlink_is_judged_by_its_target_not_by_being_a_link() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let directory = root.path().join("real-directory");
    fs::create_dir(&directory).expect("create directory");
    let file = root.path().join("real-file");
    fs::write(&file, b"x").expect("write file");

    let link_to_directory = root.path().join("link-to-directory");
    let link_to_file = root.path().join("link-to-file");
    symlink(&directory, &link_to_directory).expect("symlink to directory");
    symlink(&file, &link_to_file).expect("symlink to file");

    assert!(probe().directory_exists(&link_to_directory.to_string_lossy()));
    assert!(!probe().directory_exists(&link_to_file.to_string_lossy()));
}

#[cfg(windows)]
#[test]
fn a_trailing_separator_and_a_drive_root_are_still_directories() {
    let root = tempfile::tempdir().expect("tempdir");
    let directory = root.path().join("workspace");
    fs::create_dir(&directory).expect("create directory");

    assert!(probe().directory_exists(&format!("{}\\", directory.to_string_lossy())));
    assert!(probe().directory_exists(&format!("{}/", directory.to_string_lossy())));
    // A drive root has no parent component; a probe that walked components would trip on it.
    assert!(probe().directory_exists("C:\\"));
}

// ---------------------------------------------------------------- executable status

fn fixture_cli_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("desktop")
        .join("fixtures")
        .join("cli")
}

#[test]
fn the_repository_fixture_can_produce_the_form_this_platform_resolves() {
    // The stub is the only executable these fixtures involve, and what "executable" means differs.
    // The repository tracks the POSIX binary with its mode bit; the Windows `.exe` is excluded and
    // compiled on demand, so asserting a committed `.exe` would fail on a clean checkout.
    let directory = fixture_cli_directory();
    let source = directory.join("opencode.rs");
    assert!(
        source.is_file(),
        "the stub source is missing at {}",
        source.display()
    );

    #[cfg(windows)]
    {
        // PATHEXT resolution needs an `.exe`, so prove the recipe produces one rather than trusting
        // that it would. Built into a temporary directory: a test must not write into the repo.
        let output_directory = tempfile::tempdir().expect("tempdir");
        let output = output_directory.path().join("opencode.exe");
        let status = std::process::Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("rustc is available inside a cargo test");
        assert!(status.success(), "the stub source did not compile");
        assert!(output.is_file(), "rustc produced no {}", output.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // The committed binary carries mode 100755. A checkout with `core.fileMode=false`, or an
        // archive that dropped the bit, would leave every launch path reporting "not installed"
        // with nothing to point at.
        let stub = directory.join("opencode");
        assert!(stub.is_file(), "POSIX resolution needs {}", stub.display());
        let mode = fs::metadata(&stub).expect("metadata").permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "the fixture stub is not executable: {mode:o}"
        );
    }
}

#[test]
fn compatibility_reads_every_installation_shape_the_same_way_on_every_platform() {
    let parameter = definition("claude-code", "screenReader");
    let comparator = super::domain::compatibility::DottedVersionComparator;
    let platform = CliParameterPlatform::current();

    let missing = CliInstallationSnapshot::default();
    assert_eq!(
        super::domain::compatibility::evaluate_definition(
            &parameter,
            &missing,
            platform,
            &comparator
        ),
        CliParameterSupport::NotInstalled,
    );

    let unknown_version = CliInstallationSnapshot {
        installed: true,
        runnable: true,
        active_path: Some("/opt/claude/bin/claude".to_string()),
        version: None,
        conflict: false,
    };
    assert!(matches!(
        super::domain::compatibility::evaluate_definition(
            &parameter,
            &unknown_version,
            platform,
            &comparator
        ),
        CliParameterSupport::UnknownVersion { .. }
    ));

    let too_old = CliInstallationSnapshot {
        installed: true,
        runnable: true,
        active_path: Some("C:\\tools\\claude.exe".to_string()),
        version: Some("2.0.0".to_string()),
        conflict: false,
    };
    assert!(matches!(
        super::domain::compatibility::evaluate_definition(
            &parameter,
            &too_old,
            platform,
            &comparator
        ),
        CliParameterSupport::UnsupportedVersion { .. }
    ));

    let current = CliInstallationSnapshot {
        installed: true,
        runnable: true,
        active_path: Some("C:\\tools\\claude.exe".to_string()),
        version: Some("2.1.237".to_string()),
        conflict: false,
    };
    assert_eq!(
        super::domain::compatibility::evaluate_definition(
            &parameter,
            &current,
            platform,
            &comparator
        ),
        CliParameterSupport::Supported,
    );
}

// ---------------------------------------------------------------- argv token preservation

fn render(agent_id: &str, entries: &[(&str, CliParameterSelection)]) -> (Vec<String>, Vec<String>) {
    let catalog = catalog();
    let definitions = catalog.definitions(agent_id).expect("agent");
    let mut global = Vec::new();
    let mut invocation = Vec::new();
    for (parameter_id, selection) in entries {
        let parameter = definitions
            .iter()
            .find(|entry| &entry.id == parameter_id)
            .expect("parameter");
        let normalized = normalize_selection(parameter, selection).expect("normalize");
        let CliParameterSelection::Value { value } = normalized else {
            continue;
        };
        for token in parameter.renderer.render(parameter_id, &value) {
            match token.segment {
                CliArgumentSlot::Global => global.push(token.value),
                CliArgumentSlot::Invocation => invocation.push(token.value),
            }
        }
    }
    (global, invocation)
}

#[test]
fn a_whitespace_or_unicode_value_stays_exactly_one_argv_token() {
    let (global, invocation) = render(
        "gemini-cli",
        &[(
            "includeDirectories",
            list(&["C:/Program Files/app", "/srv/项目 目录"]),
        )],
    );

    assert!(invocation.is_empty());
    assert_eq!(
        global,
        vec![
            "--include-directories".to_string(),
            "C:/Program Files/app".to_string(),
            "--include-directories".to_string(),
            "/srv/项目 目录".to_string(),
        ],
    );
}

#[test]
fn a_repeated_flag_keeps_one_flag_per_value_and_its_order() {
    let (global, _) = render(
        "gemini-cli",
        &[("extensions", list(&["alpha", "beta", "gamma"]))],
    );

    assert_eq!(
        global,
        vec![
            "--extensions".to_string(),
            "alpha".to_string(),
            "--extensions".to_string(),
            "beta".to_string(),
            "--extensions".to_string(),
            "gamma".to_string(),
        ],
    );
}

#[test]
fn global_and_invocation_tokens_land_in_their_declared_segments() {
    // opencode puts everything after the subcommand; claude-code puts everything before it. A
    // renderer that ignored the slot would still produce the right tokens in the wrong place.
    let (opencode_global, opencode_invocation) = render(
        "opencode",
        &[
            (
                "model",
                CliParameterSelection::text("anthropic/claude-sonnet-4"),
            ),
            ("printLogs", CliParameterSelection::boolean(true)),
        ],
    );
    assert!(opencode_global.is_empty());
    assert_eq!(
        opencode_invocation,
        vec![
            "--model".to_string(),
            "anthropic/claude-sonnet-4".to_string(),
            "--print-logs".to_string(),
        ],
    );

    let (claude_global, claude_invocation) = render(
        "claude-code",
        &[("model", CliParameterSelection::text("opus"))],
    );
    assert_eq!(
        claude_global,
        vec!["--model".to_string(), "opus".to_string()]
    );
    assert!(claude_invocation.is_empty());
}

#[test]
fn a_config_key_value_token_is_encoded_once_and_joined_once() {
    let (global, _) = render(
        "codex-cli",
        &[("reasoningEffort", CliParameterSelection::text("high"))],
    );

    assert_eq!(
        global,
        vec![
            "--config".to_string(),
            "model_reasoning_effort=\"high\"".to_string(),
        ],
    );
}

// ---------------------------------------------------------------- persistence

fn repository(directory: &TempDir) -> SqliteCliParameterProfileRepository {
    let database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("database in an isolated dir");
    SqliteCliParameterProfileRepository::new(database)
}

fn replace(
    agent_id: &str,
    expected_revision: i64,
    entries: &[(&str, CliParameterSelection)],
) -> ReplaceCliParameterProfile {
    ReplaceCliParameterProfile {
        agent_id: agent_id.to_string(),
        expected_revision,
        catalog_version: CATALOG_VERSION.to_string(),
        selections: entries
            .iter()
            .map(|(id, selection)| ((*id).to_string(), selection.clone()))
            .collect::<CliParameterSelectionMap>(),
    }
}

#[test]
fn a_profile_survives_reopening_the_database_from_the_same_directory() {
    // The directory is the whole isolation boundary: no HOME, no user profile, no shared file. A
    // runner that leaked into a real user directory would pass here and poison the next job.
    let directory = tempfile::tempdir().expect("tempdir");
    {
        let repository = repository(&directory);
        repository
            .replace_if_revision(replace(
                "gemini-cli",
                0,
                &[("includeDirectories", list(&["C:/Program Files/app"]))],
            ))
            .expect("save");
    }

    let reopened = repository(&directory);
    let profile = reopened.load("gemini-cli").expect("load after reopen");
    assert_eq!(profile.revision, 1);
    assert_eq!(profile.rows.len(), 1);
    assert!(profile.rows[0].value_json.contains("C:/Program Files/app"));
}

#[test]
fn a_stale_revision_and_a_stale_catalog_are_both_refused() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repository = repository(&directory);
    repository
        .replace_if_revision(replace(
            "codex-cli",
            0,
            &[("model", CliParameterSelection::text("gpt-5.5"))],
        ))
        .expect("first save");

    let stale_revision = repository.replace_if_revision(replace(
        "codex-cli",
        0,
        &[("model", CliParameterSelection::text("gpt-5.6"))],
    ));
    assert!(stale_revision.is_err(), "a stale revision was accepted");

    let profile = repository.load("codex-cli").expect("load");
    assert_eq!(profile.revision, 1, "a refused write moved the revision");
}

/// Builds the settings facade over a real SQLite file in an isolated directory. Compatibility is
/// stubbed rather than probed: these fixtures assert the write rules, and spawning a detector would
/// make them depend on what happens to be installed on the runner.
fn settings_api(directory: &TempDir) -> CliParameterSettingsApi {
    struct FixedInstallation;
    impl CliInstallationSnapshotPort for FixedInstallation {
        fn active_installation(
            &self,
            _agent_id: &str,
        ) -> Result<CliInstallationSnapshot, CliParameterApplicationError> {
            Ok(CliInstallationSnapshot {
                installed: true,
                runnable: true,
                active_path: None,
                version: Some("99.0.0".to_string()),
                conflict: false,
            })
        }
    }
    struct SilentDiagnostics;
    impl CliParameterDiagnosticsPort for SilentDiagnostics {
        fn emit(&self, _diagnostic: &super::domain::diagnostic::CliParameterDiagnostic) {}
    }

    CliParameterSettingsApi::new(CliParameterApplicationService {
        catalog: std::sync::Arc::new(EmbeddedCliParameterCatalog),
        repository: std::sync::Arc::new(repository(directory)),
        installations: std::sync::Arc::new(FixedInstallation),
        directories: std::sync::Arc::new(FilesystemDirectoryProbe),
        diagnostics: std::sync::Arc::new(SilentDiagnostics),
        comparator: std::sync::Arc::new(LifecycleVersionComparator),
        platform: CliParameterPlatform::current(),
    })
}

#[test]
fn a_stale_catalog_version_is_refused_by_the_use_case_that_owns_it() {
    // Revision CAS lives in the repository, atomic with the write. Catalog CAS lives here, because
    // only the use case knows which catalog it just loaded. Asserting it at the repository would
    // have passed for the wrong reason: `reset_if_revision` writes the version it is handed.
    let directory = tempfile::tempdir().expect("tempdir");
    let api = settings_api(&directory);

    let saved = api
        .save_profile(&SaveCliParameterProfileInput {
            agent_id: "codex-cli".to_string(),
            expected_revision: 0,
            catalog_version: CATALOG_VERSION.to_string(),
            selections: [("model".to_string(), CliParameterSelection::text("gpt-5.5"))]
                .into_iter()
                .collect::<CliParameterSelectionMap>(),
        })
        .expect("save");
    assert_eq!(saved.revision, 1);

    let stale_catalog = api.save_profile(&SaveCliParameterProfileInput {
        agent_id: "codex-cli".to_string(),
        expected_revision: 1,
        catalog_version: "0.0.1".to_string(),
        selections: CliParameterSelectionMap::new(),
    });
    assert!(
        stale_catalog.is_err(),
        "a stale catalog version was accepted"
    );

    let stale_reset = api.reset_profile(&ResetCliParameterProfileInput {
        agent_id: "codex-cli".to_string(),
        expected_revision: 1,
        catalog_version: "0.0.1".to_string(),
    });
    assert!(
        stale_reset.is_err(),
        "a stale catalog version was accepted on reset"
    );

    assert_eq!(
        api.list_profiles()
            .expect("list")
            .iter()
            .find(|profile| profile.agent_id == "codex-cli")
            .expect("codex profile")
            .revision,
        1,
        "a refused write moved the revision",
    );
}

#[test]
fn a_reset_clears_the_rows_and_advances_the_revision_once() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repository = repository(&directory);
    repository
        .replace_if_revision(replace(
            "opencode",
            0,
            &[("printLogs", CliParameterSelection::boolean(true))],
        ))
        .expect("save");

    let reset = repository
        .reset_if_revision("opencode", 1, CATALOG_VERSION)
        .expect("reset");

    assert_eq!(reset.revision, 2);
    assert!(repository.load("opencode").expect("load").rows.is_empty());
}

#[test]
fn a_legacy_profile_is_rewritten_on_the_first_save_and_not_before() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repository = repository(&directory);
    repository
        .raw_connection_for_tests()
        .execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at)
             VALUES ('claude-code', 'model', 1, '\"default\"', '2026-01-01T00:00:00Z')",
            rusqlite::params![],
        )
        .expect("legacy row");

    let before = repository.load("claude-code").expect("load");
    assert_eq!(before.selection_schema_version, 1);
    assert_eq!(before.rows.len(), 1);

    repository
        .replace_if_revision(replace(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("opus"))],
        ))
        .expect("save");

    let after = repository.load("claude-code").expect("load after save");
    assert_eq!(after.selection_schema_version, 2);
    assert_eq!(after.rows.len(), 1);
    assert!(after.rows[0].value_json.contains("opus"));
}
