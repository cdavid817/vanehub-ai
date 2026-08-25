//! What a session's workspace can actually be asked, and where it is.
//!
//! The first consumer of the provider-neutral seam, and deliberately the read-only one: a panel has
//! to know which capabilities exist before it renders anything, and a capability answer cannot go
//! wrong in a way that damages a workspace.
//!
//! Only a session id crosses the boundary. There is no root parameter here and no constructor for a
//! target outside the resolver, so the "never accept a frontend-supplied absolute root" rule is a
//! property of the API surface rather than a check somebody has to remember.

use super::dto;
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn get_workspace_inspection_capabilities(
    api: State<'_, WorkspaceApi>,
    session_id: String,
) -> Result<dto::WorkspaceInspectionCapabilitiesDto, dto::WorkspaceInspectionErrorDto> {
    inspection_capabilities(api.inner(), session_id).await
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy.
pub(super) async fn inspection_capabilities(
    api: &WorkspaceApi,
    session_id: String,
) -> Result<dto::WorkspaceInspectionCapabilitiesDto, dto::WorkspaceInspectionErrorDto> {
    // Resolved first so the answer can name the machine even when every capability on it is
    // unavailable: "nothing works here" and "nothing works on that host" are different sentences,
    // and a reader can only act on the second.
    let target =
        api.inspection_target(&session_id)
            .map_err(|error| dto::WorkspaceInspectionErrorDto {
                reason_code: error.code().to_string(),
            })?;
    let capabilities = api
        .inspection_capabilities(&session_id)
        .await
        .map_err(|error| dto::WorkspaceInspectionErrorDto {
            reason_code: error.code().to_string(),
        })?;

    Ok(dto::WorkspaceInspectionCapabilitiesDto {
        provider: capabilities.provider.to_string(),
        // The display name, never the root. A remote path is user-configured text and a local root
        // is an absolute path on this machine; neither belongs in a payload whose job is to say
        // which workspace a panel is showing.
        target_label: target_label(&target),
        list_files: capability_dto(&capabilities.list_files),
        read_text_files: capability_dto(&capabilities.read_text_files),
        search_files: capability_dto(&capabilities.search_files),
        git_status: capability_dto(&capabilities.git_status),
        git_diff: capability_dto(&capabilities.git_diff),
        watch_mode: capabilities.watch_mode.token().to_string(),
    })
}

fn target_label(target: &crate::contexts::workspaces::api::WorkspaceTarget) -> Option<String> {
    match target {
        // A local workspace needs no label: it is this machine, which is what a reader assumes when
        // nothing says otherwise. A remote one has to say so.
        crate::contexts::workspaces::api::WorkspaceTarget::Local(_) => None,
        crate::contexts::workspaces::api::WorkspaceTarget::Remote(remote) => {
            Some(remote.display_name.clone())
        }
    }
}

fn capability_dto(
    state: &crate::contexts::workspaces::api::CapabilityState,
) -> dto::CapabilityStateDto {
    dto::CapabilityStateDto {
        available: state.available,
        reason_code: state.reason_code.map(str::to_string),
        remediation: state.remediation.map(str::to_string),
    }
}
