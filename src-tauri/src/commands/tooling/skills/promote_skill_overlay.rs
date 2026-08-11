use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn promote_skill_overlay(
    api: State<'_, SkillApi>,
    input: dto::OverlayPromotionInput,
) -> Result<dto::OverlayMutationOutcome, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::promotion(input)?;
    api.promote_overlay(&request, workspace.as_deref())
        .map(overlay_mapper::outcome)
        .map_err(Into::into)
}
