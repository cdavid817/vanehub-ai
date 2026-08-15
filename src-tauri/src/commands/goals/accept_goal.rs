use tauri::State;

use super::dto::{to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

/// Acceptance is the one transition the system never makes on its own. It is
/// rejected unless the goal's children currently derive to awaiting acceptance.
#[tauri::command]
pub(crate) fn accept_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .accept(&goal_id)
        .map(to_dto)
}
