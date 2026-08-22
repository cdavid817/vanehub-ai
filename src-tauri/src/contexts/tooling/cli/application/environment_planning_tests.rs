// Included through `#[path]` from environment_planning.rs.
use super::super::environment_service_fixtures::{healthy_npm_installation, npm_catalog, Harness};
use super::{ExecuteCliActionInput, PrepareCliActionInput};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::plan::{CliActionPlanState, CliPlanWarning};
use crate::contexts::tooling::cli::domain::snapshot::CliMutationOutcome;
use crate::contexts::tooling::cli::domain::status::CliFreshness;
use std::sync::Arc;

use crate::contexts::tooling::cli::application::environment_test_doubles::FakeSource;

/// A machine with claude-code 1.2.0 installed through npm, and an npm catalog offering 1.1.0,
/// 1.2.0, and 1.3.0.
fn installed_at(harness: &Harness, active: &str) -> Arc<FakeSource> {
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", active);
    let source = harness.register_npm_source(npm_catalog(
        "claude-code",
        &["1.3.0", "1.2.0", "1.1.0"],
        "1.3.0",
    ));
    // A refresh populates the snapshot and the persisted catalog that planning reads.
    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare refresh");
    harness
        .service
        .execute_refresh(prepared)
        .expect("execute refresh");
    source
}

fn prepare(
    harness: &Harness,
    action: CliActionKind,
    target: Option<&str>,
) -> Result<
    String,
    crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError,
> {
    let prepared = harness.service.prepare_cli_action(PrepareCliActionInput {
        agent_id: "claude-code".to_string(),
        action,
        source_id: "npm".to_string(),
        target_version: target.map(str::to_string),
        channel: Some("stable".to_string()),
    })?;
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_action_planning(prepared)
        .expect("planning runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    match operation.terminal.as_deref() {
        Some("succeeded") => Ok(operation
            .result
            .as_ref()
            .and_then(|result| result.get("planId"))
            .and_then(|value| value.as_str())
            .expect("planId")
            .to_string()),
        _ => Err(
            crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError::Validation(
                operation.error.clone().unwrap_or_default(),
            ),
        ),
    }
}

#[test]
fn the_version_the_user_selected_reaches_the_plan_and_the_process_arguments() {
    // The regression, at the layer that caused it: selecting 1.1.0 must install 1.1.0, not the
    // catalog's latest 1.3.0.
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");

    let plan_id = prepare(&harness, CliActionKind::Downgrade, Some("1.1.0")).expect("plan");
    let plan = harness.service.get_cli_action_plan(&plan_id).expect("plan");

    assert_eq!(plan.target_version.as_deref(), Some("1.1.0"));
    assert_eq!(plan.current_version.as_deref(), Some("1.2.0"));
    assert_eq!(plan.source_id.as_str(), "npm");
    // The adapter was asked to preview exactly the selected version.
    let preview = source.previews().into_iter().last().expect("preview");
    assert_eq!(preview.target_version.as_deref(), Some("1.1.0"));
    assert_eq!(
        preview.package_reference.as_deref(),
        Some("@anthropic-ai/claude-code")
    );
    // And the preview carries it into the arguments.
    assert!(plan
        .command_preview
        .args
        .iter()
        .any(|arg| arg.ends_with("@1.1.0")));
    assert!(!plan
        .command_preview
        .args
        .iter()
        .any(|arg| arg.contains("1.3.0")));

    // Executing carries it all the way to the process spec.
    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id: plan_id.clone(),
            expected_revision: 1,
        })
        .expect("prepare execution");
    harness
        .service
        .execute_cli_action(prepared)
        .expect("execute");

    let spec = source.executions().into_iter().last().expect("execution");
    assert!(spec.args.iter().any(|arg| arg.ends_with("@1.1.0")));
}

#[test]
fn a_target_equal_to_the_active_version_produces_no_plan() {
    // The second regression: equality used to derive "upgrade" and dispatch a redundant install.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");

    let error = prepare(&harness, CliActionKind::Upgrade, Some("1.2.0")).expect_err("refused");

    assert!(error.to_string().contains("already the active version"));
    assert!(harness.repository.plans.lock().expect("plans").is_empty());
}

#[test]
fn a_version_the_source_does_not_offer_is_refused_before_a_plan_exists() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");

    let error = prepare(&harness, CliActionKind::Upgrade, Some("9.9.9")).expect_err("refused");

    assert!(error.to_string().contains("9.9.9"));
    assert!(harness.repository.plans.lock().expect("plans").is_empty());
}

#[test]
fn execution_submits_only_a_plan_id_and_revision() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    // The input type has exactly two fields; there is nothing here to rebuild a command from.
    let input = ExecuteCliActionInput {
        plan_id: plan_id.clone(),
        expected_revision: 1,
    };
    let prepared = harness
        .service
        .prepare_cli_action_execution(input)
        .expect("prepare");
    harness
        .service
        .execute_cli_action(prepared)
        .expect("execute");

    let spec = source.executions().into_iter().last().expect("execution");
    assert_eq!(spec.program, "npm");
    assert!(spec.args.iter().any(|arg| arg.ends_with("@1.3.0")));
}

#[test]
fn a_plan_is_single_use() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    let first = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id: plan_id.clone(),
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(first).expect("execute");

    // A retry must build a new plan; reusing this one is refused.
    let second = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id: plan_id.clone(),
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = second.operation_id.clone();
    harness.service.execute_cli_action(second).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("already been used")));
}

#[test]
fn an_expired_plan_is_refused_before_any_process_starts() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    let executions_before = source.executions().len();

    // Ten minutes and one second later.
    harness.clock.advance_to(1_000 + 601);

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("expired")));
    assert_eq!(source.executions().len(), executions_before);
}

#[test]
fn a_changed_environment_makes_the_plan_stale_instead_of_running_it() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    let executions_before = source.executions().len();

    // Something outside VaneHub changed PATH between review and confirm.
    harness.discovery.set_fingerprint("fingerprint-CHANGED");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("environment changed")));
    assert_eq!(source.executions().len(), executions_before);
}

#[test]
fn a_superseded_revision_is_refused() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 7,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("revised")));
}

#[test]
fn a_failing_source_never_starts_a_different_one() {
    // The vendor-falls-back-to-npm regression. A plan names one source; if it fails, the operation
    // fails for that source.
    let harness = Harness::new();
    let vendor = FakeSource::new("vendor");
    vendor.set_process_failure();
    harness.register_source(Arc::clone(&vendor));
    let npm = installed_at(&harness, "1.2.0");
    let npm_executions_before = npm.executions().len();

    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    let plan = harness.service.get_cli_action_plan(&plan_id).expect("plan");
    assert_eq!(plan.source_id.as_str(), "npm");
    // The policy is recorded on the plan so the review dialog can state it.
    assert_eq!(
        plan.fallback_policy,
        crate::contexts::tooling::cli::domain::plan::CliFallbackPolicy::None
    );

    npm.set_process_failure();
    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(prepared).expect("runs");

    // npm ran once and failed. The vendor adapter was never invoked to compensate.
    assert_eq!(npm.executions().len(), npm_executions_before + 1);
    assert!(vendor.executions().is_empty());
}

#[test]
fn a_command_that_succeeded_while_verification_failed_is_applied_unverified() {
    // The stale-snapshot regression. npm installed 1.3.0; the post-detection probe still reports
    // 1.2.0, so the target was not verified. The result must say so -- and must not rewrite the
    // snapshot as though nothing happened.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("succeeded"));
    assert_eq!(
        operation
            .result
            .as_ref()
            .and_then(|result| result.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("applied-unverified")
    );

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(
        snapshot.last_mutation.as_ref().map(|m| m.outcome),
        Some(CliMutationOutcome::AppliedUnverified)
    );
    // Held values are last-known, not current. Nothing claims a rollback happened.
    assert_eq!(snapshot.freshness, CliFreshness::Stale);
}

#[test]
fn a_verified_mutation_reports_verified() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    // The machine really does move to 1.3.0 when the command runs.
    harness.probes.set_version("/path/claude", "1.3.0");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(
        operation
            .result
            .as_ref()
            .and_then(|result| result.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("verified")
    );
    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.freshness, CliFreshness::Fresh);
}

#[test]
fn a_failed_command_that_moved_the_machine_reports_changed_but_failed() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    // npm exits non-zero, but it had already replaced the binary.
    source.set_process_failure();
    harness.probes.set_version("/path/claude", "1.3.0");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(prepared).expect("runs");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(
        snapshot.last_mutation.as_ref().map(|m| m.outcome),
        Some(CliMutationOutcome::ChangedButFailed)
    );
    // The persisted snapshot describes the machine as it now is, not as it was.
    assert_eq!(
        snapshot
            .active_installation()
            .and_then(|i| i.reported_version.as_ref())
            .map(|v| v.as_str()),
        Some("1.3.0")
    );
}

#[test]
fn a_failed_command_that_changed_nothing_reports_no_change_failed() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    source.set_process_failure();

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(prepared).expect("runs");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(
        snapshot.last_mutation.as_ref().map(|m| m.outcome),
        Some(CliMutationOutcome::NoChangeFailed)
    );
    // Nothing moved, so the cached description is still accurate.
    assert_eq!(snapshot.freshness, CliFreshness::Fresh);
}

#[test]
fn a_plan_records_the_source_channel_and_transition_the_dialog_shows() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    let plan = harness.service.get_cli_action_plan(&plan_id).expect("plan");
    assert_eq!(plan.action, CliActionKind::Upgrade);
    assert_eq!(plan.channel.as_deref(), Some("stable"));
    assert_eq!(plan.state, CliActionPlanState::Draft);
    assert!(plan.requires_network);
    assert!(!plan.preconditions.is_empty());
    assert!(plan.command_preview.is_shell_free());
    assert!(plan.violations().is_empty());
}

#[test]
fn a_downgrade_warns_that_state_may_be_lost() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Downgrade, Some("1.1.0")).expect("plan");

    let plan = harness.service.get_cli_action_plan(&plan_id).expect("plan");
    assert!(plan
        .warnings
        .contains(&CliPlanWarning::DowngradeMayLoseState));
}

#[test]
fn an_unknown_plan_id_is_reported_as_not_found() {
    let harness = Harness::new();

    let error = harness
        .service
        .get_cli_action_plan("plan-nope")
        .expect_err("not found");

    assert_eq!(error.category(), "plan-not-found");
}

#[test]
fn preparing_an_action_for_an_unsupported_source_fails_without_an_operation() {
    let harness = Harness::new();

    let error = harness
        .service
        .prepare_cli_action(PrepareCliActionInput {
            agent_id: "codex-cli".to_string(),
            action: CliActionKind::Upgrade,
            // codex-cli has no WinGet distribution.
            source_id: "winget".to_string(),
            target_version: Some("1.0.0".to_string()),
            channel: None,
        })
        .map(|prepared| {
            let operation_id = prepared.operation_id.clone();
            harness
                .service
                .execute_action_planning(prepared)
                .expect("runs");
            harness.operations.find(&operation_id).expect("operation")
        })
        .expect("prepare returns an operation");

    assert_eq!(error.terminal.as_deref(), Some("failed"));
    assert!(error
        .error
        .as_deref()
        .is_some_and(|message| message.contains("not distributed through winget")));
}

#[test]
fn one_mutation_per_tool_is_enforced_by_the_backend_not_the_ui() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    // Something already holds the reservation for this tool.
    harness
        .coordinator
        .held
        .lock()
        .expect("held")
        .push(("claude-code".to_string(), "npm-global".to_string()));

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute_cli_action(prepared).expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("already running")));
}

#[test]
fn a_completed_mutation_releases_its_reservation() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(prepared).expect("runs");

    // Released exactly once, so the tool is not left permanently locked.
    assert!(harness.coordinator.currently_held().is_empty());
}
