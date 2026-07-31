use super::{
    ConnectionTestResult, ExportBundle, ImportBundle, ImportEntry, ImportResult,
    McpApplicationError, McpClockPort, McpConnectionPort, McpLoggingPort, McpOperationPort,
    McpProjectPathPort, McpServerRepository, McpServerToolEntry, McpTelemetryPort,
    PreparedConnectionTest, ServerPatch,
};
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, Scope, ServerConfiguration, ServerConfigurationDraft, ServerName,
    ServerStatus, ToolCallOutcome, TransportType,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

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
            let status = self.repository.status(&server_name)?;
            entries.extend(status.tools.into_iter().map(|tool| McpServerToolEntry {
                server_name: server_name.clone(),
                tool,
            }));
        }
        Ok(entries)
    }

    /// Invokes `tool_name` on `server_name`, re-deriving the visible+active server set for
    /// `project_path` the same way `visible_tool_catalog` does rather than trusting the caller's
    /// `server_name` outright — a server that isn't currently visible and active is rejected
    /// before any connection is attempted.
    pub(crate) async fn call_tool(
        &self,
        project_path: &str,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<ToolCallOutcome, McpApplicationError> {
        let server = self
            .repository
            .list_visible(project_path)?
            .into_iter()
            .filter(ServerConfiguration::is_active)
            .find(|server| server.name().as_str() == server_name)
            .ok_or_else(|| McpApplicationError::ServerNotFound(server_name.to_string()))?;
        Ok(self
            .connection
            .call_tool(&server, tool_name, arguments)
            .await)
    }

    pub(crate) fn import_servers(
        &self,
        data: ImportBundle,
        scope: Scope,
    ) -> Result<ImportResult, McpApplicationError> {
        let mut result = ImportResult::default();
        for (name, entry) in data.servers {
            let parsed_name = match ServerName::parse(name.clone()) {
                Ok(name) => name,
                Err(_) => {
                    result.skipped.push(name);
                    continue;
                }
            };
            if self.repository.exists(&parsed_name)? {
                result.skipped.push(name);
                continue;
            }
            let transport_type = if entry
                .command
                .as_deref()
                .is_none_or(|command| command.trim().is_empty())
            {
                TransportType::Sse
            } else {
                TransportType::Stdio
            };
            let mut draft = ServerConfigurationDraft {
                name: name.clone(),
                transport_type,
                command: entry.command,
                args: entry.args,
                env: entry.env,
                url: entry.url,
                headers: entry.headers,
                description: None,
                active: true,
                scope,
                project_path: None,
            };
            let imported = self
                .bind_project_scope(&mut draft)
                .and_then(|()| ServerConfiguration::create(draft).map_err(Into::into))
                .and_then(|server| self.repository.insert(&server, &self.clock.now()))
                .is_ok();
            if imported {
                result.imported.push(name);
            } else {
                result.skipped.push(name);
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
                TransportType::Sse | TransportType::StreamableHttp => ImportEntry {
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
        let operation = self.operations.start_connection_test(name)?;
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
        })
    }

    pub(crate) async fn execute_connection_test(
        &self,
        prepared: PreparedConnectionTest,
    ) -> Result<(), McpApplicationError> {
        let operation_id = prepared.operation.id.clone();
        let server_name = prepared.server.name().as_str().to_string();
        let outcome = self.connection.test(&prepared.server).await;
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
        if let Err(error) = persistence {
            let _ = self.operations.append_log(&operation_id, error.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::application::StartedOperation;
    use crate::contexts::tooling::mcp::domain::{ConnectionStatus, ServerStatus, ToolDescriptor};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRepository {
        servers: Mutex<BTreeMap<String, ServerConfiguration>>,
        writes: Mutex<Vec<String>>,
        outcome: Mutex<Option<ConnectionOutcome>>,
        statuses: Mutex<BTreeMap<String, ServerStatus>>,
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
            if let Some(status) = self.statuses.lock().expect("statuses").get(name) {
                return Ok(status.clone());
            }
            Ok(ServerStatus {
                name: ServerName::parse(name.to_string())?,
                connection_status: ConnectionStatus::Disconnected,
                tools: Vec::new(),
                last_connected: None,
                error: None,
                duration_ms: None,
            })
        }

        fn record_connection_outcome(
            &self,
            _name: &str,
            outcome: &ConnectionOutcome,
            timestamp: &str,
        ) -> Result<(), McpApplicationError> {
            *self.outcome.lock().expect("outcome") = Some(outcome.clone());
            self.writes.lock().expect("writes").push(timestamp.into());
            Ok(())
        }
    }

    struct FakeConnection {
        outcome: ConnectionOutcome,
        tool_call_outcome: ToolCallOutcome,
        tool_calls: Mutex<Vec<(String, String)>>,
    }

    impl FakeConnection {
        fn new(outcome: ConnectionOutcome) -> Self {
            Self {
                outcome,
                tool_call_outcome: ToolCallOutcome::success(""),
                tool_calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl McpConnectionPort for FakeConnection {
        async fn test(&self, _server: &ServerConfiguration) -> ConnectionOutcome {
            self.outcome.clone()
        }

        async fn call_tool(
            &self,
            server: &ServerConfiguration,
            tool_name: &str,
            _arguments: Value,
        ) -> ToolCallOutcome {
            self.tool_calls
                .lock()
                .expect("tool_calls")
                .push((server.name().as_str().to_string(), tool_name.to_string()));
            self.tool_call_outcome.clone()
        }
    }

    #[derive(Default)]
    struct FakeOperations {
        events: Mutex<Vec<String>>,
    }

    impl McpOperationPort for FakeOperations {
        fn start_connection_test(
            &self,
            server_name: &str,
        ) -> Result<StartedOperation, McpApplicationError> {
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
    }

    impl McpLoggingPort for FakeLogging {
        fn record_connection_outcome(
            &self,
            operation_id: &str,
            server_name: &str,
            outcome: &ConnectionOutcome,
        ) -> Result<(), McpApplicationError> {
            self.entries.lock().expect("entries").push(format!(
                "{operation_id}:{server_name}:{}",
                outcome.is_success()
            ));
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
            .call_tool(
                "D:\\code\\fixture",
                "disabled-server",
                "search",
                serde_json::json!({}),
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
            .call_tool(
                "D:\\code\\fixture",
                "active-server",
                "search",
                serde_json::json!({"q": "x"}),
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
