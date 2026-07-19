use super::{
    ExtensionApplicationError, ExtensionExecutionLog, ExtensionLogEvent, ExtensionOperationResult,
    InstalledExtension, StartedExtensionOperation,
};
use crate::contexts::tooling::extensions::domain::{
    EnablementPlan, ExtensionAction, ExtensionFrameworkId, ExtensionFrameworkState,
    ExtensionInstallationObservation, ExtensionLifecycleStatus, ExtensionRuntimeObservation,
    HostEnvironment, InstallPlan, RemovalPlan, RuntimePlan, SelfTestPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallationInspection {
    MetadataOnly,
    VerifyImport,
}

pub(crate) trait ExtensionRepository: Send + Sync {
    fn list_states(&self) -> Result<Vec<ExtensionFrameworkState>, ExtensionApplicationError>;

    fn record_transition(
        &self,
        framework_id: ExtensionFrameworkId,
        status: ExtensionLifecycleStatus,
        operation_id: &str,
        at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn record_installation(
        &self,
        framework_id: ExtensionFrameworkId,
        installed: &InstalledExtension,
        at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn record_removal(
        &self,
        framework_id: ExtensionFrameworkId,
        at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn apply_enablement(
        &self,
        plan: &EnablementPlan,
        at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn record_runtime_observation(
        &self,
        framework_id: ExtensionFrameworkId,
        observation: &ExtensionRuntimeObservation,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn record_self_test(
        &self,
        framework_id: ExtensionFrameworkId,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError>;

    fn record_failure(
        &self,
        framework_id: ExtensionFrameworkId,
        error: &str,
        at: &str,
    ) -> Result<(), ExtensionApplicationError>;
}

pub(crate) trait ExtensionEnvironmentPort: Send + Sync {
    fn observe_host(&self) -> Result<HostEnvironment, ExtensionApplicationError>;
}

pub(crate) trait ExtensionInstallationPort: Send + Sync {
    fn managed_path(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<String, ExtensionApplicationError>;

    fn inspect(
        &self,
        framework_id: ExtensionFrameworkId,
        inspection: InstallationInspection,
    ) -> Result<ExtensionInstallationObservation, ExtensionApplicationError>;

    fn install(
        &self,
        operation_id: &str,
        plan: &InstallPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<InstalledExtension, ExtensionApplicationError>;

    fn rollback_installation(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<(), ExtensionApplicationError>;

    fn remove(
        &self,
        operation_id: &str,
        plan: &RemovalPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError>;

    fn self_test(
        &self,
        operation_id: &str,
        plan: &SelfTestPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError>;
}

pub(crate) trait ExtensionRuntimePort: Send + Sync {
    fn observe(
        &self,
        framework_id: ExtensionFrameworkId,
        port: u16,
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError>;

    fn start(
        &self,
        operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError>;

    fn stop(
        &self,
        operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError>;
}

pub(crate) trait ExtensionMutationPort: Send + Sync {
    fn begin(&self, framework_id: ExtensionFrameworkId) -> Result<(), ExtensionApplicationError>;
    fn finish(&self, framework_id: ExtensionFrameworkId);
}

pub(crate) trait ExtensionOperationPort: Send + Sync {
    fn start(
        &self,
        framework_id: ExtensionFrameworkId,
        action: ExtensionAction,
        message: String,
    ) -> Result<StartedExtensionOperation, ExtensionApplicationError>;

    fn append_log(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError>;

    fn complete(&self, result: &ExtensionOperationResult) -> Result<(), ExtensionApplicationError>;

    fn fail(&self, operation_id: &str, error: String) -> Result<(), ExtensionApplicationError>;
}

pub(crate) trait ExtensionLoggingPort: Send + Sync {
    fn record(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError>;
}

pub(crate) trait ExtensionClockPort: Send + Sync {
    fn now(&self) -> String;
}
