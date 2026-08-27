// Included through `#[path]` from environment_refresh.rs.
use super::super::environment_service_fixtures::{healthy_npm_installation, npm_catalog, Harness};
use crate::contexts::tooling::cli::domain::ids::CliToolId;
use crate::contexts::tooling::cli::domain::installation::CliInstallation;
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::contexts::tooling::cli::domain::source::{
    CliSourceConfidence, CliSourceKind, CliSourceManagement,
};
use crate::contexts::tooling::cli::domain::status::{
    CliDiscoveryStatus, CliExecutableStatus, CliFreshness, CliOverallState, CliUpdateStatus,
};

fn refresh(harness: &Harness, agent_ids: &[&str]) -> String {
    let prepared = harness
        .service
        .prepare_refresh(
            agent_ids.iter().map(|id| (*id).to_string()).collect(),
            false,
        )
        .expect("prepare succeeds");
    let operation_id = prepared.operation_id.clone();
    harness
        .service
        .execute_refresh(prepared)
        .expect("execute succeeds");
    operation_id
}

#[test]
fn preparing_a_refresh_returns_an_operation_before_any_probing_happens() {
    let harness = Harness::new();

    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare");

    // The Tauri boundary returns at this point, so nothing slow may have run yet.
    assert!(harness.discovery.discovered_agents().is_empty());
    assert!(harness.probes.invocations().is_empty());

    let operation = harness
        .operations
        .find(&prepared.operation_id)
        .expect("operation exists");
    assert_eq!(operation.agent_id.as_deref(), Some("claude-code"));
    assert_eq!(operation.terminal, None);
}

#[test]
fn an_all_tool_refresh_relates_to_no_single_agent() {
    let harness = Harness::new();

    let prepared = harness
        .service
        .prepare_refresh(Vec::new(), false)
        .expect("prepare");

    let operation = harness
        .operations
        .find(&prepared.operation_id)
        .expect("operation");
    assert_eq!(operation.agent_id, None);
}

#[test]
fn preparing_a_refresh_for_an_unknown_agent_fails_before_starting_an_operation() {
    let harness = Harness::new();

    let error = harness
        .service
        .prepare_refresh(vec!["not-a-cli".to_string()], false)
        .expect_err("unknown tool");

    assert_eq!(error.category(), "unknown-tool");
    assert!(harness.operations.all().is_empty());
}

#[test]
fn a_refresh_probes_each_discovered_installation_and_records_its_version() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");
    harness.register_npm_source(npm_catalog("claude-code", &["1.3.0", "1.2.0"], "1.3.0"));

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.discovery, CliDiscoveryStatus::FoundOne);
    assert_eq!(snapshot.executable, CliExecutableStatus::Healthy);
    assert_eq!(
        snapshot.installations[0]
            .reported_version
            .as_ref()
            .map(|version| version.as_str()),
        Some("1.2.0")
    );
    assert_eq!(snapshot.freshness, CliFreshness::Fresh);
    assert!(snapshot.checked_at.is_some());

    // The version probe used the tool's own declared command, not an improvised one.
    let (path, args) = harness
        .probes
        .invocations()
        .into_iter()
        .next()
        .expect("probe");
    assert_eq!(path, "/path/claude");
    assert_eq!(args, vec!["--version".to_string()]);
}

#[test]
fn the_update_state_comes_from_the_catalog_of_the_owning_source() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");
    harness.register_npm_source(npm_catalog("claude-code", &["1.3.0", "1.2.0"], "1.3.0"));

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.update, CliUpdateStatus::Available);
    assert_eq!(snapshot.overall_state, CliOverallState::UpdateAvailable);
}

#[test]
fn an_installation_owned_by_a_source_with_no_catalog_is_not_told_it_is_outdated() {
    let harness = Harness::new();
    // The install came from WinGet. npm has a catalog and says 1.3.0 is latest.
    let mut winget_install = healthy_npm_installation("a", "/path/claude");
    winget_install.source_id =
        Some(crate::contexts::tooling::cli::domain::ids::CliSourceId::new("winget").expect("id"));
    winget_install.source_kind =
        crate::contexts::tooling::cli::domain::source::CliSourceKind::Winget;
    harness.discovery.set("claude-code", vec![winget_install]);
    harness.probes.set_version("/path/claude", "1.2.0");
    harness.register_npm_source(npm_catalog("claude-code", &["1.3.0", "1.2.0"], "1.3.0"));

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    // The npm catalog does not describe a WinGet install, so no answer is borrowed from it.
    assert_eq!(snapshot.update, CliUpdateStatus::CatalogUnavailable);
    assert_ne!(snapshot.overall_state, CliOverallState::UpdateAvailable);
}

#[test]
fn a_targeted_refresh_writes_only_the_targeted_snapshot() {
    let harness = Harness::new();
    // Two tools already have snapshots from an earlier scan.
    for agent_id in ["claude-code", "codex-cli"] {
        let mut existing = CliEnvironmentSnapshot::never_scanned(
            CliToolId::new(agent_id).expect("id"),
            "fingerprint-a".to_string(),
        );
        existing.freshness = CliFreshness::Fresh;
        existing.last_operation_id = Some(format!("op-earlier-{agent_id}"));
        harness
            .repository
            .snapshots
            .lock()
            .expect("snapshots")
            .insert(agent_id.to_string(), existing);
    }
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");

    refresh(&harness, &["claude-code"]);

    // Exactly one write, and discovery ran for exactly one tool.
    assert_eq!(harness.repository.written_agents(), vec!["claude-code"]);
    assert_eq!(harness.discovery.discovered_agents(), vec!["claude-code"]);

    // The unrelated snapshot is byte-for-byte what it was, including its operation id.
    let codex = harness.repository.snapshot("codex-cli").expect("codex");
    assert_eq!(codex.freshness, CliFreshness::Fresh);
    assert_eq!(
        codex.last_operation_id.as_deref(),
        Some("op-earlier-codex-cli")
    );
}

#[test]
fn an_all_tool_refresh_covers_every_registered_tool() {
    let harness = Harness::new();

    refresh(&harness, &[]);

    assert_eq!(
        harness.discovery.discovered_agents(),
        vec![
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli"
        ]
    );
    assert_eq!(harness.repository.written_agents().len(), 5);
}

#[test]
fn a_broken_probe_records_the_fault_without_inventing_a_version() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_failure("/path/claude", false);

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.executable, CliExecutableStatus::Broken);
    assert_eq!(snapshot.installations[0].reported_version, None);
    assert_eq!(snapshot.overall_state, CliOverallState::Broken);
    assert!(harness
        .diagnostics
        .messages()
        .iter()
        .any(|entry| entry.contains("version probe")));
}

#[test]
fn a_timed_out_probe_is_distinguished_from_a_failing_one() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_failure("/path/claude", true);

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.executable, CliExecutableStatus::TimedOut);
}

#[test]
fn nothing_discovered_yields_missing_rather_than_broken() {
    let harness = Harness::new();
    harness.discovery.set("claude-code", Vec::new());

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.discovery, CliDiscoveryStatus::NotFound);
    assert_eq!(snapshot.executable, CliExecutableStatus::NotApplicable);
    assert_eq!(snapshot.overall_state, CliOverallState::Missing);
}

#[test]
fn the_operation_reports_phases_and_per_tool_progress() {
    let harness = Harness::new();

    let operation_id = refresh(&harness, &[]);
    let operation = harness.operations.find(&operation_id).expect("operation");

    assert!(operation.phases.contains(&"preflight".to_string()));
    assert!(operation.phases.contains(&"resolving-source".to_string()));
    assert!(operation.phases.contains(&"querying-catalog".to_string()));
    assert_eq!(
        operation.phases.last().map(String::as_str),
        Some("completed")
    );
    // One unit per tool, so a five-tool refresh reports five steps.
    assert_eq!(
        operation.units,
        vec![(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]
    );
    assert_eq!(operation.terminal.as_deref(), Some("succeeded"));
}

#[test]
fn a_cancelled_refresh_stops_early_and_leaves_the_rest_untouched() {
    let harness = Harness::new();
    let prepared = harness
        .service
        .prepare_refresh(Vec::new(), false)
        .expect("prepare");
    harness.operations.cancel(&prepared.operation_id);

    harness.service.execute_refresh(prepared).expect("execute");

    // Cancelled before the first tool, so nothing was probed and nothing was written.
    assert!(harness.repository.written_agents().is_empty());
    assert!(harness.discovery.discovered_agents().is_empty());
}

#[test]
fn a_cached_catalog_is_reused_until_it_expires() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");
    let source = harness.register_npm_source(npm_catalog("claude-code", &["1.3.0"], "1.3.0"));

    refresh(&harness, &["claude-code"]);
    let first = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(first.update, CliUpdateStatus::Available);

    // A second refresh inside the catalog's lifetime must not re-query the source.
    let queries_before = source.executions().len();
    refresh(&harness, &["claude-code"]);
    assert_eq!(source.executions().len(), queries_before);
}

#[test]
fn a_source_that_cannot_answer_records_catalog_unavailable_rather_than_nothing() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("a", "/path/claude")],
    );
    harness.probes.set_version("/path/claude", "1.2.0");
    harness.register_failing_npm_source();

    refresh(&harness, &["claude-code"]);

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    assert_eq!(snapshot.update, CliUpdateStatus::CatalogUnavailable);
    // Not a broken tool: the CLI runs fine, VaneHub just could not read the catalog.
    assert_eq!(snapshot.overall_state, CliOverallState::Ready);
}

#[test]
fn a_discovery_failure_fails_the_operation_with_a_recorded_diagnostic() {
    let harness = Harness::new();
    *harness.discovery.fail_with.lock().expect("fail") = Some(
        crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError::Process(
            "PATH enumeration failed".to_string(),
        ),
    );

    let operation_id = refresh(&harness, &["claude-code"]);
    let operation = harness.operations.find(&operation_id).expect("operation");

    assert_eq!(operation.terminal.as_deref(), Some("failed"));
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("PATH enumeration failed")));
    assert!(!harness.diagnostics.messages().is_empty());
}

#[test]
fn a_detect_only_installation_stays_healthy_and_offers_no_mutation() {
    // The rule this guards: detect-only is a statement about VaneHub's capability, never about the
    // installation's health. A Homebrew-installed CLI that runs fine must not render as broken.
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![CliInstallation {
            source_id: None,
            source_kind: CliSourceKind::Homebrew,
            source_confidence: CliSourceConfidence::Inferred,
            ..healthy_npm_installation("brew", "/opt/homebrew/bin/claude")
        }],
    );
    harness
        .probes
        .set_version("/opt/homebrew/bin/claude", "1.2.0");

    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare");
    harness.service.execute_refresh(prepared).expect("refresh");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");

    assert_eq!(snapshot.executable, CliExecutableStatus::Healthy);
    let homebrew = snapshot
        .sources
        .iter()
        .find(|source| source.kind == CliSourceKind::Homebrew)
        .expect("the source it was actually installed from is summarized");
    assert_eq!(homebrew.management, CliSourceManagement::DetectOnly);
    assert_eq!(homebrew.guidance_code, Some("cli.guidance.homebrew"));
    // No catalog is borrowed: the version list stays empty rather than showing npm's.
    assert!(homebrew.available_versions.is_empty());
    assert_eq!(homebrew.available_version_count, None);
    // And nothing offers to mutate it through a source VaneHub does not drive.
    assert!(snapshot
        .allowed_actions
        .iter()
        .all(|action| action.source_id.as_str() != "homebrew"));
}

#[test]
fn a_detect_only_source_never_borrows_the_managed_catalog() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![CliInstallation {
            source_id: None,
            source_kind: CliSourceKind::Volta,
            source_confidence: CliSourceConfidence::Inferred,
            ..healthy_npm_installation("volta", "/home/dev/.volta/bin/claude")
        }],
    );
    harness
        .probes
        .set_version("/home/dev/.volta/bin/claude", "1.2.0");
    // npm has a catalog for this tool. The Volta summary must not show it.
    harness.register_npm_source(npm_catalog("claude-code", &["1.3.0", "1.2.0"], "1.3.0"));

    let prepared = harness
        .service
        .prepare_refresh(vec!["claude-code".to_string()], false)
        .expect("prepare");
    harness.service.execute_refresh(prepared).expect("refresh");

    let snapshot = harness
        .repository
        .snapshot("claude-code")
        .expect("snapshot");
    let volta = snapshot
        .sources
        .iter()
        .find(|source| source.kind == CliSourceKind::Volta)
        .expect("volta summary");

    assert!(volta.available_versions.is_empty());
    assert_eq!(volta.guidance_code, Some("cli.guidance.volta"));
    // npm's own summary still carries npm's list; the two never merge.
    let npm = snapshot
        .sources
        .iter()
        .find(|source| source.kind == CliSourceKind::Npm)
        .expect("npm summary");
    assert_eq!(npm.available_versions.len(), 2);
    assert_eq!(npm.management, CliSourceManagement::Managed);
}
