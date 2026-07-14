use crate::sdk::models::*;
use crate::sdk::service;
use crate::AppError;

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
pub fn install_sdk_dependency(request: SdkOperationRequest) -> Result<SdkOperationResult, AppError> {
    Ok(service::install(request, SdkOperationType::Install))
}

#[tauri::command]
pub fn update_sdk_dependency(request: SdkOperationRequest) -> Result<SdkOperationResult, AppError> {
    Ok(service::install(request, SdkOperationType::Update))
}

#[tauri::command]
pub fn rollback_sdk_dependency(request: SdkOperationRequest) -> Result<SdkOperationResult, AppError> {
    Ok(service::install(request, SdkOperationType::Rollback))
}

#[tauri::command]
pub fn uninstall_sdk_dependency(sdk_id: SdkId) -> Result<SdkOperationResult, AppError> {
    Ok(service::uninstall(sdk_id))
}

#[tauri::command]
pub fn get_sdk_operation_logs(sdk_id: Option<SdkId>) -> Result<Vec<SdkOperationLog>, AppError> {
    Ok(service::operation_logs(sdk_id))
}
