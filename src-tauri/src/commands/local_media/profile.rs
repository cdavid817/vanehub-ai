use tauri::State;

use super::dto::{SaveLocalMediaProfileRequest, ValidateLocalMediaProfileRequest};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::local_media::api::LocalMediaApi;
use crate::contexts::local_media::domain::{
    AudioDeviceCatalog, LocalMediaProfile, LocalMediaRuntimeStatus, ProfileFieldIssue,
};

#[tauri::command]
pub(crate) fn get_local_media_profile(
    api: State<'_, LocalMediaApi>,
) -> Result<LocalMediaProfile, CommandError> {
    api.get_profile().map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn save_local_media_profile(
    api: State<'_, LocalMediaApi>,
    request: SaveLocalMediaProfileRequest,
) -> Result<LocalMediaProfile, CommandError> {
    api.save_profile(request.profile, request.expected_revision)
        .map_err(map_command_error)
}

/// Field-level validation for the settings form.
///
/// Separate from `save` because the command error contract serializes to a bare string: a save
/// failure can carry the stable code but not which input to highlight. The page calls this to get
/// the per-field answer and `save` to commit.
#[tauri::command]
pub(crate) fn validate_local_media_profile(
    api: State<'_, LocalMediaApi>,
    request: ValidateLocalMediaProfileRequest,
) -> Result<Vec<ProfileFieldIssue>, CommandError> {
    Ok(api.validate_profile(&request.profile))
}

#[tauri::command]
pub(crate) fn get_local_media_status(
    api: State<'_, LocalMediaApi>,
) -> Result<LocalMediaRuntimeStatus, CommandError> {
    api.get_status().map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn list_local_media_audio_devices(
    api: State<'_, LocalMediaApi>,
) -> Result<AudioDeviceCatalog, CommandError> {
    api.list_audio_devices().map_err(map_command_error)
}
