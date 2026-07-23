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
use crate::contexts::operations::api::{DiagnosticLogPort, OperationLogPort, OperationsApi};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::sdk::api::SdkApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
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

pub(crate) fn assemble_agent_runtime_api(
    dependencies: AgentRuntimeDependencies,
) -> AgentRuntimeApi {
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(
        dependencies.fallback_log_directory,
    ));
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging;
    let logging = Arc::new(AgentRuntimeLoggingAdapter::new(diagnostics, operation_logs));
    let clock = Arc::new(SystemAgentRuntimeClock);
    let availability = Arc::new(RuntimeAgentAvailabilityAdapter::new(dependencies.sdk));
    let repository = Arc::new(SqliteAgentRuntimeRepository::new(
        dependencies.database.clone(),
        availability,
    ));
    let processes = Arc::new(RuntimeAgentProcessAdapter::new(
        logging.clone(),
        clock.clone(),
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
    AgentRuntimeApi::new(
        service,
        terminal_service,
        loops,
        loop_controls,
        loop_recovery,
        loop_scheduler,
    )
}
