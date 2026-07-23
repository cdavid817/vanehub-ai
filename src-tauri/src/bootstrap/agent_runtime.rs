use super::managed_mcp_relay::InvocationScopedMcpRelayAdapter;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationPorts, AgentRuntimeApplicationService, AgentTerminalApplicationPorts,
    AgentTerminalApplicationService, LoopApplicationPorts, LoopApplicationService,
    LoopControlApplicationPorts, LoopControlApplicationService, LoopOperationObserver,
    LoopOrchestratorApplicationService, LoopOrchestratorPorts, LoopProgressApplicationService,
    LoopRecoveryApplicationPorts, LoopRecoveryApplicationService, LoopVerificationApplicationPorts,
    LoopVerificationApplicationService, LoopVerifierApplicationPorts,
    LoopVerifierApplicationService, LoopWorkerApplicationPorts, LoopWorkerApplicationService,
};
use crate::contexts::agent_runtime::infrastructure::{
    AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter, InMemoryGenerationCoordinator,
    InMemoryLoopExecutionCoordinator, InMemoryLoopRoleGenerationCompletions, NativeLoopScheduler,
    PortablePtyAgentTerminalRuntime, RuntimeAgentAvailabilityAdapter,
    RuntimeAgentCliProfileAdapter, RuntimeAgentProcessAdapter, RuntimeEffectivePromptAdapter,
    SessionsAgentRuntimeAdapter, SqliteAgentRuntimeRepository, SqliteLoopRepository,
    StructuredLoopVerificationProcess, SystemAgentRuntimeClock, TauriAgentRuntimeEventAdapter,
    WorkspaceLoopProjectAdapter,
};
use crate::contexts::execution_observability::api::ExecutionTelemetryPort;
use crate::contexts::execution_observability::infrastructure::{
    CompositeExecutionTelemetry, ExecutionTelemetryLifecycle, OpenTelemetryExecutionExporter,
    OsObservabilityCredentialAdapter, RandomExecutionIdentity, SqliteExecutionTimelineRepository,
};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLogPort,
    OperationsApi,
};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::sdk::api::SdkApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

pub(crate) struct AgentRuntimeDependencies {
    pub(crate) database: NativeDatabase,
    pub(crate) app: AppHandle,
    pub(crate) operations: OperationsApi,
    pub(crate) sdk: SdkApi,
    pub(crate) cli: CliApi,
    pub(crate) cli_parameters: CliParametersApi,
    pub(crate) prompts: PromptHookApi,
    pub(crate) sessions: SessionsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) fallback_log_directory: PathBuf,
}

pub(crate) struct AgentRuntimeAssembly {
    pub(crate) api: AgentRuntimeApi,
    pub(crate) telemetry_lifecycle: ExecutionTelemetryLifecycle,
}

type ExecutionExporterSet = (
    Vec<Arc<dyn ExecutionTelemetryPort>>,
    Option<Arc<dyn ExternalLogExportPort>>,
);

pub(crate) fn assemble_agent_runtime_api(
    dependencies: AgentRuntimeDependencies,
) -> AgentRuntimeAssembly {
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(
        dependencies.fallback_log_directory,
    ));
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging.clone();
    let logging = Arc::new(AgentRuntimeLoggingAdapter::new(
        diagnostics.clone(),
        operation_logs,
    ));
    let clock = Arc::new(SystemAgentRuntimeClock);
    let availability = Arc::new(RuntimeAgentAvailabilityAdapter::new(dependencies.sdk));
    let repository = Arc::new(SqliteAgentRuntimeRepository::new(
        dependencies.database.clone(),
        availability,
    ));
    let execution_ids = Arc::new(RandomExecutionIdentity);
    let timeline = Arc::new(SqliteExecutionTimelineRepository::new(
        dependencies.database.clone(),
    ));
    let (exporters, log_exporter) = execution_exporters(timeline.as_ref(), diagnostics.clone());
    if let Some(exporter) = log_exporter {
        unified_logging.attach_external_exporter(exporter);
    }
    let telemetry = Arc::new(CompositeExecutionTelemetry::with_diagnostics(
        timeline.clone(),
        exporters,
        diagnostics,
    ));
    let telemetry_lifecycle =
        ExecutionTelemetryLifecycle::new(telemetry.clone(), Duration::from_secs(3));
    let processes = Arc::new(RuntimeAgentProcessAdapter::new(
        logging.clone(),
        clock.clone(),
        execution_ids.clone(),
        telemetry.clone(),
        Arc::new(InvocationScopedMcpRelayAdapter::new(
            dependencies.database.clone(),
        )),
    ));
    let sessions = Arc::new(SessionsAgentRuntimeAdapter::new(dependencies.sessions));
    let cli_profiles = Arc::new(RuntimeAgentCliProfileAdapter::new(
        dependencies.cli_parameters,
        dependencies.cli,
    ));
    let events = Arc::new(TauriAgentRuntimeEventAdapter::new(dependencies.app));
    let operations = Arc::new(AgentRuntimeOperationAdapter::new(dependencies.operations));
    let loop_observer =
        LoopOperationObserver::new(operations.clone(), logging.clone(), clock.clone());
    let terminal_runtime = Arc::new(PortablePtyAgentTerminalRuntime::new(
        events.clone(),
        sessions.clone(),
        logging.clone(),
        clock.clone(),
        std::env::temp_dir().join("vanehub-agent-terminal-wrappers"),
    ));
    let loop_completions = Arc::new(InMemoryLoopRoleGenerationCompletions::default());
    let service = AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
        registry: repository.clone(),
        workflows: repository.clone(),
        sessions: sessions.clone(),
        cli_profiles: cli_profiles.clone(),
        prompts: Arc::new(RuntimeEffectivePromptAdapter::new(dependencies.prompts)),
        processes,
        operations: operations.clone(),
        logging: logging.clone(),
        clock: clock.clone(),
        events: events.clone(),
        generations: Arc::new(InMemoryGenerationCoordinator::default()),
        execution_ids,
        execution_settings: timeline.clone(),
        telemetry,
        loop_completions: loop_completions.clone(),
    });
    let terminal_service = AgentTerminalApplicationService::new(AgentTerminalApplicationPorts {
        registry: repository.clone(),
        sessions: sessions.clone(),
        cli_profiles,
        terminals: terminal_runtime,
        logging,
        clock: clock.clone(),
        events: events.clone(),
        terminal_events: events,
    });
    let loop_repository = Arc::new(SqliteLoopRepository::new(dependencies.database));
    let loop_projects = Arc::new(WorkspaceLoopProjectAdapter::new(dependencies.workspaces));
    let loop_execution = Arc::new(InMemoryLoopExecutionCoordinator::default());
    let loops = LoopApplicationService::new(LoopApplicationPorts {
        loops: loop_repository.clone(),
        registry: repository,
        projects: loop_projects.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let loop_controls = LoopControlApplicationService::new(LoopControlApplicationPorts {
        loops: loop_repository.clone(),
        execution: loop_execution.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let loop_recovery = LoopRecoveryApplicationService::new(LoopRecoveryApplicationPorts {
        loops: loop_repository.clone(),
        leases: loop_execution.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let generations = Arc::new(service.clone());
    let loop_worker = LoopWorkerApplicationService::new(LoopWorkerApplicationPorts {
        iterations: loop_repository.clone(),
        roles: sessions.clone(),
        git: loop_projects.clone(),
        generations: generations.clone(),
        clock: clock.clone(),
    });
    let loop_verification =
        LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
            iterations: loop_repository.clone(),
            processes: Arc::new(StructuredLoopVerificationProcess::default()),
            observer: loop_observer.clone(),
            clock: clock.clone(),
        });
    let loop_verifier = LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
        iterations: loop_repository.clone(),
        roles: sessions,
        context: loop_projects.clone(),
        generations,
    });
    let loop_orchestrator = LoopOrchestratorApplicationService::new(LoopOrchestratorPorts {
        loops: loop_repository.clone(),
        iterations: loop_repository.clone(),
        projects: loop_projects.clone(),
        verifier_context: loop_projects,
        completions: loop_completions,
        generations: Arc::new(service.clone()),
        worker: loop_worker,
        verification: loop_verification,
        verifier: loop_verifier,
        progress: LoopProgressApplicationService::new(loop_repository),
        observer: loop_observer,
        clock,
    });
    let loop_scheduler = NativeLoopScheduler::new((*loop_execution).clone(), loop_orchestrator);
    AgentRuntimeAssembly {
        api: AgentRuntimeApi::new(
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
        ),
        telemetry_lifecycle,
    }
}

fn execution_exporters(
    timeline: &SqliteExecutionTimelineRepository,
    diagnostics: Arc<dyn DiagnosticLogPort>,
) -> ExecutionExporterSet {
    let settings = match timeline.load_settings() {
        Ok(settings) => settings,
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "settings_unavailable");
            return (Vec::new(), None);
        }
    };
    if !settings.otlp_enabled {
        return (Vec::new(), None);
    }
    let Some(endpoint) = settings.otlp_endpoint.as_deref() else {
        record_telemetry_initialization_warning(diagnostics.as_ref(), "endpoint_missing");
        return (Vec::new(), None);
    };
    let credentials = OsObservabilityCredentialAdapter::new();
    let auth_token = match credentials.load_otlp_auth() {
        Ok(token) => token,
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "credential_unavailable");
            return (Vec::new(), None);
        }
    };
    match OpenTelemetryExecutionExporter::otlp_http(
        endpoint,
        settings.sampling_ratio,
        Duration::from_secs(3),
        auth_token.as_deref().map(String::as_str),
    ) {
        Ok(exporter) => {
            let exporter = Arc::new(exporter);
            (
                vec![exporter.clone() as Arc<dyn ExecutionTelemetryPort>],
                Some(exporter as Arc<dyn ExternalLogExportPort>),
            )
        }
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "exporter_unavailable");
            (Vec::new(), None)
        }
    }
}

fn record_telemetry_initialization_warning(logging: &dyn DiagnosticLogPort, reason: &'static str) {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Warn,
        category: "execution_observability.initialization".to_string(),
        message: "Optional execution telemetry export remains disabled; local execution continues"
            .to_string(),
        context: BTreeMap::from([
            ("reason".to_string(), reason.to_string()),
            ("fallback".to_string(), "local_timeline".to_string()),
        ]),
    });
}
