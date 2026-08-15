use tauri::State;

use super::dto::{to_dto, GoalDto, GoalInputDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn create_goal(
    database: State<'_, NativeDatabase>,
    input: GoalInputDto,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .create(input.into_domain())
        .map(to_dto)
}
