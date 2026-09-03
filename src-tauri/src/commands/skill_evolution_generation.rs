use crate::contexts::skill_evolution_generation::{
    api::{GenerationApiError, GenerationPolicyUpdate, SkillEvolutionGenerationApi},
    domain::GeneratedArtifactKind,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdatePolicyInput {
    workspace_id: String,
    expected_revision: u64,
    enabled: bool,
    disclosure_version: String,
    provider_profile_id: Option<String>,
    model_id: Option<String>,
    allowed_artifact_kinds: Vec<GeneratedArtifactKind>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationQueryInput {
    workspace_id: Option<String>,
    skill_id: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSectionInput {
    dossier_id: String,
    ordinal: u8,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegenerateInput {
    job_id: String,
    expected_input_witness_hash: String,
    request_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExportInput {
    dossier_id: String,
    format: String,
}

#[tauri::command]
pub(crate) fn get_skill_evolution_generation_policy(
    api: State<'_, SkillEvolutionGenerationApi>,
    workspace_id: String,
) -> Result<Value, String> {
    api.policy(&workspace_id).map_err(command_error)
}

#[tauri::command]
pub(crate) fn update_skill_evolution_generation_policy(
    api: State<'_, SkillEvolutionGenerationApi>,
    input: UpdatePolicyInput,
) -> Result<Value, String> {
    api.update_policy(
        &GenerationPolicyUpdate {
            workspace_id: &input.workspace_id,
            expected_revision: input.expected_revision,
            enabled: input.enabled,
            disclosure_version: &input.disclosure_version,
            provider_profile_id: input.provider_profile_id.as_deref(),
            model_id: input.model_id.as_deref(),
            allowed_artifact_kinds: &input.allowed_artifact_kinds,
        },
        chrono::Utc::now().timestamp_millis(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn query_skill_evolution_generation_jobs(
    api: State<'_, SkillEvolutionGenerationApi>,
    input: GenerationQueryInput,
) -> Result<Value, String> {
    api.jobs(
        input.workspace_id.as_deref(),
        input.skill_id.as_deref(),
        input.status.as_deref(),
        input.limit.unwrap_or(20),
        input.cursor.as_deref(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn get_skill_evolution_generation_job(
    api: State<'_, SkillEvolutionGenerationApi>,
    job_id: String,
) -> Result<Option<Value>, String> {
    api.job_detail(&job_id).map_err(command_error)
}

#[tauri::command]
pub(crate) fn cancel_skill_evolution_generation_job(
    app: AppHandle,
    api: State<'_, SkillEvolutionGenerationApi>,
    job_id: String,
) -> Result<Value, String> {
    let value = api
        .cancel_job(&job_id, chrono::Utc::now().timestamp_millis())
        .map_err(command_error)?;
    emit_notification(&app, "cancelled", &value, None);
    Ok(value)
}

#[tauri::command]
pub(crate) fn regenerate_skill_evolution_generation_job(
    app: AppHandle,
    api: State<'_, SkillEvolutionGenerationApi>,
    input: RegenerateInput,
) -> Result<Value, String> {
    let value = api
        .regenerate(
            &input.job_id,
            &input.expected_input_witness_hash,
            &input.request_id,
            chrono::Utc::now().timestamp_millis(),
        )
        .map_err(command_error)?;
    emit_notification(&app, "superseded", &value, Some(&input.job_id));
    Ok(value)
}

#[tauri::command]
pub(crate) fn get_skill_evolution_generation_dossier_section(
    api: State<'_, SkillEvolutionGenerationApi>,
    input: DossierSectionInput,
) -> Result<Value, String> {
    api.dossier_section(
        &input.dossier_id,
        input.ordinal,
        input.cursor.as_deref(),
        input.limit.unwrap_or(50),
    )
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn get_skill_evolution_generation_provenance(
    api: State<'_, SkillEvolutionGenerationApi>,
    job_id: String,
) -> Result<Value, String> {
    api.provenance(&job_id).map_err(command_error)
}

#[tauri::command]
pub(crate) fn query_skill_evolution_generation_quarantine(
    api: State<'_, SkillEvolutionGenerationApi>,
    input: GenerationQueryInput,
) -> Result<Value, String> {
    api.quarantine(
        input.workspace_id.as_deref(),
        input.limit.unwrap_or(20),
        input.cursor.as_deref(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub(crate) fn handoff_skill_evolution_generation_package(
    app: AppHandle,
    api: State<'_, SkillEvolutionGenerationApi>,
    job_id: String,
) -> Result<Value, String> {
    let value = api.handoff(&job_id).map_err(command_error)?;
    emit_notification(&app, "review_ready", &value, None);
    Ok(value)
}

#[tauri::command]
pub(crate) fn export_skill_evolution_generation_dossier(
    app: AppHandle,
    api: State<'_, SkillEvolutionGenerationApi>,
    input: ExportInput,
) -> Result<Value, String> {
    api.export_dossier_to_user_file(
        &app,
        &input.dossier_id,
        &input.format,
        chrono::Utc::now().timestamp_millis(),
    )
    .map_err(command_error)
}

fn command_error(error: GenerationApiError) -> String {
    error.code().to_string()
}

fn emit_notification(app: &AppHandle, kind: &str, value: &Value, prior_job_id: Option<&str>) {
    let job_id = prior_job_id.unwrap_or_else(|| value["jobId"].as_str().unwrap_or("unknown"));
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "eventId": format!("{kind}:{job_id}:{}", value["updatedAt"].as_str().unwrap_or("0")),
        "eventKind": kind,
        "jobId": job_id,
        "workspaceId": value["workspaceId"].as_str().unwrap_or("global"),
        "seedId": value["seedId"].as_str().unwrap_or("unknown"),
        "safeFailureCode": value["safeFailureCode"],
    });
    let _ = app.emit("skill-generation:notification", payload);
}
