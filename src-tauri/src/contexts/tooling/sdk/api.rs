use crate::contexts::tooling::sdk::application::SdkApplicationService;
use std::collections::BTreeMap;

pub(crate) use crate::contexts::tooling::sdk::application::{
    PreparedSdkOperation, SdkApplicationError as SdkError, SdkEnvironmentStatus, SdkOperationLog,
    SdkOperationRequest, StartedSdkOperation,
};
pub(crate) use crate::contexts::tooling::sdk::domain::{
    definition as sdk_definition, SdkDefinition, SdkId, SdkInstallStatus, SdkOperationType,
    SdkStatus, SdkUpdateInfo, SdkVersionInfo, SdkVersionSource,
};

#[derive(Clone)]
pub(crate) struct SdkApi {
    service: SdkApplicationService,
}

impl SdkApi {
    pub(crate) fn new(service: SdkApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list_definitions(&self) -> Vec<SdkDefinition> {
        self.service.list_definitions()
    }

    pub(crate) fn list_statuses(&self) -> Result<Vec<SdkStatus>, SdkError> {
        self.service.list_statuses()
    }

    pub(crate) fn is_installed(&self, sdk_id: SdkId) -> Result<bool, SdkError> {
        self.service.is_installed(sdk_id)
    }

    pub(crate) fn check_environment(&self) -> Result<SdkEnvironmentStatus, SdkError> {
        self.service.check_environment()
    }

    pub(crate) fn get_versions(&self, sdk_id: Option<SdkId>) -> BTreeMap<SdkId, SdkVersionInfo> {
        self.service.get_versions(sdk_id)
    }

    pub(crate) fn check_updates(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<BTreeMap<SdkId, SdkUpdateInfo>, SdkError> {
        self.service.check_updates(sdk_id)
    }

    pub(crate) fn operation_logs(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<Vec<SdkOperationLog>, SdkError> {
        self.service.operation_logs(sdk_id)
    }

    pub(crate) fn prepare_operation(
        &self,
        request: SdkOperationRequest,
    ) -> Result<PreparedSdkOperation, SdkError> {
        self.service.prepare_operation(request)
    }

    pub(crate) fn execute_operation(&self, prepared: PreparedSdkOperation) -> Result<(), SdkError> {
        self.service.execute_operation(prepared)
    }
}
