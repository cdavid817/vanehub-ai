use crate::skills::models::*;
use crate::skills::service;
use crate::{AppError, RegistryStore};
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_skills(
    state: State<'_, Mutex<RegistryStore>>,
    input: SkillScopeInput,
) -> Result<SkillListResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::list_skills(&conn, input)
}

#[tauri::command]
pub fn list_skill_mount_paths(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<Vec<SkillAgentMountPath>, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::list_mount_paths(&conn)
}

#[tauri::command]
pub fn update_skill_mount_path(
    state: State<'_, Mutex<RegistryStore>>,
    agent_id: String,
    mount_path: String,
) -> Result<SkillMountMigrationReport, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::update_mount_path(&conn, &agent_id, &mount_path)
}

#[tauri::command]
pub fn create_skill(
    state: State<'_, Mutex<RegistryStore>>,
    input: SkillMutationInput,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::create_skill(&conn, input)
}

#[tauri::command]
pub fn update_skill(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
    input: SkillUpdateInput,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::update_skill(&conn, &skill_id, input)
}

#[tauri::command]
pub fn delete_skill(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::delete_skill(&conn, &skill_id, input)
}

#[tauri::command]
pub fn restore_builtin_skill(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::restore_builtin(&conn, &skill_id)
}

#[tauri::command]
pub fn set_skill_enabled(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
    input: SkillScopeInput,
    enabled: bool,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::set_skill_enabled(&conn, &skill_id, input, enabled)
}

#[tauri::command]
pub fn set_skill_agent_bindings(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
    input: SkillScopeInput,
    agent_ids: Vec<String>,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::set_skill_bindings(&conn, &skill_id, input, agent_ids)
}

#[tauri::command]
pub fn preview_skill(
    state: State<'_, Mutex<RegistryStore>>,
    skill_id: String,
    input: SkillScopeInput,
) -> Result<SkillPreview, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::preview_skill(&conn, &skill_id, input)
}

#[tauri::command]
pub fn import_skill(
    state: State<'_, Mutex<RegistryStore>>,
    input: SkillImportInput,
) -> Result<Skill, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::import_skill(&conn, input)
}

#[tauri::command]
pub fn detect_skill_drift(
    state: State<'_, Mutex<RegistryStore>>,
    input: SkillScopeInput,
) -> Result<SkillDriftReport, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::detect_drift(&conn, input)
}

#[tauri::command]
pub fn sync_skill_drift(
    state: State<'_, Mutex<RegistryStore>>,
    input: SkillScopeInput,
) -> Result<SkillSyncResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::sync_drift(&conn, input)
}

#[tauri::command]
pub fn select_workspace_directory() -> Result<Option<String>, AppError> {
    service::select_workspace_directory()
}
