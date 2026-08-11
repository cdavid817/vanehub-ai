use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn reconcile_skill_overlay(
    api: State<'_, SkillApi>,
    input: dto::OverlayReconciliationInput,
) -> Result<dto::OverlayMutationOutcome, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::reconciliation(input)?;
    api.reconcile_overlay(&request, workspace.as_deref())
        .map(overlay_mapper::outcome)
        .map_err(Into::into)
}
