use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Applies one screen's edit to one layer.
///
/// The expected revision is the one the screen rendered from, so a save from a stale copy comes
/// back as a conflict the caller can act on rather than silently reverting another edit.
#[tauri::command]
pub(crate) fn patch_personalization_policy(
    api: State<'_, PersonalizationApi>,
    input: dto::PersonalizationPolicyPatchInput,
) -> Result<dto::PersonalizationPolicyView, CommandError> {
    let scope = mapper::policy_scope(
        &input.scope_kind,
        input.agent_id.as_deref(),
        input.workspace_key.as_deref(),
    )?;
    let patch = mapper::policy_patch(&input)?;
    api.patch_policy(&scope, input.expected_revision, patch)
        .map(|record| mapper::policy_to_dto(&record))
        .map_err(map_command_error)
}
