//! The single execution entry point for an accepted operation.
//!
//! Called on a background thread by the command layer. Splitting acceptance from execution is what
//! lets a Tauri command return an operation id in microseconds while the model behind it takes
//! seconds to load.

use super::service::{LocalMediaApplicationService, LocalMediaJob, PreparedLocalMediaOperation};

impl LocalMediaApplicationService {
    pub(crate) fn execute(&self, prepared: PreparedLocalMediaOperation) {
        let PreparedLocalMediaOperation {
            operation_id, job, ..
        } = prepared;

        // Cancellation can arrive between acceptance and the thread being scheduled. Checking here
        // means a fast Escape never opens a device or starts a worker at all.
        if self.inner.operations.is_cancelled(&operation_id) {
            self.inner
                .store
                .cancel(&operation_id, self.inner.clock.now_ms());
            self.release_job_resources(&job);
            self.inner.temp.cleanup_operation(&operation_id);
            return;
        }

        match job {
            LocalMediaJob::Probe { snapshot } => self.run_probe(&operation_id, &snapshot),
            LocalMediaJob::Ocr { snapshot, claimed } => {
                self.run_ocr(&operation_id, &snapshot, &claimed)
            }
            LocalMediaJob::Stt {
                snapshot,
                recording_id,
            } => self.run_stt(&operation_id, &snapshot, &recording_id),
            LocalMediaJob::Tts {
                snapshot,
                text,
                playback_id,
            } => self.run_tts(&operation_id, &snapshot, &text, &playback_id),
        }
    }

    /// Release what acceptance already reserved when the job never runs.
    fn release_job_resources(&self, job: &LocalMediaJob) {
        match job {
            LocalMediaJob::Ocr { claimed, .. } => {
                self.inner.temp.cleanup_staged(&claimed.staged_input_id);
            }
            LocalMediaJob::Stt { recording_id, .. } => {
                self.inner.capture.cancel(recording_id);
                self.inner.temp.cleanup_recording(recording_id);
            }
            LocalMediaJob::Probe { .. } | LocalMediaJob::Tts { .. } => {}
        }
    }
}
