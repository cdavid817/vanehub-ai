use super::document_invalidation::LspDocumentInvalidationQueue;
use super::lsp_diagnostics::LspDiagnosticLogger;
use super::process_registry::LifecyclePolicy;
use super::project_root::ProcessKey;
use super::runtime_process_coordinator::{LspProcessLaunch, RuntimeProcessCoordinator};
use super::semantic_query_coordinator::SemanticQueryCoordinator;
use super::shutdown_coordinator::LspShutdownCoordinator;
use crate::contexts::code_intelligence::domain::models::{
    ConfigurationFingerprint, NormalizedSymbol, QueryStatus,
};
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

#[tokio::test]
async fn type_definition_and_implementations_reuse_the_definition_shape() {
    let fixture = SemanticFixture::new();
    let queries = coordinator();

    let type_definition = queries
        .find_type_definition(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(type_definition.status(), QueryStatus::Ready);
    let located = type_definition.value().expect("type definitions");
    assert_eq!(located.len(), 1);
    assert_eq!(located[0].file(), "src/lib.rs");
    // The fixture answers in LocationLink form with a wide target range and a narrow selection
    // range. Columns 4..9 are the selection range, so the link path is not silently falling back
    // to the enclosing target range.
    assert_eq!(located[0].range.start_column, 4);
    assert_eq!(located[0].range.end_column, 9);

    let implementations = queries
        .find_implementations(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    // Nothing implements it. `ready` with an empty list says that; `unavailable` would say the
    // server could not answer, which is a different thing and would send the agent looking again.
    assert_eq!(implementations.status(), QueryStatus::Ready);
    assert!(implementations.value().expect("implementations").is_empty());
    assert_eq!(implementations.total, 0);
    assert!(!implementations.truncated);
}

#[tokio::test]
async fn an_unadvertised_method_is_refused_without_reaching_the_server() {
    let fixture = SemanticFixture::with_mode("lsp-unadvertised");
    let queries = coordinator();

    let refused = queries
        .find_implementations(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(refused.status(), QueryStatus::Unavailable);
    assert_eq!(refused.reason_code(), Some("method_unsupported"));

    // That mode exits on an unadvertised request, so a definition query still being answered is
    // the evidence that nothing was sent -- an assertion on the outcome alone would also pass if
    // the request went out and the server merely declined it.
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
}

#[tokio::test]
async fn workspace_symbols_are_bounded_filtered_and_refuse_an_empty_query() {
    let fixture = SemanticFixture::new();
    let queries = coordinator();

    let empty = queries
        .find_workspace_symbols(fixture.launch.clone(), registry::rust(), "  ", active())
        .await;
    // Refused here rather than sent on: the servers that answer an empty query answer it with the
    // whole index, and the ones that do not disagree about what it means.
    assert_eq!(empty.status(), QueryStatus::Failed);
    assert_eq!(empty.reason_code(), Some("invalid_query"));

    // Opened by an earlier query, because workspace symbols themselves open nothing -- the fixture
    // answers with locations in that document.
    let opened = queries
        .find_definition(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            1,
            1,
            active(),
        )
        .await;
    assert_eq!(opened.status(), QueryStatus::Ready);

    let symbols = queries
        .find_workspace_symbols(fixture.launch.clone(), registry::rust(), "alpha", active())
        .await;
    assert_eq!(symbols.status(), QueryStatus::Ready);
    let found = symbols.value().expect("workspace symbols");
    assert_eq!(found.len(), 50);
    assert_eq!(symbols.total, 55);
    assert!(symbols.truncated);
    // The match outside the canonical workspace, dropped before the Agent sees it.
    assert_eq!(symbols.filtered_count, 1);
    assert_eq!(found[0].name, "alpha_0");
    assert_eq!(found[0].kind, "function");
    assert_eq!(found[0].container.as_deref(), Some("fixture"));
    assert_eq!(found[0].location.file(), "src/lib.rs");
    // No document was named, so there is no version to report and none is invented.
    assert!(symbols.document_version().is_none());
}

#[tokio::test]
async fn nested_and_flat_document_symbols_produce_the_same_shape() {
    let nested = document_symbols_from("lsp-semantic").await;
    let flat = document_symbols_from("lsp-flat-symbols").await;

    // The protocol lets a server answer in either form for the same content. Which one it picks is
    // not something the Agent should be able to tell.
    assert_eq!(nested, flat);
    assert_eq!(nested.len(), 2);
    assert_eq!(nested[0].name, "alpha");
    assert_eq!(nested[0].kind, "function");
    assert_eq!(nested[0].container, None);
    assert_eq!(nested[0].location.range.start_column, 4);
    assert_eq!(nested[0].location.range.end_column, 9);
    // The nesting is flattened away, so each entry names what encloses it or the hierarchy is lost.
    assert_eq!(nested[1].name, "inner");
    assert_eq!(nested[1].container.as_deref(), Some("alpha"));
}

#[tokio::test]
async fn a_rejected_document_never_reaches_the_server() {
    let fixture = SemanticFixture::new();
    let queries = coordinator();

    // Outside the workspace. Admission refuses it before a lease exists, so nothing is sent and
    // the server never learns the path was asked for.
    let escaped = queries
        .get_document_symbols(
            fixture.launch.clone(),
            registry::rust(),
            "../outside.rs",
            active(),
        )
        .await;
    assert_eq!(escaped.status(), QueryStatus::Failed);
    assert_eq!(escaped.reason_code(), Some("document_unavailable"));

    let missing = queries
        .get_document_symbols(
            fixture.launch.clone(),
            registry::rust(),
            "src/absent.rs",
            active(),
        )
        .await;
    assert_eq!(missing.status(), QueryStatus::Failed);
    assert_eq!(missing.reason_code(), Some("document_unavailable"));
}

async fn document_symbols_from(server_mode: &str) -> Vec<NormalizedSymbol> {
    let fixture = SemanticFixture::with_mode(server_mode);
    let queries = coordinator();
    let symbols = queries
        .get_document_symbols(
            fixture.launch.clone(),
            registry::rust(),
            "src/lib.rs",
            active(),
        )
        .await;
    assert_eq!(symbols.status(), QueryStatus::Ready, "{server_mode}");
    assert_eq!(
        symbols.document_version().map(|version| version.value()),
        Some(1),
        "{server_mode}"
    );
    symbols.into_value().expect("document symbols")
}

fn coordinator() -> SemanticQueryCoordinator {
    let processes = RuntimeProcessCoordinator::new(
        LspShutdownCoordinator::new(),
        LifecyclePolicy::default(),
        LspDiagnosticLogger::new(Arc::new(CapturingLogs::default())),
    );
    SemanticQueryCoordinator::new(processes, LspDocumentInvalidationQueue::default())
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
        Self::with_mode("lsp-semantic")
    }

    fn with_mode(server_mode: &str) -> Self {
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
                server_mode.to_owned(),
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
