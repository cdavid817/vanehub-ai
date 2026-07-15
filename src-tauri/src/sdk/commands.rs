use crate::sdk::models::*;
use crate::sdk::service;
use crate::tasks::models::OperationKind;
use crate::tasks::registry::TaskRegistry;
use crate::AppError;
use tauri::State;

#[tauri::command]
pub fn list_sdk_definitions() -> Result<Vec<SdkDefinition>, AppError> {
    Ok(service::definitions())
}

#[tauri::command]
pub fn list_sdk_statuses() -> Result<SdkStatusMap, AppError> {
    service::list_statuses()
}

#[tauri::command]
pub fn check_sdk_environment() -> Result<SdkEnvironmentStatus, AppError> {
    Ok(service::check_environment())
}

#[tauri::command]
pub fn get_sdk_versions(sdk_id: Option<SdkId>) -> Result<SdkVersionMap, AppError> {
    Ok(service::get_versions(sdk_id))
}

#[tauri::command]
pub fn check_sdk_updates(sdk_id: Option<SdkId>) -> Result<SdkUpdateMap, AppError> {
    service::check_updates(sdk_id)
}

#[tauri::command]
pub fn install_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, request.sdk_id, SdkOperationType::Install, || {
        service::install(request, SdkOperationType::Install)
    })
}

#[tauri::command]
pub fn update_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, request.sdk_id, SdkOperationType::Update, || {
        service::install(request, SdkOperationType::Update)
    })
}

#[tauri::command]
pub fn rollback_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(
        &registry,
        request.sdk_id,
        SdkOperationType::Rollback,
        || service::install(request, SdkOperationType::Rollback),
    )
}

#[tauri::command]
pub fn uninstall_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    sdk_id: SdkId,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, sdk_id, SdkOperationType::Uninstall, || {
        service::uninstall(sdk_id)
    })
}

#[tauri::command]
pub fn get_sdk_operation_logs(sdk_id: Option<SdkId>) -> Result<Vec<SdkOperationLog>, AppError> {
    Ok(service::operation_logs(sdk_id))
}

fn run_sdk_operation(
    registry: &TaskRegistry,
    sdk_id: SdkId,
    operation: SdkOperationType,
    run: impl FnOnce() -> SdkOperationResult,
) -> Result<SdkOperationResult, AppError> {
    let task = registry.start(
        OperationKind::Sdk,
        Some(sdk_id.as_str().to_string()),
        Some(format!("{operation:?} SDK operation")),
    )?;
    let mut result = run();
    result.operation_id = Some(task.id.clone());

    for log in &result.logs {
        let _ = registry.append_log(&task.id, log.line.clone());
    }

    if result.success {
        let _ = registry.complete(&task.id, None);
    } else {
        let _ = registry.fail(
            &task.id,
            result
                .error
                .clone()
                .unwrap_or_else(|| "SDK operation failed".to_string()),
        );
    }

    Ok(result)
}
