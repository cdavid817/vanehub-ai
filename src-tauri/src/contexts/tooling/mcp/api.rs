use crate::contexts::tooling::mcp::application::McpApplicationService;
pub(crate) use crate::contexts::tooling::mcp::application::{
    ExportBundle, ImportBundle, ImportEntry, ImportResult, McpApplicationError as McpError,
    PreparedConnectionTest, ServerPatch, StartedOperation,
};
pub(crate) use crate::contexts::tooling::mcp::domain::{
    ConnectionStatus, Scope, ServerConfiguration, ServerConfigurationDraft, ServerStatus,
    ToolDescriptor, TransportType,
};

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
}
