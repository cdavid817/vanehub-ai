use super::runtime_manager::{ConnectorDiagnostic, DiagnosticLevel};
use super::session_completion::wait_for_completion;
use super::sqlite_repository::SqliteCommunicationsRepository;
use crate::contexts::agent_runtime::api::{
    AgentAvailability, AgentChatConfiguration, AgentRuntimeApi, InteractionMode, SendMessageRequest,
};
use crate::contexts::communications::application::{
    AgentExecutionRequest, AgentExecutionResult, CommunicationsAgentExecutionPort,
    CommunicationsApplicationError, CommunicationsClockPort, CommunicationsLog,
    CommunicationsLogLevel, CommunicationsLoggingPort, CommunicationsOperation,
    CommunicationsOperationPort, CommunicationsSessionBindingPort,
};
use crate::contexts::communications::domain::{
    ChatBinding, ChatBindingKey, ConnectorKind, RoutingSettings,
};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind, OperationLog, OperationLogPort,
    OperationsApi,
};
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct CommunicationsAgentExecutionAdapter {
    agents: AgentRuntimeApi,
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
}

impl CommunicationsAgentExecutionAdapter {
    pub(crate) fn new(
        agents: AgentRuntimeApi,
        sessions: SessionsApi,
        workspaces: WorkspaceApi,
    ) -> Self {
        Self {
            agents,
            sessions,
            workspaces,
        }
    }
}

impl CommunicationsAgentExecutionPort for CommunicationsAgentExecutionAdapter {
    fn validate_routing(
        &self,
        routing: &RoutingSettings,
    ) -> Result<RoutingSettings, CommunicationsApplicationError> {
        let agent = self
            .agents
            .get_agent(routing.agent_id.trim())
            .map_err(|_| unavailable_agent())?;
        if agent.availability != AgentAvailability::Available {
            return Err(unavailable_agent());
        }
        if !agent
            .supported_interaction_modes
            .contains(&InteractionMode::Cli)
        {
            return Err(CommunicationsApplicationError::user_visible(
                "default-agent-no-cli",
                "The configured default Agent does not support CLI chat.",
            ));
        }
        let inspection = self
            .workspaces
            .inspect_project(routing.project_path.trim())
            .map_err(|_| CommunicationsApplicationError::failure("im-routing-invalid"))?;
        RoutingSettings::new(agent.id, inspection.path()).map_err(Into::into)
    }

    fn execute(
        &self,
        request: AgentExecutionRequest,
    ) -> Result<AgentExecutionResult, CommunicationsApplicationError> {
        let configuration = self
            .sessions
            .load_chat_configuration(&request.session_id)
            .map_err(|_| {
                CommunicationsApplicationError::user_visible(
                    "session-config-invalid",
                    "The session chat configuration could not be loaded.",
                )
            })?;
        if configuration.agent_id != request.routing.agent_id {
            return Err(CommunicationsApplicationError::user_visible(
                "session-config-invalid",
                "The session chat configuration could not be loaded.",
            ));
        }
        let configuration = AgentChatConfiguration {
            agent_id: configuration.agent_id,
            interaction_mode: InteractionMode::parse(&configuration.interaction_mode).map_err(
                |_| {
                    CommunicationsApplicationError::user_visible(
                        "session-config-invalid",
                        "The session chat configuration could not be loaded.",
                    )
                },
            )?,
            permission_mode: configuration.values.permission_mode,
            provider_id: configuration.values.provider_id,
            model_id: configuration.values.model_id,
            reasoning_depth: configuration.values.reasoning_depth,
            streaming: configuration.values.streaming,
            thinking: configuration.values.thinking,
            long_context: configuration.values.long_context,
        };
        let message = self
            .agents
            .send_message(SendMessageRequest {
                source: crate::contexts::agent_runtime::application::AgentMessageSource::InstantMessage {
                    connector_id: "managed-im".to_string(),
                },
                session_id: request.session_id.clone(),
                content: request.text,
                configuration,
                file_references: Vec::new(),
            })
            .map_err(|_| {
                CommunicationsApplicationError::user_visible(
                    "agent-start-failed",
                    "The Agent could not start this request.",
                )
            })?;
        wait_for_completion(&self.sessions, &request.session_id, &message.id)
    }
}

fn unavailable_agent() -> CommunicationsApplicationError {
    CommunicationsApplicationError::user_visible(
        "default-agent-unavailable",
        "The configured default Agent is unavailable.",
    )
}

pub(crate) struct CommunicationsSessionBindingAdapter {
    repository: SqliteCommunicationsRepository,
    sessions: SessionsApi,
    clock: Arc<dyn CommunicationsClockPort>,
    creation_lock: Mutex<()>,
}

impl CommunicationsSessionBindingAdapter {
    pub(crate) fn new(
        repository: SqliteCommunicationsRepository,
        sessions: SessionsApi,
        clock: Arc<dyn CommunicationsClockPort>,
    ) -> Self {
        Self {
            repository,
            sessions,
            clock,
            creation_lock: Mutex::new(()),
        }
    }
}

impl CommunicationsSessionBindingPort for CommunicationsSessionBindingAdapter {
    fn resolve_or_create(
        &self,
        key: &ChatBindingKey,
        routing: &RoutingSettings,
    ) -> Result<String, CommunicationsApplicationError> {
        let _guard = self
            .creation_lock
            .lock()
            .map_err(|_| CommunicationsApplicationError::failure("binding-lock-failed"))?;
        if let Some(session_id) = self.repository.find_binding(key)? {
            return Ok(session_id);
        }
        let owner = SessionOwner::connector(key.connector().as_str()).map_err(|_| {
            CommunicationsApplicationError::user_visible(
                "session-create-failed",
                "VaneHub could not create a session for this chat.",
            )
        })?;
        let session = self
            .sessions
            .prepare_creation(NewSessionRequest {
                agent_id: routing.agent_id.clone(),
                interaction_mode: InteractionMode::Cli.as_str().to_string(),
                title: Some(format!("{} IM", key.connector().as_str())),
                workspace: NewSessionWorkspace {
                    project_path: Some(routing.project_path.clone()),
                    ..Default::default()
                },
                owner,
                activation: SessionActivation::PreserveActive,
            })
            .and_then(|prepared| self.sessions.execute_creation(prepared))
            .map_err(|_| {
                CommunicationsApplicationError::user_visible(
                    "session-create-failed",
                    "VaneHub could not create a session for this chat.",
                )
            })?;
        let session_id = session.id().to_string();
        self.repository.save_binding(
            &ChatBinding::new(key.clone(), session_id.clone())?,
            &self.clock.now_rfc3339(),
        )?;
        Ok(session_id)
    }

    fn reset(&self, kind: Option<ConnectorKind>) -> Result<(), CommunicationsApplicationError> {
        self.repository.reset_bindings(kind).map(|_| ())
    }
}

#[derive(Clone)]
pub(crate) struct CommunicationsOperationAdapter {
    operations: OperationsApi,
}

impl CommunicationsOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl CommunicationsOperationPort for CommunicationsOperationAdapter {
    fn start(
        &self,
        kind: ConnectorKind,
        action: &'static str,
    ) -> Result<CommunicationsOperation, CommunicationsApplicationError> {
        self.operations
            .start(
                OperationKind::Agent,
                Some(kind.as_str().to_string()),
                Some(format!("IM connector {action}")),
            )
            .map(|operation| CommunicationsOperation { id: operation.id })
            .map_err(|_| CommunicationsApplicationError::failure("operation-start-failed"))
    }

    fn complete(&self, operation_id: &str) -> Result<(), CommunicationsApplicationError> {
        self.operations
            .complete(operation_id, None)
            .map(|_| ())
            .map_err(|_| CommunicationsApplicationError::failure("operation-complete-failed"))
    }

    fn fail(
        &self,
        operation_id: &str,
        safe_code: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.operations
            .fail(operation_id, safe_code.to_string())
            .map(|_| ())
            .map_err(|_| CommunicationsApplicationError::failure("operation-fail-failed"))
    }
}

#[derive(Debug, Default)]
pub(crate) struct SystemCommunicationsClock;

impl CommunicationsClockPort for SystemCommunicationsClock {
    fn now_rfc3339(&self) -> String {
        Utc::now().to_rfc3339()
    }

    fn days_ago_rfc3339(&self, days: u32) -> String {
        (Utc::now() - Duration::days(i64::from(days))).to_rfc3339()
    }
}

pub(crate) struct CommunicationsLoggingAdapter {
    diagnostics: Arc<dyn DiagnosticLogPort>,
    operations: Arc<dyn OperationLogPort>,
}

impl CommunicationsLoggingAdapter {
    pub(crate) fn new(
        diagnostics: Arc<dyn DiagnosticLogPort>,
        operations: Arc<dyn OperationLogPort>,
    ) -> Self {
        Self {
            diagnostics,
            operations,
        }
    }

    pub(crate) fn record_runtime(&self, event: ConnectorDiagnostic) {
        let mut context = BTreeMap::new();
        context.insert(
            "connector".to_string(),
            event.connector.as_str().to_string(),
        );
        context.insert("operation".to_string(), event.operation.to_string());
        context.insert("safeCode".to_string(), event.safe_code);
        context.insert("retryCount".to_string(), event.retry_count.to_string());
        if let Some(value) = event.internal_session_id {
            context.insert("internalSessionId".to_string(), value);
        }
        if let Some(value) = event.internal_message_id {
            context.insert("internalMessageId".to_string(), value);
        }
        if let Some(value) = event.platform_status_code {
            context.insert("platformStatusCode".to_string(), value);
        }
        if let Some(value) = event.retry_classification {
            context.insert("retryClassification".to_string(), value);
        }
        let _ = self.diagnostics.write_diagnostic(DiagnosticLog {
            severity: runtime_severity(event.level),
            category: "im.connector".to_string(),
            message: "IM connector lifecycle or delivery event".to_string(),
            context,
        });
    }
}

impl CommunicationsLoggingPort for CommunicationsLoggingAdapter {
    fn record(&self, log: CommunicationsLog) -> Result<(), CommunicationsApplicationError> {
        let mut context = BTreeMap::new();
        if let Some(connector) = log.connector {
            context.insert("connector".to_string(), connector.as_str().to_string());
        }
        if let Some(safe_code) = log.safe_code {
            context.insert("safeCode".to_string(), safe_code);
        }
        context.insert("timestamp".to_string(), log.timestamp);
        let severity = application_severity(log.level);
        match log.operation_id {
            Some(operation_id) => self
                .operations
                .write_operation(OperationLog {
                    operation_id,
                    severity,
                    category: log.event.to_string(),
                    message: log.message,
                    context,
                })
                .map_err(|_| CommunicationsApplicationError::failure("logging-write-failed")),
            None => self
                .diagnostics
                .write_diagnostic(DiagnosticLog {
                    severity,
                    category: log.event.to_string(),
                    message: log.message,
                    context,
                })
                .map_err(|_| CommunicationsApplicationError::failure("logging-write-failed")),
        }
    }
}

fn runtime_severity(level: DiagnosticLevel) -> LogSeverity {
    match level {
        DiagnosticLevel::Debug => LogSeverity::Debug,
        DiagnosticLevel::Info => LogSeverity::Info,
        DiagnosticLevel::Warn => LogSeverity::Warn,
        DiagnosticLevel::Error => LogSeverity::Error,
    }
}

fn application_severity(level: CommunicationsLogLevel) -> LogSeverity {
    match level {
        CommunicationsLogLevel::Info => LogSeverity::Info,
        CommunicationsLogLevel::Error => LogSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::test_support::TempDirectory;

    #[test]
    fn communications_logging_keeps_operation_context_and_redacts_secrets() {
        let directory = TempDirectory::new("communications-logging");
        let unified = Arc::new(UnifiedLoggingAdapter::new(directory.path().to_path_buf()));
        let diagnostics: Arc<dyn DiagnosticLogPort> = unified.clone();
        let operations: Arc<dyn OperationLogPort> = unified;
        let adapter = CommunicationsLoggingAdapter::new(diagnostics, operations);

        adapter
            .record(CommunicationsLog {
                level: CommunicationsLogLevel::Error,
                event: "communications.connector.operation",
                message: "Request failed with Bearer private-connector-token".to_string(),
                connector: Some(ConnectorKind::Telegram),
                safe_code: Some("connector-authentication-failed".to_string()),
                operation_id: Some("operation-17".to_string()),
                timestamp: "2026-07-18T00:00:00Z".to_string(),
            })
            .expect("write communications log");

        let raw = std::fs::read_to_string(
            directory
                .path()
                .join(crate::platform::logging::LOG_FILE_NAME),
        )
        .expect("unified log");
        assert!(raw.contains("operation-17"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("private-connector-token"));
    }
}
