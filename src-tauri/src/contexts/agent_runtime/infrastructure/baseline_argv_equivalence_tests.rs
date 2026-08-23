//! Baseline-vs-new argv equivalence for the native runtime read cutover.
//!
//! `fixtures/invocations.json` proves the two-slot split reproduces the recorded golden argv, but
//! its `expectedArgs` were written by hand. This module is stronger: it *recomputes* the
//! pre-cutover argv from the legacy renderer plus the pre-cutover placement algorithm — both
//! transcribed verbatim from `ee3eaf3f` — and compares it against what the live resolver and the
//! live builders produce for the same saved profile.
//!
//! Only profiles a v1 user could actually have saved are compared. Parameters the v2 registry
//! added are left inherited, so they contribute nothing on either side.

use super::cli_profile::resolve_launch;
use super::providers::{
    build_interactive_invocation, build_invocation, policy_override_selections,
    ProviderLaunchSegments,
};
use crate::contexts::agent_runtime::application::SessionExecutionMode;
use crate::contexts::permissions::api::{test_permissions_api_on, PolicyTemplateName};
use crate::contexts::permissions::application::{ClaudeCodeHookPort, PermissionsApplicationError};
use crate::contexts::tooling::api::{CliLaunchScope, CliParameterSelectionMap};
use crate::contexts::tooling::cli_parameters::{baseline_preview_args, CliParameterLaunchScope};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

const PROMPT: &str = "equivalence prompt";
const SESSION: &str = "equivalence-session";

struct NoopHook;
impl ClaudeCodeHookPort for NoopHook {
    fn install(&self) -> Result<(), PermissionsApplicationError> {
        Ok(())
    }
    fn remove(&self) -> Result<(), PermissionsApplicationError> {
        Ok(())
    }
}

/// Verbatim transcription of `build_invocation` at `ee3eaf3f`, including the hand-written
/// `--ephemeral` reshuffle the registry slot replaced.
fn baseline_chat_argv(
    agent_id: &str,
    prompt: &str,
    runtime_session_id: Option<&str>,
    managed_args: &[String],
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    match agent_id {
        "claude-code" => {
            args.extend_from_slice(managed_args);
            args.extend([
                "-p".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--verbose".to_string(),
            ]);
            if let Some(session_id) = runtime_session_id {
                args.extend(["--resume".to_string(), session_id.to_string()]);
            }
        }
        "codex-cli" => {
            args.extend(
                managed_args
                    .iter()
                    .filter(|argument| argument.as_str() != "--ephemeral")
                    .cloned(),
            );
            args.push("exec".to_string());
            if let Some(session_id) = runtime_session_id {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
            if managed_args
                .iter()
                .any(|argument| argument == "--ephemeral")
            {
                args.push("--ephemeral".to_string());
            }
            args.extend(["--json".to_string(), "-".to_string()]);
        }
        "gemini-cli" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = runtime_session_id {
                args.extend(["--resume".to_string(), session_id.to_string()]);
            }
            args.extend(["-o".to_string(), "stream-json".to_string()]);
        }
        "opencode" => {
            args.push("run".to_string());
            args.extend_from_slice(managed_args);
            if let Some(session_id) = runtime_session_id {
                args.extend(["--session".to_string(), session_id.to_string()]);
            }
            args.extend([
                "--format".to_string(),
                "json".to_string(),
                prompt.to_string(),
            ]);
        }
        "antigravity-cli" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = runtime_session_id {
                args.extend(["--conversation".to_string(), session_id.to_string()]);
            }
            args.extend([
                "-p".to_string(),
                prompt.to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
            ]);
        }
        other => panic!("unsupported agent: {other}"),
    }
    args
}

/// Verbatim transcription of the resume branch of `build_interactive_invocation` at `ee3eaf3f`.
/// The fresh branch mints a UUID and is therefore not sequence-comparable.
fn baseline_interactive_resume_argv(
    agent_id: &str,
    session_id: &str,
    managed_args: &[String],
) -> Vec<String> {
    let mut args = managed_args.to_vec();
    match agent_id {
        "claude-code" | "gemini-cli" => {
            args.extend(["--resume".to_string(), session_id.to_string()])
        }
        "codex-cli" => args.extend(["resume".to_string(), session_id.to_string()]),
        "opencode" => args.extend(["--session".to_string(), session_id.to_string()]),
        "antigravity-cli" => args.extend(["--conversation".to_string(), session_id.to_string()]),
        other => panic!("unsupported agent: {other}"),
    }
    args
}

/// The legacy encoding of `apply_policy_template_overrides` at `ee3eaf3f`. `"default"` and `false`
/// are the v1 inheritance sentinels the v2 envelope replaced with an explicit `inherit`.
fn baseline_policy_selections(
    agent_id: &str,
    template: PolicyTemplateName,
) -> BTreeMap<String, Value> {
    let entries: &[(&str, Value)] = match (agent_id, template) {
        ("claude-code", PolicyTemplateName::Readonly) => &[("permissionMode", json!("plan"))],
        ("claude-code", PolicyTemplateName::Standard) => &[("permissionMode", json!("default"))],
        ("claude-code", _) => &[("permissionMode", json!("acceptEdits"))],
        ("codex-cli", PolicyTemplateName::Readonly) => &[
            ("sandbox", json!("read-only")),
            ("approvalPolicy", json!("never")),
        ],
        ("codex-cli", PolicyTemplateName::Standard) => &[
            ("sandbox", json!("workspace-write")),
            ("approvalPolicy", json!("on-request")),
        ],
        ("codex-cli", _) => &[
            ("sandbox", json!("workspace-write")),
            ("approvalPolicy", json!("never")),
        ],
        ("gemini-cli", PolicyTemplateName::Readonly) => &[("approvalMode", json!("plan"))],
        ("gemini-cli", PolicyTemplateName::Standard) => &[("approvalMode", json!("default"))],
        ("gemini-cli", _) => &[("approvalMode", json!("yolo"))],
        ("opencode", PolicyTemplateName::Readonly) => &[("agent", json!("plan"))],
        ("opencode", PolicyTemplateName::Standard) => &[],
        ("opencode", _) => &[("autoApprove", json!(true))],
        ("antigravity-cli", PolicyTemplateName::Readonly) => {
            &[("mode", json!("plan")), ("sandbox", json!(true))]
        }
        ("antigravity-cli", PolicyTemplateName::Standard) => {
            &[("mode", json!("default")), ("sandbox", json!(false))]
        }
        ("antigravity-cli", _) => &[("mode", json!("accept-edits")), ("sandbox", json!(false))],
        other => panic!("unsupported combination: {other:?}"),
    };
    entries
        .iter()
        .map(|(id, value)| ((*id).to_string(), value.clone()))
        .collect()
}

/// Verbatim transcription of `force_gemini_standard_approval_flag` at `ee3eaf3f`.
fn baseline_force_gemini_standard(
    agent_id: &str,
    template: PolicyTemplateName,
    mut args: Vec<String>,
) -> Vec<String> {
    if agent_id != "gemini-cli" || template != PolicyTemplateName::Standard {
        return args;
    }
    if let Some(position) = args
        .iter()
        .position(|argument| argument == "--approval-mode")
    {
        let end = (position + 2).min(args.len());
        args.drain(position..end);
    }
    args.extend(["--approval-mode".to_string(), "default".to_string()]);
    args
}

/// Ordinary parameters a v1 user could have saved, expressed as the raw JSON the v1 settings
/// command wrote. Every one of them exists in both catalogs with the same flag and semantics.
fn v1_expressible_profile(agent_id: &str) -> Vec<(&'static str, &'static str)> {
    match agent_id {
        "claude-code" => vec![
            ("model", "\"sonnet\""),
            ("agent", "\"reviewer\""),
            ("safeMode", "true"),
            ("screenReader", "true"),
            ("bare", "true"),
        ],
        "codex-cli" => vec![
            ("model", "\"gpt-5.5\""),
            ("reasoningEffort", "\"high\""),
            ("profile", "\"work\""),
            ("search", "true"),
            ("oss", "true"),
            ("strictConfig", "true"),
            ("noAltScreen", "true"),
            ("ephemeral", "true"),
        ],
        "gemini-cli" => vec![
            ("model", "\"flash\""),
            ("debug", "true"),
            ("screenReader", "true"),
        ],
        "opencode" => vec![
            ("model", "\"anthropic/claude-sonnet\""),
            ("thinking", "true"),
            ("pure", "true"),
            ("printLogs", "true"),
            ("logLevel", "\"DEBUG\""),
        ],
        "antigravity-cli" => vec![
            ("model", "\"gemini-3-pro\""),
            ("effort", "\"high\""),
            ("agent", "\"planner\""),
        ],
        other => panic!("unsupported agent: {other}"),
    }
}

struct Harness {
    _directory: TempDirectory,
    database: NativeDatabase,
}

fn harness(label: &str) -> Harness {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Harness {
        database,
        _directory: directory,
    }
}

impl Harness {
    fn seed(&self, agent_id: &str) {
        let connection = self.database.connection().expect("connection");
        for (parameter_id, value_json) in v1_expressible_profile(agent_id) {
            connection
                .execute(
                    "INSERT OR REPLACE INTO cli_parameter_settings
                     (agent_id, parameter_id, enabled, value_json, updated_at)
                     VALUES (?1, ?2, 1, ?3, '2026-01-01T00:00:00Z')",
                    params![agent_id, parameter_id, value_json],
                )
                .expect("legacy row");
        }
    }

    /// The pre-cutover pipeline: legacy selections, legacy policy encoding, legacy renderer, legacy
    /// gemini fixup.
    fn baseline_managed_args(
        &self,
        agent_id: &str,
        template: PolicyTemplateName,
        scope: CliParameterLaunchScope,
    ) -> Vec<String> {
        let mut selections = BTreeMap::new();
        for (parameter_id, value_json) in v1_expressible_profile(agent_id) {
            selections.insert(
                parameter_id.to_string(),
                serde_json::from_str::<Value>(value_json).expect("legacy value"),
            );
        }
        selections.extend(baseline_policy_selections(agent_id, template));
        let rendered =
            baseline_preview_args(agent_id, &selections, scope).expect("baseline render");
        baseline_force_gemini_standard(agent_id, template, rendered)
    }

    fn resolved(
        &self,
        agent_id: &str,
        template: PolicyTemplateName,
        scope: CliLaunchScope,
    ) -> (Vec<String>, Vec<String>) {
        let permissions =
            test_permissions_api_on(self.database.clone(), template, Arc::new(NoopHook));
        let parameters = super::cli_profile_tests::equivalence_runtime_api(self.database.clone());
        let launch = resolve_launch(
            &parameters,
            &permissions,
            agent_id,
            scope,
            CliParameterSelectionMap::new(),
            SessionExecutionMode::Inherit,
            None,
        )
        .expect("resolve");
        (launch.global_args, launch.invocation_args)
    }
}

const AGENTS: [&str; 5] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
];

const TEMPLATES: [PolicyTemplateName; 3] = [
    PolicyTemplateName::Readonly,
    PolicyTemplateName::Standard,
    PolicyTemplateName::Trusted,
];

fn token_multiset(args: &[String]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for token in args {
        *counts.entry(token.as_str()).or_insert(0) += 1;
    }
    counts
}

/// The complete, reviewed set of argv differences the cutover introduces. Anything not listed here
/// must be byte-identical to the pre-cutover pipeline.
///
/// * `gemini-cli` + `standard`: v1 omitted a `default`-valued selection and then force-appended
///   `--approval-mode default` after every other token. The v2 registry declares `default` as a
///   real gemini-cli value, so it renders in catalog position and the post-render fixup is gone.
///   Same tokens, different index.
/// * `claude-code`, every template: v1 scoped `bare` to interactive launches only. `design.md`
///   decision 15 re-scopes it to chat/scripted launches, where the flag's documented behaviour
///   applies. A saved `bare` therefore moves scope — a token-set change, not a reordering.
fn accepted_difference(agent_id: &str, template: PolicyTemplateName) -> Option<&'static str> {
    if agent_id == "gemini-cli" && template == PolicyTemplateName::Standard {
        return Some("gemini-standard-approval-mode-position");
    }
    if agent_id == "claude-code" {
        return Some("claude-bare-rescoped-to-chat");
    }
    None
}

fn assert_equivalent(
    agent_id: &str,
    template: PolicyTemplateName,
    label: &str,
    baseline: &[String],
    actual: &[String],
) {
    match accepted_difference(agent_id, template) {
        None => {
            assert_eq!(
                token_multiset(baseline),
                token_multiset(actual),
                "{agent_id} / {template:?} {label} token set"
            );
            assert_eq!(baseline, actual, "{agent_id} / {template:?} {label} argv");
        }
        Some(reason) => assert_ne!(
            baseline, actual,
            "{agent_id} / {template:?} {label} is listed as differing ({reason}) but matched"
        ),
    }
}

#[test]
fn fresh_chat_argv_matches_the_pre_cutover_pipeline_for_every_provider() {
    for template in TEMPLATES {
        let harness = harness(&format!("equivalence-fresh-{template:?}"));
        for agent_id in AGENTS {
            harness.seed(agent_id);
            let baseline = baseline_chat_argv(
                agent_id,
                PROMPT,
                None,
                &harness.baseline_managed_args(agent_id, template, CliParameterLaunchScope::Chat),
            );
            let (global, invocation) = harness.resolved(agent_id, template, CliLaunchScope::Chat);
            let actual = build_invocation(
                agent_id,
                "exe".to_string(),
                PROMPT,
                None,
                ProviderLaunchSegments {
                    global: &global,
                    invocation: &invocation,
                },
            )
            .expect("invocation")
            .args;
            assert_equivalent(agent_id, template, "fresh chat", &baseline, &actual);
        }
    }
}

#[test]
fn resume_chat_argv_matches_the_pre_cutover_pipeline_for_every_provider() {
    for template in TEMPLATES {
        let harness = harness(&format!("equivalence-resume-{template:?}"));
        for agent_id in AGENTS {
            harness.seed(agent_id);
            let baseline = baseline_chat_argv(
                agent_id,
                PROMPT,
                Some(SESSION),
                &harness.baseline_managed_args(agent_id, template, CliParameterLaunchScope::Chat),
            );
            let (global, invocation) = harness.resolved(agent_id, template, CliLaunchScope::Chat);
            let actual = build_invocation(
                agent_id,
                "exe".to_string(),
                PROMPT,
                Some(SESSION),
                ProviderLaunchSegments {
                    global: &global,
                    invocation: &invocation,
                },
            )
            .expect("invocation")
            .args;
            assert_equivalent(agent_id, template, "resume chat", &baseline, &actual);
        }
    }
}

#[test]
fn interactive_resume_argv_matches_the_pre_cutover_pipeline_for_every_provider() {
    for template in TEMPLATES {
        let harness = harness(&format!("equivalence-interactive-{template:?}"));
        for agent_id in AGENTS {
            harness.seed(agent_id);
            let baseline = baseline_interactive_resume_argv(
                agent_id,
                SESSION,
                &harness.baseline_managed_args(
                    agent_id,
                    template,
                    CliParameterLaunchScope::Interactive,
                ),
            );
            let (global, invocation) =
                harness.resolved(agent_id, template, CliLaunchScope::Interactive);
            let actual = build_interactive_invocation(
                agent_id,
                "exe".to_string(),
                Some(SESSION),
                ProviderLaunchSegments {
                    global: &global,
                    invocation: &invocation,
                },
            )
            .expect("invocation")
            .args;
            assert_equivalent(agent_id, template, "interactive resume", &baseline, &actual);
        }
    }
}

/// Pins the exact shape of the gemini-standard difference so it cannot silently grow.
#[test]
fn the_gemini_standard_difference_is_only_the_approval_mode_position() {
    let harness = harness("equivalence-gemini-standard");
    harness.seed("gemini-cli");
    let baseline = baseline_chat_argv(
        "gemini-cli",
        PROMPT,
        None,
        &harness.baseline_managed_args(
            "gemini-cli",
            PolicyTemplateName::Standard,
            CliParameterLaunchScope::Chat,
        ),
    );
    let (global, invocation) = harness.resolved(
        "gemini-cli",
        PolicyTemplateName::Standard,
        CliLaunchScope::Chat,
    );
    let actual = build_invocation(
        "gemini-cli",
        "exe".to_string(),
        PROMPT,
        None,
        ProviderLaunchSegments {
            global: &global,
            invocation: &invocation,
        },
    )
    .expect("invocation")
    .args;

    assert_eq!(
        baseline,
        vec![
            "--model",
            "flash",
            "--debug",
            "--approval-mode",
            "default",
            "-o",
            "stream-json"
        ]
    );
    assert_eq!(
        actual,
        vec![
            "--model",
            "flash",
            "--approval-mode",
            "default",
            "--debug",
            "-o",
            "stream-json"
        ]
    );
    // Identical tokens: only the index moved.
    assert_eq!(token_multiset(&baseline), token_multiset(&actual));
}

/// Pins the `bare` re-scoping: the token moves from the interactive scope to the chat scope and
/// nothing else about the claude-code argv changes.
#[test]
fn claude_bare_moves_from_the_interactive_scope_to_the_chat_scope() {
    let harness = harness("equivalence-claude-bare");
    harness.seed("claude-code");
    let template = PolicyTemplateName::Readonly;

    let baseline_chat = baseline_chat_argv(
        "claude-code",
        PROMPT,
        None,
        &harness.baseline_managed_args("claude-code", template, CliParameterLaunchScope::Chat),
    );
    let (global, invocation) = harness.resolved("claude-code", template, CliLaunchScope::Chat);
    let actual_chat = build_invocation(
        "claude-code",
        "exe".to_string(),
        PROMPT,
        None,
        ProviderLaunchSegments {
            global: &global,
            invocation: &invocation,
        },
    )
    .expect("invocation")
    .args;
    assert!(!baseline_chat.iter().any(|token| token == "--bare"));
    assert!(actual_chat.iter().any(|token| token == "--bare"));

    let baseline_interactive = baseline_interactive_resume_argv(
        "claude-code",
        SESSION,
        &harness.baseline_managed_args(
            "claude-code",
            template,
            CliParameterLaunchScope::Interactive,
        ),
    );
    let (global, invocation) =
        harness.resolved("claude-code", template, CliLaunchScope::Interactive);
    let actual_interactive = build_interactive_invocation(
        "claude-code",
        "exe".to_string(),
        Some(SESSION),
        ProviderLaunchSegments {
            global: &global,
            invocation: &invocation,
        },
    )
    .expect("invocation")
    .args;
    assert!(baseline_interactive.iter().any(|token| token == "--bare"));
    assert!(!actual_interactive.iter().any(|token| token == "--bare"));

    // `--bare` is the only token that moved: dropping it from both sides makes them identical.
    let without_bare = |args: Vec<String>| -> Vec<String> {
        args.into_iter().filter(|token| token != "--bare").collect()
    };
    assert_eq!(without_bare(baseline_chat), without_bare(actual_chat));
    assert_eq!(
        without_bare(baseline_interactive),
        without_bare(actual_interactive)
    );
}

/// Every provider/template pair outside the reviewed list is byte-identical in both session
/// states, so the accepted differences really are two named cases rather than general drift.
#[test]
fn no_unlisted_provider_changes_its_argv() {
    let mut compared = 0;
    for template in TEMPLATES {
        let harness = harness(&format!("equivalence-strict-{template:?}"));
        for agent_id in AGENTS {
            if accepted_difference(agent_id, template).is_some() {
                continue;
            }
            harness.seed(agent_id);
            for session in [None, Some(SESSION)] {
                let baseline = baseline_chat_argv(
                    agent_id,
                    PROMPT,
                    session,
                    &harness.baseline_managed_args(
                        agent_id,
                        template,
                        CliParameterLaunchScope::Chat,
                    ),
                );
                let (global, invocation) =
                    harness.resolved(agent_id, template, CliLaunchScope::Chat);
                let actual = build_invocation(
                    agent_id,
                    "exe".to_string(),
                    PROMPT,
                    session,
                    ProviderLaunchSegments {
                        global: &global,
                        invocation: &invocation,
                    },
                )
                .expect("invocation")
                .args;
                assert_eq!(baseline, actual, "{agent_id} / {template:?}");
                compared += 1;
            }
        }
    }
    // Three templates x (five agents minus claude-code, minus gemini-cli on standard) x two
    // session states. A silent drop to zero comparisons would make this test vacuous.
    assert_eq!(compared, 22);
}

/// `policy_override_selections` is the only source of a governed value on the launch path, so its
/// projection is asserted against the legacy encoding transcribed from `ee3eaf3f`.
#[test]
fn the_policy_projection_matches_the_legacy_encoding_for_every_combination() {
    for template in [
        PolicyTemplateName::Readonly,
        PolicyTemplateName::Standard,
        PolicyTemplateName::Trusted,
        PolicyTemplateName::Yolo,
    ] {
        for agent_id in AGENTS {
            let legacy = baseline_policy_selections(agent_id, template);
            let projected = policy_override_selections(agent_id, template);
            assert_eq!(
                legacy.keys().collect::<Vec<_>>(),
                projected.keys().collect::<Vec<_>>(),
                "{agent_id} / {template:?} governs a different key set"
            );
            for (parameter_id, legacy_value) in &legacy {
                let projected_value = serde_json::to_value(&projected[parameter_id])
                    .expect("encode projected selection");
                // v1 overloaded `"default"` as its inheritance sentinel, so it becomes an explicit
                // inherit — except on `gemini-cli.approvalMode`, where `default` is the provider's
                // own ask-every-time mode. That collision is precisely why v1 needed a post-render
                // fixup to force the flag back into argv, and why the v2 registry declares it as a
                // real value instead. `convert_legacy` applies the same definition-aware rule.
                let default_is_a_real_provider_value =
                    agent_id == "gemini-cli" && parameter_id == "approvalMode";
                let expected = match legacy_value {
                    Value::String(text)
                        if text == "default" && !default_is_a_real_provider_value =>
                    {
                        json!({ "state": "inherit" })
                    }
                    other => json!({ "state": "value", "value": other }),
                };
                assert_eq!(
                    projected_value, expected,
                    "{agent_id} / {template:?} / {parameter_id}"
                );
            }
        }
    }
}
