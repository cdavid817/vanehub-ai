//! Characterization of the launch-affecting semantics that must survive the native runtime read
//! cutover. These lock the *legacy* module's behaviour so the v2 resolver can be proven equivalent
//! rather than merely plausible.

use super::legacy_baseline::{
    catalog_for, editable_catalog_for, is_policy_governed, load_profile,
    normalize_with_definitions, render_args, CliParameterLaunchScope, MANAGED_CLI_AGENT_IDS,
};
use super::CliParametersError;
use serde_json::Value;
use std::collections::BTreeMap;

/// The legacy renderer, reached through the private helpers the settings commands still use. The
/// public `preview_args` wrapper was removed with the runtime cutover: no launch path calls it.
fn legacy_preview(
    agent_id: &str,
    selections: &BTreeMap<String, Value>,
    scope: CliParameterLaunchScope,
) -> Result<Vec<String>, CliParametersError> {
    let normalized = normalize_with_definitions(agent_id, selections, catalog_for(agent_id)?)?;
    Ok(render_args(catalog_for(agent_id)?, &normalized, scope))
}

fn legacy_normalize(
    agent_id: &str,
    selections: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, CliParametersError> {
    normalize_with_definitions(agent_id, selections, catalog_for(agent_id)?)
}

fn selections(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
    entries
        .iter()
        .map(|(id, value)| ((*id).to_string(), value.clone()))
        .collect()
}

/// Task 1.6 — the Codex reasoning renderer is currently a `definition.id == "reasoningEffort"`
/// branch. Its argv shape is what the declarative `config-key-value` strategy has to reproduce.
#[test]
fn codex_model_reasoning_effort_renders_as_two_tokens_with_a_quoted_toml_value() {
    for effort in ["low", "medium", "high", "xhigh", "max"] {
        let args = legacy_preview(
            "codex-cli",
            &selections(&[("reasoningEffort", Value::String(effort.to_string()))]),
            CliParameterLaunchScope::Chat,
        )
        .expect("preview");
        let position = args
            .iter()
            .position(|token| token == "--config")
            .expect("--config token");
        assert_eq!(args[position], "--config");
        assert_eq!(
            args[position + 1],
            format!("model_reasoning_effort=\"{effort}\"")
        );
        // The quotes belong to the TOML value, not to shell quoting: the key=value pair is one
        // argv token and is never split.
        assert!(args[position + 1].starts_with("model_reasoning_effort=\""));
        assert!(args[position + 1].ends_with('"'));
    }
}

#[test]
fn codex_inherited_reasoning_effort_emits_no_config_token() {
    let args = legacy_preview(
        "codex-cli",
        &selections(&[("reasoningEffort", Value::String("default".to_string()))]),
        CliParameterLaunchScope::Chat,
    )
    .expect("preview");
    assert!(!args.iter().any(|token| token == "--config"));
    assert!(!args
        .iter()
        .any(|token| token.contains("model_reasoning_effort")));
}

/// Task 1.7 — policy-governed parameters are owned by Agent Policies. The user profile path must
/// refuse them on the way in and never surface them on the way out.
#[test]
fn policy_governed_parameters_cannot_be_submitted_through_the_user_profile_path() {
    let governed: [(&str, &str, Value); 9] = [
        (
            "claude-code",
            "permissionMode",
            Value::String("plan".to_string()),
        ),
        (
            "codex-cli",
            "sandbox",
            Value::String("read-only".to_string()),
        ),
        (
            "codex-cli",
            "approvalPolicy",
            Value::String("never".to_string()),
        ),
        (
            "gemini-cli",
            "approvalMode",
            Value::String("yolo".to_string()),
        ),
        ("gemini-cli", "sandbox", Value::Bool(true)),
        ("opencode", "agent", Value::String("plan".to_string())),
        ("opencode", "autoApprove", Value::Bool(true)),
        ("antigravity-cli", "mode", Value::String("plan".to_string())),
        ("antigravity-cli", "sandbox", Value::Bool(true)),
    ];
    for (agent_id, parameter_id, value) in governed {
        assert!(
            is_policy_governed(agent_id, parameter_id),
            "{agent_id}.{parameter_id} must be policy governed"
        );
        let editable = editable_catalog_for(agent_id).expect("editable catalog");
        assert!(
            !editable.iter().any(|entry| entry.id == parameter_id),
            "{agent_id}.{parameter_id} must not be an editable definition"
        );
        // The editable normalizer is what a save runs through; it rejects the whole mutation.
        let definitions = editable_catalog_for(agent_id).expect("editable catalog");
        let submitted = selections(&[(parameter_id, value)]);
        let ids = definitions
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();
        assert!(!ids.contains(&parameter_id));
        assert!(
            normalize_with_definitions(agent_id, &submitted, definitions).is_err(),
            "{agent_id}.{parameter_id} must be rejected as an unknown editable parameter"
        );
    }
}

/// Runtime-reserved arguments are constructed by VaneHub. No catalog entry may collide with one,
/// on any launch scope, for any agent.
#[test]
fn runtime_reserved_and_dangerous_arguments_are_absent_from_every_catalog() {
    let reserved = [
        "-p",
        "-o",
        "-c",
        "-",
        "--prompt",
        "--output-format",
        "--format",
        "--json",
        "--resume",
        "--session",
        "--session-id",
        "--conversation",
        "--include-partial-messages",
        "--verbose",
        "--append-system-prompt",
    ];
    for agent_id in MANAGED_CLI_AGENT_IDS {
        for definition in catalog_for(agent_id).expect("catalog") {
            assert!(
                !reserved.contains(&definition.flag.as_str()),
                "{agent_id}.{} maps to reserved {}",
                definition.id,
                definition.flag
            );
            assert!(!definition.flag.contains("dangerously"));
            for scope in [
                CliParameterLaunchScope::Chat,
                CliParameterLaunchScope::Interactive,
            ] {
                let args = legacy_preview(agent_id, &BTreeMap::new(), scope).expect("preview");
                assert!(args.iter().all(|token| !reserved.contains(&token.as_str())));
            }
        }
    }
}

#[test]
fn a_custom_value_containing_whitespace_stays_one_argv_token() {
    let args = legacy_preview(
        "claude-code",
        &selections(&[("model", Value::String("my model name".to_string()))]),
        CliParameterLaunchScope::Chat,
    )
    .expect("preview");
    assert_eq!(
        args,
        vec!["--model".to_string(), "my model name".to_string()]
    );
    assert_eq!(args.len(), 2);
}

#[test]
fn preview_never_contains_prompt_session_or_output_protocol_values() {
    for agent_id in MANAGED_CLI_AGENT_IDS {
        for scope in [
            CliParameterLaunchScope::Chat,
            CliParameterLaunchScope::Interactive,
        ] {
            let args = legacy_preview(agent_id, &BTreeMap::new(), scope).expect("preview");
            for token in &args {
                let lowered = token.to_ascii_lowercase();
                assert!(!lowered.contains("prompt"));
                assert!(!lowered.contains("session"));
                assert!(!lowered.contains("stream-json"));
                assert!(!lowered.contains("api_key"));
                assert!(!lowered.contains("token"));
            }
        }
    }
}

#[test]
fn the_editable_profile_view_excludes_every_policy_governed_selection() {
    let conn = rusqlite::Connection::open_in_memory().expect("database");
    conn.execute_batch(
        "CREATE TABLE agents (id TEXT PRIMARY KEY); \
         INSERT INTO agents VALUES ('claude-code'), ('codex-cli'), ('gemini-cli'), ('opencode');",
    )
    .expect("agents");
    super::apply_schema(&conn).expect("schema");
    for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode"] {
        let profile = load_profile(&conn, agent_id).expect("profile");
        for definition in &profile.definitions {
            assert!(!is_policy_governed(agent_id, &definition.id));
        }
        for parameter_id in profile.selections.keys() {
            assert!(!is_policy_governed(agent_id, parameter_id));
        }
    }
}

#[test]
fn legacy_normalization_rejects_unknown_parameters_and_control_characters() {
    assert!(legacy_normalize("unknown-agent", &BTreeMap::new()).is_err());
    assert!(legacy_normalize(
        "codex-cli",
        &selections(&[("nope", Value::String("x".to_string()))]),
    )
    .is_err());
    assert!(legacy_normalize(
        "claude-code",
        &selections(&[("model", Value::String("sonnet\n--resume".to_string()))]),
    )
    .is_err());
}
/// Walks a source subtree, skipping test files and `#[cfg(test)]` tails, and returns the
/// production lines matching any needle.
fn production_hits(relative_root: &str, needles: &[&str]) -> Vec<String> {
    fn walk(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_root);
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut hits = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.contains("test") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        // Everything after the first `#[cfg(test)]` in a file is test-only.
        let production = source
            .split_once("#[cfg(test)]")
            .map(|(before, _)| before)
            .unwrap_or(source.as_str());
        for (index, line) in production.lines().enumerate() {
            if needles.iter().any(|needle| line.contains(needle)) {
                hits.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
            }
        }
    }
    hits
}

/// Invariant IV — every real launch reads through the published resolver. No production code in
/// `agent_runtime` may reach the legacy selection store or the legacy renderer again.
#[test]
fn the_launch_path_no_longer_reads_the_legacy_selection_store() {
    let hits = production_hits(
        "src/contexts/agent_runtime",
        &[
            "cli_parameter_settings",
            "preview_args",
            "load_selections",
            "normalize_selections",
        ],
    );
    assert!(
        hits.is_empty(),
        "agent_runtime reached the legacy path:\n{hits:#?}"
    );
}

/// Invariant VI — Tooling owns CLI parameters and knows nothing about Permissions. The policy
/// projection is computed by `agent_runtime` and handed in as a separate resolver input.
#[test]
fn tooling_never_depends_on_permissions() {
    let hits = production_hits("src/contexts/tooling", &["contexts::permissions"]);
    assert!(
        hits.is_empty(),
        "tooling depends on permissions:\n{hits:#?}"
    );
}
