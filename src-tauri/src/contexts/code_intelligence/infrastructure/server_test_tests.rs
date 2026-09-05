use super::server_discovery::{NativeExecutableLocationPort, ServerDiscovery};
use super::server_test::{
    IsolatedServerTester, ServerTestCommand, ServerTestPhase, ServerTestPhaseStatus,
    ServerTestReason,
};
use crate::contexts::code_intelligence::domain::registry;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct FixedLocator(PathBuf);

impl NativeExecutableLocationPort for FixedLocator {
    fn locate(&self, _executable_name: &str) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

#[test]
fn discovered_typescript_server_maps_to_the_stdio_test_command() {
    let locator = Arc::new(FixedLocator(
        std::env::current_exe().expect("test executable"),
    ));
    let discovery = ServerDiscovery::new(locator).discover(registry::typescript(), None, None);

    let command = ServerTestCommand::from_discovery(&discovery, json!({}));

    assert_eq!(command.language(), registry::typescript());
    assert_eq!(command.arguments(), &["--stdio"]);
}

fn fixture_command(mode: &str, marker: Option<&str>) -> ServerTestCommand {
    let mut arguments = vec![
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lsp_stdio_server.cjs")
            .to_string_lossy()
            .into_owned(),
        mode.to_string(),
    ];
    if let Some(marker) = marker {
        arguments.push(marker.to_string());
    }
    ServerTestCommand::available(
        registry::rust(),
        "node".to_string(),
        arguments,
        json!({"fixture": true}),
    )
}

#[tokio::test]
async fn isolated_test_reports_every_successful_phase_and_gracefully_reaps() {
    let result = IsolatedServerTester::run(
        fixture_command("lsp-success", Some("Cargo.toml")),
        Duration::from_secs(5),
    )
    .await;

    for phase in [
        ServerTestPhase::Discovery,
        ServerTestPhase::Spawn,
        ServerTestPhase::Initialize,
        ServerTestPhase::Cleanup,
    ] {
        assert_eq!(
            result.phase(phase).expect("phase").status,
            ServerTestPhaseStatus::Succeeded
        );
    }
    assert!(result.negotiated_capabilities().is_some());
    assert!(result.cleaned_up());
    assert_eq!(
        result
            .phase(ServerTestPhase::Cleanup)
            .expect("cleanup")
            .reason,
        None
    );
}

#[tokio::test]
async fn malformed_initialize_result_still_runs_protocol_cleanup() {
    let result = IsolatedServerTester::run(
        fixture_command("lsp-invalid-init", Some("Cargo.toml")),
        Duration::from_secs(5),
    )
    .await;

    let initialize = result
        .phase(ServerTestPhase::Initialize)
        .expect("initialize");
    assert_eq!(initialize.status, ServerTestPhaseStatus::Failed);
    assert_eq!(initialize.reason, Some(ServerTestReason::InitializeFailed));
    assert_eq!(
        result
            .phase(ServerTestPhase::Cleanup)
            .expect("cleanup")
            .status,
        ServerTestPhaseStatus::Succeeded
    );
    assert!(result.cleaned_up());
}

#[tokio::test]
async fn initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation() {
    let result = IsolatedServerTester::run(
        fixture_command("lsp-hang", Some("Cargo.toml")),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(
        result
            .phase(ServerTestPhase::Initialize)
            .expect("initialize")
            .reason,
        Some(ServerTestReason::InitializeTimedOut)
    );
    let cleanup = result.phase(ServerTestPhase::Cleanup).expect("cleanup");
    assert_eq!(cleanup.status, ServerTestPhaseStatus::Succeeded);
    assert_eq!(cleanup.reason, Some(ServerTestReason::ForcedTermination));
    assert!(result.cleaned_up());
}

/// A caller's budget that is already gone must not take cleanup with it.
///
/// `MIN_TEST_TIMEOUT` is the smallest budget the tester accepts, and against a server that never
/// answers it is spent before cleanup begins — which is the load case, reproduced deterministically
/// rather than waited for. Before the cleanup floor, `start_kill` was issued and the wait that
/// observes it was skipped, so the phase reported failure for a child that had in fact died. A
/// caller told that cannot tell it from a process tree still running.
#[tokio::test]
async fn a_spent_caller_deadline_still_leaves_cleanup_enough_to_observe_the_kill() {
    let result = IsolatedServerTester::run(
        fixture_command("lsp-hang", Some("Cargo.toml")),
        Duration::from_millis(100),
    )
    .await;

    assert_eq!(
        result
            .phase(ServerTestPhase::Initialize)
            .expect("initialize")
            .reason,
        Some(ServerTestReason::InitializeTimedOut)
    );
    let cleanup = result.phase(ServerTestPhase::Cleanup).expect("cleanup");
    assert_eq!(cleanup.status, ServerTestPhaseStatus::Succeeded);
    assert_eq!(cleanup.reason, Some(ServerTestReason::ForcedTermination));
    assert!(result.cleaned_up());
}

/// The floor is a ceiling on waiting, not a delay.
///
/// A server that exits on `shutdown` is cleaned up as soon as it has, and the whole run finishes
/// well inside the floor. Asserted generously: what would fail this is a floor implemented as a
/// sleep, which would take at least two seconds every time, and no plausible machine takes that long
/// to reap a child that is already leaving.
#[tokio::test]
async fn a_cleanup_that_finishes_early_does_not_wait_out_the_floor() {
    let started = Instant::now();

    let result = IsolatedServerTester::run(
        fixture_command("lsp-success", Some("Cargo.toml")),
        Duration::from_secs(10),
    )
    .await;

    assert!(result.cleaned_up());
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "cleanup waited out a budget it did not need"
    );
}

#[tokio::test]
async fn unavailable_discovery_skips_process_phases_without_spawning() {
    let result = IsolatedServerTester::run(
        ServerTestCommand::unavailable(registry::rust()),
        Duration::from_secs(1),
    )
    .await;

    assert_eq!(
        result
            .phase(ServerTestPhase::Discovery)
            .expect("discovery")
            .reason,
        Some(ServerTestReason::ExecutableUnavailable)
    );
    for phase in [
        ServerTestPhase::Spawn,
        ServerTestPhase::Initialize,
        ServerTestPhase::Cleanup,
    ] {
        assert_eq!(
            result.phase(phase).expect("phase").status,
            ServerTestPhaseStatus::Skipped
        );
    }
    assert!(!result.cleaned_up());
}

#[tokio::test]
async fn spawn_failure_is_reported_without_leaking_private_executable_details() {
    let command = ServerTestCommand::available(
        registry::typescript(),
        absolute_missing_executable(),
        vec!["--stdio".to_string()],
        json!({}),
    );

    let result = IsolatedServerTester::run(command, Duration::from_secs(1)).await;

    assert_eq!(
        result.phase(ServerTestPhase::Spawn).expect("spawn").reason,
        Some(ServerTestReason::SpawnFailed)
    );
    assert!(!format!("{result:?}").contains("missing-private-server"));
}

fn absolute_missing_executable() -> String {
    std::env::temp_dir()
        .join("missing-private-server")
        .to_string_lossy()
        .into_owned()
}
