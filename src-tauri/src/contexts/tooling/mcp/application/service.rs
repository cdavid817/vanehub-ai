use super::{
    catalog_limits, configuration_limits, ConnectionTestResult, ExportBundle, ImportBundle,
    ImportEntry, ImportFailure, ImportFailureStage, ImportResult, ImportTransportType,
    McpApplicationError, McpCancellation, McpClockPort, McpConnectionPort, McpExecutionControl,
    McpLimits, McpLoggingPort, McpOperationPort, McpProjectPathPort, McpServerRepository,
    McpServerToolEntry, McpTelemetryPort, PreparedConnectionTest, ServerPatch,
};
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, McpFailureCode, Scope, ServerConfiguration, ServerConfigurationDraft,
    ServerName, ServerStatus, ToolCallOutcome, TransportType,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

const MCP_OPERATION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(crate) struct McpApplicationService {
    repository: Arc<dyn McpServerRepository>,
    connection: Arc<dyn McpConnectionPort>,
    operations: Arc<dyn McpOperationPort>,
    clock: Arc<dyn McpClockPort>,
    logging: Arc<dyn McpLoggingPort>,
    project_path: Arc<dyn McpProjectPathPort>,
    telemetry: Arc<dyn McpTelemetryPort>,
}

impl McpApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn McpServerRepository>,
        connection: Arc<dyn McpConnectionPort>,
        operations: Arc<dyn McpOperationPort>,
        clock: Arc<dyn McpClockPort>,
        logging: Arc<dyn McpLoggingPort>,
        project_path: Arc<dyn McpProjectPathPort>,
        telemetry: Arc<dyn McpTelemetryPort>,
    ) -> Self {
        Self {
            repository,
            connection,
            operations,
            clock,
            logging,
            project_path,
            telemetry,
        }
    }

    pub(crate) fn list_servers(&self) -> Result<Vec<ServerConfiguration>, McpApplicationError> {
        self.repository
            .list_visible(&self.project_path.current_project_path()?)
    }

    pub(crate) fn add_server(
        &self,
        mut draft: ServerConfigurationDraft,
    ) -> Result<(), McpApplicationError> {
        let name = ServerName::parse(draft.name.clone())?;
        if self.repository.exists(&name)? {
            return Err(duplicate_name_error(name.as_str()));
        }
        self.bind_project_scope(&mut draft)?;
        configuration_limits::validate_draft(&draft)?;
        let server = ServerConfiguration::create(draft)?;
        self.repository.insert(&server, &self.clock.now())
    }

    pub(crate) fn update_server(
        &self,
        original_name: &str,
        patch: ServerPatch,
    ) -> Result<(), McpApplicationError> {
        let current = self.load_server(original_name)?;
        let next_name = patch
            .name
            .unwrap_or_else(|| current.name().as_str().to_string());
        let parsed_name = ServerName::parse(next_name.clone())?;
        if parsed_name.as_str() != original_name && self.repository.exists(&parsed_name)? {
            return Err(duplicate_name_error(parsed_name.as_str()));
        }
        let mut draft = ServerConfigurationDraft {
            name: next_name,
            transport_type: patch.transport_type.unwrap_or(current.transport_type()),
            command: patch
                .command
                .unwrap_or_else(|| current.command().map(str::to_string)),
            args: patch
                .args
                .unwrap_or_else(|| current.args().map(<[String]>::to_vec)),
            env: patch.env.unwrap_or_else(|| current.env().cloned()),
            url: patch
                .url
                .unwrap_or_else(|| current.url().map(str::to_string)),
            headers: patch.headers.unwrap_or_else(|| current.headers().cloned()),
            description: patch
                .description
                .unwrap_or_else(|| current.description().map(str::to_string)),
            active: patch.active.unwrap_or(current.is_active()),
            scope: patch.scope.unwrap_or(current.scope()),
            project_path: current.project_path().map(str::to_string),
        };
        self.bind_project_scope(&mut draft)?;
        configuration_limits::validate_draft(&draft)?;
        let server = ServerConfiguration::create(draft)?;
        self.repository
            .replace(original_name, &server, &self.clock.now())
    }

    pub(crate) fn remove_server(&self, name: &str) -> Result<(), McpApplicationError> {
        self.repository.remove(name)
    }

    pub(crate) fn toggle_server(
        &self,
        name: &str,
        active: bool,
    ) -> Result<(), McpApplicationError> {
        self.repository.set_active(name, active, &self.clock.now())
    }

    pub(crate) fn server_status(&self, name: &str) -> Result<ServerStatus, McpApplicationError> {
        self.repository.status(name)
    }

    /// Every tool exposed by every MCP server visible and active for `project_path`, sourced
    /// from each server's last cached "Test Connection" result — never a fresh connection.
    /// `project_path` is caller-supplied rather than read from `self.project_path` because
    /// callers outside the MCP settings UI (a native API agent's own session folder) don't share
    /// this service's ambient current-project-path port.
    pub(crate) fn visible_tool_catalog(
        &self,
        project_path: &str,
    ) -> Result<Vec<McpServerToolEntry>, McpApplicationError> {
        let mut entries = Vec::new();
        for server in self
            .repository
            .list_visible(project_path)?
            .into_iter()
            .filter(ServerConfiguration::is_active)
        {
            let server_name = server.name().as_str().to_string();
            let status = match self.repository.status(&server_name) {
                Ok(status) => status,
                Err(McpApplicationError::Validation(_)) => {
                    let _ = self
                        .logging
                        .record_catalog_rejection(&server_name, McpFailureCode::Validation);
                    continue;
                }
                Err(McpApplicationError::LimitExceeded) => {
                    let _ = self
                        .logging
                        .record_catalog_rejection(&server_name, McpFailureCode::LimitExceeded);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if let Err(error) = catalog_limits::validate_catalog(&status.tools) {
                let _ = self
                    .logging
                    .record_catalog_rejection(&server_name, error.code());
                continue;
            }
            entries.extend(status.tools.into_iter().map(|tool| McpServerToolEntry {
                server_name: server_name.clone(),
                tool,
            }));
        }
        entries.sort_by(|left, right| {
            (&left.server_name, &left.tool.name).cmp(&(&right.server_name, &right.tool.name))
        });
        let maximum = McpLimits::DEFAULT.provider_tools;
        let omitted = entries.len().saturating_sub(maximum);
        entries.truncate(maximum);
        if omitted > 0 {
            let _ = self.logging.record_catalog_overflow(omitted, maximum);
        }
        Ok(entries)
    }

    /// Invokes `tool_name` on `server_name`, re-deriving the visible+active server set for
    /// `project_path` the same way `visible_tool_catalog` does rather than trusting the caller's
    /// `server_name` outright — a server that isn't currently visible and active is rejected
    /// before any connection is attempted.
    pub(crate) async fn call_tool_with_cancellation(
        &self,
        project_path: &str,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
        cancellation: Arc<AtomicBool>,
    ) -> Result<ToolCallOutcome, McpApplicationError> {
        self.call_tool_with_control(
            project_path,
            server_name,
            tool_name,
            arguments,
            McpExecutionControl::with_timeout_and_cancellation(
                MCP_OPERATION_TIMEOUT,
                McpCancellation::from_shared(cancellation),
            ),
        )
        .await
    }

    async fn call_tool_with_control(
        &self,
        project_path: &str,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
        control: McpExecutionControl,
    ) -> Result<ToolCallOutcome, McpApplicationError> {
        let server = self
            .repository
            .list_visible(project_path)?
            .into_iter()
            .filter(ServerConfiguration::is_active)
            .find(|server| server.name().as_str() == server_name)
            .ok_or_else(|| McpApplicationError::ServerNotFound(server_name.to_string()))?;
        configuration_limits::validate_server(&server)?;
        Ok(self
            .connection
            .call_tool(&server, tool_name, arguments, &control)
            .await)
    }

    pub(crate) fn import_servers(
        &self,
        data: ImportBundle,
        scope: Scope,
    ) -> Result<ImportResult, McpApplicationError> {
        if data.servers.len() > McpLimits::DEFAULT.import_server_entries {
            return Err(McpApplicationError::LimitExceeded);
        }
        let mut result = ImportResult::default();
        for (name, entry) in data.servers {
            let parsed_name = match ServerName::parse(name.clone()) {
                Ok(name) => name,
                Err(_) => {
                    result
                        .failures
                        .push(import_validation_failure(name, McpFailureCode::Validation));
                    continue;
                }
            };
            match self.repository.exists(&parsed_name) {
                Ok(true) => {
                    result.skipped.push(name);
                    continue;
                }
                Ok(false) => {}
                Err(_) => {
                    result.failures.push(import_storage_failure(name));
                    continue;
                }
            }
            let transport_type = if entry
                .command
                .as_deref()
                .is_none_or(|command| command.trim().is_empty())
            {
                match entry.transport_type {
                    Some(ImportTransportType::Sse) => TransportType::Sse,
                    Some(ImportTransportType::Http | ImportTransportType::StreamableHttp)
                    | None => TransportType::StreamableHttp,
                }
            } else {
                TransportType::Stdio
            };
            let (command, args, env, url, headers) = match transport_type {
                TransportType::Stdio => (entry.command, entry.args, entry.env, None, None),
                TransportType::Sse | TransportType::StreamableHttp => {
                    (None, None, None, entry.url, entry.headers)
                }
            };
            let mut draft = ServerConfigurationDraft {
                name: name.clone(),
                transport_type,
                command,
                args,
                env,
                url,
                headers,
                description: None,
                active: true,
                scope,
                project_path: None,
            };
            let imported = self
                .bind_project_scope(&mut draft)
                .and_then(|()| configuration_limits::validate_draft(&draft))
                .and_then(|()| ServerConfiguration::create(draft).map_err(Into::into))
                .and_then(|server| self.repository.insert(&server, &self.clock.now()));
            match imported {
                Ok(()) => result.imported.push(name),
                Err(McpApplicationError::LimitExceeded) => result.failures.push(
                    import_validation_failure(name, McpFailureCode::LimitExceeded),
                ),
                Err(McpApplicationError::Domain(_) | McpApplicationError::Validation(_)) => result
                    .failures
                    .push(import_validation_failure(name, McpFailureCode::Validation)),
                Err(_) => result.failures.push(import_storage_failure(name)),
            }
        }
        Ok(result)
    }

    pub(crate) fn export_servers(
        &self,
        names: Vec<String>,
    ) -> Result<ExportBundle, McpApplicationError> {
        let mut servers = BTreeMap::new();
        for name in names {
            let server = self.load_server(&name)?;
            let entry = match server.transport_type() {
                TransportType::Stdio => ImportEntry {
                    command: server.command().map(str::to_string),
                    args: server.args().map(<[String]>::to_vec),
                    env: server.env().cloned(),
                    ..Default::default()
                },
                TransportType::Sse => ImportEntry {
                    transport_type: Some(ImportTransportType::Sse),
                    url: server.url().map(str::to_string),
                    headers: server.headers().cloned(),
                    ..Default::default()
                },
                TransportType::StreamableHttp => ImportEntry {
                    transport_type: Some(ImportTransportType::Http),
                    url: server.url().map(str::to_string),
                    headers: server.headers().cloned(),
                    ..Default::default()
                },
            };
            servers.insert(server.name().as_str().to_string(), entry);
        }
        Ok(ExportBundle { servers })
    }

    pub(crate) fn prepare_connection_test(
        &self,
        name: &str,
    ) -> Result<PreparedConnectionTest, McpApplicationError> {
        let server = self.load_server(name)?;
        configuration_limits::validate_server(&server)?;
        let operation = self.operations.start_connection_test(name)?;
        let cancellation = self
            .operations
            .connection_test_cancellation(&operation.id)?;
        let observation_id = self
            .telemetry
            .start_connection_test(
                &operation.id,
                server.name().as_str(),
                server.transport_type(),
                &self.clock.now(),
            )
            .ok();
        Ok(PreparedConnectionTest {
            operation,
            server,
            observation_id,
            cancellation,
        })
    }

    pub(crate) async fn execute_connection_test(
        &self,
        prepared: PreparedConnectionTest,
    ) -> Result<(), McpApplicationError> {
        let operation_id = prepared.operation.id.clone();
        let server_name = prepared.server.name().as_str().to_string();
        let control = McpExecutionControl::with_timeout_and_cancellation(
            MCP_OPERATION_TIMEOUT,
            prepared.cancellation,
        );
        let outcome = self
            .connection
            .test(&prepared.server, &control, Some(&operation_id))
            .await;
        let outcome = catalog_limits::enforce_outcome(outcome);
        if let Some(observation_id) = &prepared.observation_id {
            let _ =
                self.telemetry
                    .finish_connection_test(observation_id, &outcome, &self.clock.now());
        }
        let result = ConnectionTestResult::from_outcome(operation_id.clone(), &outcome);
        let persistence = self
            .repository
            .record_connection_outcome(&server_name, &outcome, &self.clock.now())
            .and_then(|()| {
                self.logging
                    .record_connection_outcome(&operation_id, &server_name, &outcome)
            });
        if persistence.is_err() {
            let _ = self.operations.append_log(
                &operation_id,
                "MCP connection result persistence failed.".to_string(),
            );
        }
        self.finish_connection_operation(&operation_id, &server_name, &outcome, &result)
    }

    fn finish_connection_operation(
        &self,
        operation_id: &str,
        server_name: &str,
        outcome: &ConnectionOutcome,
        result: &ConnectionTestResult,
    ) -> Result<(), McpApplicationError> {
        match outcome {
            ConnectionOutcome::Connected { .. } => {
                self.operations
                    .append_log(operation_id, format!("MCP test passed for {server_name}"))?;
                self.operations
                    .complete_connection_test(operation_id, result)
            }
            ConnectionOutcome::Failed {
                error,
                error_code: Some(McpFailureCode::Cancelled),
                ..
            } => self.operations.append_log(
                operation_id,
                format!("MCP test cancelled for {server_name} after cleanup: {error}"),
            ),
            ConnectionOutcome::Failed { error, .. } => {
                self.operations.append_log(operation_id, error.clone())?;
                self.operations
                    .fail_connection_test(operation_id, error.clone())
            }
        }
    }

    fn load_server(&self, name: &str) -> Result<ServerConfiguration, McpApplicationError> {
        self.repository
            .find(name)?
            .ok_or_else(|| McpApplicationError::ServerNotFound(name.to_string()))
    }

    fn bind_project_scope(
        &self,
        draft: &mut ServerConfigurationDraft,
    ) -> Result<(), McpApplicationError> {
        draft.project_path = match draft.scope {
            Scope::User => None,
            Scope::Project => Some(self.project_path.current_project_path()?),
        };
        Ok(())
    }
}

fn duplicate_name_error(name: &str) -> McpApplicationError {
    McpApplicationError::Validation(format!("MCP server name already exists: {name}"))
}

fn import_validation_failure(name: String, code: McpFailureCode) -> ImportFailure {
    ImportFailure {
        name,
        stage: ImportFailureStage::Validation,
        error_code: Some(code),
        message: code.safe_message().to_string(),
    }
}

fn import_storage_failure(name: String) -> ImportFailure {
    ImportFailure {
        name,
        stage: ImportFailureStage::Storage,
        error_code: None,
        message: "The MCP server could not be saved.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::application::{McpLimits, StartedOperation};
    use crate::contexts::tooling::mcp::domain::{ConnectionStatus, ServerStatus, ToolDescriptor};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRepository {
        servers: Mutex<BTreeMap<String, ServerConfiguration>>,
        writes: Mutex<Vec<String>>,
        outcome: Mutex<Option<ConnectionOutcome>>,
        outcome_error: Mutex<Option<McpApplicationError>>,
        statuses: Mutex<BTreeMap<String, ServerStatus>>,
        status_errors: Mutex<BTreeMap<String, McpApplicationError>>,
        insert_errors: Mutex<BTreeMap<String, McpApplicationError>>,
    }

    impl McpServerRepository for FakeRepository {
        fn list_visible(
            &self,
            _current_project_path: &str,
        ) -> Result<Vec<ServerConfiguration>, McpApplicationError> {
            Ok(self
                .servers
                .lock()
                .expect("servers")
                .values()
                .cloned()
                .collect())
        }

        fn find(&self, name: &str) -> Result<Option<ServerConfiguration>, McpApplicationError> {
            Ok(self.servers.lock().expect("servers").get(name).cloned())
        }

        fn exists(&self, name: &ServerName) -> Result<bool, McpApplicationError> {
            Ok(self
                .servers
                .lock()
                .expect("servers")
                .contains_key(name.as_str()))
        }

        fn insert(
            &self,
            server: &ServerConfiguration,
            timestamp: &str,
        ) -> Result<(), McpApplicationError> {
            if let Some(error) = self
                .insert_errors
                .lock()
                .expect("insert_errors")
                .get(server.name().as_str())
            {
                return Err(error.clone());
            }
            self.writes.lock().expect("writes").push(timestamp.into());
            self.servers
                .lock()
                .expect("servers")
                .insert(server.name().as_str().to_string(), server.clone());
            Ok(())
        }

        fn replace(
            &self,
            original_name: &str,
            server: &ServerConfiguration,
            timestamp: &str,
        ) -> Result<(), McpApplicationError> {
            let mut servers = self.servers.lock().expect("servers");
            servers.remove(original_name);
            servers.insert(server.name().as_str().to_string(), server.clone());
            self.writes.lock().expect("writes").push(timestamp.into());
            Ok(())
        }

        fn remove(&self, name: &str) -> Result<(), McpApplicationError> {
            self.servers.lock().expect("servers").remove(name);
            Ok(())
        }

        fn set_active(
            &self,
            _name: &str,
            _active: bool,
            timestamp: &str,
        ) -> Result<(), McpApplicationError> {
            self.writes.lock().expect("writes").push(timestamp.into());
            Ok(())
        }

        fn status(&self, name: &str) -> Result<ServerStatus, McpApplicationError> {
            if let Some(error) = self.status_errors.lock().expect("status_errors").get(name) {
                return Err(error.clone());
            }
            if let Some(status) = self.statuses.lock().expect("statuses").get(name) {
                return Ok(status.clone());
            }
            Ok(ServerStatus {
                name: ServerName::parse(name.to_string())?,
                connection_status: ConnectionStatus::Disconnected,
                tools: Vec::new(),
                last_connected: None,
                error: None,
                error_code: None,
                duration_ms: None,
            })
        }

        fn record_connection_outcome(
            &self,
            _name: &str,
            outcome: &ConnectionOutcome,
            timestamp: &str,
        ) -> Result<(), McpApplicationError> {
            if let Some(error) = self.outcome_error.lock().expect("outcome_error").clone() {
                return Err(error);
            }
            *self.outcome.lock().expect("outcome") = Some(outcome.clone());
            self.writes.lock().expect("writes").push(timestamp.into());
            Ok(())
        }
    }

    struct FakeConnection {
        outcome: ConnectionOutcome,
        tool_call_outcome: ToolCallOutcome,
        tool_calls: Mutex<Vec<(String, String)>>,
        test_calls: std::sync::atomic::AtomicUsize,
    }

    impl FakeConnection {
        fn new(outcome: ConnectionOutcome) -> Self {
            Self {
                outcome,
                tool_call_outcome: ToolCallOutcome::success(""),
                tool_calls: Mutex::new(Vec::new()),
                test_calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl McpConnectionPort for FakeConnection {
        async fn test(
            &self,
            _server: &ServerConfiguration,
            _control: &McpExecutionControl,
            _operation_id: Option<&str>,
        ) -> ConnectionOutcome {
            self.test_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.outcome.clone()
        }

        async fn call_tool(
            &self,
            server: &ServerConfiguration,
            tool_name: &str,
            _arguments: Value,
            _control: &McpExecutionControl,
        ) -> ToolCallOutcome {
            self.tool_calls
                .lock()
                .expect("tool_calls")
                .push((server.name().as_str().to_string(), tool_name.to_string()));
            self.tool_call_outcome.clone()
        }
    }

    #[derive(Default)]
    struct CancellationConnection {
        cleanup_finished: std::sync::atomic::AtomicBool,
        tool_cleanup_finished: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl McpConnectionPort for CancellationConnection {
        async fn test(
            &self,
            _server: &ServerConfiguration,
            control: &McpExecutionControl,
            _operation_id: Option<&str>,
        ) -> ConnectionOutcome {
            while !control.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.cleanup_finished
                .store(true, std::sync::atomic::Ordering::SeqCst);
            ConnectionOutcome::failed_with_code(
                McpFailureCode::Cancelled.safe_message(),
                McpFailureCode::Cancelled,
                1,
            )
        }

        async fn call_tool(
            &self,
            _server: &ServerConfiguration,
            _tool_name: &str,
            _arguments: Value,
            control: &McpExecutionControl,
        ) -> ToolCallOutcome {
            while !control.is_cancelled() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.tool_cleanup_finished
                .store(true, std::sync::atomic::Ordering::SeqCst);
            ToolCallOutcome::failed_with_code(
                McpFailureCode::Cancelled.safe_message(),
                McpFailureCode::Cancelled,
            )
        }
    }

    #[derive(Default)]
    struct FakeOperations {
        events: Mutex<Vec<String>>,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
        starts: std::sync::atomic::AtomicUsize,
    }

    impl McpOperationPort for FakeOperations {
        fn start_connection_test(
            &self,
            server_name: &str,
        ) -> Result<StartedOperation, McpApplicationError> {
            self.starts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(StartedOperation {
                id: "op-fixed".to_string(),
                related_entity_id: Some(server_name.to_string()),
                message: Some(format!("Testing MCP server {server_name}")),
                created_at: "100".to_string(),
                updated_at: "100".to_string(),
            })
        }

        fn append_log(&self, _operation_id: &str, line: String) -> Result<(), McpApplicationError> {
            self.events
                .lock()
                .expect("events")
                .push(format!("log:{line}"));
            Ok(())
        }

        fn connection_test_cancellation(
            &self,
            _operation_id: &str,
        ) -> Result<McpCancellation, McpApplicationError> {
            Ok(McpCancellation::from_shared(self.cancelled.clone()))
        }

        fn complete_connection_test(
            &self,
            _operation_id: &str,
            result: &ConnectionTestResult,
        ) -> Result<(), McpApplicationError> {
            self.events
                .lock()
                .expect("events")
                .push(format!("complete:{}", result.success));
            Ok(())
        }

        fn fail_connection_test(
            &self,
            _operation_id: &str,
            error: String,
        ) -> Result<(), McpApplicationError> {
            self.events
                .lock()
                .expect("events")
                .push(format!("fail:{error}"));
            Ok(())
        }
    }

    struct FakeClock;

    impl McpClockPort for FakeClock {
        fn now(&self) -> String {
            "1700000000".to_string()
        }
    }

    #[derive(Default)]
    struct FakeLogging {
        entries: Mutex<Vec<String>>,
        error_codes: Mutex<Vec<Option<McpFailureCode>>>,
        connection_error: Mutex<Option<McpApplicationError>>,
        catalog_rejections: Mutex<Vec<(String, McpFailureCode)>>,
        catalog_overflows: Mutex<Vec<(usize, usize)>>,
    }

    impl McpLoggingPort for FakeLogging {
        fn record_connection_outcome(
            &self,
            operation_id: &str,
            server_name: &str,
            outcome: &ConnectionOutcome,
        ) -> Result<(), McpApplicationError> {
            if let Some(error) = self
                .connection_error
                .lock()
                .expect("connection_error")
                .clone()
            {
                return Err(error);
            }
            self.error_codes
                .lock()
                .expect("error_codes")
                .push(outcome.error_code());
            self.entries.lock().expect("entries").push(format!(
                "{operation_id}:{server_name}:{}",
                outcome.is_success()
            ));
            Ok(())
        }

        fn record_catalog_rejection(
            &self,
            server_name: &str,
            error_code: McpFailureCode,
        ) -> Result<(), McpApplicationError> {
            self.catalog_rejections
                .lock()
                .expect("catalog_rejections")
                .push((server_name.to_string(), error_code));
            Ok(())
        }

        fn record_catalog_overflow(
            &self,
            omitted_tools: usize,
            maximum_tools: usize,
        ) -> Result<(), McpApplicationError> {
            self.catalog_overflows
                .lock()
                .expect("catalog_overflows")
                .push((omitted_tools, maximum_tools));
            Ok(())
        }
    }

    struct FakeProjectPath;

    impl McpProjectPathPort for FakeProjectPath {
        fn current_project_path(&self) -> Result<String, McpApplicationError> {
            Ok("D:\\code\\fixture".to_string())
        }
    }

    struct FakeTelemetry;

    impl McpTelemetryPort for FakeTelemetry {
        fn start_connection_test(
            &self,
            operation_id: &str,
            _server_name: &str,
            _transport: TransportType,
            _started_at: &str,
        ) -> Result<String, McpApplicationError> {
            Ok(format!("observation-{operation_id}"))
        }

        fn finish_connection_test(
            &self,
            _observation_id: &str,
            _outcome: &ConnectionOutcome,
            _ended_at: &str,
        ) -> Result<(), McpApplicationError> {
            Ok(())
        }
    }

    fn server_draft(scope: Scope) -> ServerConfigurationDraft {
        ServerConfigurationDraft {
            name: "fixture-tools".to_string(),
            transport_type: TransportType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: None,
            headers: None,
            description: None,
            active: true,
            scope,
            project_path: None,
        }
    }

    fn server_draft_named(name: &str, active: bool) -> ServerConfigurationDraft {
        ServerConfigurationDraft {
            name: name.to_string(),
            active,
            ..server_draft(Scope::User)
        }
    }

    fn oversized_configuration_draft() -> ServerConfigurationDraft {
        let mut draft = server_draft(Scope::User);
        draft.env = Some(BTreeMap::from([(
            "OVERSIZED".to_string(),
            "x".repeat(McpLimits::DEFAULT.configuration_serialized_bytes),
        )]));
        draft
    }

    fn service(
        repository: Arc<FakeRepository>,
        operations: Arc<FakeOperations>,
        logging: Arc<FakeLogging>,
        outcome: ConnectionOutcome,
    ) -> McpApplicationService {
        McpApplicationService::new(
            repository,
            Arc::new(FakeConnection::new(outcome)),
            operations,
            Arc::new(FakeClock),
            logging,
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        )
    }

    #[test]
    fn management_use_case_binds_project_scope_and_uses_injected_clock() {
        let repository = Arc::new(FakeRepository::default());
        let service = service(
            repository.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );

        service
            .add_server(server_draft(Scope::Project))
            .expect("add server");

        let server = repository
            .find("fixture-tools")
            .expect("find")
            .expect("server");
        assert_eq!(server.project_path(), Some("D:\\code\\fixture"));
        assert_eq!(
            repository.writes.lock().expect("writes").as_slice(),
            ["1700000000"]
        );
    }

    #[test]
    fn oversized_configuration_is_rejected_before_persistence() {
        let repository = Arc::new(FakeRepository::default());
        let service = service(
            repository.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );

        let error = service
            .add_server(oversized_configuration_draft())
            .expect_err("configuration limit");

        assert_eq!(error, McpApplicationError::LimitExceeded);
        assert!(repository.servers.lock().expect("servers").is_empty());
        assert!(repository.writes.lock().expect("writes").is_empty());
    }

    #[tokio::test]
    async fn oversized_legacy_configuration_is_rejected_before_connection_or_operation_start() {
        let repository = Arc::new(FakeRepository::default());
        let server = ServerConfiguration::create(oversized_configuration_draft())
            .expect("legacy oversized server");
        repository
            .servers
            .lock()
            .expect("servers")
            .insert("fixture-tools".to_string(), server);
        let connection = Arc::new(FakeConnection::new(ConnectionOutcome::failed("unused", 0)));
        let operations = Arc::new(FakeOperations::default());
        let service = McpApplicationService::new(
            repository,
            connection.clone(),
            operations.clone(),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );

        assert!(matches!(
            service.prepare_connection_test("fixture-tools"),
            Err(McpApplicationError::LimitExceeded)
        ));
        assert!(matches!(
            service
                .call_tool_with_cancellation(
                    "D:\\code\\fixture",
                    "fixture-tools",
                    "search",
                    serde_json::json!({}),
                    Arc::new(std::sync::atomic::AtomicBool::new(false)),
                )
                .await,
            Err(McpApplicationError::LimitExceeded)
        ));
        assert_eq!(
            operations.starts.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(
            connection
                .test_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert!(connection.tool_calls.lock().expect("tool calls").is_empty());
    }

    #[test]
    fn native_import_preserves_explicit_and_historical_transport_semantics() {
        let repository = Arc::new(FakeRepository::default());
        let service = service(
            repository.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        let mut servers = BTreeMap::new();
        servers.insert(
            "command-server".to_string(),
            ImportEntry {
                transport_type: Some(ImportTransportType::Sse),
                command: Some("node".to_string()),
                args: Some(vec!["server.js".to_string()]),
                url: Some("https://ignored.example/mcp".to_string()),
                ..Default::default()
            },
        );
        for (name, transport_type) in [
            ("legacy-sse", Some(ImportTransportType::Sse)),
            ("explicit-http", Some(ImportTransportType::Http)),
            (
                "explicit-streamable",
                Some(ImportTransportType::StreamableHttp),
            ),
            ("historical-untyped", None),
        ] {
            servers.insert(
                name.to_string(),
                ImportEntry {
                    transport_type,
                    url: Some(format!("https://{name}.example/mcp")),
                    ..Default::default()
                },
            );
        }

        let result = service
            .import_servers(ImportBundle { servers }, Scope::User)
            .expect("import servers");

        assert_eq!(result.imported.len(), 5);
        assert!(result.skipped.is_empty());
        let command = repository
            .find("command-server")
            .expect("find")
            .expect("server");
        assert_eq!(command.transport_type(), TransportType::Stdio);
        assert_eq!(command.command(), Some("node"));
        assert_eq!(command.url(), None);
        assert_eq!(
            repository
                .find("legacy-sse")
                .expect("find")
                .expect("server")
                .transport_type(),
            TransportType::Sse
        );
        for name in ["explicit-http", "explicit-streamable", "historical-untyped"] {
            assert_eq!(
                repository
                    .find(name)
                    .expect("find")
                    .expect("server")
                    .transport_type(),
                TransportType::StreamableHttp
            );
        }
    }

    #[test]
    fn native_import_skips_an_existing_name_conflict() {
        let repository = Arc::new(FakeRepository::default());
        let service = service(
            repository.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("existing server");
        let original = repository
            .find("fixture-tools")
            .expect("find")
            .expect("server");
        let result = service
            .import_servers(
                ImportBundle {
                    servers: BTreeMap::from([(
                        "fixture-tools".to_string(),
                        ImportEntry {
                            url: Some("https://replacement.example/mcp".to_string()),
                            ..Default::default()
                        },
                    )]),
                },
                Scope::User,
            )
            .expect("import conflict");

        assert!(result.imported.is_empty());
        assert_eq!(result.skipped, vec!["fixture-tools"]);
        assert_eq!(
            repository.find("fixture-tools").expect("find"),
            Some(original)
        );
    }

    #[test]
    fn native_import_rejects_server_count_limit_plus_one() {
        let repository = Arc::new(FakeRepository::default());
        let service = service(
            repository,
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        let servers = (0..=McpLimits::DEFAULT.import_server_entries)
            .map(|index| {
                (
                    format!("server-{index}"),
                    ImportEntry {
                        command: Some("node".to_string()),
                        ..Default::default()
                    },
                )
            })
            .collect();

        assert_eq!(
            service.import_servers(ImportBundle { servers }, Scope::User),
            Err(McpApplicationError::LimitExceeded)
        );
    }

    #[test]
    fn native_import_reports_validation_and_storage_failures_per_entry() {
        let repository = Arc::new(FakeRepository::default());
        repository
            .insert_errors
            .lock()
            .expect("insert_errors")
            .insert(
                "storage-failure".to_string(),
                McpApplicationError::Storage("private database detail".to_string()),
            );
        let service = service(
            repository,
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        let entry = || ImportEntry {
            command: Some("node".to_string()),
            ..Default::default()
        };

        let result = service
            .import_servers(
                ImportBundle {
                    servers: BTreeMap::from([
                        ("Bad_Name".to_string(), entry()),
                        ("storage-failure".to_string(), entry()),
                        ("valid-server".to_string(), entry()),
                    ]),
                },
                Scope::User,
            )
            .expect("partial import");

        assert_eq!(result.imported, vec!["valid-server"]);
        assert!(result.skipped.is_empty());
        assert_eq!(result.failures.len(), 2);
        let validation = result
            .failures
            .iter()
            .find(|failure| failure.name == "Bad_Name")
            .expect("validation failure");
        assert_eq!(validation.stage, ImportFailureStage::Validation);
        assert_eq!(validation.error_code, Some(McpFailureCode::Validation));
        let storage = result
            .failures
            .iter()
            .find(|failure| failure.name == "storage-failure")
            .expect("storage failure");
        assert_eq!(storage.stage, ImportFailureStage::Storage);
        assert_eq!(storage.error_code, None);
        assert_eq!(storage.message, "The MCP server could not be saved.");
        assert!(!storage.message.contains("private database detail"));
    }

    #[test]
    fn native_export_round_trips_stdio_sse_and_streamable_http_unambiguously() {
        let source_repository = Arc::new(FakeRepository::default());
        let source = service(
            source_repository,
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        source
            .add_server(ServerConfigurationDraft {
                name: "round-trip-stdio".to_string(),
                ..server_draft(Scope::User)
            })
            .expect("stdio source");
        for (name, transport_type) in [
            ("round-trip-sse", TransportType::Sse),
            ("round-trip-http", TransportType::StreamableHttp),
        ] {
            source
                .add_server(ServerConfigurationDraft {
                    name: name.to_string(),
                    transport_type,
                    command: None,
                    args: None,
                    env: None,
                    url: Some(format!("https://{name}.example/mcp")),
                    headers: Some(BTreeMap::from([(
                        "Authorization".to_string(),
                        "Bearer secret".to_string(),
                    )])),
                    description: Some("not compatible export metadata".to_string()),
                    active: false,
                    scope: Scope::User,
                    project_path: None,
                })
                .expect("URL source");
        }

        let exported = source
            .export_servers(vec![
                "round-trip-stdio".to_string(),
                "round-trip-sse".to_string(),
                "round-trip-http".to_string(),
            ])
            .expect("export servers");

        let stdio = &exported.servers["round-trip-stdio"];
        assert_eq!(stdio.transport_type, None);
        assert_eq!(stdio.command.as_deref(), Some("node"));
        assert_eq!(stdio.url, None);
        let sse = &exported.servers["round-trip-sse"];
        assert_eq!(sse.transport_type, Some(ImportTransportType::Sse));
        assert_eq!(sse.command, None);
        assert!(sse.url.is_some());
        let http = &exported.servers["round-trip-http"];
        assert_eq!(http.transport_type, Some(ImportTransportType::Http));
        assert_eq!(http.command, None);
        assert!(http.url.is_some());

        let destination_repository = Arc::new(FakeRepository::default());
        let destination = service(
            destination_repository.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("unused", 0),
        );
        let result = destination
            .import_servers(exported, Scope::User)
            .expect("round-trip import");

        assert_eq!(result.imported.len(), 3);
        assert_eq!(
            destination_repository
                .find("round-trip-stdio")
                .expect("find")
                .expect("server")
                .transport_type(),
            TransportType::Stdio
        );
        assert_eq!(
            destination_repository
                .find("round-trip-sse")
                .expect("find")
                .expect("server")
                .transport_type(),
            TransportType::Sse
        );
        assert_eq!(
            destination_repository
                .find("round-trip-http")
                .expect("find")
                .expect("server")
                .transport_type(),
            TransportType::StreamableHttp
        );
    }

    #[tokio::test]
    async fn connection_use_case_coordinates_repository_log_and_successful_operation() {
        let repository = Arc::new(FakeRepository::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let outcome = ConnectionOutcome::connected(
            vec![ToolDescriptor {
                name: "search".to_string(),
                description: None,
                input_schema: None,
            }],
            17,
        );
        let service = service(
            repository.clone(),
            operations.clone(),
            logging.clone(),
            outcome.clone(),
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("add server");

        let prepared = service
            .prepare_connection_test("fixture-tools")
            .expect("prepare test");
        assert_eq!(prepared.operation.id, "op-fixed");
        service
            .execute_connection_test(prepared)
            .await
            .expect("execute test");

        assert_eq!(
            repository.outcome.lock().expect("outcome").as_ref(),
            Some(&outcome)
        );
        assert_eq!(
            logging.entries.lock().expect("entries").as_slice(),
            ["op-fixed:fixture-tools:true"]
        );
        assert_eq!(
            operations.events.lock().expect("events").as_slice(),
            ["log:MCP test passed for fixture-tools", "complete:true"]
        );
    }

    #[tokio::test]
    async fn persistence_and_logging_failures_leave_only_a_safe_operation_diagnostic() {
        for boundary in ["repository", "logging"] {
            let repository = Arc::new(FakeRepository::default());
            let operations = Arc::new(FakeOperations::default());
            let logging = Arc::new(FakeLogging::default());
            if boundary == "repository" {
                *repository.outcome_error.lock().expect("outcome_error") =
                    Some(McpApplicationError::Storage(
                        "raw-database-path-and-credential-secret".to_string(),
                    ));
            } else {
                *logging.connection_error.lock().expect("connection_error") = Some(
                    McpApplicationError::Storage("raw-logging-sink-secret".to_string()),
                );
            }
            let service = service(
                repository,
                operations.clone(),
                logging,
                ConnectionOutcome::connected(Vec::new(), 9),
            );
            service
                .add_server(server_draft(Scope::User))
                .expect("add server");
            let prepared = service
                .prepare_connection_test("fixture-tools")
                .expect("prepare test");

            service
                .execute_connection_test(prepared)
                .await
                .expect("connection remains terminal after persistence failure");

            let events = operations.events.lock().expect("events").join("\n");
            assert!(events.contains("MCP connection result persistence failed."));
            assert!(events.contains("complete:true"));
            assert!(!events.contains("raw-database-path-and-credential-secret"));
            assert!(!events.contains("raw-logging-sink-secret"));
        }
    }

    #[tokio::test]
    async fn oversized_discovery_is_recorded_as_a_safe_limit_failure() {
        let repository = Arc::new(FakeRepository::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let outcome = ConnectionOutcome::connected(
            vec![ToolDescriptor {
                name: "x".repeat(McpLimits::DEFAULT.tool_name_bytes + 1),
                description: None,
                input_schema: None,
            }],
            17,
        );
        let service = service(
            repository.clone(),
            operations.clone(),
            logging.clone(),
            outcome,
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("add server");

        let prepared = service
            .prepare_connection_test("fixture-tools")
            .expect("prepare test");
        service
            .execute_connection_test(prepared)
            .await
            .expect("execute test");

        let recorded = repository
            .outcome
            .lock()
            .expect("outcome")
            .clone()
            .expect("recorded");
        assert_eq!(recorded.error_code(), Some(McpFailureCode::LimitExceeded));
        assert_eq!(
            recorded.error(),
            Some(McpFailureCode::LimitExceeded.safe_message())
        );
        assert_eq!(
            logging.error_codes.lock().expect("error_codes").as_slice(),
            [Some(McpFailureCode::LimitExceeded)]
        );
        assert_eq!(
            operations.events.lock().expect("events").as_slice(),
            [
                format!("log:{}", McpFailureCode::LimitExceeded.safe_message()),
                format!("fail:{}", McpFailureCode::LimitExceeded.safe_message()),
            ]
        );
    }

    #[tokio::test]
    async fn connection_operation_cancellation_waits_for_owned_cleanup_before_returning() {
        let repository = Arc::new(FakeRepository::default());
        let operations = Arc::new(FakeOperations::default());
        let connection = Arc::new(CancellationConnection::default());
        let service = McpApplicationService::new(
            repository,
            connection.clone(),
            operations.clone(),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("add server");
        let prepared = service
            .prepare_connection_test("fixture-tools")
            .expect("prepare test");
        let running = tokio::spawn(async move { service.execute_connection_test(prepared).await });

        tokio::time::sleep(Duration::from_millis(40)).await;
        operations
            .cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        running.await.expect("join").expect("execute test");

        assert!(connection
            .cleanup_finished
            .load(std::sync::atomic::Ordering::SeqCst));
        let events = operations.events.lock().expect("events");
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("cancelled"));
    }

    #[tokio::test]
    async fn shared_agent_cancellation_reaches_tool_control_and_waits_for_cleanup() {
        let repository = Arc::new(FakeRepository::default());
        let connection = Arc::new(CancellationConnection::default());
        let service = McpApplicationService::new(
            repository,
            connection.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("add server");
        let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancellation_for_call = cancellation.clone();
        let running = tokio::spawn(async move {
            service
                .call_tool_with_cancellation(
                    "D:\\code\\fixture",
                    "fixture-tools",
                    "search",
                    serde_json::json!({}),
                    cancellation_for_call,
                )
                .await
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        cancellation.store(true, std::sync::atomic::Ordering::SeqCst);
        let outcome = running.await.expect("join").expect("tool call");

        assert!(outcome.is_error);
        assert_eq!(outcome.error_code, Some(McpFailureCode::Cancelled));
        assert!(connection
            .tool_cleanup_finished
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn failed_connection_uses_the_same_error_for_operation_log_and_terminal_state() {
        let repository = Arc::new(FakeRepository::default());
        let operations = Arc::new(FakeOperations::default());
        let service = service(
            repository,
            operations.clone(),
            Arc::new(FakeLogging::default()),
            ConnectionOutcome::failed("handshake failed", 23),
        );
        service
            .add_server(server_draft(Scope::User))
            .expect("add server");
        let prepared = service
            .prepare_connection_test("fixture-tools")
            .expect("prepare test");

        service
            .execute_connection_test(prepared)
            .await
            .expect("execute test");

        assert_eq!(
            operations.events.lock().expect("events").as_slice(),
            ["log:handshake failed", "fail:handshake failed"]
        );
    }

    #[test]
    fn visible_tool_catalog_includes_only_visible_active_servers_cached_tools() {
        let repository = Arc::new(FakeRepository::default());
        let service = McpApplicationService::new(
            repository.clone(),
            Arc::new(FakeConnection::new(ConnectionOutcome::failed("unused", 0))),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        service
            .add_server(server_draft_named("active-tested", true))
            .expect("add active-tested");
        service
            .add_server(server_draft_named("inactive-tested", false))
            .expect("add inactive-tested");
        service
            .add_server(server_draft_named("active-untested", true))
            .expect("add active-untested");

        let tool = ToolDescriptor {
            name: "search".to_string(),
            description: None,
            input_schema: None,
        };
        let cached = ServerStatus {
            name: ServerName::parse("placeholder").expect("name"),
            connection_status: ConnectionStatus::Connected,
            tools: vec![tool.clone()],
            last_connected: Some("1700000000".to_string()),
            error: None,
            error_code: None,
            duration_ms: Some(5),
        };
        {
            let mut statuses = repository.statuses.lock().expect("statuses");
            statuses.insert(
                "active-tested".to_string(),
                ServerStatus {
                    name: ServerName::parse("active-tested").expect("name"),
                    ..cached.clone()
                },
            );
            statuses.insert(
                "inactive-tested".to_string(),
                ServerStatus {
                    name: ServerName::parse("inactive-tested").expect("name"),
                    ..cached
                },
            );
        }

        let entries = service
            .visible_tool_catalog("D:\\code\\fixture")
            .expect("catalog");

        assert_eq!(
            entries,
            vec![McpServerToolEntry {
                server_name: "active-tested".to_string(),
                tool,
            }]
        );
    }

    #[test]
    fn visible_tool_catalog_isolates_invalid_server_caches_with_one_safe_diagnostic_each() {
        let repository = Arc::new(FakeRepository::default());
        let logging = Arc::new(FakeLogging::default());
        let service = McpApplicationService::new(
            repository.clone(),
            Arc::new(FakeConnection::new(ConnectionOutcome::failed("unused", 0))),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            logging.clone(),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        for name in ["valid-cached", "oversized-cached", "corrupt-cached"] {
            service
                .add_server(server_draft_named(name, true))
                .expect("add server");
        }
        let status = |name: &str, tool: ToolDescriptor| ServerStatus {
            name: ServerName::parse(name).expect("name"),
            connection_status: ConnectionStatus::Connected,
            tools: vec![tool],
            last_connected: Some("1700000000".to_string()),
            error: None,
            error_code: None,
            duration_ms: Some(5),
        };
        repository.statuses.lock().expect("statuses").extend([
            (
                "valid-cached".to_string(),
                status(
                    "valid-cached",
                    ToolDescriptor {
                        name: "search".to_string(),
                        description: None,
                        input_schema: Some(serde_json::json!({ "type": "object" })),
                    },
                ),
            ),
            (
                "oversized-cached".to_string(),
                status(
                    "oversized-cached",
                    ToolDescriptor {
                        name: "x".repeat(McpLimits::DEFAULT.tool_name_bytes + 1),
                        description: None,
                        input_schema: None,
                    },
                ),
            ),
        ]);
        repository
            .status_errors
            .lock()
            .expect("status_errors")
            .insert(
                "corrupt-cached".to_string(),
                McpApplicationError::Validation("untrusted malformed JSON details".to_string()),
            );

        let entries = service
            .visible_tool_catalog("D:\\code\\fixture")
            .expect("catalog");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].server_name, "valid-cached");
        assert_eq!(entries[0].tool.name, "search");
        assert_eq!(
            logging
                .catalog_rejections
                .lock()
                .expect("catalog_rejections")
                .as_slice(),
            [
                ("corrupt-cached".to_string(), McpFailureCode::Validation),
                (
                    "oversized-cached".to_string(),
                    McpFailureCode::LimitExceeded
                ),
            ]
        );
    }

    #[test]
    fn visible_tool_catalog_orders_and_caps_aggregate_mcp_tools_deterministically() {
        let repository = Arc::new(FakeRepository::default());
        let logging = Arc::new(FakeLogging::default());
        let service = McpApplicationService::new(
            repository.clone(),
            Arc::new(FakeConnection::new(ConnectionOutcome::failed("unused", 0))),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            logging.clone(),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        for server_name in ["server-c", "server-a", "server-b"] {
            service
                .add_server(server_draft_named(server_name, true))
                .expect("add server");
            let tools = (0..100)
                .rev()
                .map(|index| ToolDescriptor {
                    name: format!("tool-{index:03}"),
                    description: None,
                    input_schema: Some(serde_json::json!({ "type": "object" })),
                })
                .collect();
            repository.statuses.lock().expect("statuses").insert(
                server_name.to_string(),
                ServerStatus {
                    name: ServerName::parse(server_name).expect("name"),
                    connection_status: ConnectionStatus::Connected,
                    tools,
                    last_connected: Some("1700000000".to_string()),
                    error: None,
                    error_code: None,
                    duration_ms: Some(5),
                },
            );
        }

        let entries = service
            .visible_tool_catalog("D:\\code\\fixture")
            .expect("catalog");

        assert_eq!(entries.len(), McpLimits::DEFAULT.provider_tools);
        let selected = [0, 99, 100, 199, 200, 255].map(|index| {
            (
                entries[index].server_name.as_str(),
                entries[index].tool.name.as_str(),
            )
        });
        assert_eq!(
            selected,
            [
                ("server-a", "tool-000"),
                ("server-a", "tool-099"),
                ("server-b", "tool-000"),
                ("server-b", "tool-099"),
                ("server-c", "tool-000"),
                ("server-c", "tool-055"),
            ]
        );
        assert_eq!(
            logging
                .catalog_overflows
                .lock()
                .expect("catalog_overflows")
                .as_slice(),
            [(44, McpLimits::DEFAULT.provider_tools)]
        );
    }

    #[tokio::test]
    async fn call_tool_rejects_a_server_outside_the_visible_active_set_without_connecting() {
        let repository = Arc::new(FakeRepository::default());
        let connection = Arc::new(FakeConnection::new(ConnectionOutcome::failed("unused", 0)));
        let service = McpApplicationService::new(
            repository,
            connection.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        service
            .add_server(server_draft_named("disabled-server", false))
            .expect("add disabled-server");

        let result = service
            .call_tool_with_cancellation(
                "D:\\code\\fixture",
                "disabled-server",
                "search",
                serde_json::json!({}),
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
            )
            .await;

        assert!(matches!(
            result,
            Err(McpApplicationError::ServerNotFound(name)) if name == "disabled-server"
        ));
        assert!(connection.tool_calls.lock().expect("tool_calls").is_empty());
    }

    #[tokio::test]
    async fn call_tool_delegates_to_the_connection_port_for_a_visible_active_server() {
        let repository = Arc::new(FakeRepository::default());
        let connection = Arc::new(FakeConnection {
            outcome: ConnectionOutcome::failed("unused", 0),
            tool_call_outcome: ToolCallOutcome::success("42"),
            tool_calls: Mutex::new(Vec::new()),
            test_calls: std::sync::atomic::AtomicUsize::new(0),
        });
        let service = McpApplicationService::new(
            repository,
            connection.clone(),
            Arc::new(FakeOperations::default()),
            Arc::new(FakeClock),
            Arc::new(FakeLogging::default()),
            Arc::new(FakeProjectPath),
            Arc::new(FakeTelemetry),
        );
        service
            .add_server(server_draft_named("active-server", true))
            .expect("add active-server");

        let outcome = service
            .call_tool_with_cancellation(
                "D:\\code\\fixture",
                "active-server",
                "search",
                serde_json::json!({"q": "x"}),
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
            )
            .await
            .expect("call tool");

        assert_eq!(outcome, ToolCallOutcome::success("42"));
        assert_eq!(
            connection.tool_calls.lock().expect("tool_calls").as_slice(),
            [("active-server".to_string(), "search".to_string())]
        );
    }
}
