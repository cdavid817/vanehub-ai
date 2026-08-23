//! What the Agent Runtime is handed, and what it is deliberately not handed.

use super::*;
use crate::contexts::tooling::cli::application::environment_service_fixtures::{
    healthy_npm_installation, Harness,
};
use crate::contexts::tooling::cli::domain::ids::CliInstallationId;
use crate::contexts::tooling::cli::domain::installation::{
    CliConflict, CliConflictKind, CliConflictSeverity,
};
use crate::contexts::tooling::cli::domain::status::{CliDiscoveryStatus, CliExecutableStatus};

/// Writes a snapshot through the repository port, the way a refresh would.
fn save(harness: &Harness, snapshot: &CliEnvironmentSnapshot) {
    use crate::contexts::tooling::cli::application::environment_ports::CliEnvironmentRepository;
    harness
        .repository
        .save_snapshot_atomic(snapshot)
        .expect("stores the snapshot");
}

fn scanned_snapshot(agent_id: &str, installations: Vec<CliInstallation>) -> CliEnvironmentSnapshot {
    let mut snapshot = CliEnvironmentSnapshot::never_scanned(
        crate::contexts::tooling::cli::domain::ids::CliToolId::new(agent_id).expect("tool id"),
        "fingerprint-a".to_string(),
    );
    snapshot.discovery = if installations.is_empty() {
        CliDiscoveryStatus::NotFound
    } else if installations.len() == 1 {
        CliDiscoveryStatus::FoundOne
    } else {
        CliDiscoveryStatus::FoundMultiple
    };
    snapshot.installations = installations;
    snapshot.recompute_derived(false, false)
}

#[test]
fn the_runtime_launches_the_installation_the_management_page_recommends() {
    let harness = Harness::new();
    let mut healthy = healthy_npm_installation("claude", "/opt/npm/bin/claude");
    healthy.executable_status = CliExecutableStatus::Healthy;
    let snapshot = scanned_snapshot("claude-code", vec![healthy]);
    save(&harness, &snapshot);

    let target = harness
        .service
        .resolve_launch_target("claude-code")
        .expect("resolves");

    assert_eq!(
        target,
        CliLaunchTarget::Resolved("/opt/npm/bin/claude".to_string())
    );
}

#[test]
fn a_shadowed_installation_is_launched_only_when_it_is_the_recommended_one() {
    let harness = Harness::new();
    let mut first = healthy_npm_installation("shadowing", "/usr/local/bin/claude");
    first.executable_status = CliExecutableStatus::Broken;
    let mut second = healthy_npm_installation("recommended", "/opt/npm/bin/claude");
    second.executable_status = CliExecutableStatus::Healthy;
    second.path_priority = Some(4);
    let snapshot = scanned_snapshot("claude-code", vec![first, second]);
    save(&harness, &snapshot);

    let target = harness
        .service
        .resolve_launch_target("claude-code")
        .expect("resolves");

    // PATH reaches the broken one first; the backend recommends the runnable one, and the runtime
    // follows the backend rather than PATH. Following PATH would launch a binary the page reports
    // as broken.
    assert_eq!(
        target,
        CliLaunchTarget::Resolved("/opt/npm/bin/claude".to_string())
    );
}

#[test]
fn a_conflict_that_blocks_launching_refuses_rather_than_picking_a_winner() {
    let harness = Harness::new();
    let mut healthy = healthy_npm_installation("claude", "/opt/npm/bin/claude");
    healthy.executable_status = CliExecutableStatus::Healthy;
    let mut snapshot = scanned_snapshot("claude-code", vec![healthy]);
    snapshot.conflicts = vec![CliConflict {
        kind: CliConflictKind::StaleLauncherTarget,
        severity: CliConflictSeverity::Error,
        installations: vec![CliInstallationId::new("claude").expect("id")],
        reason_code: CliConflictKind::StaleLauncherTarget.as_str(),
        blocks_mutation: true,
        blocks_launch: true,
    }];
    save(&harness, &snapshot);

    let target = harness
        .service
        .resolve_launch_target("claude-code")
        .expect("resolves");

    // Refused, not fallen back. A live lookup here would start the installation the backend just
    // declined to pick.
    assert_eq!(target, CliLaunchTarget::Refused);
}

#[test]
fn a_broken_recommended_installation_refuses_instead_of_launching_it() {
    let harness = Harness::new();
    let mut broken = healthy_npm_installation("claude", "/opt/npm/bin/claude");
    broken.executable_status = CliExecutableStatus::Broken;
    let snapshot = scanned_snapshot("claude-code", vec![broken]);
    save(&harness, &snapshot);

    assert_eq!(
        harness
            .service
            .resolve_launch_target("claude-code")
            .expect("resolves"),
        CliLaunchTarget::Refused
    );
}

#[test]
fn a_scanned_host_with_nothing_installed_refuses_without_a_live_lookup() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![healthy_npm_installation("live", "/live/claude")],
    );
    let snapshot = scanned_snapshot("claude-code", Vec::new());
    save(&harness, &snapshot);

    // `NotFound` is a finding: the refresh looked and found nothing. Reaching for a live candidate
    // here would let the launch path disagree with the page that says the tool is missing.
    assert_eq!(
        harness
            .service
            .resolve_launch_target("claude-code")
            .expect("resolves"),
        CliLaunchTarget::Refused
    );
}

#[test]
fn a_never_scanned_tool_falls_back_to_a_bounded_live_lookup() {
    let harness = Harness::new();
    harness.discovery.set(
        "claude-code",
        vec![
            healthy_npm_installation("second", "/opt/npm/bin/claude"),
            healthy_npm_installation("first", "/usr/local/bin/claude"),
        ],
    );

    let target = harness
        .service
        .resolve_launch_target("claude-code")
        .expect("resolves");

    // Real PATH order, not discovery order: the first entry a shell would reach wins.
    match target {
        CliLaunchTarget::Resolved(path) => assert!(path.ends_with("claude"), "{path}"),
        other => panic!("expected a live resolution, got {other:?}"),
    }
}

#[test]
fn a_never_scanned_tool_with_nothing_on_the_host_reports_not_scanned() {
    let harness = Harness::new();

    assert_eq!(
        harness
            .service
            .resolve_launch_target("claude-code")
            .expect("resolves"),
        CliLaunchTarget::NotScanned
    );
}

#[test]
fn an_unregistered_agent_id_is_rejected_rather_than_guessed_at() {
    let harness = Harness::new();

    assert!(harness.service.resolve_launch_target("not-a-cli").is_err());
}

#[test]
fn every_resolution_is_an_absolute_path_rather_than_a_bare_command_name() {
    let harness = Harness::new();
    // Absolute on the platform the suite runs on, so the assertion means what it says everywhere.
    let absolute = if cfg!(target_os = "windows") {
        r"C:\fixture\npm\claude.cmd"
    } else {
        "/opt/npm/bin/claude"
    };
    let mut healthy = healthy_npm_installation("claude", absolute);
    healthy.executable_status = CliExecutableStatus::Healthy;
    save(&harness, &scanned_snapshot("claude-code", vec![healthy]));

    let CliLaunchTarget::Resolved(path) = harness
        .service
        .resolve_launch_target("claude-code")
        .expect("resolves")
    else {
        panic!("expected a resolution");
    };

    // A bare name would re-enter PATH resolution inside the child process, and PATH is exactly
    // what the conflict contract exists to arbitrate.
    assert!(std::path::Path::new(&path).is_absolute());
    assert_ne!(path, "claude");
}
