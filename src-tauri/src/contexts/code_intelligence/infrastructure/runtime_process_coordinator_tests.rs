use super::lsp_diagnostics::LspDiagnosticLogger;
use super::process_registry::{ActivationReason, LifecyclePolicy};
use super::project_root::ProcessKey;
use super::runtime_process_coordinator::{
    LspProcessAcquisition, LspProcessLaunch, RuntimeProcessCoordinator,
};
use super::shutdown_coordinator::LspShutdownCoordinator;
use crate::contexts::code_intelligence::domain::models::{
    ConfigurationFingerprint, ProcessState, ServerKind,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
use serde_json::json;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tokio::test]
async fn on_demand_process_initializes_and_registers_for_desktop_shutdown() {
    let fixture = ProcessFixture::new("lsp-success");
    let shutdown = LspShutdownCoordinator::new();
    let logs = Arc::new(CapturingLogs::default());
    let coordinator = RuntimeProcessCoordinator::new(
        shutdown,
        LifecyclePolicy::default(),
        LspDiagnosticLogger::new(logs.clone()),
    );

    let acquired = coordinator
        .acquire(fixture.launch.clone(), ActivationReason::ToolRequest, true)
        .await;
    let handle = match acquired {
        LspProcessAcquisition::Ready(handle) => handle,
        _ => panic!("process must become ready"),
    };
    assert!(handle.capabilities().definition);
    assert_eq!(
        handle
            .client()
            .pending_count()
            .await
            .expect("pending count"),
        0
    );
    assert_eq!(
        coordinator
            .status(&fixture.key)
            .await
            .expect("process status")
            .state,
        ProcessState::Ready
    );

    coordinator.release_request(handle.key()).await;
    let summary = coordinator
        .shutdown_all(Instant::now() + Duration::from_secs(3))
        .await;
    assert_eq!(summary.total, 1);
    assert_eq!(summary.failed, 0);
    let events = logged_events(&logs);
    assert!(events.iter().any(|event| event == "lifecycle"));
    assert!(events.iter().any(|event| event == "shutdown"));
}

#[tokio::test]
async fn configuration_replacement_and_trust_revocation_stop_live_processes() {
    for revoke in [false, true] {
        let fixture = ProcessFixture::new("lsp-success");
        let shutdown = LspShutdownCoordinator::new();
        let coordinator =
            RuntimeProcessCoordinator::new(shutdown.clone(), LifecyclePolicy::default(), logger());
        assert!(matches!(
            coordinator
                .acquire(
                    fixture.launch.clone(),
                    ActivationReason::Prewarm {
                        inventory: true,
                        manifest: false,
                    },
                    true,
                )
                .await,
            LspProcessAcquisition::Ready(_)
        ));

        if revoke {
            coordinator
                .revoke_workspace(fixture.key.session_root_ref())
                .await;
        } else {
            coordinator.configuration_replaced().await;
        }
        assert!(coordinator.status(&fixture.key).await.is_none());
        assert_eq!(
            shutdown
                .shutdown_all(Instant::now() + Duration::from_secs(2))
                .await
                .total,
            0
        );
    }
}

#[tokio::test]
async fn failed_start_enters_backoff_and_idle_prewarm_is_cleaned_up() {
    let failed = ProcessFixture::new("exit");
    let failed_coordinator = RuntimeProcessCoordinator::new(
        LspShutdownCoordinator::new(),
        LifecyclePolicy::new(
            2,
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_secs(1),
            Duration::from_secs(10),
        ),
        logger(),
    );
    assert!(matches!(
        failed_coordinator
            .acquire(failed.launch.clone(), ActivationReason::ToolRequest, true,)
            .await,
        LspProcessAcquisition::Warming
    ));
    let backoff = failed_coordinator
        .status(&failed.key)
        .await
        .expect("backoff status");
    assert_eq!(backoff.state, ProcessState::Backoff);
    assert_eq!(backoff.restart_count, 1);

    let idle = ProcessFixture::new("lsp-success");
    let idle_coordinator = RuntimeProcessCoordinator::new(
        LspShutdownCoordinator::new(),
        LifecyclePolicy::new(
            1,
            Duration::from_secs(1),
            Duration::from_secs(1),
            Duration::from_secs(1),
            Duration::from_millis(20),
        ),
        logger(),
    );
    assert!(matches!(
        idle_coordinator
            .acquire(
                idle.launch.clone(),
                ActivationReason::Prewarm {
                    inventory: true,
                    manifest: false,
                },
                true,
            )
            .await,
        LspProcessAcquisition::Ready(_)
    ));
    tokio::time::sleep(Duration::from_millis(30)).await;
    idle_coordinator.tick().await;
    assert!(idle_coordinator.status(&idle.key).await.is_none());
}

#[tokio::test]
async fn production_coordinator_logs_crash_restart_and_protocol_limit_events() {
    for (mode, expected) in [
        ("lsp-crash", "crash"),
        ("lsp-protocol-limit", "protocol_limit"),
    ] {
        let fixture = ProcessFixture::new(mode);
        let logs = Arc::new(CapturingLogs::default());
        let coordinator = RuntimeProcessCoordinator::new(
            LspShutdownCoordinator::new(),
            LifecyclePolicy::new(
                2,
                Duration::from_millis(10),
                Duration::from_millis(20),
                Duration::from_secs(1),
                Duration::from_secs(10),
            ),
            LspDiagnosticLogger::new(logs.clone()),
        );
        assert!(matches!(
            coordinator
                .acquire(fixture.launch.clone(), ActivationReason::ToolRequest, true)
                .await,
            LspProcessAcquisition::Ready(_)
        ));
        for _ in 0..40 {
            if coordinator
                .status(&fixture.key)
                .await
                .is_some_and(|status| status.state == ProcessState::Backoff)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(
            coordinator
                .status(&fixture.key)
                .await
                .expect("backoff status")
                .state,
            ProcessState::Backoff
        );
        let events = logged_events(&logs);
        assert!(
            events.iter().any(|event| event == expected),
            "expected {expected} event for {mode}, got {events:?}"
        );
        assert!(
            events.iter().any(|event| event == "crash"),
            "expected crash event for {mode}, got {events:?}"
        );

        if mode == "lsp-crash" {
            for _ in 0..10 {
                coordinator.tick().await;
                if logged_events(&logs).iter().any(|event| event == "restart") {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            let events = logged_events(&logs);
            assert!(events.iter().any(|event| event == "restart"));
            let _ = coordinator
                .shutdown_all(Instant::now() + Duration::from_secs(2))
                .await;
        }
    }
}

#[derive(Default)]
struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

impl DiagnosticLogPort for CapturingLogs {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.0.lock().expect("logs").push(log);
        Ok(())
    }
}

fn logger() -> LspDiagnosticLogger {
    LspDiagnosticLogger::new(Arc::new(CapturingLogs::default()))
}

fn logged_events(logs: &CapturingLogs) -> Vec<String> {
    logs.0
        .lock()
        .expect("logs")
        .iter()
        .filter_map(|log| log.context.get("event").cloned())
        .collect()
}

struct ProcessFixture {
    _directory: tempfile::TempDir,
    key: ProcessKey,
    launch: LspProcessLaunch,
}

impl ProcessFixture {
    fn new(mode: &str) -> Self {
        let directory = tempfile::tempdir().expect("workspace");
        std::fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .expect("manifest");
        let key = ProcessKey::new(
            directory.path(),
            directory.path(),
            ServerKind::RustAnalyzer,
            ConfigurationFingerprint::new("fixture-config").expect("fingerprint"),
        )
        .expect("process key");
        let arguments = vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/lsp_stdio_server.cjs")
                .to_string_lossy()
                .into_owned(),
            mode.to_string(),
        ];
        let launch = LspProcessLaunch {
            key: key.clone(),
            executable: "node".to_string(),
            arguments,
            initialization_options: json!({}),
        };
        Self {
            _directory: directory,
            key,
            launch,
        }
    }
}
