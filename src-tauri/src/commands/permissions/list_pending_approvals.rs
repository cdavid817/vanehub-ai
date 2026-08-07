use super::dto::PendingApprovalEntry;
use super::mapper::pending_approval_to_dto;
use crate::commands::error::CommandError;
use crate::contexts::permissions::api::PermissionsApi;
use tauri::State;

/// The frontend pull side of `permissions-approval`'s push-and-pull reconciliation
/// (design.md D7) — called on mount/reconnect to recover from a missed `permission://request`
/// event. Infallible (a pure in-memory read) but still returns `Result` to match this codebase's
/// command-boundary convention.
#[tauri::command]
pub(crate) fn list_pending_approvals(
    permissions: State<'_, PermissionsApi>,
) -> Result<Vec<PendingApprovalEntry>, CommandError> {
    Ok(permissions
        .list_pending_approvals()
        .into_iter()
        .map(pending_approval_to_dto)
        .collect())
}
