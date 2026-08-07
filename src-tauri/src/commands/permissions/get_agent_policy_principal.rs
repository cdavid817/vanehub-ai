use super::dto::{GetAgentPolicyPrincipalInput, PrincipalEntry};
use super::mapper::principal_to_dto;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::permissions::api::PermissionsApi;
use tauri::State;

/// The read-side counterpart to `apply_policy_template` (`add-permissions-settings-ui`'s agent
/// policy list) — reports an agent's current template, synthesizing the effective default when
/// none is assigned yet, without ever creating a principal row as a side effect of reading.
#[tauri::command]
pub(crate) fn get_agent_policy_principal(
    permissions: State<'_, PermissionsApi>,
    input: GetAgentPolicyPrincipalInput,
) -> Result<PrincipalEntry, CommandError> {
    permissions
        .find_principal(&input.agent_id)
        .map(principal_to_dto)
        .map_err(map_command_error)
}
