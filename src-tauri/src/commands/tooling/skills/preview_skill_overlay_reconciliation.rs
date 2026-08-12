use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn preview_skill_overlay_reconciliation(
    api: State<'_, SkillApi>,
    input: dto::OverlayReconciliationInput,
) -> Result<dto::OverlayReconciliationPreview, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::reconciliation(input)?;
    api.preview_overlay_reconciliation(&request, workspace.as_deref())
        .map(overlay_mapper::reconciliation_preview)
        .map_err(Into::into)
}
