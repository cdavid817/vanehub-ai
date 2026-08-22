mod audio_capture;
mod audio_playback;

/// Serializes every test that touches a real audio host.
///
/// cpal opens the platform backend -- WASAPI on Windows, which initializes COM per thread -- and
/// the full native suite runs thousands of tests across a large thread pool alongside process and
/// PTY tests. One run in three produced a `STATUS_ACCESS_VIOLATION` during teardown before this
/// existed. Holding one host at a time removes the only concurrency dimension this change added,
/// and costs nothing: these tests are a handful and each takes milliseconds.
///
/// The guard tolerates poisoning. A panicking test must fail on its own assertion, not turn every
/// later audio test into an unrelated lock error.
#[cfg(test)]
pub(super) fn audio_host_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
mod persistence;
mod staging;
mod support;
mod workers;

pub(crate) use audio_capture::{CpalAudioCapture, CpalDeviceCatalog};
pub(crate) use audio_playback::RodioPlayback;
pub(crate) use persistence::{apply_schema, SqliteLocalMediaProfileRepository};
pub(crate) use staging::FilesystemMediaTempStore;
pub(crate) use support::{
    resolve_worker_bridge_root, OperationsApiBridge, RandomIdFactory, SystemLocalMediaClock,
    UnifiedLocalMediaDiagnostics,
};
pub(crate) use workers::build_supervisor;
#[cfg(test)]
pub(crate) use workers::LOCAL_MEDIA_WORKER_PROTOCOL;
