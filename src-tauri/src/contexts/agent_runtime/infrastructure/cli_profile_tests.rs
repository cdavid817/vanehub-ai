//! Runtime read-cutover coverage: policy projection, dual-read of legacy and v2 rows, and slot
//! placement, exercised against a real SQLite-backed resolver and a real `PermissionsApi`.

use super::cli_profile::resolve_launch;
use crate::contexts::agent_runtime::application::SessionExecutionMode;
use crate::contexts::permissions::api::{
    test_permissions_api_on, PermissionsApi, PolicyTemplateName,
};
use crate::contexts::permissions::application::{ClaudeCodeHookPort, PermissionsApplicationError};
use crate::contexts::tooling::api::{
    CliLaunchScope, CliParameterRuntimeApi, CliParameterSelection, CliParameterSelectionMap,
};
use crate::contexts::tooling::cli_parameters::api::CliInstallationSnapshot;
use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::ports::CliInstallationSnapshotPort;
use crate::contexts::tooling::cli_parameters::application::service::CliParameterApplicationService;
use crate::contexts::tooling::cli_parameters::domain::definition::CliParameterPlatform;
use crate::contexts::tooling::cli_parameters::infrastructure::{
    EmbeddedCliParameterCatalog, FilesystemDirectoryProbe, LifecycleVersionComparator,
    SqliteCliParameterProfileRepository,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::sync::Arc;

const MANAGED: [&str; 5] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
];

struct NoopParameterDiagnostics;
impl crate::contexts::tooling::cli_parameters::application::ports::CliParameterDiagnosticsPort
    for NoopParameterDiagnostics
{
    fn emit(&self, _diagnostic: &crate::contexts::tooling::api::CliParameterDiagnostic) {}
}

/// Captures what the resolver hands to the diagnostics port, so a test can prove the emission
/// happens inside the resolver rather than depending on the caller keeping the returned list.
#[derive(Default)]
struct RecordingParameterDiagnostics {
    emitted: std::sync::Mutex<Vec<crate::contexts::tooling::api::CliParameterDiagnostic>>,
}

impl crate::contexts::tooling::cli_parameters::application::ports::CliParameterDiagnosticsPort
    for RecordingParameterDiagnostics
{
    fn emit(&self, diagnostic: &crate::contexts::tooling::api::CliParameterDiagnostic) {
        self.emitted
            .lock()
            .expect("emitted")
            .push(diagnostic.clone());
    }
}

/// Builds a runtime api whose diagnostics port records instead of discarding.
fn recording_runtime_api(
    database: NativeDatabase,
    diagnostics: Arc<RecordingParameterDiagnostics>,
) -> CliParameterRuntimeApi {
    CliParameterRuntimeApi::new(CliParameterApplicationService {
        catalog: Arc::new(EmbeddedCliParameterCatalog),
        repository: Arc::new(SqliteCliParameterProfileRepository::new(database)),
        installations: Arc::new(StubInstallations::new(Some("99.99.99"))),
        directories: Arc::new(FilesystemDirectoryProbe),
        diagnostics,
        comparator: Arc::new(LifecycleVersionComparator),
        platform: CliParameterPlatform::current(),
    })
}

/// Installed with a version high enough that nothing in the registry is version-gated out, unless
/// a test asks for an older one. The version is interior-mutable so a test can simulate a CLI
/// upgrade or downgrade between two launches.
struct StubInstallations {
    version: std::sync::Mutex<Option<String>>,
}

impl StubInstallations {
    fn new(version: Option<&str>) -> Self {
        Self {
            version: std::sync::Mutex::new(version.map(str::to_string)),
        }
    }
}

impl CliInstallationSnapshotPort for StubInstallations {
    fn active_installation(
        &self,
        _agent_id: &str,
    ) -> Result<CliInstallationSnapshot, CliParameterApplicationError> {
        Ok(CliInstallationSnapshot {
            installed: true,
            runnable: true,
            active_path: Some("/usr/bin/managed-cli".to_string()),
            version: self.version.lock().expect("version").clone(),
            conflict: false,
        })
    }
}

struct NoopClaudeCodeHook;
impl ClaudeCodeHookPort for NoopClaudeCodeHook {
    fn install(&self) -> Result<(), PermissionsApplicationError> {
        Ok(())
    }
    fn remove(&self) -> Result<(), PermissionsApplicationError> {
        Ok(())
    }
}

fn runtime_api_with(
    database: NativeDatabase,
    installations: Arc<StubInstallations>,
) -> CliParameterRuntimeApi {
    CliParameterRuntimeApi::new(CliParameterApplicationService {
        catalog: Arc::new(EmbeddedCliParameterCatalog),
        repository: Arc::new(SqliteCliParameterProfileRepository::new(database)),
        installations: installations.clone(),
        directories: Arc::new(FilesystemDirectoryProbe),
        diagnostics: Arc::new(NoopParameterDiagnostics),
        comparator: Arc::new(LifecycleVersionComparator),
        platform: CliParameterPlatform::current(),
    })
}

fn runtime_api(database: NativeDatabase, version: Option<&str>) -> CliParameterRuntimeApi {
    runtime_api_with(database, Arc::new(StubInstallations::new(version)))
}

/// Shared with the baseline argv equivalence tests so both suites resolve through identical
/// wiring; only the stored rows differ.
pub(super) fn equivalence_runtime_api(database: NativeDatabase) -> CliParameterRuntimeApi {
    runtime_api(database, Some("99.99.99"))
}

fn permissions_api(
    database: NativeDatabase,
    default_template: PolicyTemplateName,
) -> PermissionsApi {
    // The repository wiring belongs to `permissions`; this test only supplies the shared database.
    test_permissions_api_on(database, default_template, Arc::new(NoopClaudeCodeHook))
}

struct Harness {
    _directory: TempDirectory,
    database: NativeDatabase,
    parameters: CliParameterRuntimeApi,
    permissions: PermissionsApi,
}

fn harness(label: &str, template: PolicyTemplateName) -> Harness {
    harness_with_version(label, template, Some("99.99.99"))
}

fn harness_with_version(
    label: &str,
    template: PolicyTemplateName,
    version: Option<&str>,
) -> Harness {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Harness {
        parameters: runtime_api(database.clone(), version),
        permissions: permissions_api(database.clone(), template),
        database,
        _directory: directory,
    }
}

impl Harness {
    /// Writes a row exactly the way the still-legacy settings command writes it: a raw JSON scalar
    /// under the v1 selection schema.
    fn write_legacy_row(&self, agent_id: &str, parameter_id: &str, value_json: &str) {
        let connection = self.database.connection().expect("connection");
        connection
            .execute(
                "INSERT OR REPLACE INTO cli_parameter_settings
                 (agent_id, parameter_id, enabled, value_json, updated_at)
                 VALUES (?1, ?2, 1, ?3, '2026-01-01T00:00:00Z')",
                params![agent_id, parameter_id, value_json],
            )
            .expect("legacy row");
    }

    fn chat(&self, agent_id: &str) -> (Vec<String>, Vec<String>) {
        self.launch(
            agent_id,
            CliLaunchScope::Chat,
            CliParameterSelectionMap::new(),
        )
    }

    fn interactive(&self, agent_id: &str) -> (Vec<String>, Vec<String>) {
        self.launch(
            agent_id,
            CliLaunchScope::Interactive,
            CliParameterSelectionMap::new(),
        )
    }

    fn launch(
        &self,
        agent_id: &str,
        scope: CliLaunchScope,
        overrides: CliParameterSelectionMap,
    ) -> (Vec<String>, Vec<String>) {
        let resolved = resolve_launch(
            &self.parameters,
            &self.permissions,
            agent_id,
            scope,
            overrides,
            SessionExecutionMode::Inherit,
            None,
        )
        .expect("resolve");
        (resolved.global_args, resolved.invocation_args)
    }

    fn env(&self, agent_id: &str) -> std::collections::BTreeMap<String, String> {
        resolve_launch(
            &self.parameters,
            &self.permissions,
            agent_id,
            CliLaunchScope::Interactive,
            CliParameterSelectionMap::new(),
            SessionExecutionMode::Inherit,
            None,
        )
        .expect("resolve")
        .env
    }
}

fn contains_pair(args: &[String], flag: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == flag && pair[1] == value)
}

#[test]
fn every_managed_cli_projects_every_policy_for_chat_and_terminal() {
    for template in [
        PolicyTemplateName::Readonly,
        PolicyTemplateName::Standard,
        PolicyTemplateName::Trusted,
        PolicyTemplateName::Yolo,
    ] {
        let harness = harness(&format!("cli-profile-policy-{template:?}"), template);
        for agent_id in MANAGED {
            for (global, _invocation) in [harness.chat(agent_id), harness.interactive(agent_id)] {
                match (agent_id, template) {
                    ("claude-code", PolicyTemplateName::Readonly) => {
                        assert!(contains_pair(&global, "--permission-mode", "plan"))
                    }
                    ("claude-code", PolicyTemplateName::Standard) => {
                        assert!(!global.iter().any(|token| token == "--permission-mode"))
                    }
                    ("claude-code", _) => {
                        assert!(contains_pair(&global, "--permission-mode", "acceptEdits"))
                    }
                    ("codex-cli", PolicyTemplateName::Readonly) => {
                        assert!(contains_pair(&global, "--sandbox", "read-only"));
                        assert!(contains_pair(&global, "--ask-for-approval", "never"));
                    }
                    ("codex-cli", PolicyTemplateName::Standard) => {
                        assert!(contains_pair(&global, "--sandbox", "workspace-write"));
                        assert!(contains_pair(&global, "--ask-for-approval", "on-request"));
                    }
                    ("codex-cli", _) => {
                        assert!(contains_pair(&global, "--sandbox", "workspace-write"));
                        assert!(contains_pair(&global, "--ask-for-approval", "never"));
                    }
                    ("gemini-cli", PolicyTemplateName::Readonly) => {
                        assert!(contains_pair(&global, "--approval-mode", "plan"))
                    }
                    ("gemini-cli", PolicyTemplateName::Standard) => {
                        assert!(contains_pair(&global, "--approval-mode", "default"))
                    }
                    ("gemini-cli", _) => {
                        assert!(contains_pair(&global, "--approval-mode", "yolo"))
                    }
                    ("opencode", PolicyTemplateName::Readonly) => {
                        assert!(contains_pair(&_invocation, "--agent", "plan"))
                    }
                    ("opencode", PolicyTemplateName::Standard) => {
                        assert!(!_invocation.iter().any(|token| token == "--agent"));
                        assert!(!_invocation.iter().any(|token| token == "--auto"));
                    }
                    ("opencode", _) => assert!(_invocation.iter().any(|token| token == "--auto")),
                    ("antigravity-cli", PolicyTemplateName::Readonly) => {
                        assert!(contains_pair(&global, "--mode", "plan"));
                        assert!(global.iter().any(|token| token == "--sandbox"));
                    }
                    ("antigravity-cli", PolicyTemplateName::Standard) => {
                        assert!(!global.iter().any(|token| token == "--mode"));
                        assert!(!global.iter().any(|token| token == "--sandbox"));
                    }
                    ("antigravity-cli", _) => {
                        assert!(contains_pair(&global, "--mode", "accept-edits"));
                        assert!(!global.iter().any(|token| token == "--sandbox"));
                    }
                    other => panic!("unexpected managed CLI: {other:?}"),
                }
            }
        }
    }
}

#[test]
fn opencode_standard_injects_the_permission_env_var_and_readonly_does_not() {
    let standard = harness("cli-profile-env-standard", PolicyTemplateName::Standard);
    assert_eq!(
        standard
            .env("opencode")
            .get("OPENCODE_PERMISSION")
            .map(String::as_str),
        Some(r#"{"edit":"ask","bash":"ask"}"#)
    );
    let readonly = harness("cli-profile-env-readonly", PolicyTemplateName::Readonly);
    assert!(!readonly.env("opencode").contains_key("OPENCODE_PERMISSION"));
    assert_eq!(
        standard
            .env("claude-code")
            .get("VANEHUB_PERMISSION_HOOK_SCOPE")
            .map(String::as_str),
        Some("managed")
    );
}

/// Invariant II — a profile saved by the still-legacy settings page reaches the next real launch.
#[test]
fn a_legacy_row_written_by_the_old_settings_page_reaches_the_next_launch() {
    let harness = harness("cli-profile-legacy-read", PolicyTemplateName::Standard);
    harness.write_legacy_row("claude-code", "model", "\"sonnet\"");
    harness.write_legacy_row("codex-cli", "search", "true");
    harness.write_legacy_row("codex-cli", "ephemeral", "true");

    let (global, _) = harness.chat("claude-code");
    assert!(contains_pair(&global, "--model", "sonnet"));

    let (codex_global, codex_invocation) = harness.chat("codex-cli");
    assert!(codex_global.iter().any(|token| token == "--search"));
    // Registry-declared placement: `--ephemeral` belongs to the `exec` grammar, so it is an
    // invocation token rather than a global one. The hand-written reshuffle is gone.
    assert_eq!(codex_invocation, vec!["--ephemeral".to_string()]);
    assert!(!codex_global.iter().any(|token| token == "--ephemeral"));
}

/// Invariant III — a v2 envelope written by a future settings cutover is read by the next launch.
#[test]
fn a_v2_row_is_read_by_the_next_launch() {
    let harness = harness("cli-profile-v2-read", PolicyTemplateName::Standard);
    harness.write_legacy_row(
        "claude-code",
        "model",
        r#"{"state":"value","value":"opus"}"#,
    );
    harness.write_legacy_row(
        "claude-code",
        "safeMode",
        r#"{"state":"value","value":true}"#,
    );
    harness.write_legacy_row("claude-code", "bare", r#"{"state":"inherit"}"#);

    let (global, _) = harness.chat("claude-code");
    assert!(contains_pair(&global, "--model", "opus"));
    assert!(global.iter().any(|token| token == "--safe-mode"));
    assert!(!global.iter().any(|token| token == "--bare"));
}

/// A value the registry genuinely allows to contain spaces stays exactly one argv token — it is
/// never split, quoted, or shell-escaped. `--model` cannot carry one (its pattern forbids
/// whitespace), so the guarantee is proven on the path list that can.
#[test]
fn a_whitespace_bearing_value_stays_one_argv_token() {
    let harness = harness("cli-profile-whitespace", PolicyTemplateName::Standard);
    let spaced = harness._directory.path().join("my project dir");
    std::fs::create_dir_all(&spaced).expect("create directory with a space");
    let spaced = spaced.to_str().expect("utf-8 path").to_string();
    harness.write_legacy_row(
        "gemini-cli",
        "includeDirectories",
        &serde_json::json!({ "state": "value", "value": [spaced.clone()] }).to_string(),
    );

    let (global, _) = harness.chat("gemini-cli");
    let position = global
        .iter()
        .position(|token| token == "--include-directories")
        .expect("include-directories flag");
    assert_eq!(global[position + 1], spaced);
    assert!(global[position + 1].contains(' '));
    // One flag, one value: the path was not split on its space.
    assert_eq!(
        global
            .iter()
            .filter(|token| *token == "--include-directories")
            .count(),
        1
    );
}

/// A directory that disappeared after it was saved is a warning, not a failed launch.
#[test]
fn a_missing_include_directory_is_dropped_instead_of_failing_the_launch() {
    let harness = harness("cli-profile-missing-dir", PolicyTemplateName::Standard);
    harness.write_legacy_row(
        "gemini-cli",
        "includeDirectories",
        r#"{"state":"value","value":["/definitely/not/here/vanehub"]}"#,
    );
    let (global, _) = harness.chat("gemini-cli");
    assert!(!global.iter().any(|token| token == "--include-directories"));
    // The policy projection still applies, so the launch is intact.
    assert!(contains_pair(&global, "--approval-mode", "default"));
}

/// Invariant VII — an unusable stored value neither panics, nor rewrites, nor reaches argv.
#[test]
fn malformed_unknown_and_unsupported_rows_produce_no_token_and_do_not_fail_the_launch() {
    let harness = harness("cli-profile-quarantine", PolicyTemplateName::Standard);
    harness.write_legacy_row("claude-code", "model", "not-json");
    harness.write_legacy_row("claude-code", "removedParameter", "\"x\"");
    harness.write_legacy_row("claude-code", "effort", "\"nonsense-effort\"");
    harness.write_legacy_row("claude-code", "safeMode", "true");

    let (global, _) = harness.chat("claude-code");
    assert!(!global.iter().any(|token| token == "--model"));
    assert!(!global.iter().any(|token| token == "--effort"));
    // The one valid selection still applies.
    assert!(global.iter().any(|token| token == "--safe-mode"));

    // The original rows are untouched.
    let connection = harness.database.connection().expect("connection");
    let stored: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cli_parameter_settings WHERE agent_id = 'claude-code'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(stored, 4);
}

#[test]
fn a_policy_governed_legacy_row_never_reaches_argv_through_the_user_profile_path() {
    let harness = harness("cli-profile-governed-row", PolicyTemplateName::Readonly);
    // A row that predates migration 61's cleanup. The user path must ignore it; the policy
    // projection remains the only source of `--sandbox`.
    harness.write_legacy_row("codex-cli", "sandbox", "\"workspace-write\"");
    let (global, _) = harness.chat("codex-cli");
    assert!(contains_pair(&global, "--sandbox", "read-only"));
    assert!(!contains_pair(&global, "--sandbox", "workspace-write"));
}

#[test]
fn a_message_override_beats_the_saved_profile_and_policy_beats_both() {
    let harness = harness("cli-profile-precedence", PolicyTemplateName::Readonly);
    harness.write_legacy_row("claude-code", "model", "\"sonnet\"");
    let (global, _) = harness.launch(
        "claude-code",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::from([(
            "model".to_string(),
            CliParameterSelection::text("opus"),
        )]),
    );
    assert!(contains_pair(&global, "--model", "opus"));
    assert!(!contains_pair(&global, "--model", "sonnet"));
    // The policy still owns permission mode regardless of profile or message.
    assert!(contains_pair(&global, "--permission-mode", "plan"));
}

#[test]
fn an_inherited_profile_emits_no_user_profile_token() {
    let harness = harness("cli-profile-inherit", PolicyTemplateName::Standard);
    let (global, invocation) = harness.chat("gemini-cli");
    // Only the policy projection contributes; nothing user-owned is emitted.
    assert_eq!(
        global,
        vec!["--approval-mode".to_string(), "default".to_string()]
    );
    assert!(invocation.is_empty());
}

#[test]
fn a_version_gated_value_is_omitted_when_the_active_version_is_too_old() {
    let harness = harness_with_version(
        "cli-profile-version-gate",
        PolicyTemplateName::Standard,
        Some("2.1.100"),
    );
    harness.write_legacy_row("claude-code", "screenReader", "true");
    let (global, _) = harness.interactive("claude-code");
    assert!(!global.iter().any(|token| token == "--ax-screen-reader"));

    let supported = harness_with_version(
        "cli-profile-version-ok",
        PolicyTemplateName::Standard,
        Some("2.1.181"),
    );
    supported.write_legacy_row("claude-code", "screenReader", "true");
    let (global, _) = supported.interactive("claude-code");
    assert!(global.iter().any(|token| token == "--ax-screen-reader"));
}

#[test]
fn scope_filters_chat_only_and_interactive_only_parameters() {
    let harness = harness("cli-profile-scope", PolicyTemplateName::Standard);
    harness.write_legacy_row("codex-cli", "ephemeral", "true");
    harness.write_legacy_row("codex-cli", "noAltScreen", "true");

    let (chat_global, chat_invocation) = harness.chat("codex-cli");
    assert_eq!(chat_invocation, vec!["--ephemeral".to_string()]);
    assert!(!chat_global.iter().any(|token| token == "--no-alt-screen"));

    let (interactive_global, interactive_invocation) = harness.interactive("codex-cli");
    assert!(interactive_global
        .iter()
        .any(|token| token == "--no-alt-screen"));
    assert!(interactive_invocation.is_empty());
}

#[test]
fn an_unassigned_agent_resolves_the_configured_default_template() {
    let harness = harness("cli-profile-default-template", PolicyTemplateName::Trusted);
    let (_, invocation) = harness.chat("opencode");
    assert!(invocation.iter().any(|token| token == "--auto"));
}

#[test]
fn template_lookup_failure_fails_the_launch_instead_of_guessing_a_default() {
    let harness = harness("cli-profile-template-failure", PolicyTemplateName::Standard);
    harness
        .database
        .connection()
        .expect("connection")
        .execute_batch("DROP TABLE agent_principals;")
        .expect("drop");
    assert!(resolve_launch(
        &harness.parameters,
        &harness.permissions,
        "claude-code",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::new(),
        SessionExecutionMode::Inherit,
        None,
    )
    .is_err());
}

#[test]
fn an_unmanaged_agent_is_refused_before_any_resolution() {
    let harness = harness("cli-profile-unmanaged", PolicyTemplateName::Standard);
    assert!(resolve_launch(
        &harness.parameters,
        &harness.permissions,
        "onepiece",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::new(),
        SessionExecutionMode::Inherit,
        None,
    )
    .is_err());
}

#[test]
fn no_resolved_token_carries_a_prompt_session_or_output_protocol_value() {
    let harness = harness("cli-profile-reserved", PolicyTemplateName::Trusted);
    for agent_id in MANAGED {
        for (global, invocation) in [harness.chat(agent_id), harness.interactive(agent_id)] {
            for token in global.iter().chain(invocation.iter()) {
                let lowered = token.to_ascii_lowercase();
                assert!(!lowered.contains("prompt"), "{agent_id}: {token}");
                assert!(!lowered.contains("session"), "{agent_id}: {token}");
                assert!(!lowered.contains("stream-json"), "{agent_id}: {token}");
                assert!(!lowered.contains("dangerously"), "{agent_id}: {token}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Launch-time re-evaluation. Nothing about a saved profile, an Agent policy, or a detected CLI
// version may be frozen into a session: every new process resolves from current state.
// ---------------------------------------------------------------------------------------------

/// A profile edited after the session exists reaches the next fresh launch and the next resume,
/// while the snapshot an already-running process holds is untouched.
#[test]
fn a_profile_change_reaches_the_next_launch_and_leaves_a_running_process_alone() {
    let harness = harness("reeval-profile", PolicyTemplateName::Standard);
    harness.write_legacy_row("claude-code", "model", "\"sonnet\"");

    // The launch a live process is already running on.
    let (running_global, _) = harness.chat("claude-code");
    assert!(contains_pair(&running_global, "--model", "sonnet"));

    harness.write_legacy_row("claude-code", "model", "\"opus\"");

    // The already-resolved snapshot is an owned value; the live process keeps its original argv.
    assert!(contains_pair(&running_global, "--model", "sonnet"));

    for (global, _) in [
        harness.chat("claude-code"),
        harness.interactive("claude-code"),
    ] {
        assert!(contains_pair(&global, "--model", "opus"));
        assert!(!contains_pair(&global, "--model", "sonnet"));
    }
}

/// The policy template is re-resolved per launch: two adapters over the same database and the same
/// stored profile produce different governed argv when the assigned template differs.
#[test]
fn a_policy_change_reaches_the_next_launch() {
    let directory = TempDirectory::new("reeval-policy");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let parameters = runtime_api(database.clone(), Some("99.99.99"));

    let resolve = |template: PolicyTemplateName| -> Vec<String> {
        let permissions = permissions_api(database.clone(), template);
        resolve_launch(
            &parameters,
            &permissions,
            "codex-cli",
            CliLaunchScope::Chat,
            CliParameterSelectionMap::new(),
            SessionExecutionMode::Inherit,
            None,
        )
        .expect("resolve")
        .global_args
    };

    let readonly = resolve(PolicyTemplateName::Readonly);
    assert!(contains_pair(&readonly, "--sandbox", "read-only"));

    let trusted = resolve(PolicyTemplateName::Trusted);
    assert!(contains_pair(&trusted, "--sandbox", "workspace-write"));
    assert!(!contains_pair(&trusted, "--sandbox", "read-only"));
}

/// Compatibility is recomputed from the lifecycle snapshot on every launch, so upgrading the CLI
/// makes a previously omitted gated value start rendering without any profile change.
#[test]
fn a_cli_version_change_reevaluates_compatibility_on_the_next_launch() {
    let directory = TempDirectory::new("reeval-version");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let installations = Arc::new(StubInstallations::new(Some("2.1.100")));
    let parameters = runtime_api_with(database.clone(), installations.clone());
    let permissions = permissions_api(database.clone(), PolicyTemplateName::Standard);

    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT OR REPLACE INTO cli_parameter_settings
             (agent_id, parameter_id, enabled, value_json, updated_at)
             VALUES ('claude-code', 'screenReader', 1, 'true', '2026-01-01T00:00:00Z')",
            params![],
        )
        .expect("legacy row");
    drop(connection);

    let resolve = || -> Vec<String> {
        resolve_launch(
            &parameters,
            &permissions,
            "claude-code",
            CliLaunchScope::Interactive,
            CliParameterSelectionMap::new(),
            SessionExecutionMode::Inherit,
            None,
        )
        .expect("resolve")
        .global_args
    };

    // Too old: the gated value is omitted, and the stored row is kept for repair.
    assert!(!resolve().iter().any(|token| token == "--ax-screen-reader"));

    *installations.version.lock().expect("version") = Some("2.1.181".to_string());

    // Upgraded in place: the very next launch renders it, with no save in between.
    assert!(resolve().iter().any(|token| token == "--ax-screen-reader"));

    // And a downgrade takes it away again.
    *installations.version.lock().expect("version") = Some("2.0.0".to_string());
    assert!(!resolve().iter().any(|token| token == "--ax-screen-reader"));
}

/// The snapshot handed to a process carries argv and environment only — no policy template, no
/// compatibility verdict, and no catalog state that could go stale inside a session.
#[test]
fn the_launch_snapshot_carries_no_policy_or_compatibility_state() {
    let harness = harness("reeval-snapshot", PolicyTemplateName::Trusted);
    let resolved = resolve_launch(
        &harness.parameters,
        &harness.permissions,
        "codex-cli",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::new(),
        SessionExecutionMode::Inherit,
        None,
    )
    .expect("resolve");
    // Exhaustive destructuring: adding a cacheable field to the snapshot fails to compile here.
    let super::cli_profile::ResolvedLaunch {
        global_args,
        invocation_args,
        env,
    } = resolved;
    assert!(!global_args.is_empty());
    assert!(invocation_args.is_empty());
    assert!(env.is_empty());
}

// ---------------------------------------------------------------------------------------------
// Diagnostics: emitted by the resolver, associated with the triggering operation, and free of any
// prompt, credential, token, session identifier, or secret environment value.
// ---------------------------------------------------------------------------------------------

/// `resolve_launch` deliberately discards the returned diagnostic list. The resolver must still
/// have emitted every diagnostic through its port, so a launch never silently swallows one.
#[test]
fn resolver_diagnostics_are_emitted_even_though_the_launch_caller_discards_them() {
    let directory = TempDirectory::new("diagnostics-not-dropped");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let recorder = Arc::new(RecordingParameterDiagnostics::default());
    let parameters = recording_runtime_api(database.clone(), recorder.clone());
    let permissions = permissions_api(database.clone(), PolicyTemplateName::Standard);

    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT OR REPLACE INTO cli_parameter_settings
             (agent_id, parameter_id, enabled, value_json, updated_at)
             VALUES ('claude-code', 'removedParameter', 1, '\"x\"', '2026-01-01T00:00:00Z')",
            params![],
        )
        .expect("legacy row");
    drop(connection);

    // `resolve_launch` returns only argv and env; the diagnostics are not part of its result.
    let launch = resolve_launch(
        &parameters,
        &permissions,
        "claude-code",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::new(),
        SessionExecutionMode::Inherit,
        Some("operation-77"),
    )
    .expect("resolve");
    assert!(launch.global_args.iter().all(|token| token != "--x"));

    let emitted = recorder.emitted.lock().expect("emitted");
    let quarantined = emitted
        .iter()
        .find(|entry| {
            entry.code
                == crate::contexts::tooling::api::CliParameterDiagnostic::new(
                    crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnosticCode::LegacySelectionQuarantined,
                    "claude-code",
                    None,
                )
                .code
        })
        .expect("the quarantine diagnostic must have been emitted");
    assert_eq!(quarantined.agent_id, "claude-code");
    assert_eq!(
        quarantined.parameter_id.as_deref(),
        Some("removedParameter")
    );
    assert_eq!(
        quarantined.details.get("operationId").map(String::as_str),
        Some("operation-77")
    );
}

/// A diagnostic carries a stable code, the agent id, an optional parameter id, a severity, a
/// localization key, and a remediation. Its details are bounded facts — never a raw user value.
#[test]
fn an_emitted_diagnostic_carries_only_stable_safe_fields() {
    let directory = TempDirectory::new("diagnostics-safe-fields");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let recorder = Arc::new(RecordingParameterDiagnostics::default());
    let parameters = recording_runtime_api(database.clone(), recorder.clone());
    let permissions = permissions_api(database.clone(), PolicyTemplateName::Standard);

    // Values that look exactly like the things that must never be logged.
    let secrets = [
        ("model", "\"sk-live-abcdef0123456789\""),
        ("agent", "\"Bearer ghp_realtokenvalue\""),
        ("advisor", "\"session-id-9f2c-secret\""),
    ];
    let connection = database.connection().expect("connection");
    for (parameter_id, value_json) in secrets {
        connection
            .execute(
                "INSERT OR REPLACE INTO cli_parameter_settings
                 (agent_id, parameter_id, enabled, value_json, updated_at)
                 VALUES ('claude-code', ?1, 1, ?2, '2026-01-01T00:00:00Z')",
                params![parameter_id, value_json],
            )
            .expect("legacy row");
    }
    drop(connection);

    let launch = resolve_launch(
        &parameters,
        &permissions,
        "claude-code",
        CliLaunchScope::Chat,
        CliParameterSelectionMap::new(),
        SessionExecutionMode::Inherit,
        Some("operation-88"),
    )
    .expect("resolve");

    let emitted = recorder.emitted.lock().expect("emitted");
    assert!(
        !emitted.is_empty(),
        "the malformed values must produce diagnostics"
    );
    let encoded = serde_json::to_string(&*emitted).expect("encode diagnostics");
    for forbidden in [
        "sk-live-abcdef0123456789",
        "ghp_realtokenvalue",
        "session-id-9f2c-secret",
        "Bearer",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "a diagnostic leaked {forbidden}: {encoded}"
        );
    }
    // Only the bounded detail keys the domain defines may appear.
    let allowed_details = [
        "reason",
        "operationId",
        "storedValue",
        "requiredParameterId",
        "conflictsWith",
        "missingCount",
        "scope",
        "support",
    ];
    for diagnostic in emitted.iter() {
        for key in diagnostic.details.keys() {
            assert!(
                allowed_details.contains(&key.as_str()),
                "unexpected diagnostic detail key: {key}"
            );
        }
        if let Some(stored) = diagnostic.details.get("storedValue") {
            assert!(stored.starts_with("<redacted len="), "raw value in details");
        }
        assert!(diagnostic
            .message_key
            .starts_with("cliParameters.diagnostics."));
    }
    // argv is a different question from diagnostics. A value the user typed into `--model` is
    // meant to reach `--model`; VaneHub holds no credential on this path, and the registry already
    // refuses any credential-bearing *flag*. What must never happen is a rejected value being
    // echoed back through a diagnostic — asserted above — or a rejected value reaching argv.
    let argv = format!("{:?}{:?}", launch.global_args, launch.invocation_args);
    assert!(
        !argv.contains("ghp_realtokenvalue"),
        "a quarantined value reached argv: {argv}"
    );
    assert!(
        !argv.contains("Bearer"),
        "a quarantined value reached argv: {argv}"
    );
}

/// The prompt, the runtime session id, and the environment a launch carries are runtime-owned and
/// never reach the diagnostics port, whatever the profile contains.
#[test]
fn no_diagnostic_ever_contains_a_prompt_session_id_or_environment_value() {
    let directory = TempDirectory::new("diagnostics-no-runtime-values");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let recorder = Arc::new(RecordingParameterDiagnostics::default());
    let parameters = recording_runtime_api(database.clone(), recorder.clone());

    for template in [
        PolicyTemplateName::Readonly,
        PolicyTemplateName::Standard,
        PolicyTemplateName::Trusted,
        PolicyTemplateName::Yolo,
    ] {
        let permissions = permissions_api(database.clone(), template);
        for agent_id in MANAGED {
            for scope in [CliLaunchScope::Chat, CliLaunchScope::Interactive] {
                resolve_launch(
                    &parameters,
                    &permissions,
                    agent_id,
                    scope,
                    CliParameterSelectionMap::new(),
                    SessionExecutionMode::Inherit,
                    Some("operation-99"),
                )
                .expect("resolve");
            }
        }
    }

    let emitted = recorder.emitted.lock().expect("emitted");
    let encoded = serde_json::to_string(&*emitted).expect("encode diagnostics");
    for forbidden in [
        "prompt",
        "OPENCODE_PERMISSION",
        "VANEHUB_PERMISSION_HOOK_SCOPE",
        "stream-json",
        "--resume",
        "api_key",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "a diagnostic leaked {forbidden}: {encoded}"
        );
    }
}
