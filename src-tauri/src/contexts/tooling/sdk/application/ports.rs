use super::{
    SdkApplicationError, SdkEnvironmentStatus, SdkLogEvent, SdkOperationLog, SdkOperationResult,
    SdkPackageLog, StartedSdkOperation,
};
use crate::contexts::tooling::sdk::domain::{
    SdkDefinition, SdkId, SdkLifecyclePlan, SdkOperationType,
};

pub(crate) trait SdkRepository: Send + Sync {
    fn installed_version(
        &self,
        definition: SdkDefinition,
    ) -> Result<Option<String>, SdkApplicationError>;

    fn install_path(&self, sdk_id: SdkId) -> Result<String, SdkApplicationError>;

    fn operation_logs(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<Vec<SdkOperationLog>, SdkApplicationError>;

    fn append_operation_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError>;
}

pub(crate) trait SdkPackageExecutionPort: Send + Sync {
    fn environment(&self) -> Result<SdkEnvironmentStatus, SdkApplicationError>;

    fn available_versions(
        &self,
        definition: SdkDefinition,
    ) -> Result<Vec<String>, SdkApplicationError>;

    fn latest_version(&self, definition: SdkDefinition) -> Result<String, SdkApplicationError>;

    fn execute(
        &self,
        operation_id: &str,
        plan: &SdkLifecyclePlan,
        emit: &mut dyn FnMut(SdkPackageLog),
    ) -> Result<Option<String>, SdkApplicationError>;
}

pub(crate) trait SdkOperationPort: Send + Sync {
    fn start(
        &self,
        sdk_id: SdkId,
        operation: SdkOperationType,
        message: String,
    ) -> Result<StartedSdkOperation, SdkApplicationError>;

    fn append_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError>;

    fn complete(&self, result: &SdkOperationResult) -> Result<(), SdkApplicationError>;

    fn fail(&self, operation_id: &str, error: String) -> Result<(), SdkApplicationError>;
}

pub(crate) trait SdkLoggingPort: Send + Sync {
    fn record(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError>;
}

pub(crate) trait SdkClockPort: Send + Sync {
    fn now(&self) -> String;
}
