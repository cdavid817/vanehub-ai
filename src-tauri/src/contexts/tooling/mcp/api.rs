use crate::contexts::tooling::mcp::application::McpApplicationService;
pub(crate) use crate::contexts::tooling::mcp::application::{
    ExportBundle, ImportBundle, ImportEntry, ImportFailureStage, ImportResult, ImportTransportType,
    McpApplicationError as McpError, McpLimits, McpServerToolEntry, PreparedConnectionTest,
    ServerPatch, StartedOperation,
};
pub(crate) use crate::contexts::tooling::mcp::domain::{
    ConnectionStatus, McpFailureCode, Scope, ServerConfiguration, ServerConfigurationDraft,
    ServerStatus, ToolCallOutcome, ToolDescriptor, TransportType,
};
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct McpApi {
    service: McpApplicationService,
}

impl McpApi {
    pub(crate) fn new(service: McpApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list_servers(&self) -> Result<Vec<ServerConfiguration>, McpError> {
        self.service.list_servers()
    }

    pub(crate) fn add_server(&self, draft: ServerConfigurationDraft) -> Result<(), McpError> {
        self.service.add_server(draft)
    }

    pub(crate) fn update_server(&self, name: &str, patch: ServerPatch) -> Result<(), McpError> {
        self.service.update_server(name, patch)
    }

    pub(crate) fn remove_server(&self, name: &str) -> Result<(), McpError> {
        self.service.remove_server(name)
    }

    pub(crate) fn toggle_server(&self, name: &str, active: bool) -> Result<(), McpError> {
        self.service.toggle_server(name, active)
    }

    pub(crate) fn server_status(&self, name: &str) -> Result<ServerStatus, McpError> {
        self.service.server_status(name)
    }

    pub(crate) fn import_servers(
        &self,
        bundle: ImportBundle,
        scope: Scope,
    ) -> Result<ImportResult, McpError> {
        self.service.import_servers(bundle, scope)
    }

    pub(crate) fn export_servers(&self, names: Vec<String>) -> Result<ExportBundle, McpError> {
        self.service.export_servers(names)
    }

    pub(crate) fn prepare_connection_test(
        &self,
        name: &str,
    ) -> Result<PreparedConnectionTest, McpError> {
        self.service.prepare_connection_test(name)
    }

    pub(crate) async fn execute_connection_test(
        &self,
        prepared: PreparedConnectionTest,
    ) -> Result<(), McpError> {
        self.service.execute_connection_test(prepared).await
    }

    pub(crate) fn visible_tool_catalog(
        &self,
        project_path: &str,
    ) -> Result<Vec<McpServerToolEntry>, McpError> {
        self.service.visible_tool_catalog(project_path)
    }

    pub(crate) async fn call_tool_with_cancellation(
        &self,
        project_path: &str,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
        cancellation: Arc<AtomicBool>,
    ) -> Result<ToolCallOutcome, McpError> {
        self.service
            .call_tool_with_cancellation(
                project_path,
                server_name,
                tool_name,
                arguments,
                cancellation,
            )
            .await
    }
}
