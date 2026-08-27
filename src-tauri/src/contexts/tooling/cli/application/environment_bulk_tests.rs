// Included through `#[path]` from environment_bulk.rs.
use super::super::environment_service_fixtures::{healthy_npm_installation, npm_catalog, Harness};
use crate::contexts::tooling::cli::domain::bulk::CliBulkSkipReason;
use crate::contexts::tooling::cli::domain::snapshot::CliMutationOutcome;

/// Installs `agent_id` at `active` with an npm catalog whose latest is `latest`, then refreshes so
/// the snapshot and catalog planning reads are populated.
fn install(harness: &Harness, agent_id: &str, active: &str, latest: &str) {
    install_returning_source(harness, agent_id, active, latest);
}

/// The same, handing back the source adapter so a test can make its process fail.
fn install_returning_source(
    harness: &Harness,
    agent_id: &str,
    active: &str,
    latest: &str,
) -> std::sync::Arc<crate::contexts::tooling::cli::application::environment_test_doubles::FakeSource>
{
    let path = format!("/path/{agent_id}");
    harness
        .discovery
        .set(agent_id, vec![healthy_npm_installation(agent_id, &path)]);
    harness.probes.set_version(&path, active);
    let source = harness.register_npm_source(npm_catalog(agent_id, &[latest, active], latest));

    let prepared = harness
        .service
        .prepare_refresh(vec![agent_id.to_string()], false)
        .expect("prepare refresh");
    harness
        .service
        .execute_refresh(prepared)
        .expect("execute refresh");
    source
}

fn prepare_bulk(harness: &Harness, agent_ids: &[&str]) -> String {
    let prepared = harness
        .service
        .prepare_cli_bulk_upgrade(agent_ids.iter().map(|id| (*id).to_string()).collect())
        .expect("prepare bulk");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_bulk_planning(prepared)
        .expect("planning runs");
    harness
        .operations
        .find(&operation_id)
        .and_then(|operation| operation.result)
        .and_then(|result| {
            result
                .get("planId")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .expect("bulk plan id")
}

#[test]
fn an_outdated_tool_becomes_an_eligible_item_with_its_own_plan() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");

    let plan_id = prepare_bulk(&harness, &["claude-code"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert_eq!(plan.items.len(), 1);
    let item = &plan.items[0];
    assert_eq!(item.agent_id.as_str(), "claude-code");
    assert_eq!(item.current_version.as_deref(), Some("1.2.0"));
    assert_eq!(item.target_version.as_deref(), Some("1.3.0"));
    assert_eq!(item.source_id.as_str(), "npm");
    // The item points at a real, single-use plan that was persisted with the batch.
    assert!(harness.repository.plan(item.plan_id.as_str()).is_some());
}

#[test]
fn an_up_to_date_tool_is_skipped_as_already_current() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.3.0", "1.3.0");

    let plan_id = prepare_bulk(&harness, &["claude-code"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert!(plan.items.is_empty());
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.skipped[0].reason, CliBulkSkipReason::AlreadyCurrent);
    // Recorded, but not presented as something to fix.
    assert!(plan.actionable_skips().is_empty());
    assert!(!plan.has_work());
}

#[test]
fn a_tool_that_was_never_scanned_is_skipped_as_not_installed() {
    let harness = Harness::new();

    let plan_id = prepare_bulk(&harness, &["codex-cli"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert_eq!(plan.skipped[0].reason, CliBulkSkipReason::NotInstalled);
}

#[test]
fn a_broken_tool_is_skipped_rather_than_upgraded() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_failure("/path/claude", false);
    harness.register_npm_source(npm_catalog("claude-code", &["1.3.0"], "1.3.0"));
    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare");
    harness.service.execute_refresh(prepared).expect("refresh");

    let plan_id = prepare_bulk(&harness, &["claude-code"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert_eq!(plan.skipped[0].reason, CliBulkSkipReason::Broken);
    // Worth surfacing: the user can act on a broken install.
    assert_eq!(plan.actionable_skips().len(), 1);
}

#[test]
fn a_tool_with_no_readable_catalog_is_skipped_with_its_own_reason() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");
    harness.register_failing_npm_source();
    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare");
    harness.service.execute_refresh(prepared).expect("refresh");

    let plan_id = prepare_bulk(&harness, &["claude-code"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert_eq!(
        plan.skipped[0].reason,
        CliBulkSkipReason::CatalogUnavailable
    );
}

#[test]
fn a_mixed_batch_lists_both_the_work_and_the_reasons() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    install(&harness, "codex-cli", "2.0.0", "2.0.0");

    let plan_id = prepare_bulk(&harness, &["claude-code", "codex-cli"]);
    let plan = harness
        .service
        .get_cli_bulk_action_plan(&plan_id)
        .expect("plan");

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].agent_id.as_str(), "claude-code");
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.skipped[0].agent_id.as_str(), "codex-cli");
    assert!(plan.has_work());
}

#[test]
fn executing_a_batch_runs_every_item_through_the_single_action_path() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let prepared = harness
        .service
        .prepare_cli_bulk_execution(&plan_id, 1)
        .expect("prepare execution");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_bulk_action(prepared)
        .expect("execute");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("succeeded"));
    assert_eq!(operation.units, vec![(1, 1)]);
    // The per-tool reservation was taken and released, not bypassed.
    assert!(harness.coordinator.currently_held().is_empty());
}

#[test]
fn a_superseded_batch_revision_is_refused() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let prepared = harness
        .service
        .prepare_cli_bulk_execution(&plan_id, 9)
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_bulk_action(prepared)
        .expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("revised")));
}

#[test]
fn an_expired_batch_is_refused_before_any_item_runs() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    let plan_id = prepare_bulk(&harness, &["claude-code"]);
    harness.clock.advance_to(1_000 + 601);

    let prepared = harness
        .service
        .prepare_cli_bulk_execution(&plan_id, 1)
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_bulk_action(prepared)
        .expect("runs");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("expired")));
}

#[test]
fn an_unknown_bulk_plan_id_is_reported_as_not_found() {
    let harness = Harness::new();

    let error = harness
        .service
        .get_cli_bulk_action_plan("bulk-nope")
        .expect_err("not found");

    assert_eq!(error.category(), "plan-not-found");
}

#[test]
fn doctor_returns_unknown_for_a_cli_with_no_documented_probe() {
    let harness = Harness::new();
    install(&harness, "gemini-cli", "1.0.0", "1.0.0");

    let prepared = harness
        .service
        .prepare_cli_doctor("gemini-cli")
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_doctor(prepared)
        .expect("execute");

    let operation = harness.operations.find(&operation_id).expect("operation");
    let doctor = operation
        .result
        .as_ref()
        .and_then(|result| result.get("doctor"))
        .and_then(|value| value.as_str());
    // Unknown rather than a health verdict invented from the absence of a probe.
    assert_eq!(doctor, Some("unknown"));
    assert_eq!(
        operation
            .result
            .as_ref()
            .and_then(|result| result.get("reason"))
            .and_then(|value| value.as_str()),
        Some("undocumented-probe")
    );
}

#[test]
fn doctor_runs_the_declared_probe_for_a_cli_that_has_one() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    harness.probes.set_version("/path/claude-code", "1.2.0");

    let prepared = harness
        .service
        .prepare_cli_doctor("claude-code")
        .expect("prepare");
    harness
        .service
        .execute_cli_doctor(prepared)
        .expect("execute");

    // Claude Code declares `doctor`; the probe was invoked with exactly that argv.
    assert!(harness
        .probes
        .invocations()
        .iter()
        .any(|(_, args)| args == &vec!["doctor".to_string()]));
}

#[test]
fn doctor_on_a_tool_that_is_not_installed_reports_unknown() {
    let harness = Harness::new();

    let prepared = harness
        .service
        .prepare_cli_doctor("claude-code")
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_doctor(prepared)
        .expect("execute");

    let operation = harness.operations.find(&operation_id).expect("operation");
    assert_eq!(
        operation
            .result
            .as_ref()
            .and_then(|result| result.get("reason"))
            .and_then(|value| value.as_str()),
        Some("not-installed")
    );
}

#[test]
fn doctor_for_an_unknown_agent_fails_without_starting_an_operation() {
    let harness = Harness::new();

    let error = harness
        .service
        .prepare_cli_doctor("not-a-cli")
        .expect_err("unknown");

    assert_eq!(error.category(), "unknown-tool");
    assert!(harness.operations.all().is_empty());
}

#[test]
fn expired_draft_plans_are_swept_in_bounded_batches() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    prepare_bulk(&harness, &["claude-code"]);

    // Nothing expired yet.
    assert_eq!(harness.service.expire_stale_plans().expect("sweep"), 0);

    harness.clock.advance_to(1_000 + 601);
    assert_eq!(harness.service.expire_stale_plans().expect("sweep"), 1);
    // Idempotent: a second sweep finds nothing left in draft.
    assert_eq!(harness.service.expire_stale_plans().expect("sweep"), 0);
}

/// Runs a prepared batch and returns its per-item results.
fn run_bulk_items(harness: &Harness, plan_id: &str) -> Vec<serde_json::Value> {
    let prepared = harness
        .service
        .prepare_cli_bulk_execution(plan_id, 1)
        .expect("prepare bulk execution");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_cli_bulk_action(prepared)
        .expect("bulk runs");
    harness
        .operations
        .find(&operation_id)
        .and_then(|operation| operation.result)
        .and_then(|result| {
            result
                .get("items")
                .and_then(|items| items.as_array())
                .cloned()
        })
        .expect("item results")
}

#[test]
fn every_bulk_item_reports_a_real_mutation_outcome() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    // The machine really moves, so the item verifies.
    harness.probes.set_version("/path/claude-code", "1.3.0");
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let items = run_bulk_items(&harness, &plan_id);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["agentId"], "claude-code");
    assert_eq!(items[0]["status"], "completed");
    // The five-state outcome the single-action path produced, not a label saying a process ran.
    assert_eq!(items[0]["outcome"], CliMutationOutcome::Verified.as_str());
    assert!(items[0]["reason"].is_null());
}

#[test]
fn a_bulk_item_whose_verification_failed_reports_applied_unverified() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    // The probe keeps reporting the old version after the command succeeded.
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let items = run_bulk_items(&harness, &plan_id);

    assert_eq!(
        items[0]["outcome"],
        CliMutationOutcome::AppliedUnverified.as_str()
    );
}

#[test]
fn a_bulk_item_whose_command_failed_reports_a_failing_outcome() {
    let harness = Harness::new();
    let source = install_returning_source(&harness, "claude-code", "1.2.0", "1.3.0");
    source.set_process_failure();
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let items = run_bulk_items(&harness, &plan_id);

    // Failed and nothing observed to have moved.
    assert_eq!(items[0]["status"], "completed");
    assert_eq!(
        items[0]["outcome"],
        CliMutationOutcome::NoChangeFailed.as_str()
    );
}

#[test]
fn a_cancelled_batch_reports_every_item_rather_than_dropping_them() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    install(&harness, "codex-cli", "1.0.0", "2.0.0");
    let plan_id = prepare_bulk(&harness, &["claude-code", "codex-cli"]);

    let prepared = harness
        .service
        .prepare_cli_bulk_execution(&plan_id, 1)
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.operations.cancel(&operation_id);
    harness
        .service
        .execute_cli_bulk_action(prepared)
        .expect("bulk runs");

    let items = harness
        .operations
        .find(&operation_id)
        .and_then(|operation| operation.result)
        .and_then(|result| {
            result
                .get("items")
                .and_then(|items| items.as_array())
                .cloned()
        })
        .expect("items");

    // Both tools are accounted for. A missing entry would read as "nothing to report".
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|item| item["status"] == "completed" || item["status"] == "skipped"));
}

#[test]
fn a_skipped_tool_keeps_its_stable_reason_in_the_item_results() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    // Already at latest: excluded at planning time, still reported at execution time.
    install(&harness, "gemini-cli", "3.0.0", "3.0.0");
    let plan_id = prepare_bulk(&harness, &["claude-code", "gemini-cli"]);

    let items = run_bulk_items(&harness, &plan_id);

    let skipped = items
        .iter()
        .find(|item| item["agentId"] == "gemini-cli")
        .expect("skipped tool is reported");
    assert_eq!(skipped["status"], "skipped");
    assert_eq!(
        skipped["reason"],
        CliBulkSkipReason::AlreadyCurrent.as_str()
    );
    assert!(skipped["outcome"].is_null());
}

#[test]
fn no_bulk_item_result_uses_the_ran_placeholder() {
    let harness = Harness::new();
    install(&harness, "claude-code", "1.2.0", "1.3.0");
    let plan_id = prepare_bulk(&harness, &["claude-code"]);

    let items = run_bulk_items(&harness, &plan_id);

    // `"ran"` said a process started and nothing about whether the machine changed.
    let serialized = serde_json::to_string(&items).expect("serialize");
    assert!(!serialized.contains("\"ran\""), "{serialized}");
    assert!(!serialized.contains("unknown"), "{serialized}");
}

#[test]
fn one_failing_item_does_not_erase_the_others() {
    let harness = Harness::new();
    let failing = install_returning_source(&harness, "claude-code", "1.2.0", "1.3.0");
    install(&harness, "codex-cli", "1.0.0", "2.0.0");
    failing.set_process_failure();
    let plan_id = prepare_bulk(&harness, &["claude-code", "codex-cli"]);

    let items = run_bulk_items(&harness, &plan_id);

    assert_eq!(items.len(), 2);
    // Both reported, and the batch operation itself succeeded: collecting every outcome is what
    // the orchestration is for, so an item that failed is not an orchestration failure.
    assert!(items.iter().all(|item| !item["status"].is_null()));
}
