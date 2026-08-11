use super::managed_mcp_relay::InvocationScopedMcpRelayAdapter;
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, AgentRuntimeApiServices};
use crate::contexts::agent_runtime::application::{
    AgentCodeIntelligenceResponderPort, AgentRetrievalPort, AgentRuntimeApplicationPorts,
    AgentRuntimeApplicationService, AgentTerminalApplicationPorts, AgentTerminalApplicationService,
    AgentWorkspaceMutationPort, ExpertRoleApplicationPorts, ExpertRoleApplicationService,
    LoopApplicationPorts, LoopApplicationService, LoopControlApplicationPorts,
    LoopControlApplicationService, LoopOperationObserver, LoopOrchestratorApplicationService,
    LoopOrchestratorPorts, LoopProgressApplicationService, LoopRecoveryApplicationPorts,
    LoopRecoveryApplicationService, LoopVerificationApplicationPorts,
    LoopVerificationApplicationService, LoopVerifierApplicationPorts,
    LoopVerifierApplicationService, LoopWorkerApplicationPorts, LoopWorkerApplicationService,
};
use crate::contexts::agent_runtime::infrastructure::{
    builtin_expert_roles, AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter,
    CompositeAgentProcessGateway, CredentialAwareAgentRegistry, HttpOnePieceModelDiscoveryAdapter,
    InMemoryAgentMessageTerminalCompletions, InMemoryGenerationCoordinator,
    InMemoryLoopExecutionCoordinator, InMemoryLoopRoleGenerationCompletions,
    InMemorySeatTurnCompletions, NativeAgentCoreInstructionsAdapter, NativeLoopScheduler,
    NativeSeatTurnCoordinator, OsApiCredentialAdapter, PermissionsPortAdapter,
    PortablePtyAgentTerminalRuntime, RuntimeAgentApiAdapter, RuntimeAgentAvailabilityAdapter,
    RuntimeAgentCliProfileAdapter, RuntimeAgentMcpToolAdapter, RuntimeAgentMemoryExtractionAdapter,
    RuntimeAgentPersonalizationAdapter, RuntimeAgentProcessAdapter, RuntimeAgentSkillAdapter,
    RuntimeEffectivePromptAdapter, RuntimeOnePiecePlanningAdapter, SessionsAgentRuntimeAdapter,
    SqliteAgentMemoryRepository, SqliteAgentRuntimeRepository, SqliteExpertRoleRepository,
    SqliteLoopRepository, StructuredLoopVerificationProcess, SystemAgentRuntimeClock,
    SystemExpertRoleClock, TauriAgentRuntimeEventAdapter, TerminalExecutionObservability,
    UuidExpertRoleIds, WorkspaceLoopProjectAdapter,
};
use crate::contexts::desktop::api::DesktopSettingsApi;
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
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::tooling::mcp::api::McpApi;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::sdk::api::SdkApi;
use crate::contexts::tooling::skills::api::SkillApi;
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
    pub(crate) cli: CliApi,
    pub(crate) cli_parameters: CliParametersApi,
    pub(crate) prompts: PromptHookApi,
    pub(crate) skills: SkillApi,
    pub(crate) mcp: McpApi,
    pub(crate) sessions: SessionsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) permissions: PermissionsApi,
    pub(crate) shared_registry: SharedAgentRegistry,
    /// Consumed by `RuntimeAgentApiAdapter`'s `recall` tool (Task 13). A concrete
    /// `Arc<retrieval::DeferredAgentRetrieval>`, coerced here — `assemble_retrieval` itself needs
    /// this function's own output (`AgentRuntimeApi`), so the real `RetrievalApi` cannot exist
    /// yet; `runtime.rs`'s `setup` binds it right after `assemble_retrieval` returns.
    pub(crate) retrieval: Arc<dyn AgentRetrievalPort>,
    pub(crate) code_intelligence: Arc<dyn AgentCodeIntelligenceResponderPort>,
    pub(crate) workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    pub(crate) desktop_settings: DesktopSettingsApi,
}

#[derive(Clone)]
pub(crate) struct SharedAgentRegistry {
    pub(crate) repository: Arc<SqliteAgentRuntimeRepository>,
    pub(crate) registry: Arc<CredentialAwareAgentRegistry>,
    api_credentials: Arc<OsApiCredentialAdapter>,
    logging: Arc<AgentRuntimeLoggingAdapter>,
    clock: Arc<SystemAgentRuntimeClock>,
    unified_logging: Arc<UnifiedLoggingAdapter>,
}

pub(crate) fn assemble_shared_agent_registry(
    database: NativeDatabase,
    sdk: SdkApi,
    fallback_log_directory: PathBuf,
) -> SharedAgentRegistry {
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging.clone();
    let logging = Arc::new(AgentRuntimeLoggingAdapter::new(diagnostics, operation_logs));
    let clock = Arc::new(SystemAgentRuntimeClock);
    let availability = Arc::new(RuntimeAgentAvailabilityAdapter::new(sdk));
    let repository = Arc::new(SqliteAgentRuntimeRepository::new(database, availability));
    let api_credentials = Arc::new(OsApiCredentialAdapter::new());
    let registry = Arc::new(CredentialAwareAgentRegistry::new(
        repository.clone(),
        api_credentials.clone(),
        logging.clone(),
        clock.clone(),
    ));
    SharedAgentRegistry {
        repository,
        registry,
        api_credentials,
        logging,
        clock,
        unified_logging,
    }
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
) -> Result<AgentRuntimeAssembly, String> {
    let shared = dependencies.shared_registry;
    let unified_logging = shared.unified_logging;
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let logging = shared.logging;
    let clock = shared.clock;
    let repository = shared.repository;
    let api_credentials = shared.api_credentials;
    let registry = shared.registry;
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
    let provider_registry = Arc::new(
        crate::contexts::agent_runtime::infrastructure::providers::builtin_cli_provider_registry()
            .map_err(|error| error.to_string())?,
    );
    let cli_processes = Arc::new(RuntimeAgentProcessAdapter::new(
        logging.clone(),
        clock.clone(),
        execution_ids.clone(),
        telemetry.clone(),
        Arc::new(InvocationScopedMcpRelayAdapter::new(
            dependencies.database.clone(),
        )),
        provider_registry.clone(),
    ));
    let sessions = Arc::new(SessionsAgentRuntimeAdapter::new(dependencies.sessions));
    let agent_skills = Arc::new(RuntimeAgentSkillAdapter::new(dependencies.skills));
    let agent_memories = Arc::new(SqliteAgentMemoryRepository::new(
        dependencies.database.clone(),
    ));
    let agent_mcp_tools = Arc::new(RuntimeAgentMcpToolAdapter::new(dependencies.mcp));
    let agent_permissions = Arc::new(PermissionsPortAdapter::new(
        dependencies.permissions.clone(),
    ));
    let agent_personalization = Arc::new(RuntimeAgentPersonalizationAdapter::new(
        dependencies.desktop_settings,
    ));
    let agent_memory_extraction = Arc::new(RuntimeAgentMemoryExtractionAdapter::new(
        api_credentials.clone(),
        repository.clone(),
    ));
    let onepiece_planning = Arc::new(RuntimeOnePiecePlanningAdapter::new(
        api_credentials.clone(),
        repository.clone(),
    ));
    let code_intelligence = Arc::new(
        crate::contexts::agent_runtime::infrastructure::RuntimeAgentCodeIntelligenceAdapter::new(
            dependencies.code_intelligence,
        ),
    );
    let api_processes = Arc::new(RuntimeAgentApiAdapter::new_with_code_intelligence(
        api_credentials.clone(),
        repository.clone(),
        sessions.clone(),
        logging.clone(),
        clock.clone(),
        agent_skills,
        Arc::new(NativeAgentCoreInstructionsAdapter),
        agent_memories.clone(),
        agent_mcp_tools,
        agent_permissions,
        dependencies.retrieval,
        code_intelligence,
        dependencies.workspace_mutations,
        agent_personalization.clone(),
    ));
    let tool_approvals = api_processes.clone();
    let processes: Arc<dyn crate::contexts::agent_runtime::application::AgentProcessGateway> =
        Arc::new(CompositeAgentProcessGateway::new(
            cli_processes,
            api_processes,
        ));
    let cli_profiles = Arc::new(RuntimeAgentCliProfileAdapter::new(
        dependencies.cli_parameters,
        dependencies.cli,
        dependencies.permissions,
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
        TerminalExecutionObservability::new(
            execution_ids.clone(),
            timeline.clone(),
            telemetry.clone(),
        ),
        std::env::temp_dir().join("vanehub-agent-terminal-wrappers"),
        provider_registry,
    ));
    let loop_completions = Arc::new(InMemoryLoopRoleGenerationCompletions::default());
    let seat_completions = Arc::new(InMemorySeatTurnCompletions::default());
    // Shared with the expert role service below: the roster a seat is briefed with reads the same
    // roles the settings page writes, so a role edited mid-session takes effect on the next turn.
    let expert_role_repository = Arc::new(SqliteExpertRoleRepository::new(
        dependencies.database.clone(),
    ));
    let service = AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
        registry: registry.clone(),
        workflows: repository.clone(),
        sessions: sessions.clone(),
        cli_profiles: cli_profiles.clone(),
        prompts: Arc::new(RuntimeEffectivePromptAdapter::new(dependencies.prompts)),
        processes: processes.clone(),
        operations: operations.clone(),
        logging: logging.clone(),
        clock: clock.clone(),
        events: events.clone(),
        generations: Arc::new(InMemoryGenerationCoordinator::default()),
        execution_ids: execution_ids.clone(),
        execution_settings: timeline.clone(),
        telemetry: telemetry.clone(),
        loop_completions: loop_completions.clone(),
        seat_completions: seat_completions.clone(),
        expert_roles: expert_role_repository.clone(),
        history: sessions.clone(),
        message_completions: Arc::new(InMemoryAgentMessageTerminalCompletions::default()),
        api_agents: repository.clone(),
        api_credentials: api_credentials.clone(),
        onepiece_model_discovery: Arc::new(HttpOnePieceModelDiscoveryAdapter),
        tool_approvals: tool_approvals.clone(),
        memories: agent_memories,
        memory_extraction: agent_memory_extraction,
        personalization: agent_personalization,
    });
    let terminal_service = AgentTerminalApplicationService::new(AgentTerminalApplicationPorts {
        registry: registry.clone(),
        sessions: sessions.clone(),
        cli_profiles: cli_profiles.clone(),
        terminals: terminal_runtime,
        logging: logging.clone(),
        clock: clock.clone(),
        events: events.clone(),
        terminal_events: events,
    });
    let expert_roles = ExpertRoleApplicationService::new(ExpertRoleApplicationPorts {
        repository: expert_role_repository,
        clock: Arc::new(SystemExpertRoleClock),
        ids: Arc::new(UuidExpertRoleIds),
        builtins: builtin_expert_roles(),
    });
    let loop_repository = Arc::new(SqliteLoopRepository::new(dependencies.database));
    let loop_projects = Arc::new(WorkspaceLoopProjectAdapter::new(dependencies.workspaces));
    let loop_execution = Arc::new(InMemoryLoopExecutionCoordinator::default());
    let loops = LoopApplicationService::new(LoopApplicationPorts {
        loops: loop_repository.clone(),
        registry: registry.clone(),
        api_agents: repository.clone(),
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
        registry: registry.clone(),
        roles: sessions.clone(),
        git: loop_projects.clone(),
        generations: generations.clone(),
        clock: clock.clone(),
    });
    let guarded_validation = Arc::new(StructuredLoopVerificationProcess::default());
    let loop_verification =
        LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
            iterations: loop_repository.clone(),
            processes: guarded_validation.clone(),
            observer: loop_observer.clone(),
            clock: clock.clone(),
        });
    let loop_verifier = LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
        iterations: loop_repository.clone(),
        registry,
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
    let seat_turns = NativeSeatTurnCoordinator::new(service.clone());
    Ok(AgentRuntimeAssembly {
        api: AgentRuntimeApi::new(AgentRuntimeApiServices {
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
            expert_roles,
            seat_turns,
            guarded_validation,
            onepiece_planning,
        }),
        telemetry_lifecycle,
    })
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
