use std::path::Path;
use tauri::State;

use super::dto::{
    CleanupStagedRequest, ComposerScopeRequest, LocalMediaOperationHandleDto, OperationRequest,
    ProbeEngineRequest, RecordingRequest, StageOcrSourceRequest, StartOcrRequest, StartTtsRequest,
    StopPlaybackRequest,
};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::local_media::api::LocalMediaApi;
use crate::contexts::local_media::domain::{
    LocalMediaOperationResult, RecordingHandle, StagedOcrSource,
};

#[tauri::command]
pub(crate) fn start_local_media_probe(
    api: State<'_, LocalMediaApi>,
    request: ProbeEngineRequest,
) -> Result<LocalMediaOperationHandleDto, CommandError> {
    let engine = mapper::parse_engine(&request.engine).map_err(map_command_error)?;
    let prepared = api.prepare_probe(engine).map_err(map_command_error)?;
    Ok(mapper::accept(api.inner(), prepared))
}

#[tauri::command]
pub(crate) fn stage_local_media_ocr_source(
    api: State<'_, LocalMediaApi>,
    request: StageOcrSourceRequest,
) -> Result<StagedOcrSource, CommandError> {
    api.stage_ocr_source(Path::new(&request.path))
        .map_err(map_command_error)
}

/// The file a fixture picker "chose".
///
/// Exists only under `desktop-e2e`. It replaces exactly one thing -- which path the system dialog
/// would have returned -- and the caller still passes that path to the real
/// `stage_local_media_ocr_source`, so admission, staging, limits and cleanup are unchanged.
/// Failing rather than falling back matters: a headless runner has nobody to answer a real dialog,
/// so a fallback would hang instead of reporting a misconfiguration.
#[cfg(feature = "desktop-e2e")]
#[tauri::command]
pub(crate) fn fixture_local_media_ocr_source() -> Result<String, CommandError> {
    crate::contexts::local_media::infrastructure::fixtures::fixture_ocr_source()
        .map(|path| path.to_string_lossy().to_string())
        .ok_or_else(|| {
            CommandError::stable_code(
                crate::commands::error::CommandErrorCategory::Unavailable,
                "FIXTURE_OCR_SOURCE_UNAVAILABLE",
            )
        })
}

#[tauri::command]
pub(crate) fn start_local_media_ocr(
    api: State<'_, LocalMediaApi>,
    request: StartOcrRequest,
) -> Result<LocalMediaOperationHandleDto, CommandError> {
    let staged = mapper::parse_staged_input(&request.staged_input_id).map_err(map_command_error)?;
    let scope = mapper::parse_scope(&request.composer_scope_id).map_err(map_command_error)?;
    let prepared = api
        .prepare_ocr(&staged, Some(scope))
        .map_err(map_command_error)?;
    Ok(mapper::accept(api.inner(), prepared))
}

#[tauri::command]
pub(crate) fn cleanup_local_media_staged_source(
    api: State<'_, LocalMediaApi>,
    request: CleanupStagedRequest,
) -> Result<(), CommandError> {
    let staged = mapper::parse_staged_input(&request.staged_input_id).map_err(map_command_error)?;
    api.cleanup_staged(&staged);
    Ok(())
}

#[tauri::command]
pub(crate) fn start_microphone_recording(
    api: State<'_, LocalMediaApi>,
    request: ComposerScopeRequest,
) -> Result<RecordingHandle, CommandError> {
    let scope = mapper::parse_scope(&request.composer_scope_id).map_err(map_command_error)?;
    api.start_recording(scope).map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn stop_recording_and_transcribe(
    api: State<'_, LocalMediaApi>,
    request: RecordingRequest,
) -> Result<LocalMediaOperationHandleDto, CommandError> {
    let recording = mapper::parse_recording(&request.recording_id).map_err(map_command_error)?;
    let scope = mapper::parse_scope(&request.composer_scope_id).map_err(map_command_error)?;
    let prepared = api
        .prepare_transcription(&recording, scope)
        .map_err(map_command_error)?;
    Ok(mapper::accept(api.inner(), prepared))
}

#[tauri::command]
pub(crate) fn cancel_microphone_recording(
    api: State<'_, LocalMediaApi>,
    request: RecordingRequest,
) -> Result<(), CommandError> {
    let recording = mapper::parse_recording(&request.recording_id).map_err(map_command_error)?;
    let scope = mapper::parse_scope(&request.composer_scope_id).map_err(map_command_error)?;
    api.cancel_recording(&recording, &scope)
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn start_local_media_tts(
    api: State<'_, LocalMediaApi>,
    request: StartTtsRequest,
) -> Result<LocalMediaOperationHandleDto, CommandError> {
    let scope = mapper::parse_scope(&request.composer_scope_id).map_err(map_command_error)?;
    let prepared = api
        .prepare_tts(request.text, scope)
        .map_err(map_command_error)?;
    Ok(mapper::accept(api.inner(), prepared))
}

#[tauri::command]
pub(crate) fn stop_local_media_playback(
    api: State<'_, LocalMediaApi>,
    request: StopPlaybackRequest,
) -> Result<(), CommandError> {
    let playback =
        mapper::parse_playback(request.playback_id.as_deref()).map_err(map_command_error)?;
    api.stop_playback(playback.as_ref());
    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_local_media_operation(
    api: State<'_, LocalMediaApi>,
    request: OperationRequest,
) -> Result<(), CommandError> {
    api.cancel_operation(&request.operation_id);
    Ok(())
}

/// `Ok(None)` while the operation is still running; an expired result is an error rather than
/// `None`, so the composer can stop polling and say the result was dropped.
#[tauri::command]
pub(crate) fn get_local_media_operation_result(
    api: State<'_, LocalMediaApi>,
    request: OperationRequest,
) -> Result<Option<LocalMediaOperationResult>, CommandError> {
    api.get_operation_result(&request.operation_id)
        .map_err(map_command_error)
}
