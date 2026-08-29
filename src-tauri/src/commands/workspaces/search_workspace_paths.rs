//! Quick Open, over the provider-neutral seam.
//!
//! Only a session id, a query, a cursor, and a page size cross the boundary. No root, because there
//! is no constructor for a target outside the resolver — so "a caller cannot name a directory to
//! search" is a property of this surface rather than a check somebody has to remember to write.
//!
//! The search id is the caller's, and one panel reuses one id for every keystroke. That is what
//! makes the newest request supersede the ones it replaced under the registry's own lock. The
//! earlier version of this file argued that cancellation was unnecessary because the walk is
//! bounded and finishes on its own — which is true of one walk and false of the thirty a held-down
//! key produces, each holding a blocking thread for an answer nobody is waiting for.

use super::{dto, mapper};
use crate::contexts::workspaces::api::{WorkspaceApi, WorkspacePathSearchRequest};
use tauri::State;

#[tauri::command]
pub(crate) async fn search_workspace_paths(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    query: String,
    search_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::WorkspacePathSearchDto, dto::WorkspaceInspectionErrorDto> {
    search_paths(api.inner(), session_id, query, search_id, cursor, limit).await
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy.
pub(super) async fn search_paths(
    api: &WorkspaceApi,
    session_id: String,
    query: String,
    search_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::WorkspacePathSearchDto, dto::WorkspaceInspectionErrorDto> {
    let result = api
        .search_workspace_paths(
            &session_id,
            WorkspacePathSearchRequest {
                query,
                search_id,
                cursor,
                limit,
            },
        )
        .await
        .map_err(|error| dto::WorkspaceInspectionErrorDto {
            reason_code: error.code().to_string(),
        })?;

    Ok(dto::WorkspacePathSearchDto {
        generation: result.generation,
        coverage: mapper::coverage_to_dto(result.result.coverage),
        matches: result
            .result
            .matches
            .into_iter()
            .map(|entry| dto::WorkspacePathMatchDto {
                name: entry.name,
                path: entry.path,
                kind: entry.kind.to_string(),
            })
            .collect(),
        next_cursor: result.result.next_cursor,
    })
}
