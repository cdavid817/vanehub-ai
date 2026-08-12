use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::{OverlayMutationOperation, SkillApi};
use tauri::State;

#[tauri::command]
pub(crate) fn create_skill_overlay_patch(
    api: State<'_, SkillApi>,
    input: dto::OverlayPatchInput,
) -> Result<dto::OverlayMutationOutcome, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::patch(input)?;
    api.overlay_mutation(
        OverlayMutationOperation::CreatePatch,
        &request,
        workspace.as_deref(),
    )
    .map(overlay_mapper::outcome)
    .map_err(Into::into)
}
