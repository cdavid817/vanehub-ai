use super::document_invalidation::LspDocumentInvalidationQueue;
use super::lsp_diagnostics::LspDiagnosticLogger;
use super::process_registry::LifecyclePolicy;
use super::project_root::ProcessKey;
use super::runtime_process_coordinator::{LspProcessLaunch, RuntimeProcessCoordinator};
use super::semantic_query_coordinator::SemanticQueryCoordinator;
use super::shutdown_coordinator::LspShutdownCoordinator;
use crate::contexts::code_intelligence::domain::models::{ConfigurationFingerprint, QueryStatus};
use crate::contexts::code_intelligence::domain::registry;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tokio::test]
async fn semantic_queries_sync_filter_bound_normalize_and_cancel() {
    let fixture = SemanticFixture::new();
    let shutdown = LspShutdownCoordinator::new();
    let logs = Arc::new(CapturingLogs::default());
    let processes = RuntimeProcessCoordinator::new(
        shutdown,
        LifecyclePolicy::default(),
        LspDiagnosticLogger::new(logs.clone()),
    );
    let invalidations = LspDocumentInvalidationQueue::default();
    let queries = SemanticQueryCoordinator::new(processes.clone(), invalidations.clone());

    let definition = queries
        .find_definition(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(definition.status(), QueryStatus::Ready);
    assert_eq!(definition.value().expect("definitions").len(), 1);
    assert_eq!(
        definition.value().expect("definitions")[0].file(),
        "src/lib.rs"
    );

    let references = queries
        .find_references(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(references.status(), QueryStatus::Ready);
    assert_eq!(references.value().expect("references").len(), 50);
    assert_eq!(references.total, 55);
    assert_eq!(references.filtered_count, 1);
    assert!(references.truncated);

    let hover = queries
        .get_hover(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(hover.status(), QueryStatus::Ready);
    assert_eq!(
        hover
            .value()
            .and_then(Option::as_ref)
            .and_then(|value| value.signature.as_deref()),
        Some("fn alpha()")
    );

    let diagnostics = queries
        .get_diagnostics(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            active(),
        )
        .await;
    assert_eq!(diagnostics.status(), QueryStatus::Ready);
    assert_eq!(diagnostics.value().expect("diagnostics").len(), 1);
    assert_eq!(
        diagnostics.value().expect("diagnostics")[0].message,
        "Fixture warning"
    );
    let statuses = processes.status_snapshots().await;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].diagnostic_count, 1);
    assert!(statuses[0].last_response_at.is_some());
    assert!(statuses[0].capabilities.is_some());

    std::fs::write(&fixture.document, "pub fn alpha() {}\n").expect("edit document");
    invalidations.publish(fixture.workspace.path(), "src/lib.rs");
    let synchronized = queries
        .find_definition(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(synchronized.status(), QueryStatus::Ready);
    assert_eq!(
        synchronized
            .document_version()
            .map(|version| version.value()),
        Some(2)
    );
    assert_eq!(
        synchronized.value().expect("synchronized definition")[0]
            .range
            .end_column,
        4
    );

    let invalid_position = queries
        .find_definition(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            99,
            1,
            active(),
        )
        .await;
    assert_eq!(invalid_position.status(), QueryStatus::Failed);
    assert_eq!(invalid_position.reason_code(), Some("invalid_position"));

    let cancelled = Arc::new(AtomicBool::new(false));
    let pending = tokio::spawn({
        let queries = queries.clone();
        let launch = fixture.launch.clone();
        let cancelled = cancelled.clone();
        async move {
            queries
                .find_definition(launch, registry::rust(), "src/lib.rs", 1, 2, cancelled)
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancelled.store(true, Ordering::Release);
    let cancelled = pending.await.expect("cancelled query");
    assert_eq!(cancelled.status(), QueryStatus::Failed);
    assert_eq!(cancelled.reason_code(), Some("generation_cancelled"));

    let diagnostics = queries
        .get_diagnostics(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            active(),
        )
        .await;
    assert_eq!(diagnostics.status(), QueryStatus::Ready);
    assert!(diagnostics
        .value()
        .expect("current empty diagnostics")
        .is_empty());
    assert_eq!(processes.status_snapshots().await[0].diagnostic_count, 0);

    let summary = processes
        .shutdown_all(Instant::now() + Duration::from_secs(3))
        .await;
    assert_eq!(summary.failed, 0);
    let events = logs
        .0
        .lock()
        .expect("logs")
        .iter()
        .filter_map(|log| log.context.get("event").cloned())
        .collect::<Vec<_>>();
    for expected in ["diagnostics_count", "cancelled", "shutdown"] {
        assert!(events.iter().any(|event| event == expected));
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

fn active() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

struct SemanticFixture {
    workspace: tempfile::TempDir,
    document: std::path::PathBuf,
    launch: LspProcessLaunch,
}

impl SemanticFixture {
    fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::write(
            workspace.path().join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .expect("manifest");
        std::fs::create_dir(workspace.path().join("src")).expect("source directory");
        let document = workspace.path().join("src/lib.rs");
        std::fs::write(&document, "fn alpha() {}\n").expect("source");
        let key = ProcessKey::new(
            workspace.path(),
            workspace.path(),
            registry::rust(),
            ConfigurationFingerprint::new("semantic-fixture").expect("fingerprint"),
        )
        .expect("process key");
        let launch = LspProcessLaunch {
            key,
            executable: "node".to_owned(),
            arguments: vec![
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/lsp_stdio_server.cjs")
                    .to_string_lossy()
                    .into_owned(),
                "lsp-semantic".to_owned(),
            ],
            initialization_options: json!({}),
        };
        Self {
            workspace,
            document,
            launch,
        }
    }
}
