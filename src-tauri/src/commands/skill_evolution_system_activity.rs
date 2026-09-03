use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::contexts::skill_evolution_system_activity::api::SkillEvolutionSystemActivityApi;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemActivityCommandError {
    code: String,
}

type SystemActivityResult = Result<Value, SystemActivityCommandError>;

fn boundary(result: Result<Value, String>) -> SystemActivityResult {
    result.map_err(|code| SystemActivityCommandError { code })
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[tauri::command]
pub(crate) fn list_system_activity_sessions(
    api: State<'_, SkillEvolutionSystemActivityApi>,
) -> SystemActivityResult {
    boundary(api.list_sessions())
}

#[tauri::command]
pub(crate) fn query_system_activity_timeline(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    input: Value,
) -> SystemActivityResult {
    boundary(api.query_timeline(input))
}

#[tauri::command]
pub(crate) fn get_system_activity_read_state(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    session_id: String,
) -> SystemActivityResult {
    boundary(api.read_state(&session_id, now_ms()))
}

#[tauri::command]
pub(crate) fn advance_system_activity_read_cursor(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    session_id: String,
    through_sequence: u64,
    expected_revision: u64,
) -> SystemActivityResult {
    boundary(api.advance_read_cursor(&session_id, through_sequence, expected_revision, now_ms()))
}

#[tauri::command]
pub(crate) fn mark_system_activity_unread(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    session_id: String,
    from_sequence: u64,
    expected_revision: u64,
) -> SystemActivityResult {
    boundary(api.mark_unread(&session_id, from_sequence, expected_revision, now_ms()))
}

#[tauri::command]
pub(crate) fn get_system_activity_preferences(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    scope_kind: String,
    canonical_scope_id: String,
) -> SystemActivityResult {
    boundary(api.preferences(&scope_kind, &canonical_scope_id))
}

#[tauri::command]
pub(crate) fn update_system_activity_preferences(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    input: Value,
) -> SystemActivityResult {
    boundary(api.update_preferences(input, now_ms()))
}

#[tauri::command]
pub(crate) fn get_system_activity_dashboard(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    scope_kind: String,
    canonical_scope_id: String,
) -> SystemActivityResult {
    boundary(api.dashboard(&scope_kind, &canonical_scope_id))
}

#[tauri::command]
pub(crate) fn get_system_activity_health(
    api: State<'_, SkillEvolutionSystemActivityApi>,
) -> SystemActivityResult {
    boundary(api.health())
}

#[tauri::command]
pub(crate) fn open_system_activity_notification(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    request_id: String,
    visible_sequence: u64,
) -> SystemActivityResult {
    boundary(api.open_notification(&request_id, visible_sequence, now_ms()))
}

#[tauri::command]
pub(crate) fn dismiss_system_activity_notification(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    request_id: String,
) -> SystemActivityResult {
    boundary(api.dismiss_notification(&request_id, now_ms()))
}

#[tauri::command]
pub(crate) fn claim_system_activity_digests(
    api: State<'_, SkillEvolutionSystemActivityApi>,
) -> SystemActivityResult {
    boundary(api.claim_due_digests(now_ms()))
}

#[tauri::command]
pub(crate) fn begin_system_activity_rebuild(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    scope_kind: String,
    canonical_scope_id: String,
    item_budget: u64,
) -> SystemActivityResult {
    boundary(api.begin_rebuild(&scope_kind, &canonical_scope_id, item_budget, now_ms()))
}

#[tauri::command]
pub(crate) fn advance_system_activity_rebuild(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    rebuild_id: String,
    batch_limit: u64,
) -> SystemActivityResult {
    boundary(api.advance_rebuild(&rebuild_id, batch_limit, now_ms()))
}

#[tauri::command]
pub(crate) fn validate_system_activity_rebuild(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    rebuild_id: String,
) -> SystemActivityResult {
    boundary(api.validate_rebuild(&rebuild_id, now_ms()))
}

#[tauri::command]
pub(crate) fn activate_system_activity_rebuild(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    rebuild_id: String,
) -> SystemActivityResult {
    boundary(api.activate_rebuild(&rebuild_id, now_ms()))
}

#[tauri::command]
pub(crate) fn cancel_system_activity_rebuild(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    rebuild_id: String,
) -> SystemActivityResult {
    boundary(api.cancel_rebuild(&rebuild_id, now_ms()))
}

#[tauri::command]
pub(crate) fn export_system_activity(
    api: State<'_, SkillEvolutionSystemActivityApi>,
    input: Value,
) -> SystemActivityResult {
    boundary(api.export(input, now_ms()))
}
