use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// What a reset would remove, and the token that authorises removing exactly that.
///
/// The token names the scope and statuses it was issued for, so a screen cannot preview one scope
/// and delete another. It is returned to the caller and quoted back on execute.
#[tauri::command]
pub(crate) fn preview_personalization_reset(
    api: State<'_, PersonalizationApi>,
    input: dto::ResetScopeInput,
) -> Result<dto::ResetPreviewView, CommandError> {
    let scope = mapper::scope_filter(input.scope_kind.as_deref(), input.workspace_key.as_deref())?;
    let statuses = mapper::reset_statuses(input.include_archived);
    api.preview_memory_reset(&scope, &statuses)
        .map(|preview| mapper::reset_preview_to_dto(&preview))
        .map_err(map_command_error)
}
