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
        action: Some(action),
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
            .recommended_installation()
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
            action: Some(CliActionKind::Upgrade),
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

/// Runs one upgrade to 1.3.0 and returns the recorded operation.
fn run_upgrade(
    harness: &Harness,
    prepare_operation: impl FnOnce(&Harness, &str),
) -> crate::contexts::tooling::cli::application::environment_test_doubles::RecordedOperation {
    let plan_id = prepare(harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    prepare_operation(harness, &operation_id);
    harness.service.execute_cli_action(prepared).expect("runs");
    harness.operations.find(&operation_id).expect("operation")
}

fn result_field<'a>(
    operation: &'a crate::contexts::tooling::cli::application::environment_test_doubles::RecordedOperation,
    field: &str,
) -> &'a serde_json::Value {
    operation
        .result
        .as_ref()
        .expect("result")
        .get(field)
        .unwrap_or(&serde_json::Value::Null)
}

#[test]
fn cancelling_before_the_process_starts_reports_cancelled_and_changes_nothing() {
    // Cancellation during the download phase: nothing has been applied, so the operation is simply
    // cancelled -- not a failure, and not a change.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let operation = run_upgrade(&harness, |harness, operation_id| {
        harness.operations.cancel(operation_id);
    });

    assert_eq!(result_field(&operation, "outcome"), "cancelled");
    assert_eq!(result_field(&operation, "termination"), "cancelled");
    // A cancelled process reports no exit code, and none is invented for it.
    assert!(result_field(&operation, "exitCode").is_null());
    assert_eq!(result_field(&operation, "warning"), true);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(
        snapshot.last_mutation.as_ref().map(|m| m.outcome),
        Some(CliMutationOutcome::Cancelled)
    );
    // The reservation is released on the cancelled path too.
    assert!(harness.coordinator.currently_held().is_empty());
}

#[test]
fn cancelling_a_process_that_already_moved_the_machine_is_not_a_clean_cancellation() {
    // Cancellation during process execution. npm was interrupted, but it had already replaced the
    // binary -- reporting this as `cancelled` would tell the user nothing happened.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let operation = run_upgrade(&harness, |harness, operation_id| {
        harness.operations.cancel(operation_id);
        harness.probes.set_version("/path/claude", "1.3.0");
    });

    assert_eq!(result_field(&operation, "outcome"), "changed-but-failed");
    assert_eq!(result_field(&operation, "termination"), "cancelled");
    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    // The snapshot describes 1.3.0, because that is what is on the machine.
    assert_eq!(
        snapshot.last_mutation.as_ref().map(|m| m.outcome),
        Some(CliMutationOutcome::ChangedButFailed)
    );
}

#[test]
fn cancelling_the_mutation_does_not_cancel_the_look_at_what_it_did() {
    // Sharing the operation's cancellation flag with post-mutation detection makes
    // `changed-but-failed` unreachable after a cancellation: the probes stop before they observe
    // the binary the package manager already replaced.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let probe_calls_before = harness.probes.invocations().len();
    let operation = run_upgrade(&harness, |harness, operation_id| {
        harness.operations.cancel(operation_id);
    });

    assert!(
        harness.probes.invocations().len() > probe_calls_before,
        "post-mutation detection did not probe after the cancellation"
    );
    assert_eq!(result_field(&operation, "observedVersion"), "1.2.0");
}

#[test]
fn detection_is_skipped_rather_than_racing_a_package_manager_mid_write() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let operation = run_upgrade(&harness, |harness, _| {
        // Another operation is writing the same npm resource.
        harness.coordinator.block_detection();
    });

    assert_eq!(
        result_field(&operation, "warnings")
            .as_array()
            .expect("warnings"),
        &vec![serde_json::json!("detection-skipped-while-busy")]
    );
    // The command succeeded; verification could not run, so the change is presumed applied rather
    // than claimed verified.
    assert_eq!(result_field(&operation, "outcome"), "applied-unverified");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    // Last-known, labelled stale. Never a half-written tree presented as the machine's state.
    assert_eq!(snapshot.freshness, CliFreshness::Stale);
    assert!(harness
        .diagnostics
        .messages()
        .iter()
        .any(|entry| entry.contains("skipped post-mutation detection")));
}

#[test]
fn a_verification_that_saw_a_different_version_says_which_one_it_saw() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    // The probe keeps reporting 1.2.0 after the upgrade to 1.3.0 "succeeded".
    let operation = run_upgrade(&harness, |_, _| {});

    assert_eq!(result_field(&operation, "targetVersion"), "1.3.0");
    assert_eq!(result_field(&operation, "observedVersion"), "1.2.0");
    assert_eq!(
        result_field(&operation, "warnings")
            .as_array()
            .expect("warnings"),
        &vec![serde_json::json!("target-version-not-observed")]
    );
}

#[test]
fn the_persisted_operation_context_carries_identity_phase_and_timing() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    harness.probes.set_version("/path/claude", "1.3.0");
    let operation = run_upgrade(&harness, |_, _| {});

    assert_eq!(result_field(&operation, "agentId"), "claude-code");
    assert_eq!(result_field(&operation, "sourceId"), "npm");
    assert_eq!(result_field(&operation, "action"), "upgrade");
    assert_eq!(result_field(&operation, "phase"), "completed");
    assert_eq!(result_field(&operation, "termination"), "exited");
    assert_eq!(result_field(&operation, "exitCode"), 0);
    assert_eq!(result_field(&operation, "outcome"), "verified");
    assert_eq!(result_field(&operation, "outputTruncated"), false);
    // A clean verified run is the one case that needs no warning.
    assert_eq!(result_field(&operation, "warning"), false);
    assert!(result_field(&operation, "elapsedMs").is_u64());
}

#[test]
fn the_persisted_context_carries_no_path_credential_or_process_output() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let operation = run_upgrade(&harness, |_, _| {});

    // The adapter emitted output onto the operation, which is where output belongs.
    assert!(!operation.output.is_empty());
    // The record itself carries none of it, and no path from the machine it ran on.
    let serialized = operation.result.as_ref().expect("result").to_string();
    assert!(!serialized.contains("/path/claude"), "{serialized}");
    assert!(!serialized.contains("fixture output"), "{serialized}");
    assert!(!serialized.contains('\\'), "{serialized}");
}

#[test]
fn the_execution_phase_chain_stops_offering_cancel_once_writing_begins() {
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    // A source that fetches an installer first, as the vendor source does.
    source.set_downloads_first();
    let operation = run_upgrade(&harness, |_, _| {});

    let downloading = operation
        .phases
        .iter()
        .position(|phase| phase == "downloading")
        .expect("downloading phase");
    let mutating = operation
        .phases
        .iter()
        .position(|phase| phase == "mutating")
        .expect("mutating phase");
    assert!(operation
        .phases
        .starts_with(&["preflight".to_string(), "resolving-source".to_string()]));
    assert!(downloading < mutating);
    // Cancel is offered while fetching and withdrawn the moment the installer runs.
    assert!(operation.cancellable[downloading]);
    assert!(!operation.cancellable[mutating]);
    // And verification follows, whatever the process did.
    assert!(operation
        .phases
        .iter()
        .any(|phase| phase == "refreshing-environment"));
    assert!(operation
        .phases
        .iter()
        .any(|phase| phase == "verifying-executable"));
}

#[test]
fn elapsed_time_never_runs_backwards() {
    use super::elapsed_ms;
    use crate::contexts::tooling::cli::application::environment_test_doubles::timestamp;

    assert_eq!(elapsed_ms(timestamp(1_000), timestamp(1_002)), 2_000);
    assert_eq!(elapsed_ms(timestamp(1_000), timestamp(1_000)), 0);
    // A clock correction mid-operation must not read as a multi-century run.
    assert_eq!(elapsed_ms(timestamp(1_000), timestamp(900)), 0);
}

/// Every terminal path a mutation can take must leave the coordinator empty.
///
/// The reservation is held by an `Arc` whose `Drop` releases it, and `release` is idempotent, so
/// one assertion per path is enough to prove exactly-once: a missed release leaves the tool locked
/// forever, and a double release would let a second holder in.
#[test]
fn every_terminal_path_releases_the_reservation_exactly_once() {
    // Success.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    harness.probes.set_version("/path/claude", "1.3.0");
    run_upgrade(&harness, |_, _| {});
    assert!(harness.coordinator.currently_held().is_empty(), "success");

    // Command failure.
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    source.set_process_failure();
    run_upgrade(&harness, |_, _| {});
    assert!(harness.coordinator.currently_held().is_empty(), "failure");

    // Cancellation.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    run_upgrade(&harness, |harness, operation_id| {
        harness.operations.cancel(operation_id)
    });
    assert!(harness.coordinator.currently_held().is_empty(), "cancelled");

    // Post-detection could not run.
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    run_upgrade(&harness, |harness, _| harness.coordinator.block_detection());
    assert!(
        harness.coordinator.currently_held().is_empty(),
        "post-detection skipped"
    );
}

#[test]
fn an_adapter_error_releases_the_reservation_before_returning() {
    // The adapter itself fails, so `run_action` returns early with `?` after the reservation was
    // taken. `Drop` is what covers this path; an explicit release alone would miss it.
    let harness = Harness::new();
    let source = installed_at(&harness, "1.2.0");
    *source.execute_error.lock().expect("execute error") = Some(
        crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError::Process(
            "npm is not installed".to_string(),
        ),
    );

    run_upgrade(&harness, |_, _| {});

    assert!(harness.coordinator.currently_held().is_empty());
}

#[test]
fn a_repository_failure_after_the_mutation_still_releases_the_reservation() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    // Saving the post-mutation snapshot fails, so `verify_and_persist` returns with `?`.
    *harness.repository.save_error.lock().expect("save error") = Some(
        crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError::Storage(
            "disk is full".to_string(),
        ),
    );

    let prepared = harness
        .service
        .prepare_cli_action_execution(ExecuteCliActionInput {
            plan_id,
            expected_revision: 1,
        })
        .expect("prepare");
    harness.service.execute_cli_action(prepared).expect("runs");

    // The machine may have changed and the write failed; the tool must still not be left locked.
    assert!(harness.coordinator.currently_held().is_empty());
}

#[test]
fn a_repeated_cancel_request_does_not_free_a_second_slot() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let operation = run_upgrade(&harness, |harness, operation_id| {
        harness.operations.cancel(operation_id);
        // The user clicks cancel again. Cancelling twice is one cancellation.
        harness.operations.cancel(operation_id);
    });

    assert_eq!(result_field(&operation, "termination"), "cancelled");
    assert!(harness.coordinator.currently_held().is_empty());
}

#[test]
fn a_second_mutation_for_the_same_tool_is_refused_while_the_first_holds_it() {
    let harness = Harness::new();
    installed_at(&harness, "1.2.0");
    let plan_id = prepare(&harness, CliActionKind::Upgrade, Some("1.3.0")).expect("plan");
    // Someone else already holds this tool.
    use crate::contexts::tooling::cli::application::environment_ports::CliMutationCoordinator;
    let _held = harness
        .coordinator
        .try_reserve(
            &crate::contexts::tooling::cli::domain::ids::CliToolId::new("claude-code")
                .expect("tool id"),
            &crate::contexts::tooling::cli::domain::source::CliMutationKey::npm_global(),
        )
        .expect("reserve")
        .expect("granted");

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
    // Refused, and the existing holder still holds exactly one reservation.
    assert_eq!(harness.coordinator.currently_held().len(), 1);
}
