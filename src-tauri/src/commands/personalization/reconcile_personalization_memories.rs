use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Rebuilds derived state from the authoritative files.
///
/// The repair path: what it reports is what it found out of step, per surface, so an operator can
/// tell "nothing was wrong" from "something was and is now fixed".
#[tauri::command]
pub(crate) fn reconcile_personalization_memories(
    api: State<'_, PersonalizationApi>,
) -> Result<dto::MaintenanceResultView, CommandError> {
    api.reconcile_memories()
        .map(mapper::reconcile_to_dto)
        .map_err(map_command_error)
}
