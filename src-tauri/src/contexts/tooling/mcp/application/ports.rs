use super::{ConnectionTestResult, McpApplicationError, StartedOperation};
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, ServerConfiguration, ServerName, ServerStatus,
};
use async_trait::async_trait;

pub(crate) trait McpServerRepository: Send + Sync {
    fn list_visible(
        &self,
        current_project_path: &str,
    ) -> Result<Vec<ServerConfiguration>, McpApplicationError>;

    fn find(&self, name: &str) -> Result<Option<ServerConfiguration>, McpApplicationError>;

    fn exists(&self, name: &ServerName) -> Result<bool, McpApplicationError>;

    fn insert(
        &self,
        server: &ServerConfiguration,
        timestamp: &str,
    ) -> Result<(), McpApplicationError>;

    fn replace(
        &self,
        original_name: &str,
        server: &ServerConfiguration,
        timestamp: &str,
    ) -> Result<(), McpApplicationError>;

    fn remove(&self, name: &str) -> Result<(), McpApplicationError>;

    fn set_active(
        &self,
        name: &str,
        active: bool,
        timestamp: &str,
    ) -> Result<(), McpApplicationError>;

    fn status(&self, name: &str) -> Result<ServerStatus, McpApplicationError>;

    fn record_connection_outcome(
        &self,
        name: &str,
        outcome: &ConnectionOutcome,
        timestamp: &str,
    ) -> Result<(), McpApplicationError>;
}

#[async_trait]
pub(crate) trait McpConnectionPort: Send + Sync {
    async fn test(&self, server: &ServerConfiguration) -> ConnectionOutcome;
}

pub(crate) trait McpOperationPort: Send + Sync {
    fn start_connection_test(
        &self,
        server_name: &str,
    ) -> Result<StartedOperation, McpApplicationError>;

    fn append_log(&self, operation_id: &str, line: String) -> Result<(), McpApplicationError>;

    fn complete_connection_test(
        &self,
        operation_id: &str,
        result: &ConnectionTestResult,
    ) -> Result<(), McpApplicationError>;

    fn fail_connection_test(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<(), McpApplicationError>;
}

pub(crate) trait McpClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait McpLoggingPort: Send + Sync {
    fn record_connection_outcome(
        &self,
        operation_id: &str,
        server_name: &str,
        outcome: &ConnectionOutcome,
    ) -> Result<(), McpApplicationError>;
}

pub(crate) trait McpProjectPathPort: Send + Sync {
    fn current_project_path(&self) -> Result<String, McpApplicationError>;
}
