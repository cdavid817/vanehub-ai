use super::json_rpc_actor::{JsonRpcActorLimits, JsonRpcErrorObject, ServerRequestHandler};
use super::lsp_framing::FrameLimits;
use super::lsp_stdio_child::ManagedLspStdio;
use super::shutdown_coordinator::{ActiveLspProcess, LspShutdownCoordinator};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct RejectServerRequests;

impl ServerRequestHandler for RejectServerRequests {
    fn handle(&self, _method: &str, _params: Value) -> Result<Value, JsonRpcErrorObject> {
        Err(JsonRpcErrorObject::method_not_found())
    }
}

fn active_process(mode: &str) -> ActiveLspProcess {
    let arguments = vec![
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lsp_stdio_server.cjs")
            .to_string_lossy()
            .into_owned(),
        mode.to_string(),
    ];
    let limits = JsonRpcActorLimits::new(8, 8, 8, 8, 8, 4).expect("actor limits");
    let (client, _events, process) = ManagedLspStdio::spawn(
        "node",
        &arguments,
        &BTreeMap::new(),
        FrameLimits::default(),
        1024,
        limits,
        Arc::new(RejectServerRequests),
    )
    .expect("fixture process");
    ActiveLspProcess::new(client, process)
}

#[tokio::test]
async fn shutdown_runs_processes_concurrently_and_forces_unresponsive_trees() {
    let coordinator = LspShutdownCoordinator::new();
    coordinator
        .register(active_process("lsp-success"))
        .await
        .expect("success process");
    coordinator
        .register(active_process("lsp-hang"))
        .await
        .expect("hanging process");
    let started = Instant::now();

    let summary = coordinator
        .shutdown_all(started + Duration::from_millis(900))
        .await;

    assert_eq!(summary.total, 2);
    // Concurrency is what `graceful == 1` proves, not the elapsed time below: the deadline
    // handed to `shutdown_all` is absolute, so a serial implementation would finish at
    // roughly the same wall-clock moment. What it could not do is keep this process graceful
    // — waiting on `lsp-hang` first would push `lsp-success` past the deadline and force it
    // too, giving `graceful == 0, forced == 2`.
    assert_eq!(summary.graceful, 1);
    assert_eq!(summary.forced, 1);
    assert_eq!(summary.failed, 0);
    // The only thing wall-clock adds is "forced termination terminates". The regression worth
    // catching is a force path that never returns, and a generous ceiling catches that just as
    // well as a tight one — while a tight one fails on a loaded machine, where a spurious red
    // costs more than the milliseconds it claims to guard.
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(!coordinator.is_accepting());
}

#[tokio::test]
async fn repeated_and_concurrent_shutdown_calls_are_idempotent() {
    let coordinator = LspShutdownCoordinator::new();
    coordinator
        .register(active_process("lsp-hang"))
        .await
        .expect("hanging process");
    let first = coordinator.clone();
    let second = coordinator.clone();
    let deadline = Instant::now() + Duration::from_secs(2);

    let (left, right) = tokio::join!(first.shutdown_all(deadline), second.shutdown_all(deadline));
    let repeated = coordinator.shutdown_all(deadline).await;

    assert_eq!(left, right);
    assert_eq!(left, repeated);
    assert_eq!(left.total, 1);
}

#[tokio::test]
async fn shutdown_rejects_late_process_registration() {
    let coordinator = LspShutdownCoordinator::new();
    let deadline = Instant::now() + Duration::from_millis(500);
    assert_eq!(coordinator.shutdown_all(deadline).await.total, 0);

    let rejected = coordinator
        .register(active_process("lsp-hang"))
        .await
        .expect_err("registration rejected");

    rejected
        .force_shutdown(Instant::now() + Duration::from_secs(2))
        .await
        .expect("rejected process cleanup");
}
