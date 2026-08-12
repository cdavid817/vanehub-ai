use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn disable_skill_overlay_mutation(
    api: State<'_, SkillApi>,
    input: dto::OverlayMutationStateInput,
) -> Result<dto::OverlayMutationOutcome, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let (request, operation) = overlay_mapper::disable_mutation_state(input)?;
    api.overlay_mutation(operation, &request, workspace.as_deref())
        .map(overlay_mapper::outcome)
        .map_err(Into::into)
}
