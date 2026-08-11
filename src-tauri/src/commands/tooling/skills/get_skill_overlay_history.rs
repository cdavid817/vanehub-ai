use super::{overlay_dto as dto, overlay_error::OverlayCommandError, overlay_mapper};
use crate::contexts::tooling::skills::api::{OverlayHistoryQuery, OverlayKey, SkillApi};
use tauri::State;

#[tauri::command]
pub(crate) fn get_skill_overlay_history(
    api: State<'_, SkillApi>,
    input: dto::OverlayHistoryInput,
) -> Result<dto::OverlayHistoryPage, OverlayCommandError> {
    let workspace = input.target.workspace_path.clone();
    let (canonical_skill_id, scope, workspace_identity) = overlay_mapper::target(input.target)?;
    let key = OverlayKey {
        canonical_skill_id,
        scope,
        workspace_identity,
    };
    let query = OverlayHistoryQuery::bounded(input.cursor, input.limit, 100);
    api.overlay_history(&key, workspace.as_deref(), &query)
        .map(overlay_mapper::history)
        .map_err(Into::into)
}
