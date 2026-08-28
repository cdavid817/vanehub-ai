//! Engine probe.
//!
//! A probe checks the *saved* profile, never the settings form's current contents. The settings
//! page says so in its own copy, and this is where that promise is kept: the profile is re-read
//! from the repository rather than passed in.

use super::cleanup::OperationMediaGuard;
use super::service::{
    EngineRuntimeState, LocalMediaApplicationService, LocalMediaJob, PreparedLocalMediaOperation,
};
use super::worker_contract::{WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    EngineReadiness, LocalMediaEngine, LocalMediaError, LocalMediaOperationKind,
    LocalMediaOperationResult, LocalMediaPhase, LocalMediaProfileSnapshot,
};

impl LocalMediaApplicationService {
    pub(crate) fn prepare_probe(
        &self,
        engine: LocalMediaEngine,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(engine, &profile)?;

        let (operation_id, snapshot, accepted_at) =
            self.accept_operation(LocalMediaOperationKind::Probe, engine, &profile, None)?;

        let mut state = self.engine_state_for(engine);
        state.readiness = Some(EngineReadiness::Checking);
        self.record_engine_state(engine, state);

        Ok(PreparedLocalMediaOperation {
            operation_id,
            kind: LocalMediaOperationKind::Probe,
            accepted_at,
            job: LocalMediaJob::Probe { snapshot },
        })
    }

    pub(super) fn run_probe(&self, operation_id: &str, snapshot: &LocalMediaProfileSnapshot) {
        let engine = snapshot.engine();
        self.advance(operation_id, LocalMediaPhase::LoadingEngine);
        let flag = self.inner.operations.cancellation_flag(operation_id);

        // Covers the canary's input and output for every exit below, including the failing ones.
        let _media = OperationMediaGuard::new(self.inner.temp.clone(), operation_id);

        let outcome = self
            .inner
            .workers
            .call(snapshot, WorkerCall::Probe, flag.clone());

        // Metadata alone is not readiness. The failure this exists for is a model the runtime
        // accepts on load and cannot execute, which a construction-only probe reports as `Ready`
        // and the user then meets on their first real operation.
        let outcome = match outcome {
            Ok(WorkerReply::Probe(reply)) => {
                self.advance(operation_id, LocalMediaPhase::Processing);
                match self.run_readiness_canary(operation_id, snapshot, flag) {
                    Ok(()) => Ok(WorkerReply::Probe(reply)),
                    Err(error) => Err(error),
                }
            }
            other => other,
        };
        let now_ms = self.inner.clock.now_ms();

        match outcome {
            Ok(WorkerReply::Probe(reply)) => {
                self.record_engine_state(
                    engine,
                    EngineRuntimeState {
                        readiness: Some(EngineReadiness::Ready),
                        installed_version: reply.package_version.clone(),
                        model_identity: reply.model_identity.clone(),
                        device_summary: reply.device.clone(),
                        last_checked_at: Some(self.inner.clock.now_iso()),
                        checked_revision: Some(snapshot.profile_revision()),
                    },
                );
                self.inner.diagnostics.record(
                    "probe.succeeded",
                    &[
                        ("engine", engine.as_str().to_string()),
                        ("profileRevision", snapshot.profile_revision().to_string()),
                    ],
                );
                self.finish_probe(operation_id, now_ms);
            }
            Ok(_) => {
                // A reply of the wrong shape is a protocol failure, not a partial success.
                self.fail_probe(
                    operation_id,
                    engine,
                    snapshot,
                    LocalMediaError::new(
                        crate::contexts::local_media::domain::LocalMediaErrorCode::WorkerProtocolError,
                    ),
                    now_ms,
                );
            }
            Err(error) => self.fail_probe(operation_id, engine, snapshot, error, now_ms),
        }
    }

    fn finish_probe(&self, operation_id: &str, now_ms: u64) {
        // The probe's result is the whole runtime status, so the settings page gets every engine's
        // state from one read rather than three racing queries.
        match self.inner.repository.load() {
            Ok(profile) => {
                let status = self.status_for(&profile);
                self.inner.store.succeed(
                    operation_id,
                    LocalMediaOperationResult::Probe(status),
                    now_ms,
                );
                self.inner.operations.succeed(operation_id);
            }
            Err(error) => {
                self.inner
                    .operations
                    .fail(operation_id, error.code().as_str());
                self.inner.store.fail(operation_id, error, now_ms);
            }
        }
    }

    fn fail_probe(
        &self,
        operation_id: &str,
        engine: LocalMediaEngine,
        snapshot: &LocalMediaProfileSnapshot,
        error: LocalMediaError,
        now_ms: u64,
    ) {
        if error.is_cancelled() {
            // A cancelled probe leaves readiness untouched: the user learned nothing, so the
            // previous answer is still the best one available.
            self.inner.operations.cancel(operation_id);
            self.inner.store.cancel(operation_id, now_ms);
            return;
        }
        self.record_engine_state(
            engine,
            EngineRuntimeState {
                readiness: Some(EngineReadiness::Unavailable {
                    code: error.code(),
                    // Whatever the engine itself named, and nothing when it named nothing.
                    field: error.field().map(str::to_string),
                }),
                installed_version: None,
                model_identity: None,
                device_summary: None,
                last_checked_at: Some(self.inner.clock.now_iso()),
                checked_revision: Some(snapshot.profile_revision()),
            },
        );
        self.inner.diagnostics.record(
            "probe.failed",
            &[
                ("engine", engine.as_str().to_string()),
                ("code", error.code().as_str().to_string()),
            ],
        );
        self.inner
            .operations
            .fail(operation_id, error.code().as_str());
        self.inner.store.fail(operation_id, error, now_ms);
    }

    pub(super) fn advance(&self, operation_id: &str, phase: LocalMediaPhase) {
        self.inner.store.set_phase(operation_id, phase);
        self.inner.operations.phase(operation_id, phase.as_str());
    }
}
