use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_skill_overlay_summary(
    api: State<'_, SkillApi>,
    input: dto::OverlayTargetInput,
) -> Result<dto::OverlaySummary, OverlayCommandError> {
    let workspace = input.workspace_path.clone();
    let (skill_id, _, _) = overlay_mapper::target(input)?;
    api.overlay_summary(&skill_id, workspace.as_deref())
        .map(overlay_mapper::summary)
        .map_err(Into::into)
}
