use tauri::State;

use super::dto::CliEnvironmentSnapshotDto;
use super::error::{command_error, CliEnvironmentCommandError};
use super::mapper;
use crate::contexts::tooling::cli::api::CliEnvironmentApi;

/// Persisted snapshots for every registered CLI.
///
/// Direct rather than operation-backed: it reads storage and computes the environment fingerprint,
/// and starts no process, no probe, and no network request. Returning an operation id here would
/// make the caller poll for something already known.
#[tauri::command]
pub(crate) fn list_cli_environments(
    api: State<'_, CliEnvironmentApi>,
) -> Result<Vec<CliEnvironmentSnapshotDto>, CliEnvironmentCommandError> {
    api.list_environments()
        .map(|snapshots| snapshots.into_iter().map(mapper::snapshot_to_dto).collect())
        .map_err(command_error)
}
