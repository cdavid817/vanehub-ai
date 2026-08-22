//! Local synthesis and playback.
//!
//! The generated WAV is operation-owned and short-lived. It is never cached across requests,
//! because the text it was generated from is the user's draft.

use super::cleanup::OperationMediaGuard;
use super::service::{LocalMediaApplicationService, LocalMediaJob, PreparedLocalMediaOperation};
use super::worker_contract::{TtsWorkerRequest, WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode,
    LocalMediaOperationKind, LocalMediaOperationResult, LocalMediaPhase, LocalMediaProfileSnapshot,
    PlaybackId, SpeechPlaybackResult, MAX_TTS_CODE_POINTS,
};

impl LocalMediaApplicationService {
    pub(crate) fn prepare_tts(
        &self,
        text: String,
        composer_scope: ComposerScopeId,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Tts, &profile)?;

        if text.trim().is_empty() {
            return Err(
                LocalMediaError::new(LocalMediaErrorCode::TtsTextTooLong).with_number("actual", 0)
            );
        }
        // Code points, not bytes: the limit is about how much speech this produces, and a byte
        // limit would silently be four times stricter for Chinese than for English.
        let code_points = text.chars().count();
        if code_points > MAX_TTS_CODE_POINTS {
            return Err(LocalMediaError::new(LocalMediaErrorCode::TtsTextTooLong)
                .with_number("limit", MAX_TTS_CODE_POINTS as i64)
                .with_number("actual", code_points as i64));
        }

        // Starting a new utterance stops the previous one. The user asked for this text to be read
        // now; layering two synthesized voices is never what they meant.
        self.inner.playback.stop(None);

        let (operation_id, snapshot, accepted_at) = self.accept_operation(
            LocalMediaOperationKind::Tts,
            LocalMediaEngine::Tts,
            &profile,
            Some(composer_scope),
        )?;
        let playback_id = PlaybackId::new(self.inner.ids.next(PlaybackId::PREFIX));

        Ok(PreparedLocalMediaOperation {
            operation_id,
            kind: LocalMediaOperationKind::Tts,
            accepted_at,
            job: LocalMediaJob::Tts {
                snapshot,
                text,
                playback_id,
            },
        })
    }

    pub(super) fn run_tts(
        &self,
        operation_id: &str,
        snapshot: &LocalMediaProfileSnapshot,
        text: &str,
        playback_id: &PlaybackId,
    ) {
        let guard = OperationMediaGuard::new(self.inner.temp.clone(), operation_id);
        let flag = self.inner.operations.cancellation_flag(operation_id);

        let output_path = match self.inner.temp.authorize_output_wav(operation_id) {
            Ok(path) => path,
            Err(error) => {
                self.settle_failure(operation_id, error, self.inner.clock.now_ms());
                return;
            }
        };

        self.advance(operation_id, LocalMediaPhase::GeneratingAudio);
        let call = WorkerCall::Synthesize(TtsWorkerRequest {
            text: text.to_string(),
            output_path: output_path.clone(),
        });
        let generated = self.inner.workers.call(snapshot, call, flag.clone());

        let reply = match generated {
            Ok(WorkerReply::Synthesize(reply)) => reply,
            Ok(_) => {
                self.settle_failure(
                    operation_id,
                    LocalMediaError::new(LocalMediaErrorCode::WorkerProtocolError),
                    self.inner.clock.now_ms(),
                );
                return;
            }
            Err(error) => {
                self.settle_failure(operation_id, error, self.inner.clock.now_ms());
                return;
            }
        };

        // A worker that names any other path is treated as protocol-invalid and its output is not
        // played. This is the check that keeps a compromised or buggy worker from turning playback
        // into an arbitrary-file read.
        if let Err(error) = self
            .inner
            .temp
            .verify_output_wav(operation_id, &reply.audio_path)
        {
            self.settle_failure(operation_id, error, self.inner.clock.now_ms());
            return;
        }

        self.advance(operation_id, LocalMediaPhase::Playing);
        let device = snapshot.tts().output_device_id.clone();
        let played =
            self.inner
                .playback
                .play_blocking(playback_id, &output_path, device.as_deref(), flag);
        let settled_ms = self.inner.clock.now_ms();

        match played {
            Ok(duration_ms) => {
                let result = SpeechPlaybackResult {
                    playback_id: playback_id.clone(),
                    sample_rate: reply.sample_rate,
                    duration_ms: if duration_ms > 0 {
                        duration_ms
                    } else {
                        reply.duration_ms
                    },
                    device_summary: device,
                };
                self.inner.diagnostics.record(
                    "tts.completed",
                    &[
                        ("sampleRate", reply.sample_rate.to_string()),
                        ("durationMs", result.duration_ms.to_string()),
                    ],
                );
                self.inner.store.succeed(
                    operation_id,
                    LocalMediaOperationResult::Tts(result),
                    settled_ms,
                );
                self.inner.operations.succeed(operation_id);
            }
            Err(error) => self.settle_failure(operation_id, error, settled_ms),
        }

        // The guard would do this anyway; running it here makes the ordering explicit -- the file
        // is gone before the operation is observable as terminal.
        drop(guard);
    }
}
