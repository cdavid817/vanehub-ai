use super::server_discovery::{NativeExecutableLocationPort, ServerDiscovery};
use super::server_test::{
    IsolatedServerTester, ServerTestCommand, ServerTestPhase, ServerTestPhaseStatus,
    ServerTestReason,
};
use crate::contexts::code_intelligence::domain::models::ServerKind;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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
    let discovery =
        ServerDiscovery::new(locator).discover(ServerKind::TypeScriptLanguageServer, None);

    let command = ServerTestCommand::from_discovery(&discovery, json!({}));

    assert_eq!(command.server_kind(), ServerKind::TypeScriptLanguageServer);
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
        ServerKind::RustAnalyzer,
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

#[tokio::test]
async fn unavailable_discovery_skips_process_phases_without_spawning() {
    let result = IsolatedServerTester::run(
        ServerTestCommand::unavailable(ServerKind::RustAnalyzer),
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
        ServerKind::TypeScriptLanguageServer,
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
