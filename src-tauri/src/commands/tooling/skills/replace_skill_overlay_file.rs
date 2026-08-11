use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::{OverlayMutationOperation, SkillApi};
use tauri::State;

#[tauri::command]
pub(crate) fn replace_skill_overlay_file(
    api: State<'_, SkillApi>,
    input: dto::OverlayFileInput,
) -> Result<dto::OverlayMutationOutcome, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let request = overlay_mapper::file(input)?;
    api.overlay_mutation(
        OverlayMutationOperation::ReplaceFile,
        &request,
        workspace.as_deref(),
    )
    .map(overlay_mapper::outcome)
    .map_err(Into::into)
}
