use crate::contexts::skill_evolution_curation::api::*;
use serde_json::Value;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub(crate) fn query_skill_curator_queue(
    api: State<'_, SkillEvolutionCurationApi>,
    input: CuratorQueueQuery,
) -> Result<Value, CuratorApiError> {
    api.queue(input)
}

#[tauri::command]
pub(crate) fn get_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    candidate_id: String,
) -> Result<Value, CuratorApiError> {
    api.detail(&candidate_id)
}

#[tauri::command]
pub(crate) fn query_skill_curator_audit(
    api: State<'_, SkillEvolutionCurationApi>,
    input: CuratorAuditQuery,
) -> Result<Value, CuratorApiError> {
    api.audit(input)
}

#[tauri::command]
pub(crate) fn get_skill_curator_policy(
    api: State<'_, SkillEvolutionCurationApi>,
    workspace_id: String,
) -> Result<Value, CuratorApiError> {
    api.policy(&workspace_id)
}

#[tauri::command]
pub(crate) fn dispatch_skill_curator_notifications(
    api: State<'_, SkillEvolutionCurationApi>,
) -> Result<Value, CuratorApiError> {
    api.dispatch_notifications(now_ms())
}

#[tauri::command]
pub(crate) fn update_skill_curator_policy(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorPolicyInput,
) -> Result<Value, CuratorApiError> {
    api.update_policy(input, now_ms())
}

#[tauri::command]
pub(crate) async fn save_skill_curator_draft(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorDraftInput,
) -> Result<Value, CuratorApiError> {
    api.save_draft(input, now_ms()).await
}

#[tauri::command]
pub(crate) fn preview_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorPreviewInput,
) -> Result<Value, CuratorApiError> {
    api.preview(input, now_ms())
}

#[tauri::command]
pub(crate) fn approve_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorApproveInput,
) -> Result<Value, CuratorApiError> {
    api.approve(input, now_ms())
}

#[tauri::command]
pub(crate) fn reject_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorRejectInput,
) -> Result<Value, CuratorApiError> {
    api.reject(input, now_ms())
}

#[tauri::command]
pub(crate) fn defer_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorDeferInput,
) -> Result<Value, CuratorApiError> {
    api.defer(input, now_ms())
}

#[tauri::command]
pub(crate) fn resume_skill_curator_candidate(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorResumeInput,
) -> Result<Value, CuratorApiError> {
    api.resume(input, now_ms())
}

#[tauri::command]
pub(crate) fn retry_skill_curator_application(
    api: State<'_, SkillEvolutionCurationApi>,
    _window: WebviewWindow,
    input: CuratorRetryInput,
) -> Result<Value, CuratorApiError> {
    api.retry(input, now_ms())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
