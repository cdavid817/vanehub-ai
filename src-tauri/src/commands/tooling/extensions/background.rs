use super::mapper;
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::extensions::api::{
    ExtensionAction, ExtensionApi, ExtensionError, ExtensionFrameworkId,
};

pub(super) fn start_operation(
    api: &ExtensionApi,
    framework_id: ExtensionFrameworkId,
    action: ExtensionAction,
) -> Result<OperationTask, ExtensionError> {
    let prepared = api.prepare_operation(
        crate::contexts::tooling::extensions::api::ExtensionOperationRequest {
            framework_id,
            action,
        },
    )?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    spawn_operation(api.clone(), prepared);
    Ok(operation)
}

fn spawn_operation(
    api: ExtensionApi,
    prepared: crate::contexts::tooling::extensions::api::PreparedExtensionOperation,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.execute_operation(prepared);
    });
}
