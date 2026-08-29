//! Quick Open, over the provider-neutral seam.
//!
//! Only a session id, a query, a cursor, and a page size cross the boundary. No root, because there
//! is no constructor for a target outside the resolver — so "a caller cannot name a directory to
//! search" is a property of this surface rather than a check somebody has to remember to write.
//!
//! Cancellation is not a parameter here and cannot be. A Tauri command is one round trip; a reader
//! who keeps typing abandons the answer to the previous keystroke on the frontend side, and the
//! bounded walk on this side finishes on its own. Adding a cancellation token would be adding a
//! mechanism for stopping something that stops anyway.

use super::dto;
use crate::contexts::workspaces::api::{WorkspaceApi, WorkspacePathSearchRequest};
use tauri::State;

#[tauri::command]
pub(crate) async fn search_workspace_paths(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    query: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::WorkspacePathSearchDto, dto::WorkspaceInspectionErrorDto> {
    search_paths(api.inner(), session_id, query, cursor, limit).await
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy.
pub(super) async fn search_paths(
    api: &WorkspaceApi,
    session_id: String,
    query: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::WorkspacePathSearchDto, dto::WorkspaceInspectionErrorDto> {
    let result = api
        .search_workspace_paths(
            &session_id,
            WorkspacePathSearchRequest {
                query,
                cursor,
                limit,
            },
        )
        .await
        .map_err(|error| dto::WorkspaceInspectionErrorDto {
            reason_code: error.code().to_string(),
        })?;

    Ok(dto::WorkspacePathSearchDto {
        coverage: dto::WorkspaceSearchCoverageDto {
            state: result.coverage.state.token().to_string(),
            reason_code: result.coverage.reason_code.map(str::to_string),
        },
        matches: result
            .matches
            .into_iter()
            .map(|entry| dto::WorkspacePathMatchDto {
                name: entry.name,
                path: entry.path,
                kind: entry.kind.to_string(),
            })
            .collect(),
        next_cursor: result.next_cursor,
    })
}
