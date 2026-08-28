use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::personalization::api::PersonalizationApi;
use tauri::State;

/// Removes one memory and everything derived from it.
///
/// Reports per surface rather than as one boolean: the file, the projection row, the index line
/// and the retrieval entry fail independently, and a partial delete must say so rather than leave
/// a memory recallable that the user believes is gone.
#[tauri::command]
pub(crate) fn delete_personalization_memory(
    api: State<'_, PersonalizationApi>,
    memory_id: String,
    expected_revision: Option<u64>,
) -> Result<dto::MaintenanceResultView, CommandError> {
    let id = mapper::memory_id(&memory_id)?;
    api.delete_memory(&id, expected_revision)
        .map(|outcome| dto::MaintenanceResultView {
            matched: usize::from(outcome.deleted_file),
            deleted_files: usize::from(outcome.deleted_file),
            removed_projection_rows: usize::from(outcome.deleted_projection_row),
            revoked_retrieval_entries: usize::from(outcome.revoked_retrieval_entry),
            quarantined: 0,
            failures: outcome
                .failures
                .iter()
                .map(|failure| failure.phase.as_str().to_string())
                .collect(),
        })
        .map_err(map_command_error)
}
