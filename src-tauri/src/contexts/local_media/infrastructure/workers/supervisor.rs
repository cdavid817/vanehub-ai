//! Three independent engine slots.
//!
//! Independence is the whole design: PaddleOCR, CTranslate2, and sherpa-onnx load conflicting
//! native runtimes, so one crashing, hanging, or being restarted for a profile change must be
//! invisible to the other two. Nothing in this module holds a lock across a worker call, and no
//! state is shared between slots.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::process::{ChildProcessTransport, WorkerSession, WorkerTransport};
use crate::contexts::local_media::application::ports::WorkerSupervisorPort;
use crate::contexts::local_media::application::worker_contract::{WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, LocalMediaProfileSnapshot, WorkerState,
};

/// One running worker, behind a trait so the supervisor can be tested without a process.
pub(super) trait WorkerHandle: Send {
    fn call(
        &mut self,
        snapshot: &LocalMediaProfileSnapshot,
        call: &WorkerCall,
        cancelled: Arc<AtomicBool>,
        timeout: Duration,
        cancel_grace: Duration,
    ) -> Result<WorkerReply, LocalMediaError>;
    fn is_poisoned(&self) -> bool;
    fn profile_revision(&self) -> i64;
    fn shutdown(&mut self, grace: Duration);
}

impl WorkerHandle for WorkerSession {
    fn call(
        &mut self,
        snapshot: &LocalMediaProfileSnapshot,
        call: &WorkerCall,
        cancelled: Arc<AtomicBool>,
        timeout: Duration,
        cancel_grace: Duration,
    ) -> Result<WorkerReply, LocalMediaError> {
        WorkerSession::call(self, snapshot, call, cancelled, timeout, cancel_grace)
    }

    fn is_poisoned(&self) -> bool {
        WorkerSession::is_poisoned(self)
    }

    fn profile_revision(&self) -> i64 {
        WorkerSession::profile_revision(self)
    }

    fn shutdown(&mut self, grace: Duration) {
        WorkerSession::shutdown(self, grace)
    }
}

pub(super) trait WorkerLauncher: Send + Sync {
    fn launch(
        &self,
        engine: LocalMediaEngine,
        snapshot: &LocalMediaProfileSnapshot,
    ) -> Result<Box<dyn WorkerHandle>, LocalMediaError>;
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SupervisorPolicy {
    /// Concurrent admissions per engine, including the one being served. `2` lets OnePiece queue
    /// one OCR behind the composer's without the queue becoming a place work accumulates.
    pub(super) queue_depth: usize,
    pub(super) call_timeout: Duration,
    pub(super) cancel_grace: Duration,
    pub(super) shutdown_grace: Duration,
    pub(super) restart_backoff: Duration,
    /// Consecutive launch failures before the slot stops trying. A broken Python environment must
    /// not turn every composer click into another spawn attempt.
    pub(super) max_consecutive_failures: u32,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            queue_depth: 2,
            call_timeout: Duration::from_secs(300),
            cancel_grace: Duration::from_secs(3),
            shutdown_grace: Duration::from_secs(2),
            restart_backoff: Duration::from_secs(2),
            max_consecutive_failures: 3,
        }
    }
}

#[derive(Default)]
struct Slot {
    worker: Option<Box<dyn WorkerHandle>>,
    in_flight: usize,
    consecutive_failures: u32,
    restart_not_before: Option<Instant>,
    quarantined: bool,
    busy: bool,
}

pub(crate) struct LocalMediaWorkerSupervisor {
    launcher: Arc<dyn WorkerLauncher>,
    policy: SupervisorPolicy,
    slots: Mutex<HashMap<LocalMediaEngine, Slot>>,
}

impl LocalMediaWorkerSupervisor {
    pub(super) fn new(launcher: Arc<dyn WorkerLauncher>, policy: SupervisorPolicy) -> Self {
        Self {
            launcher,
            policy,
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Reserve one of the engine's admission permits, or refuse.
    ///
    /// Reserving before launching is deliberate: a refused call must not have started a Python
    /// process as a side effect.
    fn reserve(&self, engine: LocalMediaEngine) -> Result<(), LocalMediaError> {
        let Ok(mut slots) = self.slots.lock() else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable));
        };
        let slot = slots.entry(engine).or_default();
        if slot.quarantined {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable)
                .with_text("engine", engine.as_str()));
        }
        if slot.in_flight >= self.policy.queue_depth {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineBusy)
                .with_text("engine", engine.as_str())
                .with_number("queueDepth", slot.in_flight as i64));
        }
        slot.in_flight += 1;
        Ok(())
    }

    fn release(&self, engine: LocalMediaEngine) {
        if let Ok(mut slots) = self.slots.lock() {
            if let Some(slot) = slots.get_mut(&engine) {
                slot.in_flight = slot.in_flight.saturating_sub(1);
                slot.busy = slot.in_flight > 0 && slot.worker.is_some();
            }
        }
    }

    /// Take the running worker out of its slot for the duration of a call.
    ///
    /// Checking it out rather than calling it under the slot lock is what keeps one engine's
    /// multi-second inference from blocking a status read for the other two.
    fn checkout(
        &self,
        engine: LocalMediaEngine,
        snapshot: &LocalMediaProfileSnapshot,
    ) -> Result<Box<dyn WorkerHandle>, LocalMediaError> {
        {
            let Ok(mut slots) = self.slots.lock() else {
                return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable));
            };
            let slot = slots.entry(engine).or_default();
            if let Some(worker) = slot.worker.take() {
                let stale = worker.profile_revision() != snapshot.profile_revision();
                if !worker.is_poisoned() && !stale {
                    slot.busy = true;
                    return Ok(worker);
                }
                // A stale worker holds a model loaded from paths the user has since changed.
                let mut worker = worker;
                worker.shutdown(self.policy.shutdown_grace);
            }
            if let Some(not_before) = slot.restart_not_before {
                if Instant::now() < not_before {
                    return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable)
                        .with_text("engine", engine.as_str()));
                }
            }
        }

        match self.launcher.launch(engine, snapshot) {
            Ok(worker) => {
                if let Ok(mut slots) = self.slots.lock() {
                    let slot = slots.entry(engine).or_default();
                    slot.consecutive_failures = 0;
                    slot.restart_not_before = None;
                    slot.busy = true;
                }
                Ok(worker)
            }
            Err(error) => {
                if let Ok(mut slots) = self.slots.lock() {
                    let slot = slots.entry(engine).or_default();
                    slot.consecutive_failures += 1;
                    slot.restart_not_before = Some(Instant::now() + self.policy.restart_backoff);
                    if slot.consecutive_failures >= self.policy.max_consecutive_failures {
                        slot.quarantined = true;
                    }
                    slot.busy = false;
                }
                Err(error)
            }
        }
    }

    fn checkin(&self, engine: LocalMediaEngine, worker: Box<dyn WorkerHandle>) {
        if worker.is_poisoned() {
            return;
        }
        if let Ok(mut slots) = self.slots.lock() {
            let slot = slots.entry(engine).or_default();
            slot.worker = Some(worker);
            slot.busy = false;
        }
    }

    #[cfg(test)]
    pub(super) fn debug_reserve_all(&self, engine: LocalMediaEngine) -> usize {
        let mut reserved = 0;
        while self.reserve(engine).is_ok() {
            reserved += 1;
        }
        reserved
    }

    #[cfg(test)]
    pub(super) fn debug_release(&self, engine: LocalMediaEngine) {
        self.release(engine);
    }
}

impl WorkerSupervisorPort for LocalMediaWorkerSupervisor {
    fn call(
        &self,
        snapshot: &LocalMediaProfileSnapshot,
        call: WorkerCall,
        cancelled: Arc<AtomicBool>,
    ) -> Result<WorkerReply, LocalMediaError> {
        let engine = snapshot.engine();
        self.reserve(engine)?;

        let outcome = (|| {
            let mut worker = self.checkout(engine, snapshot)?;
            let result = worker.call(
                snapshot,
                &call,
                cancelled,
                self.policy.call_timeout,
                self.policy.cancel_grace,
            );
            self.checkin(engine, worker);
            result
        })();

        self.release(engine);
        outcome
    }

    fn state(&self, engine: LocalMediaEngine) -> WorkerState {
        let Ok(slots) = self.slots.lock() else {
            return WorkerState::Stopped;
        };
        let Some(slot) = slots.get(&engine) else {
            return WorkerState::Stopped;
        };
        if slot.quarantined {
            return WorkerState::Quarantined;
        }
        match (&slot.worker, slot.busy) {
            (_, true) => WorkerState::Busy,
            (Some(_), false) => WorkerState::Idle,
            (None, false) if slot.restart_not_before.is_some() => WorkerState::Restarting,
            (None, false) => WorkerState::Stopped,
        }
    }

    /// Stop idle workers that predate the newly saved profile, and give a quarantined slot a fresh
    /// start: a save is the user's answer to whatever made it fail.
    fn retire_stale(&self, current_revision: i64) {
        let Ok(mut slots) = self.slots.lock() else {
            return;
        };
        for slot in slots.values_mut() {
            slot.quarantined = false;
            slot.consecutive_failures = 0;
            slot.restart_not_before = None;
            let is_stale = slot
                .worker
                .as_ref()
                .is_some_and(|worker| worker.profile_revision() != current_revision);
            if is_stale {
                if let Some(mut worker) = slot.worker.take() {
                    worker.shutdown(self.policy.shutdown_grace);
                }
            }
        }
    }

    fn shutdown_all(&self) {
        let Ok(mut slots) = self.slots.lock() else {
            return;
        };
        for slot in slots.values_mut() {
            if let Some(mut worker) = slot.worker.take() {
                worker.shutdown(self.policy.shutdown_grace);
            }
            slot.busy = false;
            slot.in_flight = 0;
        }
    }
}

/// Launches real Python workers.
pub(crate) struct PythonWorkerLauncher {
    bridge_root: PathBuf,
    media_root: PathBuf,
    working_directory: PathBuf,
    handshake_timeout: Duration,
}

impl PythonWorkerLauncher {
    pub(crate) fn new(
        bridge_root: PathBuf,
        media_root: PathBuf,
        working_directory: PathBuf,
    ) -> Self {
        Self {
            bridge_root,
            media_root,
            working_directory,
            handshake_timeout: Duration::from_secs(30),
        }
    }
}

impl WorkerLauncher for PythonWorkerLauncher {
    fn launch(
        &self,
        engine: LocalMediaEngine,
        snapshot: &LocalMediaProfileSnapshot,
    ) -> Result<Box<dyn WorkerHandle>, LocalMediaError> {
        // The working directory is an application-owned empty directory, not the project or the
        // user's home: a relative import or a stray write must not land somewhere meaningful.
        let _ = std::fs::create_dir_all(&self.working_directory);
        let transport: Box<dyn WorkerTransport> = Box::new(ChildProcessTransport::spawn(
            engine,
            snapshot.python_executable(),
            &self.bridge_root,
            &self.media_root,
            &self.working_directory,
            &super::environment::current_environment(),
        )?);
        let session = WorkerSession::establish(
            engine,
            transport,
            self.handshake_timeout,
            snapshot.profile_revision(),
        )?;
        Ok(Box::new(session))
    }
}

/// Assemble the production supervisor.
pub(crate) fn build_supervisor(
    bridge_root: PathBuf,
    media_root: PathBuf,
    working_directory: PathBuf,
) -> LocalMediaWorkerSupervisor {
    LocalMediaWorkerSupervisor::new(
        Arc::new(PythonWorkerLauncher::new(
            bridge_root,
            media_root,
            working_directory,
        )),
        SupervisorPolicy::default(),
    )
}

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
