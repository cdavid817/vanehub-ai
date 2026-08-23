use super::{
    CliApplicationError, CliDetectionResult, CliLogEvent, CliOperationRequest, CliOperationResult,
    CliToolStatus, StartedCliOperation,
};
use crate::contexts::tooling::cli::domain::ToolDefinition;

pub(crate) trait CliStatusRepository: Send + Sync {
    fn load(&self, definition: ToolDefinition) -> Result<CliToolStatus, CliApplicationError>;

    fn save(&self, status: &CliToolStatus) -> Result<(), CliApplicationError>;

    fn has_cached_statuses(&self) -> Result<bool, CliApplicationError>;
}

pub(crate) trait CliDetectionPort: Send + Sync {
    fn detect(
        &self,
        definition: ToolDefinition,
        operation_id: &str,
    ) -> Result<CliDetectionResult, CliApplicationError>;
}

pub(crate) trait CliExecutableLocatorPort: Send + Sync {
    fn resolve(&self, definition: ToolDefinition, cached_path: Option<&str>) -> Option<String>;
}

pub(crate) trait CliOperationPort: Send + Sync {
    fn start(
        &self,
        request: &CliOperationRequest,
    ) -> Result<StartedCliOperation, CliApplicationError>;

    fn append_log(&self, event: &CliLogEvent) -> Result<(), CliApplicationError>;

    fn complete(
        &self,
        operation_id: &str,
        result: &CliOperationResult,
    ) -> Result<(), CliApplicationError>;
}

pub(crate) trait CliLoggingPort: Send + Sync {
    fn record(&self, event: &CliLogEvent) -> Result<(), CliApplicationError>;
}

pub(crate) trait CliClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait NativeConfigPort: Send + Sync {
    /// Discover the active model from a CLI's native configuration file.
    /// `workspace_path`, when available, lets a CLI check per-project state
    /// (e.g. Claude Code's project-scoped usage cache) in addition to its
    /// global configuration file.
    /// Returns `Ok(None)` when no source is available, unreadable, or does
    /// not contain a model value — callers must fall back to their own
    /// defaults.
    fn discover_model(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<String>, CliApplicationError>;
}
