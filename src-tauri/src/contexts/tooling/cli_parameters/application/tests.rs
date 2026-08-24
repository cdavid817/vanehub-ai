use super::fakes::{harness, TEST_CATALOG_VERSION};
use super::models::{
    CliLaunchExecutionContext, PreviewCliParameterProfileInput, ResetCliParameterProfileInput,
    ResolveCliLaunchParametersInput, SaveCliParameterProfileInput,
};
use crate::contexts::tooling::cli_parameters::domain::compatibility::CliInstallationSnapshot;
use crate::contexts::tooling::cli_parameters::domain::definition::CliLaunchScope;
use crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnosticCode;
use crate::contexts::tooling::cli_parameters::domain::selection::{
    CliParameterSelection, CliParameterSelectionMap,
};

fn selections(entries: &[(&str, CliParameterSelection)]) -> CliParameterSelectionMap {
    entries
        .iter()
        .map(|(id, selection)| ((*id).to_string(), selection.clone()))
        .collect()
}

fn preview_input(
    agent_id: &str,
    scope: CliLaunchScope,
    entries: &[(&str, CliParameterSelection)],
) -> PreviewCliParameterProfileInput {
    PreviewCliParameterProfileInput {
        agent_id: agent_id.to_string(),
        catalog_version: TEST_CATALOG_VERSION.to_string(),
        scope,
        selections: selections(entries),
        request_id: Some("req-1".to_string()),
    }
}

fn save_input(
    agent_id: &str,
    expected_revision: i64,
    entries: &[(&str, CliParameterSelection)],
) -> SaveCliParameterProfileInput {
    SaveCliParameterProfileInput {
        agent_id: agent_id.to_string(),
        expected_revision,
        catalog_version: TEST_CATALOG_VERSION.to_string(),
        selections: selections(entries),
    }
}

#[test]
fn list_returns_every_managed_profile_in_catalog_order() {
    let harness = harness();
    let profiles = harness.service.list_profiles().expect("list");
    assert_eq!(
        profiles
            .iter()
            .map(|profile| profile.agent_id.as_str())
            .collect::<Vec<_>>(),
        [
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli"
        ]
    );
    assert!(profiles
        .iter()
        .all(|profile| profile.catalog_version == TEST_CATALOG_VERSION));
}

#[test]
fn a_listed_profile_omits_policy_governed_definitions() {
    let harness = harness();
    let profiles = harness.service.list_profiles().expect("list");
    let claude = profiles
        .iter()
        .find(|profile| profile.agent_id == "claude-code")
        .expect("claude profile");
    let ids = claude
        .fields
        .iter()
        .map(|field| field.definition.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["model", "screenReader"]);
    assert!(!claude.selections.contains_key("permissionMode"));
}

#[test]
fn preview_does_not_mutate_persistence() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("save");
    let writes_before = *harness.repository.writes.lock().expect("lock");
    let revision_before = harness.repository.revision("claude-code");

    harness
        .service
        .preview_profile(&preview_input(
            "claude-code",
            CliLaunchScope::Chat,
            &[("model", CliParameterSelection::text("opus"))],
        ))
        .expect("preview");

    assert_eq!(
        *harness.repository.writes.lock().expect("lock"),
        writes_before
    );
    assert_eq!(harness.repository.revision("claude-code"), revision_before);
}

#[test]
fn preview_differs_between_chat_and_interactive_scope() {
    let harness = harness();
    let entries = [("ephemeral", CliParameterSelection::boolean(true))];
    let chat = harness
        .service
        .preview_profile(&preview_input("codex-cli", CliLaunchScope::Chat, &entries))
        .expect("chat preview");
    let interactive = harness
        .service
        .preview_profile(&preview_input(
            "codex-cli",
            CliLaunchScope::Interactive,
            &entries,
        ))
        .expect("interactive preview");

    assert_eq!(chat.segments.invocation_values(), ["--ephemeral"]);
    assert!(interactive.segments.is_empty());
    assert_eq!(chat.request_id.as_deref(), Some("req-1"));
}

#[test]
fn preview_returns_a_structured_field_error_for_an_invalid_value() {
    let harness = harness();
    let error = harness
        .service
        .preview_profile(&preview_input(
            "claude-code",
            CliLaunchScope::Chat,
            &[("model", CliParameterSelection::text("  "))],
        ))
        .expect_err("must reject");
    assert_eq!(error.code().as_str(), "CLI_PARAMETER_INVALID_VALUE");
    assert_eq!(error.parameter_id(), Some("model"));
}

#[test]
fn an_unsatisfied_dependency_blocks_save_and_is_reported_in_preview() {
    let harness = harness();
    let preview = harness
        .service
        .preview_profile(&preview_input(
            "codex-cli",
            CliLaunchScope::Chat,
            &[("localProvider", CliParameterSelection::text("ollama"))],
        ))
        .expect("preview");
    assert!(preview
        .diagnostics
        .iter()
        .any(|entry| entry.code == CliParameterDiagnosticCode::DependencyNotSatisfied));

    let error = harness
        .service
        .save_profile(&save_input(
            "codex-cli",
            0,
            &[("localProvider", CliParameterSelection::text("ollama"))],
        ))
        .expect_err("must reject");
    assert_eq!(
        error.code().as_str(),
        "CLI_PARAMETER_DEPENDENCY_UNSATISFIED"
    );
    assert_eq!(harness.repository.revision("codex-cli"), 0);
}

#[test]
fn a_satisfied_dependency_saves() {
    let harness = harness();
    let profile = harness
        .service
        .save_profile(&save_input(
            "codex-cli",
            0,
            &[
                ("oss", CliParameterSelection::boolean(true)),
                ("localProvider", CliParameterSelection::text("ollama")),
            ],
        ))
        .expect("save");
    assert_eq!(profile.revision, 1);
}

#[test]
fn a_policy_governed_parameter_cannot_be_saved_through_the_user_path() {
    let harness = harness();
    let error = harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("permissionMode", CliParameterSelection::text("plan"))],
        ))
        .expect_err("must reject");
    assert_eq!(error.code().as_str(), "CLI_PARAMETER_UNKNOWN_PARAMETER");
    assert_eq!(harness.repository.revision("claude-code"), 0);
}

#[test]
fn a_stale_revision_and_a_stale_catalog_version_are_both_rejected() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("save");

    let stale_revision = harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("opus"))],
        ))
        .expect_err("stale revision");
    assert_eq!(
        stale_revision.code().as_str(),
        "CLI_PARAMETER_REVISION_CONFLICT"
    );

    let mut stale_catalog = save_input("claude-code", 1, &[]);
    stale_catalog.catalog_version = "0.0.1".to_string();
    let error = harness
        .service
        .save_profile(&stale_catalog)
        .expect_err("stale catalog");
    assert_eq!(error.code().as_str(), "CLI_PARAMETER_CATALOG_MISMATCH");
    assert_eq!(
        error
            .details()
            .get("expectedCatalogVersion")
            .map(String::as_str),
        Some(TEST_CATALOG_VERSION)
    );
}

#[test]
fn reset_increments_the_revision_once_and_restores_inheritance() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("save");
    let profile = harness
        .service
        .reset_profile(&ResetCliParameterProfileInput {
            agent_id: "claude-code".to_string(),
            expected_revision: 1,
            catalog_version: TEST_CATALOG_VERSION.to_string(),
        })
        .expect("reset");
    assert_eq!(profile.revision, 2);
    assert_eq!(profile.selections["model"], CliParameterSelection::Inherit);
}

#[test]
fn a_missing_cli_still_loads_and_saves_an_ungated_parameter() {
    let harness = harness();
    harness.installations.set(
        "claude-code",
        CliInstallationSnapshot {
            installed: false,
            runnable: false,
            active_path: None,
            version: None,
            conflict: false,
        },
    );
    let profile = harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("ungated save is allowed while the CLI is missing");
    assert!(profile
        .diagnostics
        .iter()
        .any(|entry| entry.code == CliParameterDiagnosticCode::CliNotInstalled));
    assert_eq!(profile.revision, 1);
}

#[test]
fn a_version_gated_parameter_cannot_receive_a_new_value_while_unsupported() {
    let harness = harness();
    harness.installations.set(
        "claude-code",
        CliInstallationSnapshot {
            installed: true,
            runnable: true,
            active_path: Some("/usr/bin/claude".to_string()),
            version: Some("2.1.100".to_string()),
            conflict: false,
        },
    );
    let error = harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("screenReader", CliParameterSelection::boolean(true))],
        ))
        .expect_err("must reject");
    assert_eq!(error.code().as_str(), "CLI_PARAMETER_UNSUPPORTED_VERSION");
}

#[test]
fn an_installation_conflict_becomes_a_non_blocking_diagnostic() {
    let harness = harness();
    harness.installations.set(
        "claude-code",
        CliInstallationSnapshot {
            installed: true,
            runnable: true,
            active_path: Some("/usr/bin/claude".to_string()),
            version: Some("2.2.0".to_string()),
            conflict: true,
        },
    );
    let profile = harness
        .service
        .load_profile("claude-code")
        .expect("load")
        .view;
    let conflict = profile
        .diagnostics
        .iter()
        .find(|entry| entry.code == CliParameterDiagnosticCode::ActiveInstallationConflict)
        .expect("conflict diagnostic");
    assert!(!conflict.blocking);
}

#[test]
fn a_legacy_profile_loads_with_repair_diagnostics_and_keeps_valid_rows() {
    let harness = harness();
    harness.repository.seed_legacy(
        "claude-code",
        &[
            ("model", "\"sonnet\""),
            ("removed", "\"x\""),
            ("broken", "not-json"),
        ],
    );
    let profile = harness
        .service
        .load_profile("claude-code")
        .expect("load")
        .view;
    assert_eq!(
        profile.selections["model"],
        CliParameterSelection::text("sonnet")
    );
    let quarantined = profile
        .diagnostics
        .iter()
        .filter(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
        .count();
    assert_eq!(quarantined, 2);
}

#[test]
fn a_message_override_beats_the_saved_profile_value() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("save");
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: selections(&[("model", CliParameterSelection::text("opus"))]),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: Default::default(),
        })
        .expect("resolve");
    assert_eq!(resolved.global_tokens, ["--model", "opus"]);
    assert_eq!(harness.repository.revision("claude-code"), 1);
}

#[test]
fn the_saved_profile_is_used_when_no_message_override_exists() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("model", CliParameterSelection::text("sonnet"))],
        ))
        .expect("save");
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: Default::default(),
        })
        .expect("resolve");
    assert_eq!(resolved.global_tokens, ["--model", "sonnet"]);
}

#[test]
fn an_inherited_selection_emits_no_user_profile_token() {
    let harness = harness();
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: Default::default(),
        })
        .expect("resolve");
    assert!(resolved.global_tokens.is_empty());
    assert!(resolved.invocation_tokens.is_empty());
}

#[test]
fn a_policy_override_is_rendered_and_a_user_editable_id_is_refused_on_that_path() {
    let harness = harness();
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: selections(&[(
                "permissionMode",
                CliParameterSelection::text("plan"),
            )]),
            execution_context: Default::default(),
        })
        .expect("resolve");
    assert_eq!(resolved.global_tokens, ["--permission-mode", "plan"]);

    let error = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: selections(&[("model", CliParameterSelection::text("opus"))]),
            execution_context: Default::default(),
        })
        .expect_err("a user-editable id is not a policy value");
    assert_eq!(error.code().as_str(), "CLI_PARAMETER_UNKNOWN_PARAMETER");
}

#[test]
fn an_unsupported_saved_value_is_omitted_from_a_launch_with_a_diagnostic() {
    let harness = harness();
    harness
        .service
        .save_profile(&save_input(
            "claude-code",
            0,
            &[("screenReader", CliParameterSelection::boolean(true))],
        ))
        .expect("save while supported");
    harness.installations.set(
        "claude-code",
        CliInstallationSnapshot {
            installed: true,
            runnable: true,
            active_path: Some("/usr/bin/claude".to_string()),
            version: Some("2.1.100".to_string()),
            conflict: false,
        },
    );
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Interactive,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: Default::default(),
        })
        .expect("resolve");
    assert!(resolved.global_tokens.is_empty());
    assert!(resolved
        .diagnostics
        .iter()
        .any(|entry| entry.code == CliParameterDiagnosticCode::UnsupportedByActiveVersion));
}

#[test]
fn an_unknown_agent_is_rejected_on_every_entry_point() {
    let harness = harness();
    assert!(harness.service.load_profile("nope").is_err());
    assert!(harness
        .service
        .preview_profile(&preview_input("nope", CliLaunchScope::Chat, &[]))
        .is_err());
    assert!(harness
        .service
        .save_profile(&save_input("nope", 0, &[]))
        .is_err());
    assert_eq!(*harness.repository.writes.lock().expect("lock"), 0);
}

#[test]
fn repeated_identical_diagnostics_are_emitted_once_per_load() {
    let harness = harness();
    harness.installations.set(
        "claude-code",
        CliInstallationSnapshot {
            installed: false,
            runnable: false,
            active_path: None,
            version: None,
            conflict: false,
        },
    );
    harness.service.load_profile("claude-code").expect("load");
    let emitted = harness.diagnostics.emitted.lock().expect("lock");
    let not_installed = emitted
        .iter()
        .filter(|entry| entry.code == CliParameterDiagnosticCode::CliNotInstalled)
        .count();
    assert_eq!(not_installed, 1);
}

#[test]
fn a_launch_diagnostic_is_associated_with_the_triggering_operation() {
    let harness = harness();
    harness
        .repository
        .seed_legacy("claude-code", &[("removedParameter", "\"x\"")]);
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Chat,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: CliLaunchExecutionContext {
                operation_id: Some("operation-42".to_string()),
            },
        })
        .expect("resolve");

    let quarantined = resolved
        .diagnostics
        .iter()
        .find(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
        .expect("quarantine diagnostic");
    assert_eq!(
        quarantined.details.get("operationId").map(String::as_str),
        Some("operation-42")
    );
    let emitted = harness.diagnostics.emitted.lock().expect("lock");
    assert!(emitted
        .iter()
        .any(|entry| entry.details.get("operationId").map(String::as_str) == Some("operation-42")));
}

#[test]
fn a_terminal_launch_without_an_operation_still_resolves() {
    let harness = harness();
    harness
        .repository
        .seed_legacy("claude-code", &[("model", "\"sonnet\"")]);
    let resolved = harness
        .service
        .resolve_launch_parameters(&ResolveCliLaunchParametersInput {
            agent_id: "claude-code".to_string(),
            scope: CliLaunchScope::Interactive,
            message_overrides: CliParameterSelectionMap::new(),
            policy_overrides: CliParameterSelectionMap::new(),
            execution_context: CliLaunchExecutionContext::default(),
        })
        .expect("resolve");
    assert_eq!(resolved.global_tokens, ["--model", "sonnet"]);
    assert!(resolved
        .diagnostics
        .iter()
        .all(|entry| !entry.details.contains_key("operationId")));
}
