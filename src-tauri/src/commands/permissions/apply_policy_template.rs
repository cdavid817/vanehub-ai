use super::dto::{ApplyPolicyTemplateInput, PrincipalEntry};
use super::mapper::{parse_template, principal_to_dto};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::permissions::api::PermissionsApi;
use tauri::State;

/// Replaces the legacy `set_agent_tool_trust` boolean toggle
/// (`permissions-approval`'s policy-template picker). Confirm-to-increase-trust is a frontend
/// concern (the caller only invokes this after the user has confirmed, when
/// `PolicyTemplateName::requires_confirmation_to_assign` is `true`) — this command applies the
/// change unconditionally once called.
#[tauri::command]
pub(crate) fn apply_policy_template(
    permissions: State<'_, PermissionsApi>,
    input: ApplyPolicyTemplateInput,
) -> Result<PrincipalEntry, CommandError> {
    let template = parse_template(&input.template).ok_or_else(|| {
        CommandError::validation(format!("unknown policy template: {}", input.template))
    })?;
    let principal = permissions
        .assign_template(&input.agent_id, template)
        .map_err(map_command_error)?;
    Ok(principal_to_dto((principal, true)))
}
