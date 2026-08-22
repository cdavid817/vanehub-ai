use super::*;
use crate::contexts::local_media::application::worker_contract::{ProbeReply, SttWorkerRequest};
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaOperationId, LocalMediaOperationKind, LocalMediaProfile,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A launcher that hands out scripted sessions instead of spawning Python.
struct StubLauncher {
    launches: AtomicUsize,
    outcomes: Mutex<Vec<Result<StubBehaviour, LocalMediaErrorCode>>>,
}

#[derive(Clone, Copy)]
enum StubBehaviour {
    Succeed,
    FailRequest(LocalMediaErrorCode),
    PoisonOnRequest,
}

struct StubSession {
    behaviour: StubBehaviour,
    revision: i64,
    poisoned: bool,
    calls: Arc<AtomicUsize>,
}

impl WorkerHandle for StubSession {
    fn call(
        &mut self,
        _snapshot: &LocalMediaProfileSnapshot,
        _call: &WorkerCall,
        _cancelled: Arc<AtomicBool>,
        _timeout: Duration,
        _cancel_grace: Duration,
    ) -> Result<WorkerReply, LocalMediaError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.behaviour {
            StubBehaviour::Succeed => Ok(WorkerReply::Probe(ProbeReply {
                package_version: Some("1.0.0".to_string()),
                device: Some("cpu".to_string()),
                model_identity: Some("stub".to_string()),
            })),
            StubBehaviour::FailRequest(code) => Err(LocalMediaError::new(code)),
            StubBehaviour::PoisonOnRequest => {
                self.poisoned = true;
                Err(LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed))
            }
        }
    }

    fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    fn profile_revision(&self) -> i64 {
        self.revision
    }

    fn shutdown(&mut self, _grace: Duration) {
        self.poisoned = true;
    }
}

struct StubLauncherHandle {
    launcher: Arc<StubLauncher>,
    calls: Arc<AtomicUsize>,
}

impl WorkerLauncher for StubLauncherHandle {
    fn launch(
        &self,
        _engine: LocalMediaEngine,
        snapshot: &LocalMediaProfileSnapshot,
    ) -> Result<Box<dyn WorkerHandle>, LocalMediaError> {
        self.launcher.launches.fetch_add(1, Ordering::SeqCst);
        let mut outcomes = self.launcher.outcomes.lock().expect("outcomes");
        let outcome = if outcomes.is_empty() {
            Ok(StubBehaviour::Succeed)
        } else {
            outcomes.remove(0)
        };
        match outcome {
            Ok(behaviour) => Ok(Box::new(StubSession {
                behaviour,
                revision: snapshot.profile_revision(),
                poisoned: false,
                calls: self.calls.clone(),
            })),
            Err(code) => Err(LocalMediaError::new(code)),
        }
    }
}

struct Fixture {
    supervisor: LocalMediaWorkerSupervisor,
    launcher: Arc<StubLauncher>,
    calls: Arc<AtomicUsize>,
}

fn fixture(outcomes: Vec<Result<StubBehaviour, LocalMediaErrorCode>>) -> Fixture {
    let launcher = Arc::new(StubLauncher {
        launches: AtomicUsize::new(0),
        outcomes: Mutex::new(outcomes),
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let supervisor = LocalMediaWorkerSupervisor::new(
        Arc::new(StubLauncherHandle {
            launcher: launcher.clone(),
            calls: calls.clone(),
        }),
        SupervisorPolicy {
            queue_depth: 2,
            call_timeout: Duration::from_millis(200),
            cancel_grace: Duration::from_millis(20),
            shutdown_grace: Duration::from_millis(20),
            restart_backoff: Duration::from_millis(0),
            max_consecutive_failures: 3,
        },
    );
    Fixture {
        supervisor,
        launcher,
        calls,
    }
}

fn snapshot(engine: LocalMediaEngine, revision: i64) -> LocalMediaProfileSnapshot {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = revision;
    profile.enabled = true;
    profile.ocr.enabled = true;
    profile.stt.enabled = true;
    profile.tts.enabled = true;
    LocalMediaProfileSnapshot::capture(
        LocalMediaOperationId::new(format!("lmo-{:032x}", revision)),
        LocalMediaOperationKind::Probe,
        engine,
        &profile,
        Some(ComposerScopeId::new("session-1")),
        "2026-01-01T00:00:01Z".to_string(),
    )
}

fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[test]
fn a_worker_starts_lazily_on_the_first_call() {
    let fixture = fixture(vec![]);
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Ocr),
        WorkerState::Stopped
    );
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 0);

    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect("call");
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 1);
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Ocr),
        WorkerState::Idle
    );
}

#[test]
fn a_second_call_reuses_the_running_worker() {
    let fixture = fixture(vec![]);
    for _ in 0..3 {
        fixture
            .supervisor
            .call(
                &snapshot(LocalMediaEngine::Ocr, 1),
                WorkerCall::Probe,
                flag(),
            )
            .expect("call");
    }
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 3);
}

#[test]
fn each_engine_gets_its_own_worker() {
    let fixture = fixture(vec![]);
    for engine in LocalMediaEngine::ALL {
        fixture
            .supervisor
            .call(&snapshot(engine, 1), WorkerCall::Probe, flag())
            .expect("call");
    }
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 3);
}

#[test]
fn a_poisoned_worker_is_replaced_on_the_next_call() {
    let fixture = fixture(vec![
        Ok(StubBehaviour::PoisonOnRequest),
        Ok(StubBehaviour::Succeed),
    ]);
    let error = fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Stt, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect_err("crash");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerCrashed);

    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Stt, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect("replacement worker");
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 2);
}

#[test]
fn one_engines_crash_does_not_disturb_another() {
    let fixture = fixture(vec![Ok(StubBehaviour::PoisonOnRequest)]);
    let _ = fixture.supervisor.call(
        &snapshot(LocalMediaEngine::Stt, 1),
        WorkerCall::Probe,
        flag(),
    );
    assert!(fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Tts, 1),
            WorkerCall::Probe,
            flag()
        )
        .is_ok());
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Tts),
        WorkerState::Idle
    );
}

#[test]
fn repeated_start_failures_quarantine_the_slot_instead_of_spinning() {
    let fixture = fixture(vec![
        Err(LocalMediaErrorCode::WorkerStartFailed),
        Err(LocalMediaErrorCode::WorkerStartFailed),
        Err(LocalMediaErrorCode::WorkerStartFailed),
    ]);
    for _ in 0..3 {
        let _ = fixture.supervisor.call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag(),
        );
    }
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Ocr),
        WorkerState::Quarantined
    );

    // A quarantined slot refuses without attempting a fourth launch.
    let error = fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect_err("quarantined");
    assert_eq!(error.code(), LocalMediaErrorCode::EngineUnavailable);
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 3);
}

#[test]
fn a_new_profile_revision_clears_the_quarantine() {
    let fixture = fixture(vec![
        Err(LocalMediaErrorCode::WorkerStartFailed),
        Err(LocalMediaErrorCode::WorkerStartFailed),
        Err(LocalMediaErrorCode::WorkerStartFailed),
        Ok(StubBehaviour::Succeed),
    ]);
    for _ in 0..3 {
        let _ = fixture.supervisor.call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag(),
        );
    }
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Ocr),
        WorkerState::Quarantined
    );

    // Fixing the configuration is what a user does next; a save has to make the engine reachable
    // again without restarting the application.
    fixture.supervisor.retire_stale(2);
    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Ocr, 2),
            WorkerCall::Probe,
            flag(),
        )
        .expect("retry after save");
}

#[test]
fn an_idle_worker_on_a_stale_revision_is_replaced_before_its_next_job() {
    let fixture = fixture(vec![]);
    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Tts, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect("first");
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 1);

    fixture.supervisor.retire_stale(2);
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Tts),
        WorkerState::Stopped
    );

    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Tts, 2),
            WorkerCall::Probe,
            flag(),
        )
        .expect("second");
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 2);
}

#[test]
fn a_call_arriving_on_a_stale_revision_replaces_the_worker() {
    let fixture = fixture(vec![]);
    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Tts, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect("first");
    fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Tts, 5),
            WorkerCall::Probe,
            flag(),
        )
        .expect("newer revision");
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 2);
}

#[test]
fn an_engine_already_running_a_job_rejects_past_its_queue_bound() {
    // The permit is held for the duration of a call, so a concurrent caller sees the bound. This
    // exercises it directly rather than racing threads.
    let fixture = fixture(vec![]);
    let permits = fixture.supervisor.debug_reserve_all(LocalMediaEngine::Ocr);
    assert_eq!(permits, 2);

    let error = fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect_err("queue full");
    assert_eq!(error.code(), LocalMediaErrorCode::EngineBusy);
    assert_eq!(
        fixture.launcher.launches.load(Ordering::SeqCst),
        0,
        "no worker for a refused call"
    );
}

#[test]
fn a_released_permit_lets_the_next_caller_through() {
    let fixture = fixture(vec![]);
    fixture.supervisor.debug_reserve_all(LocalMediaEngine::Ocr);
    fixture.supervisor.debug_release(LocalMediaEngine::Ocr);
    assert!(fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Ocr, 1),
            WorkerCall::Probe,
            flag()
        )
        .is_ok());
}

#[test]
fn a_request_error_leaves_the_worker_available() {
    let fixture = fixture(vec![Ok(StubBehaviour::FailRequest(
        LocalMediaErrorCode::ModelNotFound,
    ))]);
    let error = fixture
        .supervisor
        .call(
            &snapshot(LocalMediaEngine::Stt, 1),
            WorkerCall::Probe,
            flag(),
        )
        .expect_err("model error");
    assert_eq!(error.code(), LocalMediaErrorCode::ModelNotFound);
    assert_eq!(
        fixture.supervisor.state(LocalMediaEngine::Stt),
        WorkerState::Idle
    );
    assert_eq!(fixture.launcher.launches.load(Ordering::SeqCst), 1);
}

#[test]
fn shutdown_stops_every_slot() {
    let fixture = fixture(vec![]);
    for engine in LocalMediaEngine::ALL {
        fixture
            .supervisor
            .call(&snapshot(engine, 1), WorkerCall::Probe, flag())
            .expect("call");
    }
    fixture.supervisor.shutdown_all();
    for engine in LocalMediaEngine::ALL {
        assert_eq!(fixture.supervisor.state(engine), WorkerState::Stopped);
    }
}

#[test]
fn a_transcribe_call_reaches_the_worker_unchanged() {
    let fixture = fixture(vec![]);
    let call = WorkerCall::Transcribe(SttWorkerRequest {
        audio_path: PathBuf::from("/tmp/local-media/recordings/lmr-1/input.wav"),
    });
    assert!(fixture
        .supervisor
        .call(&snapshot(LocalMediaEngine::Stt, 1), call, flag())
        .is_ok());
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
}
