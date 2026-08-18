use super::*;
use std::collections::{BTreeMap, VecDeque};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct FakePlan {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
    stays_running: bool,
    spawn_fails: bool,
}

struct FakeFactory {
    plans: Mutex<VecDeque<FakePlan>>,
    stdin: Arc<Mutex<Vec<u8>>>,
    shutdowns: Arc<AtomicUsize>,
    observed: Arc<Mutex<Vec<RunnerLaunchSpec>>>,
}

impl FakeFactory {
    fn new(plans: Vec<FakePlan>) -> Self {
        Self {
            plans: Mutex::new(plans.into()),
            stdin: Arc::new(Mutex::new(Vec::new())),
            shutdowns: Arc::new(AtomicUsize::new(0)),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl LocalProcessFactory for FakeFactory {
    fn spawn(&self, spec: &RunnerLaunchSpec) -> Result<SpawnedLocalProcess, RunnerError> {
        self.observed.lock().expect("observed").push(spec.clone());
        let plan = self
            .plans
            .lock()
            .expect("plans")
            .pop_front()
            .expect("fake plan");
        if plan.spawn_fails {
            return Err(RunnerError::new(RunnerErrorKind::Spawn));
        }
        let status = if plan.stays_running {
            None
        } else {
            Some(plan.exit_code)
        };
        Ok(SpawnedLocalProcess {
            native_id: 42,
            child: Box::new(FakeChild {
                status: Arc::new(Mutex::new(status)),
                shutdowns: self.shutdowns.clone(),
            }),
            stdin: Box::new(CaptureWriter(self.stdin.clone())),
            stdout: Box::new(Cursor::new(plan.stdout)),
            stderr: Box::new(Cursor::new(plan.stderr)),
        })
    }
}

struct FakeChild {
    status: Arc<Mutex<Option<Option<i32>>>>,
    shutdowns: Arc<AtomicUsize>,
}

impl LocalChildControl for FakeChild {
    fn wait_until(&mut self, deadline: Instant) -> Result<Option<Option<i32>>, ()> {
        let status = *self.status.lock().map_err(|_| ())?;
        if status.is_some() {
            return Ok(status);
        }
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
        Ok(*self.status.lock().map_err(|_| ())?)
    }

    fn shutdown(&mut self, _: Instant) -> Result<Option<i32>, ()> {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        *self.status.lock().map_err(|_| ())? = Some(Some(-9));
        Ok(Some(-9))
    }
}

struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl Write for CaptureWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("stdin").extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn local_conformance_preserves_spawn_input_order_natural_exit_and_cleanup() {
    let factory = Arc::new(FakeFactory::new(vec![FakePlan {
        stdout: b"one\ntwo\n".to_vec(),
        stderr: b"warning".to_vec(),
        exit_code: Some(0),
        stays_running: false,
        spawn_fails: false,
    }]));
    let runner = LocalRunner::with_factory(factory.clone());
    let prepared = runner
        .prepare(&RunnerSelection::local(), launch_spec(true))
        .expect("prepare");
    let handle = runner.spawn(prepared).expect("spawn");
    runner.send_input(&handle, b"prompt\n").expect("stdin");
    let events = events_until_exit(&runner, &handle);

    assert_eq!(factory.stdin.lock().expect("stdin").as_slice(), b"prompt\n");
    assert_eq!(events.last(), Some(&RunnerEvent::Exited(Some(0))));
    assert_eq!(joined_stream(&events, true), b"one\ntwo\n");
    assert_eq!(joined_stream(&events, false), b"warning");
    assert_eq!(
        runner.inspect(&handle).expect("inspect"),
        RunnerInspection::Exited(Some(0))
    );
    assert_eq!(runner.capabilities().recovery, RunnerRecoveryMode::None);
    runner.cleanup(&handle).expect("cleanup");
    runner.cleanup(&handle).expect("idempotent cleanup");
    let observed = factory.observed.lock().expect("observed");
    assert_eq!(observed[0].executable, "fixture-cli");
    assert_eq!(observed[0].cwd.as_deref(), Some("workspace"));
    assert_eq!(observed[0].environment["TRACEPARENT"], "00-safe");
}

#[test]
fn cancellation_race_and_running_cleanup_reap_the_owned_tree_once() {
    let factory = Arc::new(FakeFactory::new(vec![running_plan(), running_plan()]));
    let runner = LocalRunner::with_factory(factory.clone());
    let first = runner
        .spawn(
            runner
                .prepare(&RunnerSelection::local(), launch_spec(false))
                .expect("prepare"),
        )
        .expect("spawn");
    assert!(runner.cancel(&first).expect("cancel"));
    assert_eq!(
        events_until_exit(&runner, &first).last(),
        Some(&RunnerEvent::Exited(Some(-9)))
    );
    assert!(!runner.cancel(&first).expect("natural cancellation race"));
    runner.cleanup(&first).expect("cleanup");

    let second = runner
        .spawn(
            runner
                .prepare(&RunnerSelection::local(), launch_spec(false))
                .expect("prepare"),
        )
        .expect("spawn");
    runner.cleanup(&second).expect("running cleanup");
    runner.cleanup(&second).expect("idempotent cleanup");
    assert_eq!(factory.shutdowns.load(Ordering::SeqCst), 2);
}

#[test]
fn invalid_input_and_spawn_failure_are_classified_without_live_handles() {
    let factory = Arc::new(FakeFactory::new(vec![FakePlan {
        spawn_fails: true,
        ..running_plan()
    }]));
    let runner = LocalRunner::with_factory(factory);
    let prepared = runner
        .prepare(&RunnerSelection::local(), launch_spec(false))
        .expect("prepare");
    assert_eq!(
        runner.spawn(prepared).expect_err("spawn").kind,
        RunnerErrorKind::Spawn
    );
    let missing = RunnerHandle {
        id: "missing".into(),
        reference: local_reference(),
        process_reference: None,
    };
    assert_eq!(
        runner
            .send_input(&missing, b"nope")
            .expect_err("input")
            .kind,
        RunnerErrorKind::Input
    );
}

#[cfg(windows)]
#[test]
fn local_runner_windows_spawn_cancel_benchmark_records_bounded_cleanup() {
    let runner = LocalRunner::new();
    let mut samples = Vec::new();
    for index in 0..8 {
        let started = Instant::now();
        let prepared = runner
            .prepare(
                &RunnerSelection::local(),
                RunnerLaunchSpec {
                    session_id: Some(format!("benchmark-session-{index}")),
                    executable: "ping.exe".into(),
                    arguments: vec!["-n".into(), "30".into(), "127.0.0.1".into()],
                    cwd: None,
                    environment: BTreeMap::new(),
                    pipe_stdin: false,
                },
            )
            .expect("prepare benchmark process");
        let handle = runner.spawn(prepared).expect("spawn benchmark process");
        runner.cancel(&handle).expect("cancel benchmark process");
        runner.cleanup(&handle).expect("cleanup benchmark process");
        assert!(runner.inspect(&handle).is_err());
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    samples.sort_by(f64::total_cmp);
    eprintln!(
        "RUNNER_WINDOWS_BENCHMARK samples=8 p50_ms={:.3} p95_ms={:.3} live_handles=0",
        samples[3], samples[7]
    );
}

fn events_until_exit(runner: &LocalRunner, handle: &RunnerHandle) -> Vec<RunnerEvent> {
    let mut events = Vec::new();
    while let Some(event) = runner.next_event(handle).expect("event") {
        let exited = matches!(event, RunnerEvent::Exited(_));
        events.push(event);
        if exited {
            break;
        }
    }
    events
}

fn joined_stream(events: &[RunnerEvent], stdout: bool) -> Vec<u8> {
    events
        .iter()
        .filter_map(|event| match (stdout, event) {
            (true, RunnerEvent::Stdout(bytes)) | (false, RunnerEvent::Stderr(bytes)) => {
                Some(bytes.as_slice())
            }
            _ => None,
        })
        .flatten()
        .copied()
        .collect()
}

fn running_plan() -> FakePlan {
    FakePlan {
        stdout: Vec::new(),
        stderr: Vec::new(),
        exit_code: None,
        stays_running: true,
        spawn_fails: false,
    }
}

fn launch_spec(pipe_stdin: bool) -> RunnerLaunchSpec {
    RunnerLaunchSpec {
        session_id: Some("session-1".into()),
        executable: "fixture-cli".into(),
        arguments: vec!["--json".into()],
        cwd: Some("workspace".into()),
        environment: BTreeMap::from([("TRACEPARENT".into(), "00-safe".into())]),
        pipe_stdin,
    }
}

fn local_reference() -> RunnerReference {
    RunnerReference {
        kind: RunnerKind::Local,
        target_id: "local".into(),
        target_revision: None,
        recovery: RunnerRecoveryMode::None,
        authority_witness: "local-runner-v1".into(),
    }
}
