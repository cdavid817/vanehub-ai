use tauri::State;

use super::dto::{to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn abandon_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .abandon(&goal_id)
        .map(to_dto)
}
