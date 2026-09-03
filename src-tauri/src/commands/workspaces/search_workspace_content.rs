//! Content search, and the command that stops one.
//!
//! Two commands rather than one with a mode, because they run at the same time: the whole point of
//! cancelling is that the search is still going, so the cancel has to reach the process while the
//! search command has not returned.
//!
//! The search id comes from the caller. An id this side generated would arrive with the answer,
//! which is exactly too late to cancel the search that produced it.

use super::{dto, mapper};
use crate::contexts::workspaces::api::{WorkspaceApi, WorkspaceContentSearchRequest};
use tauri::State;

#[tauri::command]
pub(crate) async fn search_workspace_content(
    api: State<'_, WorkspaceApi>,
    session_id: String,
    query: String,
    search_id: String,
    limit: Option<usize>,
) -> Result<dto::WorkspaceContentSearchDto, dto::WorkspaceInspectionErrorDto> {
    search_content(api.inner(), session_id, query, search_id, limit).await
}

/// Stops a running search.
///
/// Answers whether one was running rather than refusing when none was. A caller cannot know whether
/// their cancel beat the search's own completion, and turning that ordinary race into an error
/// would put a failure on screen for a keystroke that worked exactly as intended.
#[tauri::command]
pub(crate) async fn cancel_workspace_search(
    api: State<'_, WorkspaceApi>,
    search_id: String,
) -> Result<bool, dto::WorkspaceInspectionErrorDto> {
    Ok(api.cancel_workspace_search(&search_id))
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy.
pub(super) async fn search_content(
    api: &WorkspaceApi,
    session_id: String,
    query: String,
    search_id: String,
    limit: Option<usize>,
) -> Result<dto::WorkspaceContentSearchDto, dto::WorkspaceInspectionErrorDto> {
    let result = api
        .search_workspace_content(
            &session_id,
            WorkspaceContentSearchRequest {
                query,
                search_id,
                limit,
            },
        )
        .await
        .map_err(|error| dto::WorkspaceInspectionErrorDto {
            reason_code: error.code().to_string(),
        })?;

    Ok(dto::WorkspaceContentSearchDto {
        generation: result.generation,
        coverage: mapper::coverage_to_dto(result.result.coverage),
        matches: result
            .result
            .matches
            .into_iter()
            .map(|entry| dto::WorkspaceContentMatchDto {
                path: entry.path,
                line: entry.line,
                column: entry.column,
                snippet: entry.snippet,
                snippet_truncated: entry.snippet_truncated,
            })
            .collect(),
    })
}
