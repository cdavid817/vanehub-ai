use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationPorts, AgentRuntimeApplicationService,
};
use crate::contexts::agent_runtime::infrastructure::{
    AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter, InMemoryGenerationCoordinator,
    RuntimeAgentAvailabilityAdapter, RuntimeAgentCliProfileAdapter, RuntimeAgentProcessAdapter,
    RuntimeEffectivePromptAdapter, SessionsAgentRuntimeAdapter, SqliteAgentRuntimeRepository,
    SystemAgentRuntimeClock, TauriAgentRuntimeEventAdapter,
};
use crate::contexts::operations::api::{DiagnosticLogPort, OperationLogPort, OperationsApi};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::sdk::api::SdkApi;
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
    let service = AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
        registry: repository.clone(),
        workflows: repository,
        sessions: Arc::new(SessionsAgentRuntimeAdapter::new(dependencies.sessions)),
        cli_profiles: Arc::new(RuntimeAgentCliProfileAdapter::new(
            dependencies.cli_parameters,
            dependencies.cli,
        )),
        prompts: Arc::new(RuntimeEffectivePromptAdapter::new(dependencies.prompts)),
        processes,
        operations: Arc::new(AgentRuntimeOperationAdapter::new(dependencies.operations)),
        logging,
        clock,
        events: Arc::new(TauriAgentRuntimeEventAdapter::new(dependencies.app)),
        generations: Arc::new(InMemoryGenerationCoordinator::default()),
    });
    AgentRuntimeApi::new(service)
}
