//! The published local-media contract.
//!
//! Two audiences, one surface. The command layer uses the full set; another native context uses
//! only the artifact-OCR pair. Neither sees a worker handle, a Python path, a profile repository,
//! or a temp path -- that exclusion is what makes "exactly one PaddleOCR runtime owner" a
//! structural property rather than a convention.

use std::path::Path;

use super::application::{
    LocalMediaApplicationService, LocalMediaDependencies, PreparedLocalMediaOperation,
};
use super::domain::{
    AudioDeviceCatalog, ComposerScopeId, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode,
    LocalMediaOperationResult, LocalMediaProfile, LocalMediaRuntimeStatus, OcrResult, PlaybackId,
    ProfileFieldIssue, PythonEnvironmentDiscovery, RecordingHandle, RecordingId, StagedInputId,
    StagedOcrSource,
};

pub(crate) use super::application::PreparedLocalMediaOperation as PreparedLocalMediaWork;
#[cfg(test)]
pub(crate) use super::domain::OcrLine;
pub(crate) use super::domain::{OcrPage, OcrResult as SharedOcrResult};
#[cfg(test)]
pub(crate) use super::infrastructure::LOCAL_MEDIA_WORKER_PROTOCOL;

#[derive(Clone)]
pub(crate) struct LocalMediaApi {
    service: LocalMediaApplicationService,
}

impl LocalMediaApi {
    pub(crate) fn new(dependencies: LocalMediaDependencies) -> Self {
        Self {
            service: LocalMediaApplicationService::new(dependencies),
        }
    }

    // ------------------------------------------------------------- profile ---

    pub(crate) fn get_profile(&self) -> Result<LocalMediaProfile, LocalMediaError> {
        self.service.get_profile()
    }

    pub(crate) fn save_profile(
        &self,
        profile: LocalMediaProfile,
        expected_revision: i64,
    ) -> Result<LocalMediaProfile, LocalMediaError> {
        self.service.save_profile(profile, expected_revision)
    }

    pub(crate) fn validate_profile(&self, profile: &LocalMediaProfile) -> Vec<ProfileFieldIssue> {
        self.service.validate(profile)
    }

    pub(crate) fn get_status(&self) -> Result<LocalMediaRuntimeStatus, LocalMediaError> {
        self.service.get_status()
    }

    pub(crate) fn list_audio_devices(&self) -> Result<AudioDeviceCatalog, LocalMediaError> {
        self.service.list_audio_devices()
    }

    pub(crate) fn discover_python_environments(
        &self,
    ) -> Result<PythonEnvironmentDiscovery, LocalMediaError> {
        self.service.discover_python_environments()
    }

    // ---------------------------------------------------------- operations ---

    pub(crate) fn prepare_probe(
        &self,
        engine: LocalMediaEngine,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        self.service.prepare_probe(engine)
    }

    pub(crate) fn stage_ocr_source(
        &self,
        source: &Path,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        self.service.stage_ocr_source(source)
    }

    pub(crate) fn prepare_ocr(
        &self,
        staged_input_id: &StagedInputId,
        composer_scope: Option<ComposerScopeId>,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        self.service.prepare_ocr(staged_input_id, composer_scope)
    }

    pub(crate) fn start_recording(
        &self,
        composer_scope: ComposerScopeId,
    ) -> Result<RecordingHandle, LocalMediaError> {
        self.service.start_recording(composer_scope)
    }

    pub(crate) fn prepare_transcription(
        &self,
        recording_id: &RecordingId,
        composer_scope: ComposerScopeId,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        self.service
            .prepare_transcription(recording_id, composer_scope)
    }

    pub(crate) fn cancel_recording(
        &self,
        recording_id: &RecordingId,
        composer_scope: &ComposerScopeId,
    ) -> Result<(), LocalMediaError> {
        self.service.cancel_recording(recording_id, composer_scope)
    }

    pub(crate) fn prepare_tts(
        &self,
        text: String,
        composer_scope: ComposerScopeId,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        self.service.prepare_tts(text, composer_scope)
    }

    pub(crate) fn stop_playback(&self, playback_id: Option<&PlaybackId>) {
        self.service.stop_playback(playback_id)
    }

    pub(crate) fn cancel_operation(&self, operation_id: &str) {
        self.service.cancel_operation(operation_id)
    }

    pub(crate) fn get_operation_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<LocalMediaOperationResult>, LocalMediaError> {
        self.service.get_operation_result(operation_id)
    }

    pub(crate) fn cleanup_staged(&self, staged_input_id: &StagedInputId) {
        self.service.cleanup_staged(staged_input_id)
    }

    /// Run the accepted work. Called on a background thread by the command layer, which is the
    /// only layer allowed to know about `tauri::async_runtime`.
    pub(crate) fn execute(&self, prepared: PreparedLocalMediaOperation) {
        self.service.execute(prepared)
    }

    // ------------------------------------------------------------ lifecycle ---

    pub(crate) fn sweep_stale_media(&self) {
        self.service.sweep_stale_media()
    }

    pub(crate) fn shutdown(&self) {
        self.service.shutdown()
    }

    // ------------------------------------------------- cross-context (OCR) ---

    /// Admit a managed artifact's bytes and start OCR against the shared PaddleOCR runtime.
    ///
    /// Bytes, not a path. Sharing the composer's runtime must not add a host-path parameter to the
    /// OnePiece tool schema, and the surest way to guarantee that is for this entry point to be
    /// incapable of accepting one.
    pub(crate) fn prepare_artifact_ocr(
        &self,
        bytes: &[u8],
        display_name: &str,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        let staged = self.service.stage_ocr_artifact(bytes, display_name)?;
        self.service.prepare_ocr(&staged.staged_input_id, None)
    }

    /// Read a completed OCR result. Returns `Ok(None)` while the operation is still running.
    pub(crate) fn get_ocr_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<OcrResult>, LocalMediaError> {
        match self.service.get_operation_result(operation_id)? {
            Some(LocalMediaOperationResult::Ocr(result)) => Ok(Some(result)),
            Some(_) => Err(LocalMediaError::new(
                LocalMediaErrorCode::WorkerProtocolError,
            )),
            None => Ok(None),
        }
    }
}
