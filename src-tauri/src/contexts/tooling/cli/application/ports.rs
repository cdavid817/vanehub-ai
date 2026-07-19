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

pub(crate) trait CliPackagePort: Send + Sync {
    fn validate(
        &self,
        definition: ToolDefinition,
        status: &CliToolStatus,
        confirmed_active_path: Option<&str>,
    ) -> Result<(), CliApplicationError>;

    fn execute(
        &self,
        operation_id: &str,
        definition: ToolDefinition,
        status: &CliToolStatus,
        target_version: &str,
        emit: &mut dyn FnMut(CliLogEvent),
    ) -> Result<(), CliApplicationError>;
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

    fn fail(&self, operation_id: &str, error: String) -> Result<(), CliApplicationError>;
}

pub(crate) trait CliLoggingPort: Send + Sync {
    fn record(&self, event: &CliLogEvent) -> Result<(), CliApplicationError>;
}

pub(crate) trait CliClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait CliMutationPort: Send + Sync {
    fn try_acquire(&self, agent_id: &str) -> Result<bool, CliApplicationError>;

    fn release(&self, agent_id: &str) -> Result<(), CliApplicationError>;

    fn try_acquire_many(&self, agent_ids: &[String]) -> Result<Vec<String>, CliApplicationError>;

    fn release_many(&self, agent_ids: &[String]) -> Result<(), CliApplicationError>;
}
