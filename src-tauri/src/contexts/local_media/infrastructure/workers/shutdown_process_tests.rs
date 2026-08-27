//! What happens to a real Python worker when the application shuts down mid-request.
//!
//! A transport double cannot answer this. The question is whether an operating-system process
//! survives, and the two mechanisms that would hide the answer -- a Windows job object that reaps
//! children when the parent's handles close, and a pipe EOF that a cooperative worker notices --
//! are both invisible to a stub. So this test starts the real bridge under a real interpreter,
//! parks it inside an uninterruptible sleep, and asks the supervisor to shut down.
//!
//! The evidence is a marker the fixture writes *after* its sleep returns. If the host killed the
//! worker the marker never appears; if the host merely stopped waiting for it, the marker appears
//! a few seconds later. The two outcomes are indistinguishable from the call's return value, which
//! is why the marker exists.

use super::*;
use crate::contexts::local_media::application::worker_contract::{TtsWorkerRequest, WorkerCall};
use crate::contexts::local_media::domain::{
    LocalMediaEngine, LocalMediaOperationId, LocalMediaOperationKind, LocalMediaProfile,
    LocalMediaProfileSnapshot,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Long enough that the sleep cannot plausibly finish before the assertion, short enough that a
/// leak is reported in seconds rather than minutes.
const HANG_SECONDS: u64 = 8;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_path_buf()
}

/// An absolute interpreter path, or `None` when this machine has no Python.
///
/// Absolute because the launcher refuses a bare name, and `sys.executable` because a launcher such
/// as `py` is not itself the interpreter.
/// Spawned through the shared platform adapter rather than `std::process::Command`, because the
/// architecture gate forbids a second process-construction path anywhere under `src/` -- test file
/// or not -- and a probe is not a good reason to open one.
fn interpreter() -> Option<PathBuf> {
    let candidates: &[&str] = if cfg!(windows) {
        &["python", "py", "python3"]
    } else {
        &["python3", "python"]
    };
    let arguments = vec![
        "-c".to_string(),
        "import sys; print(sys.executable)".to_string(),
    ];
    for candidate in candidates {
        let Ok(mut child) = crate::platform::process::ManagedChild::spawn_in(
            candidate,
            &arguments,
            &BTreeMap::new(),
            None,
        ) else {
            continue;
        };
        let mut printed = String::new();
        if let Ok(mut stdout) = child.take_stdout() {
            let _ = std::io::Read::read_to_string(&mut stdout, &mut printed);
        }
        let _ = child.shutdown(Instant::now() + Duration::from_secs(10));
        let resolved = PathBuf::from(printed.trim());
        if resolved.is_absolute() && resolved.is_file() {
            return Some(resolved);
        }
    }
    None
}

struct Harness {
    _root: tempfile::TempDir,
    media_root: PathBuf,
    scenario: PathBuf,
    supervisor: Arc<LocalMediaWorkerSupervisor>,
    snapshot: LocalMediaProfileSnapshot,
    output: PathBuf,
}

impl Harness {
    /// The marker the fixture writes once its sleep returns.
    fn completion_marker(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}.sherpa-onnx.hang-completed",
            self.scenario.display()
        ))
    }

    fn start_marker(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}.sherpa-onnx.hang-started",
            self.scenario.display()
        ))
    }
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create a fixture directory");
    }
    std::fs::write(path, body).expect("write a fixture file");
}

fn harness(python: &Path) -> Harness {
    let root = tempfile::TempDir::new().expect("harness root");
    // Canonical, because the worker resolves every media path before checking containment.
    let base = root.path().canonicalize().expect("canonical harness root");
    let media_root = base.join("local-media");
    let operation = "lmo-00000000000000000000000000000001";
    let output = media_root
        .join("operations")
        .join(operation)
        .join("out.wav");
    write(&output.with_file_name("placeholder"), "");
    let scenario = base.join("scenario.json");
    write(
        &scenario,
        &format!(r#"{{"sherpa-onnx":{{"behaviour":"hang","hangSeconds":{HANG_SECONDS}}}}}"#),
    );
    let model = base.join("voice.onnx");
    write(&model, "placeholder");
    let tokens = base.join("tokens.txt");
    write(&tokens, "placeholder");

    let mut variables = BTreeMap::new();
    variables.insert(
        "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE".to_string(),
        scenario.to_string_lossy().to_string(),
    );
    let overlay = crate::contexts::local_media::infrastructure::workers::environment::WorkerEnvironmentOverlay {
        python_path_suffix: vec![
            repository_root().join("tests/desktop/fixtures/local-media-python")
        ],
        variables,
    };

    let mut profile = LocalMediaProfile::disabled_default("2026-08-23T00:00:00Z".to_string());
    profile.revision = 1;
    profile.enabled = true;
    profile.tts.enabled = true;
    profile.tts.python_executable = python.to_string_lossy().to_string();
    profile.tts.model_path = model.to_string_lossy().to_string();
    profile.tts.tokens_path = tokens.to_string_lossy().to_string();

    let supervisor = LocalMediaWorkerSupervisor::new(
        Arc::new(
            PythonWorkerLauncher::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/local-media-worker"),
                media_root.clone(),
                base.join("worker-cwd"),
            )
            .with_environment_overlay(overlay),
        ),
        SupervisorPolicy::default(),
    );

    let snapshot = LocalMediaProfileSnapshot::capture(
        LocalMediaOperationId::new(operation),
        LocalMediaOperationKind::Tts,
        LocalMediaEngine::Tts,
        &profile,
        None,
        "2026-08-23T00:00:00Z".to_string(),
    );

    Harness {
        _root: root,
        media_root,
        scenario,
        supervisor: Arc::new(supervisor),
        snapshot,
        output,
    }
}

fn wait_for(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn shutdown_terminates_a_worker_that_is_still_serving_a_request() {
    let Some(python) = interpreter() else {
        // NOT RUN, stated rather than silently passed. The packaged worker resolves whichever
        // interpreter the user configured, and a build machine without one is not evidence.
        eprintln!("NOT RUN: no Python interpreter; the in-flight shutdown test needs one.");
        return;
    };
    let harness = harness(&python);
    let _ = &harness.media_root;

    let supervisor = harness.supervisor.clone();
    let snapshot = harness.snapshot.clone();
    let output = harness.output.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker = std::thread::spawn(move || {
        supervisor.call(
            &snapshot,
            WorkerCall::Synthesize(TtsWorkerRequest {
                text: "in flight at shutdown".to_string(),
                output_path: output,
            }),
            cancelled,
        )
    });

    // Only once the engine is provably inside its sleep. A worker that failed to launch would
    // otherwise produce the same absent completion marker for the wrong reason.
    assert!(
        wait_for(&harness.start_marker(), Duration::from_secs(60)),
        "the worker never reached the scripted hang"
    );

    let started = Instant::now();
    harness.supervisor.shutdown_all();
    assert!(
        started.elapsed() < Duration::from_secs(HANG_SECONDS),
        "shutdown waited for the hung request instead of bounding it"
    );

    // Past the point the abandoned sleep would have finished.
    std::thread::sleep(Duration::from_secs(HANG_SECONDS + 4));
    assert!(
        !harness.completion_marker().exists(),
        "the in-flight worker outlived shutdown: it finished its sleep after the host stopped \
         waiting, which means the process was left running"
    );

    let _ = worker.join();
}
