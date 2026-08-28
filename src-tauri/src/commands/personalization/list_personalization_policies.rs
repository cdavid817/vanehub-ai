use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Every stored policy layer.
///
/// Only layers that exist. A layer with no row inherits, and inventing an empty one for it would
/// show the user an override they never created.
#[tauri::command]
pub(crate) fn list_personalization_policies(
    api: State<'_, PersonalizationApi>,
) -> Result<Vec<dto::PersonalizationPolicyView>, CommandError> {
    api.all_policies()
        .map(|records| records.iter().map(mapper::policy_to_dto).collect())
        .map_err(map_command_error)
}
