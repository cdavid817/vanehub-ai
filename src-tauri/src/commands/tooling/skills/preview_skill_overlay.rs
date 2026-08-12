use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_skill_overlay(
    api: State<'_, SkillApi>,
    input: dto::OverlayPreviewInput,
) -> Result<dto::OverlayPreview, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::preview(input)?;
    api.overlay_preview(&request, workspace.as_deref())
        .map(overlay_mapper::preview_to_dto)
        .map_err(Into::into)
}
