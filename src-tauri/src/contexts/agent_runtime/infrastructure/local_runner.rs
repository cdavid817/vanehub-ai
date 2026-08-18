use crate::contexts::agent_runtime::application::{
    AgentRunner, PreparedRunnerLaunch, RunnerCapabilities, RunnerError, RunnerErrorKind,
    RunnerEvent, RunnerHandle, RunnerInspection, RunnerKind, RunnerLaunchSpec, RunnerRecoveryMode,
    RunnerReference, RunnerSelection,
};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::process::ManagedChild;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const EVENT_QUEUE_CAPACITY: usize = 256;
const STREAM_CHUNK_BYTES: usize = 8 * 1024;
const OUTPUT_DRAIN_GRACE: Duration = Duration::from_millis(500);
const CANCEL_DEADLINE: Duration = Duration::from_secs(2);

pub(crate) struct LocalRunner {
    handles: Mutex<HashMap<String, Arc<LocalProcess>>>,
    sequence: AtomicU64,
    factory: Arc<dyn LocalProcessFactory>,
}

struct LocalProcess {
    child: Arc<Mutex<Box<dyn LocalChildControl>>>,
    stdin: Mutex<Option<Box<dyn Write + Send>>>,
    events: Mutex<Receiver<RunnerEvent>>,
    inspection: Arc<Mutex<RunnerInspection>>,
}

impl Default for LocalRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalRunner {
    pub(crate) fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(0),
            factory: Arc::new(SystemLocalProcessFactory),
        }
    }

    #[cfg(test)]
    fn with_factory(factory: Arc<dyn LocalProcessFactory>) -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(0),
            factory,
        }
    }

    fn with_process<T>(
        &self,
        handle: &RunnerHandle,
        error_kind: RunnerErrorKind,
        action: impl FnOnce(&LocalProcess) -> Result<T, RunnerError>,
    ) -> Result<T, RunnerError> {
        let process = self
            .handles
            .lock()
            .map_err(|_| RunnerError::new(error_kind))?
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| RunnerError::new(error_kind))?;
        action(&process)
    }
}

impl AgentRunner for LocalRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Local
    }

    fn capabilities(&self) -> RunnerCapabilities {
        RunnerCapabilities {
            interactive_input: true,
            pty: false,
            cancellation: true,
            inspection: true,
            recovery: RunnerRecoveryMode::None,
        }
    }

    fn prepare(
        &self,
        selection: &RunnerSelection,
        spec: RunnerLaunchSpec,
    ) -> Result<PreparedRunnerLaunch, RunnerError> {
        selection.validate()?;
        if selection.kind != RunnerKind::Local {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        spec.validate_for(RunnerKind::Local)?;
        Ok(PreparedRunnerLaunch {
            reference: RunnerReference::local(),
            spec,
            preparation_id: None,
            admission_id: None,
        })
    }

    fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
        prepared.spec.validate_for(RunnerKind::Local)?;
        let spawned = self.factory.spawn(&prepared.spec)?;
        let native_id = spawned.native_id;
        let id = format!(
            "local-runner-{native_id}-{}",
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        );
        let child = Arc::new(Mutex::new(spawned.child));
        let inspection = Arc::new(Mutex::new(RunnerInspection::Running));
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        monitor_process(
            child.clone(),
            inspection.clone(),
            sender,
            spawned.stdout,
            spawned.stderr,
        );
        let process = LocalProcess {
            child,
            stdin: Mutex::new(prepared.spec.pipe_stdin.then_some(spawned.stdin)),
            events: Mutex::new(receiver),
            inspection,
        };
        let handle = RunnerHandle {
            id: id.clone(),
            reference: prepared.reference,
            process_reference: Some(id.clone()),
        };
        self.handles
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?
            .insert(id, Arc::new(process));
        Ok(handle)
    }

    fn send_input(&self, handle: &RunnerHandle, content: &[u8]) -> Result<(), RunnerError> {
        self.with_process(handle, RunnerErrorKind::Input, |process| {
            let mut input = process
                .stdin
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Input))?
                .take()
                .ok_or_else(|| RunnerError::new(RunnerErrorKind::Input))?;
            input
                .write_all(content)
                .map_err(|_| RunnerError::new(RunnerErrorKind::Input))
        })
    }

    fn next_event(&self, handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
        self.with_process(handle, RunnerErrorKind::Disconnected, |process| {
            process
                .events
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Disconnected))?
                .recv()
                .map(Some)
                .or(Ok(None))
        })
    }

    fn cancel(&self, handle: &RunnerHandle) -> Result<bool, RunnerError> {
        self.with_process(handle, RunnerErrorKind::Cancellation, |process| {
            if !matches!(
                *process
                    .inspection
                    .lock()
                    .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?,
                RunnerInspection::Running
            ) {
                return Ok(false);
            }
            let mut child = process
                .child
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
            if child
                .wait_until(Instant::now())
                .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?
                .is_some()
            {
                return Ok(false);
            }
            child
                .shutdown(Instant::now() + CANCEL_DEADLINE)
                .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
            Ok(true)
        })
    }

    fn inspect(&self, handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
        self.with_process(handle, RunnerErrorKind::Inspection, |process| {
            Ok(process
                .inspection
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Inspection))?
                .clone())
        })
    }

    fn cleanup(&self, handle: &RunnerHandle) -> Result<(), RunnerError> {
        let process = self
            .handles
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?
            .remove(&handle.id);
        if let Some(process) = process {
            if matches!(
                *process
                    .inspection
                    .lock()
                    .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?,
                RunnerInspection::Running
            ) {
                process
                    .child
                    .lock()
                    .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?
                    .shutdown(Instant::now() + CANCEL_DEADLINE)
                    .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?;
            }
        }
        Ok(())
    }

    fn recover(
        &self,
        _: &RunnerReference,
        _: Option<&str>,
    ) -> Result<RunnerInspection, RunnerError> {
        Ok(RunnerInspection::Unknown)
    }
}

fn monitor_process(
    child: Arc<Mutex<Box<dyn LocalChildControl>>>,
    inspection: Arc<Mutex<RunnerInspection>>,
    sender: SyncSender<RunnerEvent>,
    stdout: Box<dyn Read + Send>,
    stderr: Box<dyn Read + Send>,
) {
    let accepting = Arc::new(AtomicBool::new(true));
    let (done_sender, done_receiver) = mpsc::channel();
    spawn_reader(
        stdout,
        sender.clone(),
        accepting.clone(),
        done_sender.clone(),
        true,
    );
    spawn_reader(
        stderr,
        sender.clone(),
        accepting.clone(),
        done_sender,
        false,
    );
    thread::spawn(move || {
        let status = loop {
            let result = child.lock().map_err(|_| ()).and_then(|mut child| {
                child
                    .wait_until(Instant::now() + Duration::from_millis(25))
                    .map_err(|_| ())
            });
            match result {
                Ok(Some(status)) => break status,
                Ok(None) => continue,
                Err(()) => break None,
            }
        };
        for _ in 0..2 {
            let _ = done_receiver.recv_timeout(OUTPUT_DRAIN_GRACE);
        }
        accepting.store(false, Ordering::SeqCst);
        if let Ok(mut current) = inspection.lock() {
            *current = RunnerInspection::Exited(status);
        }
        let _ = sender.send(RunnerEvent::Exited(status));
    });
}

fn spawn_reader(
    mut reader: impl Read + Send + 'static,
    sender: SyncSender<RunnerEvent>,
    accepting: Arc<AtomicBool>,
    done: mpsc::Sender<()>,
    stdout: bool,
) {
    thread::spawn(move || {
        let mut chunk = [0_u8; STREAM_CHUNK_BYTES];
        while accepting.load(Ordering::SeqCst) {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(length) => {
                    let bytes = chunk[..length].to_vec();
                    let event = if stdout {
                        RunnerEvent::Stdout(bytes)
                    } else {
                        RunnerEvent::Stderr(bytes)
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = done.send(());
    });
}

struct SpawnedLocalProcess {
    native_id: u32,
    child: Box<dyn LocalChildControl>,
    stdin: Box<dyn Write + Send>,
    stdout: Box<dyn Read + Send>,
    stderr: Box<dyn Read + Send>,
}

trait LocalProcessFactory: Send + Sync {
    fn spawn(&self, spec: &RunnerLaunchSpec) -> Result<SpawnedLocalProcess, RunnerError>;
}

trait LocalChildControl: Send {
    fn wait_until(&mut self, deadline: Instant) -> Result<Option<Option<i32>>, ()>;
    fn shutdown(&mut self, deadline: Instant) -> Result<Option<i32>, ()>;
}

struct SystemLocalProcessFactory;

impl LocalProcessFactory for SystemLocalProcessFactory {
    fn spawn(&self, spec: &RunnerLaunchSpec) -> Result<SpawnedLocalProcess, RunnerError> {
        let cwd = spec
            .cwd
            .as_deref()
            .map(normalize_windows_extended_length_path)
            .map(std::path::PathBuf::from);
        let mut child = ManagedChild::spawn_in(
            &spec.executable,
            &spec.arguments,
            &spec.environment,
            cwd.as_deref(),
        )
        .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?;
        let native_id = child.id().unwrap_or_default();
        let stdin = child
            .take_stdin()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?;
        let stdout = child
            .take_stdout()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?;
        let stderr = child
            .take_stderr()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?;
        Ok(SpawnedLocalProcess {
            native_id,
            child: Box::new(child),
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
            stderr: Box::new(stderr),
        })
    }
}

impl LocalChildControl for ManagedChild {
    fn wait_until(&mut self, deadline: Instant) -> Result<Option<Option<i32>>, ()> {
        ManagedChild::wait_until(self, deadline)
            .map(|status| status.map(|status| status.code()))
            .map_err(|_| ())
    }

    fn shutdown(&mut self, deadline: Instant) -> Result<Option<i32>, ()> {
        ManagedChild::shutdown(self, deadline)
            .map(|status| status.code())
            .map_err(|_| ())
    }
}

#[cfg(test)]
#[path = "local_runner_tests.rs"]
mod tests;
