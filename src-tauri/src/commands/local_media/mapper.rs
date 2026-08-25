use super::dto::LocalMediaOperationHandleDto;
use crate::contexts::local_media::api::{LocalMediaApi, PreparedLocalMediaWork};
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, PlaybackId,
    RecordingId, StagedInputId,
};

fn invalid(code: LocalMediaErrorCode, field: &str) -> LocalMediaError {
    LocalMediaError::new(code).with_text("field", field)
}

/// Every identifier crossing the IPC boundary is re-parsed here rather than trusted.
///
/// The renderer is not an attacker in the usual sense, but it is the one place a malformed value
/// can enter, and these values become directory names and ownership checks further down.
pub(super) fn parse_scope(value: &str) -> Result<ComposerScopeId, LocalMediaError> {
    ComposerScopeId::parse(value)
        .ok_or_else(|| invalid(LocalMediaErrorCode::EngineUnconfigured, "composerScopeId"))
}

pub(super) fn parse_staged_input(value: &str) -> Result<StagedInputId, LocalMediaError> {
    StagedInputId::parse(value)
        .ok_or_else(|| invalid(LocalMediaErrorCode::InputNotFound, "stagedInputId"))
}

pub(super) fn parse_recording(value: &str) -> Result<RecordingId, LocalMediaError> {
    RecordingId::parse(value)
        .ok_or_else(|| invalid(LocalMediaErrorCode::RecordingNotFound, "recordingId"))
}

/// An absent playback id means "stop whatever is playing"; a present one must be well formed.
pub(super) fn parse_playback(value: Option<&str>) -> Result<Option<PlaybackId>, LocalMediaError> {
    value
        .map(|raw| {
            PlaybackId::parse(raw).ok_or_else(|| {
                invalid(LocalMediaErrorCode::PlaybackDeviceUnavailable, "playbackId")
            })
        })
        .transpose()
}

pub(super) fn parse_engine(value: &str) -> Result<LocalMediaEngine, LocalMediaError> {
    LocalMediaEngine::parse(value)
        .ok_or_else(|| invalid(LocalMediaErrorCode::EngineUnconfigured, "engine"))
}

fn handle_dto(prepared: &PreparedLocalMediaWork) -> LocalMediaOperationHandleDto {
    LocalMediaOperationHandleDto {
        operation_id: prepared.operation_id.clone(),
        kind: prepared.kind.as_str().to_string(),
        accepted_at: prepared.accepted_at.clone(),
    }
}

/// Return the handle immediately and run the work on a blocking pool thread.
///
/// The split lives here rather than in the application layer because `tauri::async_runtime` is a
/// transport concern; the application service exposes `prepare` and `execute` precisely so this is
/// the only place that has to know which of them blocks.
pub(super) fn accept(
    api: &LocalMediaApi,
    prepared: PreparedLocalMediaWork,
) -> LocalMediaOperationHandleDto {
    let handle = handle_dto(&prepared);
    let api = api.clone();
    tauri::async_runtime::spawn_blocking(move || api.execute(prepared));
    handle
}
