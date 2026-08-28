use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// What one Agent would actually resolve to, rendered for a person.
///
/// Built from the same snapshot the runtime would get, so the screen cannot drift from the
/// behaviour, minus the surfaces a screen must not carry.
#[tauri::command]
pub(crate) fn preview_effective_personalization(
    api: State<'_, PersonalizationApi>,
    input: dto::EffectivePreviewInput,
) -> Result<dto::EffectivePreviewView, CommandError> {
    let request = mapper::resolution_request(input)?;
    api.preview(request)
        .map(mapper::preview_to_dto)
        .map_err(map_command_error)
}
