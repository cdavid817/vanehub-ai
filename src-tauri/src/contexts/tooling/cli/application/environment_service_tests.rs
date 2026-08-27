// Included through `#[path]` from environment_service.rs, so `super` is that module and
// `super::super` is the application layer where the shared harness lives.
use super::super::environment_service_fixtures::Harness;
use super::super::environment_test_doubles::{timestamp, tool_id};
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::contexts::tooling::cli::domain::status::{CliFreshness, CliOverallState};

#[test]
fn listing_returns_one_snapshot_per_registered_tool_in_catalog_order() {
    let harness = Harness::new();

    let snapshots = harness
        .service
        .list_cli_environments()
        .expect("list succeeds");

    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.agent_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli"
        ]
    );
}

#[test]
fn a_tool_with_no_stored_snapshot_claims_nothing_rather_than_missing() {
    let harness = Harness::new();

    let snapshots = harness.service.list_cli_environments().expect("list");
    let claude = &snapshots[0];

    // Never scanned is not the same as not installed. Reporting Missing here would be a finding
    // VaneHub has not made.
    assert_eq!(claude.overall_state, CliOverallState::Unknown);
    assert_eq!(claude.freshness, CliFreshness::Never);
    assert_eq!(claude.checked_at, None);
    assert!(claude.installations.is_empty());
}

#[test]
fn listing_starts_no_process_probe_or_source_query() {
    let harness = Harness::new();

    harness.service.list_cli_environments().expect("list");

    // Bounded read: the command boundary may return this directly, so nothing here may block on a
    // package manager or a child process.
    assert!(harness.discovery.discovered_agents().is_empty());
    assert!(harness.probes.invocations().is_empty());
    assert!(harness.operations.all().is_empty());
}

#[test]
fn a_stored_snapshot_from_the_same_environment_is_returned_unchanged() {
    let harness = Harness::new();
    let mut stored =
        CliEnvironmentSnapshot::never_scanned(tool_id("codex-cli"), "fingerprint-a".to_string());
    stored.freshness = CliFreshness::Fresh;
    stored.checked_at = Some(timestamp(1_000));
    harness
        .repository
        .snapshots
        .lock()
        .expect("snapshots")
        .insert("codex-cli".to_string(), stored);

    let snapshots = harness.service.list_cli_environments().expect("list");
    let codex = snapshots
        .iter()
        .find(|snapshot| snapshot.agent_id == tool_id("codex-cli"))
        .expect("codex snapshot");

    assert_eq!(codex.freshness, CliFreshness::Fresh);
    assert!(codex.checked_at.is_some());
}

#[test]
fn a_snapshot_from_a_different_environment_stays_visible_but_is_marked_stale() {
    let harness = Harness::new();
    let mut stored =
        CliEnvironmentSnapshot::never_scanned(tool_id("codex-cli"), "fingerprint-OLD".to_string());
    stored.freshness = CliFreshness::Fresh;
    stored.last_operation_id = Some("op-earlier".to_string());
    harness
        .repository
        .snapshots
        .lock()
        .expect("snapshots")
        .insert("codex-cli".to_string(), stored);

    let snapshots = harness.service.list_cli_environments().expect("list");
    let codex = snapshots
        .iter()
        .find(|snapshot| snapshot.agent_id == tool_id("codex-cli"))
        .expect("codex snapshot");

    // Labelled, not discarded: the page keeps showing what it knows instead of blanking out.
    assert_eq!(codex.freshness, CliFreshness::Stale);
    assert_eq!(codex.last_operation_id.as_deref(), Some("op-earlier"));
    assert_eq!(codex.environment_fingerprint, "fingerprint-OLD");
}

#[test]
fn an_unregistered_agent_id_is_reported_as_unknown() {
    let harness = Harness::new();

    let error = harness
        .service
        .resolve_tool("not-a-cli")
        .expect_err("unknown tool");

    assert_eq!(error.category(), "unknown-tool");
}

#[test]
fn a_registered_agent_id_resolves_to_its_definition() {
    let harness = Harness::new();

    let (id, definition) = harness.service.resolve_tool("opencode").expect("resolves");

    assert_eq!(id.as_str(), "opencode");
    assert_eq!(definition.display_name, "OpenCode CLI");
    assert!(!definition.distributions.is_empty());
}
