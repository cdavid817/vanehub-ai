use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// The stable key a workspace scope is addressed by.
///
/// `None` means the input described no workspace at all, which is different from a workspace this
/// build could not identify: the picker offers nothing to select in the first case, and the caller
/// would otherwise store a policy under a key nothing resolves to.
#[tauri::command]
pub(crate) fn resolve_personalization_workspace(
    api: State<'_, PersonalizationApi>,
    input: dto::WorkspaceScopeInput,
) -> Result<Option<dto::WorkspaceScopeView>, CommandError> {
    api.resolve_workspace(&mapper::workspace_request(input))
        .map(|identity| identity.as_ref().map(mapper::workspace_to_dto))
        .map_err(map_command_error)
}
