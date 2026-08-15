use tauri::State;

use super::dto::{to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn list_goals(database: State<'_, NativeDatabase>) -> Result<Vec<GoalDto>, String> {
    api::build_service(database.inner().clone())
        .list()
        .map(|details| details.into_iter().map(to_dto).collect())
}
