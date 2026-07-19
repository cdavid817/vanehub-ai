use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::communications::api::{CommunicationsApi, WeChatAuthorizationApi};
use crate::contexts::communications::application::{
    CommunicationsApplicationPorts, CommunicationsApplicationService, CommunicationsClockPort,
    CommunicationsLoggingPort,
};
use crate::contexts::communications::infrastructure::{
    BusyMessageProvider, CommunicationsAgentExecutionAdapter, CommunicationsCredentialAdapter,
    CommunicationsInboundBridge, CommunicationsLoggingAdapter, CommunicationsOperationAdapter,
    CommunicationsSessionBindingAdapter, CommunicationsTransportAdapter, ConnectorRuntimeManager,
    SqliteCommunicationsRepository, SystemCommunicationsClock,
};
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::operations::api::{DiagnosticLogPort, OperationLogPort, OperationsApi};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct CommunicationsComposition {
    pub(crate) api: CommunicationsApi,
    pub(crate) wechat_authorization: WeChatAuthorizationApi,
}

pub(crate) struct CommunicationsDependencies {
    pub(crate) database: NativeDatabase,
    pub(crate) operations: OperationsApi,
    pub(crate) agents: AgentRuntimeApi,
    pub(crate) sessions: SessionsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) desktop_settings: DesktopSettingsApi,
    pub(crate) fallback_log_directory: PathBuf,
}

pub(crate) fn assemble_communications(
    dependencies: CommunicationsDependencies,
) -> Result<CommunicationsComposition, String> {
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(
        dependencies.fallback_log_directory,
    ));
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging;
    let logging = Arc::new(CommunicationsLoggingAdapter::new(
        diagnostics,
        operation_logs,
    ));
    let desktop_settings = dependencies.desktop_settings.clone();
    let busy_message: BusyMessageProvider = Arc::new(move || {
        let language = desktop_settings
            .get_settings()
            .map(|view| view.settings.application_language().as_str().to_string())
            .unwrap_or_else(|_| "en".to_string());
        if language.to_ascii_lowercase().starts_with("zh") {
            "待处理消息过多，请稍后重试。".to_string()
        } else {
            "Too many pending messages. Please try again later.".to_string()
        }
    });
    let inbound = Arc::new(CommunicationsInboundBridge::new(
        logging.clone(),
        busy_message,
    ));
    let runtime = ConnectorRuntimeManager::new(inbound.clone());
    let repository = SqliteCommunicationsRepository::new(dependencies.database);
    let clock: Arc<dyn CommunicationsClockPort> = Arc::new(SystemCommunicationsClock);
    let service = CommunicationsApplicationService::new(CommunicationsApplicationPorts {
        repository: Arc::new(repository.clone()),
        credentials: Arc::new(CommunicationsCredentialAdapter::new()),
        transports: Arc::new(CommunicationsTransportAdapter::new(
            runtime,
            repository.clone(),
        )),
        agents: Arc::new(CommunicationsAgentExecutionAdapter::new(
            dependencies.agents,
            dependencies.sessions.clone(),
            dependencies.workspaces,
        )),
        sessions: Arc::new(CommunicationsSessionBindingAdapter::new(
            repository,
            dependencies.sessions,
            clock.clone(),
        )),
        operations: Arc::new(CommunicationsOperationAdapter::new(dependencies.operations)),
        clock,
        logging: logging as Arc<dyn CommunicationsLoggingPort>,
    });
    let api = CommunicationsApi::new(service);
    inbound
        .attach(api.clone())
        .map_err(|error| error.safe_code().to_string())?;
    Ok(CommunicationsComposition {
        wechat_authorization: WeChatAuthorizationApi::new(api.clone()),
        api,
    })
}
