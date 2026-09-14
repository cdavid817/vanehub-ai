use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::retrieval::api::RetrievalApi;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

/// What one Agent would actually resolve to, rendered for a person.
///
/// Two kinds of preview, told apart by whether the session id names a stored session. A bound
/// session supplies its own Agent, mode and workspace from the owner, so the caller's fields
/// cannot describe a session that does not exist; a hypothetical preview uses the caller's fields
/// and establishes no runtime authority. Both render from the same resolver the runtime uses.
#[tauri::command]
pub(crate) fn preview_effective_personalization(
    api: State<'_, PersonalizationApi>,
    sessions: State<'_, SessionsApi>,
    retrieval: State<'_, RetrievalApi>,
    input: dto::EffectivePreviewInput,
) -> Result<dto::EffectivePreviewView, CommandError> {
    let stored = sessions.find(&input.session_id).ok().flatten();
    let resolution = mapper::preview_resolution(&api, input, stored.as_ref())?;
    let preview = api.preview(resolution.request).map_err(map_command_error)?;
    let channel = mapper::preview_channel(
        &api,
        retrieval.is_configured(),
        resolution.kind,
        &resolution.agent_id,
        &preview,
    );
    Ok(mapper::preview_to_dto(preview, channel))
}
