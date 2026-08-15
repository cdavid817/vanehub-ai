use tauri::State;

use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

#[tauri::command]
pub(crate) fn delete_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
) -> Result<(), String> {
    api::build_service(database.inner().clone()).delete(&goal_id)
}
