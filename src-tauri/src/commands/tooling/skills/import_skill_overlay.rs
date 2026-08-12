use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn import_skill_overlay(
    api: State<'_, SkillApi>,
    input: dto::OverlayImportInput,
) -> Result<dto::OverlayImportReview, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::import(input)?;
    api.import_overlay(&request, workspace.as_deref())
        .map(overlay_mapper::import_review)
        .map_err(Into::into)
}
