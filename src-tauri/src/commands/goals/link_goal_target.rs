use tauri::State;

use super::dto::{parse_target_kind, to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn link_goal_target(
    database: State<'_, NativeDatabase>,
    goal_id: String,
    target_kind: String,
    target_id: String,
) -> Result<GoalDto, String> {
    let kind = parse_target_kind(&target_kind)?;
    api::build_service(database.inner().clone())
        .link(&goal_id, kind, &target_id)
        .map(to_dto)
}
