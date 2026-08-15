use tauri::State;

use super::dto::{to_dto, GoalDto};
use crate::contexts::goals::api;
use crate::platform::database::NativeDatabase;

/// Moves a draft or abandoned goal into active work. Reopening an achieved
/// goal is the same transition but has its own command so the UI intent stays
/// legible at the call site.
#[tauri::command]
pub(crate) fn activate_goal(
    database: State<'_, NativeDatabase>,
    goal_id: String,
) -> Result<GoalDto, String> {
    api::build_service(database.inner().clone())
        .activate(&goal_id)
        .map(to_dto)
}
