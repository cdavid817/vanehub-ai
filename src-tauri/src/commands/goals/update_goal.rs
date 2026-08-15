use tauri::State;

use super::dto::{to_dto, GoalDto, GoalInputDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn update_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
    input: GoalInputDto,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .update(&goal_id, input.into_domain())
        .map(to_dto)
}
