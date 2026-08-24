//! Hold-to-talk: native capture, then one whole-utterance transcription.
//!
//! There is no partial transcript and no draft snapshot. The operation carries only the composer
//! scope; what the transcript is appended to is decided by the frontend when the result arrives,
//! against whatever the draft is *then*.

use super::cleanup::OperationMediaGuard;
use super::service::{LocalMediaApplicationService, LocalMediaJob, PreparedLocalMediaOperation};
use super::worker_contract::{SttWorkerRequest, WorkerCall, WorkerReply};
use crate::contexts::local_media::application::ports::StartCaptureRequest;
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode,
    LocalMediaOperationKind, LocalMediaOperationResult, LocalMediaPhase, LocalMediaProfileSnapshot,
    RecordingHandle, RecordingId, RecordingOutcome, RecordingSummary, TranscriptionProvenance,
    TranscriptionResult,
};

impl LocalMediaApplicationService {
    pub(crate) fn start_recording(
        &self,
        composer_scope: ComposerScopeId,
    ) -> Result<RecordingHandle, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Stt, &profile)?;

        let recording_id = RecordingId::new(self.inner.ids.next(RecordingId::PREFIX));
        let max_duration_ms = u64::from(profile.recording_limit_seconds()) * 1_000;
        let destination = self.inner.temp.authorize_recording_wav(&recording_id)?;

        // Claim the singleton slot before opening the device so two presses cannot both reach
        // `cpal`. Releasing it on failure is the reason this is a scoped block rather than a guard.
        {
            let Ok(mut active) = self.inner.active_recording.lock() else {
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::AudioCaptureStartFailed,
                ));
            };
            if active.is_some() {
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::RecordingAlreadyActive,
                ));
            }
            *active = Some(RecordingSummary {
                recording_id: recording_id.clone(),
                composer_scope: composer_scope.clone(),
                started_at_ms: self.inner.clock.now_ms(),
                max_duration_ms,
            });
        }

        let started = self.inner.capture.start(StartCaptureRequest {
            recording_id: recording_id.clone(),
            device_id: profile.stt.microphone_device_id.clone(),
            max_duration_ms,
            destination,
        });

        match started {
            Ok(sample_rate) => {
                self.inner.diagnostics.record(
                    "recording.started",
                    &[("sampleRate", sample_rate.to_string())],
                );
                Ok(RecordingHandle {
                    recording_id,
                    started_at: self.inner.clock.now_iso(),
                    max_duration_ms,
                })
            }
            Err(error) => {
                self.release_recording_slot();
                self.inner.temp.cleanup_recording(&recording_id);
                self.inner.diagnostics.record(
                    "recording.start_failed",
                    &[("code", error.code().as_str().to_string())],
                );
                Err(error)
            }
        }
    }

    pub(crate) fn cancel_recording(
        &self,
        recording_id: &RecordingId,
        composer_scope: &ComposerScopeId,
    ) -> Result<(), LocalMediaError> {
        self.owned_recording(recording_id, composer_scope)?;
        self.inner.capture.cancel(recording_id);
        self.release_recording_slot();
        self.inner.temp.cleanup_recording(recording_id);
        self.inner.diagnostics.record("recording.cancelled", &[]);
        Ok(())
    }

    pub(crate) fn prepare_transcription(
        &self,
        recording_id: &RecordingId,
        composer_scope: ComposerScopeId,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        self.owned_recording(recording_id, &composer_scope)?;
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Stt, &profile)?;

        let (operation_id, snapshot, accepted_at) = self.accept_operation(
            LocalMediaOperationKind::Stt,
            LocalMediaEngine::Stt,
            &profile,
            Some(composer_scope),
        )?;

        Ok(PreparedLocalMediaOperation {
            operation_id,
            kind: LocalMediaOperationKind::Stt,
            accepted_at,
            job: LocalMediaJob::Stt {
                snapshot,
                recording_id: recording_id.clone(),
            },
        })
    }

    pub(super) fn run_stt(
        &self,
        operation_id: &str,
        snapshot: &LocalMediaProfileSnapshot,
        recording_id: &RecordingId,
    ) {
        let guard = OperationMediaGuard::new(self.inner.temp.clone(), operation_id);
        let now_ms = self.inner.clock.now_ms();

        self.advance(operation_id, LocalMediaPhase::FinalizingRecording);
        let committed = self.inner.capture.finish(recording_id);
        self.release_recording_slot();

        let committed = match committed {
            Ok(committed) => committed,
            Err(error) => {
                self.inner.temp.cleanup_recording(recording_id);
                self.settle_failure(operation_id, error, now_ms);
                return;
            }
        };

        let max_duration_ms = u64::from(snapshot.stt().max_recording_seconds) * 1_000;
        let outcome = RecordingOutcome::evaluate(committed.duration_ms, max_duration_ms);
        let RecordingOutcome::Committed { limit_reached } = outcome else {
            // A tap rather than a hold. No inference runs and the partial WAV goes away
            // immediately; the composer shows this as an outcome, not an error.
            self.inner.temp.cleanup_recording(recording_id);
            self.settle_failure(
                operation_id,
                LocalMediaError::new(LocalMediaErrorCode::RecordingTooShort)
                    .with_number("durationMs", committed.duration_ms as i64),
                now_ms,
            );
            return;
        };

        let audio_path = match self.inner.temp.authorize_recording_wav(recording_id) {
            Ok(path) => path,
            Err(error) => {
                self.inner.temp.cleanup_recording(recording_id);
                self.settle_failure(operation_id, error, now_ms);
                return;
            }
        };

        self.advance(operation_id, LocalMediaPhase::Queued);
        let flag = self.inner.operations.cancellation_flag(operation_id);
        self.advance(operation_id, LocalMediaPhase::Processing);
        let outcome = self.inner.workers.call(
            snapshot,
            WorkerCall::Transcribe(SttWorkerRequest {
                audio_path,
                // A real utterance: the user's own voice-activity setting decides.
                bypass_voice_activity_filter: false,
            }),
            flag,
        );

        // The captured utterance is deleted whether transcription succeeded, failed, or was
        // cancelled. Nothing downstream needs the audio again.
        self.inner.temp.cleanup_recording(recording_id);
        let settled_ms = self.inner.clock.now_ms();

        match outcome {
            Ok(WorkerReply::Transcribe(reply)) => {
                let result = TranscriptionResult {
                    text: crate::contexts::local_media::domain::normalize_recognized_text(
                        &reply.text,
                    ),
                    detected_language: reply.detected_language.clone(),
                    language_probability: reply.language_probability,
                    duration_ms: reply.duration_ms.or(Some(committed.duration_ms)),
                    limit_reached: limit_reached || committed.limit_reached,
                    provenance: TranscriptionProvenance {
                        engine: "faster-whisper".to_string(),
                        engine_version: reply.engine_version.clone(),
                        profile_revision: snapshot.profile_revision(),
                        device: reply
                            .device
                            .clone()
                            .unwrap_or_else(|| snapshot.stt().device.as_str().to_string()),
                    },
                };
                self.inner.diagnostics.record(
                    "stt.completed",
                    &[
                        ("durationMs", committed.duration_ms.to_string()),
                        ("sampleCount", committed.sample_count.to_string()),
                        ("noSpeechDetected", result.is_empty().to_string()),
                        ("limitReached", result.limit_reached.to_string()),
                    ],
                );
                self.inner.store.succeed(
                    operation_id,
                    LocalMediaOperationResult::Stt(result),
                    settled_ms,
                );
                self.inner.operations.succeed(operation_id);
            }
            Ok(_) => self.settle_failure(
                operation_id,
                LocalMediaError::new(LocalMediaErrorCode::WorkerProtocolError),
                settled_ms,
            ),
            Err(error) => self.settle_failure(operation_id, error, settled_ms),
        }

        drop(guard);
    }

    /// Both the id and the scope must match the active recording.
    fn owned_recording(
        &self,
        recording_id: &RecordingId,
        composer_scope: &ComposerScopeId,
    ) -> Result<RecordingSummary, LocalMediaError> {
        let Ok(active) = self.inner.active_recording.lock() else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::RecordingNotFound));
        };
        match active.as_ref() {
            Some(summary) if summary.owned_by(recording_id, composer_scope) => Ok(summary.clone()),
            _ => Err(LocalMediaError::new(LocalMediaErrorCode::RecordingNotFound)),
        }
    }

    fn release_recording_slot(&self) {
        if let Ok(mut active) = self.inner.active_recording.lock() {
            *active = None;
        }
    }
}
