use crate::bootstrap::NativeCodeIntelligenceResponder;
use crate::contexts::agent_runtime::application::{
    code_intelligence_tool_definitions, AgentCodeIntelligenceContext, AgentCodeIntelligencePort,
    AgentCodeIntelligenceStatus, AgentDocumentInput, AgentDocumentPositionInput,
};
use crate::contexts::agent_runtime::infrastructure::RuntimeAgentCodeIntelligenceAdapter;
use crate::contexts::code_intelligence::api::{
    resolve_language, CodeIntelligenceApi, LspConfiguration, ProcessState,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn native_lsp_runtime_covers_tools_reconfiguration_trust_and_desktop_shutdown() {
    let fixture = NativeLspFixture::new();
    let logs = Arc::new(CapturingLogs::default());
    let api = CodeIntelligenceApi::from_database(
        NativeDatabase::new(fixture.data.path().to_path_buf()).expect("database"),
        logs.clone(),
    );
    let adapter = Arc::new(RuntimeAgentCodeIntelligenceAdapter::new(Arc::new(
        NativeCodeIntelligenceResponder::new(api.clone()),
    )));
    let context = AgentCodeIntelligenceContext::from_session_workspace(
        fixture.workspace.path().to_string_lossy().into_owned(),
    );

    let mut tool_names = code_intelligence_tool_definitions()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    tool_names.sort();
    assert_eq!(
        tool_names,
        [
            "find_call_hierarchy",
            "find_definition",
            "find_implementations",
            "find_references",
            "find_type_definition",
            "find_workspace_symbols",
            "get_diagnostics",
            "get_document_symbols",
            "get_hover",
        ]
    );
    assert!(!adapter.is_available(&context));

    let configuration = fixture.configuration(json!({"revision": 1}));
    api.save_configuration(&configuration)
        .expect("save configuration");
    assert!(!adapter.is_available(&context));
    api.update_workspace_trust(fixture.workspace.path(), true)
        .expect("trust workspace");
    assert!(adapter.is_available(&context));
    wait_for_ready(&api).await;

    let definition_outcome = definition(adapter.clone(), context.clone()).await;
    assert_eq!(
        definition_outcome.metadata.status,
        AgentCodeIntelligenceStatus::Ready
    );
    assert_eq!(definition_outcome.value.expect("definition").len(), 1);

    let reference_outcome = references(adapter.clone(), context.clone()).await;
    assert_eq!(
        reference_outcome.metadata.status,
        AgentCodeIntelligenceStatus::Ready
    );
    assert_eq!(reference_outcome.value.expect("references").len(), 50);
    assert!(reference_outcome.metadata.truncated);

    let mut definition_samples = Vec::new();
    let mut reference_samples = Vec::new();
    for _ in 0..7 {
        let started = Instant::now();
        let measured = definition(adapter.clone(), context.clone()).await;
        definition_samples.push(started.elapsed().as_micros());
        assert_eq!(measured.value.expect("definition").len(), 1);
        let started = Instant::now();
        let measured = references(adapter.clone(), context.clone()).await;
        reference_samples.push(started.elapsed().as_micros());
        assert_eq!(measured.value.expect("references").len(), 50);
    }
    definition_samples.sort_unstable();
    reference_samples.sort_unstable();
    eprintln!(
        "LSP_PERFORMANCE dataset=repo-small@1 definitionP50Micros={} definitionP95Micros={} referencesP50Micros={} referencesP95Micros={} maxResponseItems=50",
        definition_samples[3],
        definition_samples[6],
        reference_samples[3],
        reference_samples[6]
    );

    let hover = hover(adapter.clone(), context.clone()).await;
    assert_eq!(hover.metadata.status, AgentCodeIntelligenceStatus::Ready);
    assert_eq!(
        hover
            .value
            .expect("hover envelope")
            .expect("hover")
            .signature
            .as_deref(),
        Some("fn alpha()")
    );

    let diagnostics = diagnostics(adapter.clone(), context.clone()).await;
    assert_eq!(
        diagnostics.metadata.status,
        AgentCodeIntelligenceStatus::Ready
    );
    assert_eq!(diagnostics.value.expect("diagnostics").len(), 1);
    let status = ready_status(&api).await;
    assert_eq!(status.diagnostic_count, 1);
    assert!(status.last_response_at.is_some());
    assert!(status.negotiated_capabilities.is_some());

    api.save_configuration(&fixture.configuration(json!({"revision": 2})))
        .expect("replace configuration");
    wait_for_empty(&api).await;
    assert!(adapter.is_available(&context));
    wait_for_ready(&api).await;
    assert_eq!(
        definition(adapter.clone(), context.clone())
            .await
            .metadata
            .status,
        AgentCodeIntelligenceStatus::Ready
    );

    api.update_workspace_trust(fixture.workspace.path(), false)
        .expect("revoke trust");
    wait_for_empty(&api).await;
    assert!(!adapter.is_available(&context));

    api.update_workspace_trust(fixture.workspace.path(), true)
        .expect("restore trust");
    assert!(adapter.is_available(&context));
    wait_for_ready(&api).await;
    api.shutdown(Instant::now() + Duration::from_secs(3))
        .await
        .expect("desktop LSP shutdown");
    wait_for_empty(&api).await;

    let lifecycle = wait_for_lifecycle_events(&fixture.lifecycle, 3).await;
    assert!(
        lifecycle
            .lines()
            .filter(|event| *event == "shutdown")
            .count()
            >= 3
    );
    assert!(lifecycle.lines().filter(|event| *event == "exit").count() >= 3);
    let logs = logs.0.lock().expect("logs");
    assert!(logs.iter().any(|log| {
        log.context.get("event").map(String::as_str) == Some("shutdown")
            && log.context.get("forced").map(String::as_str) == Some("false")
    }));
    for transition in [("starting", "initializing"), ("initializing", "ready")] {
        assert!(logs.iter().any(|log| {
            log.context.get("fromState").map(String::as_str) == Some(transition.0)
                && log.context.get("toState").map(String::as_str) == Some(transition.1)
        }));
    }
}

async fn definition(
    adapter: Arc<RuntimeAgentCodeIntelligenceAdapter>,
    context: AgentCodeIntelligenceContext,
) -> crate::contexts::agent_runtime::application::AgentCodeIntelligenceOutcome<
    Vec<crate::contexts::agent_runtime::application::AgentCodeLocation>,
> {
    tokio::task::spawn_blocking(move || adapter.find_definition(&context, &position(), active()))
        .await
        .expect("definition task")
}

async fn references(
    adapter: Arc<RuntimeAgentCodeIntelligenceAdapter>,
    context: AgentCodeIntelligenceContext,
) -> crate::contexts::agent_runtime::application::AgentCodeIntelligenceOutcome<
    Vec<crate::contexts::agent_runtime::application::AgentCodeLocation>,
> {
    tokio::task::spawn_blocking(move || adapter.find_references(&context, &position(), active()))
        .await
        .expect("references task")
}

async fn hover(
    adapter: Arc<RuntimeAgentCodeIntelligenceAdapter>,
    context: AgentCodeIntelligenceContext,
) -> crate::contexts::agent_runtime::application::AgentCodeIntelligenceOutcome<
    Option<crate::contexts::agent_runtime::application::AgentCodeHover>,
> {
    tokio::task::spawn_blocking(move || adapter.get_hover(&context, &position(), active()))
        .await
        .expect("hover task")
}

async fn diagnostics(
    adapter: Arc<RuntimeAgentCodeIntelligenceAdapter>,
    context: AgentCodeIntelligenceContext,
) -> crate::contexts::agent_runtime::application::AgentCodeIntelligenceOutcome<
    Vec<crate::contexts::agent_runtime::application::AgentCodeDiagnostic>,
> {
    tokio::task::spawn_blocking(move || {
        adapter.get_diagnostics(
            &context,
            &AgentDocumentInput {
                relative_path: "src/lib.rs".to_owned(),
            },
            active(),
        )
    })
    .await
    .expect("diagnostics task")
}

fn position() -> AgentDocumentPositionInput {
    AgentDocumentPositionInput {
        relative_path: "src/lib.rs".to_owned(),
        line: 1,
        column: 1,
    }
}

fn active() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

const PROCESS_STATE_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const PROCESS_STATE_POLL_INTERVAL: Duration = Duration::from_millis(20);

async fn wait_for_ready(api: &CodeIntelligenceApi) {
    let deadline = Instant::now() + PROCESS_STATE_WAIT_TIMEOUT;
    loop {
        let last_observed = match api.server_statuses().await {
            Ok(statuses) if statuses.len() == 1 && statuses[0].state == ProcessState::Ready => {
                return;
            }
            Ok(statuses) => summarize_process_states(&statuses),
            Err(error) => format!("status error: {error:?}"),
        };
        if Instant::now() >= deadline {
            panic!("LSP server did not become ready; last observed: {last_observed}");
        }
        tokio::time::sleep(PROCESS_STATE_POLL_INTERVAL).await;
    }
}

async fn wait_for_empty(api: &CodeIntelligenceApi) {
    let deadline = Instant::now() + PROCESS_STATE_WAIT_TIMEOUT;
    loop {
        let last_observed = match api.server_statuses().await {
            Ok(statuses) if statuses.is_empty() => return,
            Ok(statuses) => summarize_process_states(&statuses),
            Err(error) => format!("status error: {error:?}"),
        };
        if Instant::now() >= deadline {
            panic!("LSP processes were not cleaned up; last observed: {last_observed}");
        }
        tokio::time::sleep(PROCESS_STATE_POLL_INTERVAL).await;
    }
}

async fn wait_for_lifecycle_events(path: &Path, expected_count: usize) -> String {
    let deadline = Instant::now() + PROCESS_STATE_WAIT_TIMEOUT;
    loop {
        let lifecycle = std::fs::read_to_string(path).unwrap_or_default();
        let shutdown_count = lifecycle
            .lines()
            .filter(|event| *event == "shutdown")
            .count();
        let exit_count = lifecycle.lines().filter(|event| *event == "exit").count();
        if shutdown_count >= expected_count && exit_count >= expected_count {
            return lifecycle;
        }
        assert!(
            Instant::now() < deadline,
            "LSP lifecycle events did not settle; shutdown={shutdown_count}, exit={exit_count}"
        );
        tokio::time::sleep(PROCESS_STATE_POLL_INTERVAL).await;
    }
}

fn summarize_process_states(
    statuses: &[crate::contexts::code_intelligence::api::ServerStatus],
) -> String {
    if statuses.is_empty() {
        return "empty status snapshot".to_owned();
    }
    statuses
        .iter()
        .map(|status| format!("{:?}", status.state))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn ready_status(
    api: &CodeIntelligenceApi,
) -> crate::contexts::code_intelligence::api::ServerStatus {
    api.server_statuses()
        .await
        .expect("server statuses")
        .into_iter()
        .next()
        .expect("ready server")
}

#[derive(Default)]
struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

impl DiagnosticLogPort for CapturingLogs {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.0.lock().expect("logs").push(log);
        Ok(())
    }
}

struct NativeLspFixture {
    data: TempDirectory,
    workspace: TempDirectory,
    executable: PathBuf,
    lifecycle: PathBuf,
}

impl NativeLspFixture {
    fn new() -> Self {
        let data = TempDirectory::new("native-lsp-e2e-data");
        let workspace = TempDirectory::new("native-lsp-e2e-workspace");
        std::fs::create_dir(workspace.path().join("src")).expect("source directory");
        std::fs::write(
            workspace.path().join("Cargo.toml"),
            "[package]\nname='native-lsp-e2e'\nversion='0.1.0'\n",
        )
        .expect("manifest");
        std::fs::write(workspace.path().join("src/lib.rs"), "fn alpha() {}\n").expect("source");
        let lifecycle = workspace.path().join("lsp-lifecycle.txt");
        let executable = write_wrapper(workspace.path(), &lifecycle);
        Self {
            data,
            workspace,
            executable,
            lifecycle,
        }
    }

    fn configuration(&self, options: serde_json::Value) -> LspConfiguration {
        let mut configuration = LspConfiguration {
            enabled: true,
            ..LspConfiguration::default()
        };
        let rust = configuration
            .languages
            .get_mut(
                &resolve_language("rust")
                    .expect("rust is registered")
                    .language_id(),
            )
            .expect("Rust configuration");
        rust.enabled = true;
        rust.executable_override = Some(self.executable.to_string_lossy().into_owned());
        rust.initialization_options = options;
        configuration
    }
}

fn write_wrapper(directory: &Path, lifecycle: &Path) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lsp_stdio_server.cjs");
    #[cfg(windows)]
    let (path, body) = (
        directory.join("rust-analyzer.cmd"),
        format!(
            "@echo off\r\nnode \"{}\" lsp-native-e2e \"{}\"\r\n",
            fixture.display(),
            lifecycle.display()
        ),
    );
    #[cfg(not(windows))]
    let (path, body) = (
        directory.join("rust-analyzer"),
        format!(
            "#!/bin/sh\nexec node '{}' lsp-native-e2e '{}'\n",
            fixture.display(),
            lifecycle.display()
        ),
    );
    std::fs::write(&path, body).expect("LSP wrapper");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path)
            .expect("wrapper metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&path, permissions).expect("wrapper permissions");
    }
    path
}
