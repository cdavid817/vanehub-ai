//! What is retained about one file, so a panel knows whether to offer a link.
//!
//! Only a session id and a workspace-relative path cross the boundary. The digest the journal
//! stores is computed on this side from the session's own root, which is the whole reason a caller
//! cannot ask about a file in somebody else's workspace: they would have to know a root, and there
//! is nowhere to supply one.

use super::dto;
use crate::bootstrap::SessionFileEvidence;
use crate::commands::error::{map_command_error, CommandError};
use tauri::State;

#[tauri::command]
pub(crate) async fn get_file_evidence_links(
    api: State<'_, SessionFileEvidence>,
    session_id: String,
    relative_path: String,
) -> Result<dto::FileEvidenceLinksDto, CommandError> {
    let links = api
        .links_for(&session_id, &relative_path)
        .map_err(CommandError::validation)
        .map_err(map_command_error)?;
    Ok(dto::FileEvidenceLinksDto {
        observations: links.observations,
        run_ids: links.run_ids,
        command_ids: links.command_ids,
        truncated: links.truncated,
    })
}
