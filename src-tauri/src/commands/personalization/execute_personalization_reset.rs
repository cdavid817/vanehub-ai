use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::personalization::domain::{ResetConfirmationToken, ResetMemoryRequest};
use chrono::Utc;
use tauri::State;

/// Deletes what the preview counted, and only that.
///
/// The token is quoted back from the preview and names the scope and statuses it was issued for,
/// so a screen cannot preview one scope and delete another. The typed phrase is checked as well: a
/// token is something a screen holds, while the phrase is something the user typed.
#[tauri::command]
pub(crate) fn execute_personalization_reset(
    api: State<'_, PersonalizationApi>,
    input: dto::ResetScopeInput,
    confirmation_token: String,
    typed_phrase: String,
) -> Result<dto::MaintenanceResultView, CommandError> {
    let scope = mapper::scope_filter(input.scope_kind.as_deref(), input.workspace_key.as_deref())?;
    let statuses = mapper::reset_statuses(input.include_archived);
    let request = ResetMemoryRequest {
        token: ResetConfirmationToken {
            value: confirmation_token,
            // The issuing time is the caller's claim, so it is not what expiry is judged against;
            // the domain compares its own TTL against the clock it was given.
            issued_at: Utc::now(),
            scope: scope.clone(),
            statuses: statuses.clone(),
        },
        scope,
        statuses,
        typed_phrase,
    };
    api.reset_memories(&request)
        .map(mapper::reset_outcome_to_dto)
        .map_err(map_command_error)
}
