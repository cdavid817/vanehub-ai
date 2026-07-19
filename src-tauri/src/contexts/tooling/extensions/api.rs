use crate::contexts::tooling::extensions::application::ExtensionApplicationService;

pub(crate) use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError as ExtensionError, ExtensionInstallPreview,
    ExtensionOperationRequest, ExtensionOverview, PreparedExtensionOperation,
    StartedExtensionOperation,
};
#[cfg(test)]
pub(crate) use crate::contexts::tooling::extensions::domain::ExtensionEnvironmentReason;
pub(crate) use crate::contexts::tooling::extensions::domain::{
    ExtensionAction, ExtensionCapabilityId, ExtensionEnvironment, ExtensionFrameworkDefinition,
    ExtensionFrameworkId, ExtensionFrameworkStatus, ExtensionLifecycleStatus,
    ExtensionModelRequirement,
};

#[derive(Clone)]
pub(crate) struct ExtensionApi {
    service: ExtensionApplicationService,
}

impl ExtensionApi {
    pub(crate) fn new(service: ExtensionApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn overview(&self) -> Result<ExtensionOverview, ExtensionError> {
        self.service.overview()
    }

    pub(crate) fn refresh_health(&self) -> Result<ExtensionOverview, ExtensionError> {
        self.service.refresh_health()
    }

    pub(crate) fn install_preview(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<ExtensionInstallPreview, ExtensionError> {
        self.service.install_preview(framework_id)
    }

    pub(crate) fn prepare_operation(
        &self,
        request: ExtensionOperationRequest,
    ) -> Result<PreparedExtensionOperation, ExtensionError> {
        self.service.prepare_operation(request)
    }

    pub(crate) fn execute_operation(
        &self,
        prepared: PreparedExtensionOperation,
    ) -> Result<(), ExtensionError> {
        self.service.execute_operation(prepared)
    }
}
