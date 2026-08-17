use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_messages(
    api: State<'_, SessionsApi>,
    evidence: State<'_, SkillEvolutionEvidenceApi>,
    session_id: String,
    limit: Option<i64>,
    before_id: Option<String>,
) -> Result<Vec<dto::ChatMessage>, CommandError> {
    let records = api
        .list_messages(&session_id, limit, before_id)
        .map_err(map_command_error)?;
    let ids = mapper::message_ids(&records);
    let feedback = evidence
        .feedback_for_messages(&ids)
        .map_err(|_| CommandError::storage("evidence feedback query failed"))?;
    Ok(mapper::messages_to_dto_with_feedback(records, &feedback))
}
