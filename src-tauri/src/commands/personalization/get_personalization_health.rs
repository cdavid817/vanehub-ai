use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Whether long-term memory is usable, and how much is waiting for review.
///
/// Answers during maintenance rather than failing: a screen that could not render at all while
/// startup migration runs would be worse than one that says so.
#[tauri::command]
pub(crate) fn get_personalization_health(
    api: State<'_, PersonalizationApi>,
) -> Result<dto::PersonalizationHealthView, CommandError> {
    Ok(mapper::health_to_dto(&api))
}
