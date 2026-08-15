use tauri::State;

use super::dto::{to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn get_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .get(&goal_id)
        .map(to_dto)
}
