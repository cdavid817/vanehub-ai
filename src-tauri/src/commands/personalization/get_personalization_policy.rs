use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// One layer, or nothing when that layer holds nothing.
#[tauri::command]
pub(crate) fn get_personalization_policy(
    api: State<'_, PersonalizationApi>,
    scope_kind: String,
    agent_id: Option<String>,
    workspace_key: Option<String>,
) -> Result<Option<dto::PersonalizationPolicyView>, CommandError> {
    let scope = mapper::policy_scope(&scope_kind, agent_id.as_deref(), workspace_key.as_deref())?;
    api.policy(&scope)
        .map(|record| record.as_ref().map(mapper::policy_to_dto))
        .map_err(map_command_error)
}
