//! Explicit-argument external process construction and bounded execution.

mod managed_child;
mod stderr_drain;
#[cfg(windows)]
mod windows_job;

pub(crate) use managed_child::{ManagedChild, ManagedTokioChild};
pub(crate) use stderr_drain::{BlockingStderrDrain, StderrCapture, TokioStderrDrain};

use crate::platform::network;
use process_wrap::std as process_std;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ProcessError {
    #[error("{0}")]
    InvalidExecutable(&'static str),
    #[error("failed to start external process: {0}")]
    Spawn(String),
    #[error("failed while waiting for external process: {0}")]
    Wait(String),
    #[error("managed process pipe is unavailable: {0}")]
    PipeUnavailable(&'static str),
    #[error("managed process shutdown exceeded its deadline")]
    ShutdownTimedOut,
    #[error("command timed out after {timeout_seconds} seconds")]
    TimedOut {
        timeout_seconds: u64,
        stdout: String,
        stderr: String,
        output_truncated: bool,
    },
    #[error("command was cancelled")]
    Cancelled {
        stdout: String,
        stderr: String,
        output_truncated: bool,
    },
}

/// How long output collection waits for the pipes after the child is done. A pipe reaches
/// EOF the moment its last writer closes, so this only elapses when a descendant inherited
/// the pipe and outlived the child — without a bound, collection blocks forever and the
/// caller's timeout never takes effect.
const OUTPUT_DRAIN_GRACE: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) stdout_bytes: Vec<u8>,
    pub(crate) stderr_bytes: Vec<u8>,
    pub(crate) output_truncated: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessCancellation {
    cancelled: Arc<AtomicBool>,
}

impl ProcessCancellation {
    pub(crate) fn from_signal(cancelled: Arc<AtomicBool>) -> Self {
        Self { cancelled }
    }

    /// Production cancels through the `from_signal` flag its owner already holds; this
    /// setter exists for tests that drive cancellation directly.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl ProcessOutput {
    pub(crate) fn success(&self) -> bool {
        self.status.success()
    }

    pub(crate) fn status_label(&self) -> String {
        self.status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| self.status.to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProcessRequest {
    executable: OsString,
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
    environment: BTreeMap<OsString, OsString>,
    clear_environment: bool,
    timeout: Duration,
    cancellation: Option<ProcessCancellation>,
    output_limit: Option<usize>,
}

impl ProcessRequest {
    pub(crate) fn new(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            current_dir: None,
            environment: BTreeMap::new(),
            clear_environment: false,
            timeout: Duration::from_secs(30),
            cancellation: None,
            output_limit: None,
        }
    }

    pub(crate) fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub(crate) fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub(crate) fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub(crate) fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn environment_value(&self, key: &str) -> Option<&OsStr> {
        self.environment
            .get(OsStr::new(key))
            .map(OsString::as_os_str)
    }

    pub(crate) fn env_clear(mut self) -> Self {
        self.clear_environment = true;
        self
    }

    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) fn cancellation(mut self, cancellation: ProcessCancellation) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    pub(crate) fn output_limit(mut self, output_limit: usize) -> Self {
        self.output_limit = Some(output_limit);
        self
    }

    pub(crate) fn command(&self) -> Result<Command, ProcessError> {
        let executable = self.executable.to_string_lossy();
        let mut command = std_command(&executable)?;
        command.args(&self.args);
        if self.clear_environment {
            command.env_clear();
        }
        command.envs(&self.environment);
        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }
        Ok(command)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProcessAdapter;

impl ProcessAdapter {
    pub(crate) fn execute(&self, request: &ProcessRequest) -> Result<ProcessOutput, ProcessError> {
        output_with_control(
            request.command()?,
            request.timeout,
            request.cancellation.as_ref(),
            request.output_limit,
        )
    }
}

pub(crate) fn spawn_detached(
    executable: &std::path::Path,
    args: &[OsString],
    current_dir: &std::path::Path,
) -> Result<(), ProcessError> {
    validate_executable(&executable.to_string_lossy())?;
    if !current_dir.is_dir() {
        return Err(ProcessError::Spawn(
            "working directory is unavailable".to_string(),
        ));
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| ProcessError::Spawn(error.to_string()))
}

pub(crate) fn validate_executable(executable: &str) -> Result<(), ProcessError> {
    let trimmed = executable.trim();
    if trimmed.is_empty() {
        return Err(ProcessError::InvalidExecutable(
            "command executable cannot be empty",
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(ProcessError::InvalidExecutable(
            "command executable cannot contain control characters",
        ));
    }
    Ok(())
}

/// Windows allocates a console for a console-subsystem child unless this flag is set, and every
/// capability probe the app runs (`where`, `reg`, `node --version`) is one. The app itself is a GUI
/// subsystem process with no console to inherit, so each probe would otherwise flash its own window.
#[cfg(windows)]
pub(super) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The std and tokio builders both expose `creation_flags` on Windows, but through unrelated types.
/// One trait keeps a single name at both call sites and gives the non-Windows no-op one home.
trait SuppressConsoleWindow {
    fn suppress_console_window(&mut self);
}

impl SuppressConsoleWindow for Command {
    fn suppress_console_window(&mut self) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW);
        }
    }
}

impl SuppressConsoleWindow for tokio::process::Command {
    fn suppress_console_window(&mut self) {
        #[cfg(windows)]
        self.creation_flags(CREATE_NO_WINDOW);
    }
}

pub(crate) fn std_command(executable: &str) -> Result<Command, ProcessError> {
    validate_executable(executable)?;
    let mut command = Command::new(OsStr::new(executable));
    network::apply_to_std_command(&mut command);
    command.suppress_console_window();
    Ok(command)
}

pub(crate) fn tokio_command(executable: &str) -> Result<tokio::process::Command, ProcessError> {
    validate_executable(executable)?;
    let mut command = tokio::process::Command::new(OsStr::new(executable));
    network::apply_to_tokio_command(&mut command);
    command.suppress_console_window();
    Ok(command)
}

pub(crate) fn command_exists(command_name: &str, timeout: Duration) -> bool {
    let resolver = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let request = ProcessRequest::new(resolver)
        .arg(command_name)
        .timeout(timeout);
    ProcessAdapter
        .execute(&request)
        .map(|output| output.success())
        .unwrap_or(false)
}

pub(crate) fn audit_command(category: &str, executable: &str, args: &[String]) {
    let args_label = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    let fallback = crate::platform::logging::fallback_log_dir();
    let log_dir = crate::platform::logging::active_log_dir(fallback);
    let _ = crate::platform::logging::write_message(
        &log_dir,
        crate::platform::logging::LogLevel::Info,
        category,
        &format!("executing {executable}{args_label}"),
        BTreeMap::new(),
    );
}

/// Contains a bounded-execution command so its descendants can be reached on teardown.
///
/// Distinct from the containment [`ManagedChild`] uses: this one must not gate completion
/// on the tree draining, and must not kill the tree when the handle is released.
#[cfg(windows)]
fn add_execution_containment(command: &mut process_std::CommandWrap) {
    command.wrap(windows_job::TerminateTreeJobObject::new());
}

/// `ProcessGroupChild::try_wait` tries the group first but falls back to the directly
/// launched child, so a surviving group member does not hide that child's exit. That
/// fallback is what makes this wrapper usable here, and it is what
/// `successful_command_leaves_its_descendant_running` pins when the suite runs on Unix.
#[cfg(unix)]
fn add_execution_containment(command: &mut process_std::CommandWrap) {
    command.wrap(process_std::ProcessGroup::leader());
}

#[cfg(not(any(windows, unix)))]
fn add_execution_containment(_command: &mut process_std::CommandWrap) {}

fn output_with_control(
    mut command: Command,
    timeout: Duration,
    cancellation: Option<&ProcessCancellation>,
    output_limit: Option<usize>,
) -> Result<ProcessOutput, ProcessError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    // The containment primitive is used purely as a kill handle: completion is still
    // decided by `try_wait` on the process launched here, so a command that exits while
    // a descendant lives is still reported as finished rather than as a timeout.
    let mut wrapped = process_std::CommandWrap::from(command);
    add_execution_containment(&mut wrapped);
    let mut child = wrapped
        .spawn()
        .map_err(|error| ProcessError::Spawn(error.to_string()))?;
    let stdout = child
        .stdout()
        .take()
        .ok_or_else(|| ProcessError::Wait("stdout pipe is unavailable".to_string()))?;
    let stderr = child
        .stderr()
        .take()
        .ok_or_else(|| ProcessError::Wait("stderr pipe is unavailable".to_string()))?;
    let limit = output_limit.unwrap_or(usize::MAX);
    let stdout_reader = spawn_pipe_reader(stdout, limit);
    let stderr_reader = spawn_pipe_reader(stderr, limit);
    let start = Instant::now();

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancellation.is_some_and(ProcessCancellation::is_cancelled) => {
                let _ = child.kill();
                let _ = child.wait();
                let drain = Instant::now() + OUTPUT_DRAIN_GRACE;
                let (stdout, stdout_truncated) = collect_pipe(&stdout_reader, drain)?;
                let (stderr, stderr_truncated) = collect_pipe(&stderr_reader, drain)?;
                return Err(ProcessError::Cancelled {
                    stdout: decode_output(&stdout),
                    stderr: decode_output(&stderr),
                    output_truncated: stdout_truncated || stderr_truncated,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let drain = Instant::now() + OUTPUT_DRAIN_GRACE;
                let (stdout, stdout_truncated) = collect_pipe(&stdout_reader, drain)?;
                let (stderr, stderr_truncated) = collect_pipe(&stderr_reader, drain)?;
                return Err(ProcessError::TimedOut {
                    timeout_seconds: timeout.as_secs(),
                    stdout: decode_output(&stdout),
                    stderr: decode_output(&stderr),
                    output_truncated: stdout_truncated || stderr_truncated,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(ProcessError::Wait(error.to_string())),
        }
    };

    let drain = Instant::now() + OUTPUT_DRAIN_GRACE;
    let (stdout_bytes, stdout_truncated) = collect_pipe(&stdout_reader, drain)?;
    let (stderr_bytes, stderr_truncated) = collect_pipe(&stderr_reader, drain)?;
    Ok(ProcessOutput {
        stdout: decode_output(&stdout_bytes),
        stderr: decode_output(&stderr_bytes),
        status,
        stdout_bytes,
        stderr_bytes,
        output_truncated: stdout_truncated || stderr_truncated,
    })
}

#[derive(Default)]
struct PipeCapture {
    bytes: Vec<u8>,
    truncated: bool,
    finished: bool,
    failure: Option<String>,
}

struct PipeReader {
    capture: Mutex<PipeCapture>,
    finished: Condvar,
}

/// Streams a child pipe into a shared buffer the collector can take at any point.
///
/// The reader thread is deliberately never joined: a descendant that inherited the pipe
/// keeps it from reaching EOF, which would park this thread in `read` indefinitely. It
/// exits on its own once the last writer closes.
fn spawn_pipe_reader<R: Read + Send + 'static>(mut pipe: R, limit: usize) -> Arc<PipeReader> {
    let reader = Arc::new(PipeReader {
        capture: Mutex::new(PipeCapture::default()),
        finished: Condvar::new(),
    });
    let sink = Arc::clone(&reader);
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let failure = loop {
            match pipe.read(&mut buffer) {
                Ok(0) => break None,
                Ok(count) => {
                    let Ok(mut capture) = sink.capture.lock() else {
                        return;
                    };
                    let remaining = limit.saturating_sub(capture.bytes.len());
                    capture
                        .bytes
                        .extend_from_slice(&buffer[..count.min(remaining)]);
                    capture.truncated |= count > remaining;
                }
                Err(error) => break Some(error.to_string()),
            }
        };
        if let Ok(mut capture) = sink.capture.lock() {
            capture.failure = failure;
            capture.finished = true;
            sink.finished.notify_all();
        }
    });
    reader
}

/// Takes everything read so far, waiting until the pipe closes or `deadline` passes.
fn collect_pipe(reader: &PipeReader, deadline: Instant) -> Result<(Vec<u8>, bool), ProcessError> {
    let mut capture = reader
        .capture
        .lock()
        .map_err(|_| ProcessError::Wait("process output reader panicked".to_string()))?;
    while !capture.finished {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let (guard, wait) = reader
            .finished
            .wait_timeout(capture, remaining)
            .map_err(|_| ProcessError::Wait("process output reader panicked".to_string()))?;
        capture = guard;
        if wait.timed_out() {
            break;
        }
    }
    if let Some(failure) = capture.failure.take() {
        return Err(ProcessError::Wait(failure));
    }
    Ok((std::mem::take(&mut capture.bytes), capture.truncated))
}

fn decode_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn explicit_arguments_are_never_concatenated_into_a_shell_program() {
        let request = ProcessRequest::new("fixture-program")
            .arg("literal; echo should-not-run")
            .arg("$(also-literal)");
        let command = request.command().expect("command");

        assert_eq!(command.get_program(), OsStr::new("fixture-program"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![
                OsStr::new("literal; echo should-not-run"),
                OsStr::new("$(also-literal)")
            ]
        );
    }

    #[test]
    fn adapter_kills_a_process_after_the_configured_timeout() {
        let request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_timeout_child_fixture",
            ])
            .timeout(Duration::from_millis(100));

        let error = ProcessAdapter.execute(&request).expect_err("timeout");

        assert!(matches!(error, ProcessError::TimedOut { .. }));
    }

    #[test]
    fn adapter_bounds_output_and_honors_cancellation() {
        let output_request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_output_child_fixture",
                "--nocapture",
            ])
            .output_limit(64);
        let output = ProcessAdapter
            .execute(&output_request)
            .expect("bounded output");
        assert!(output.output_truncated);
        assert!(output.stdout.len() <= 64);

        let cancellation = ProcessCancellation::default();
        let cancel_request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_timeout_child_fixture",
            ])
            .timeout(Duration::from_secs(10))
            .cancellation(cancellation.clone());
        let running = thread::spawn(move || ProcessAdapter.execute(&cancel_request));
        thread::sleep(Duration::from_millis(100));
        cancellation.cancel();
        assert!(matches!(
            running.join().expect("process thread"),
            Err(ProcessError::Cancelled { .. })
        ));
    }

    #[test]
    fn output_collection_is_bounded_when_a_descendant_inherits_the_pipe() {
        // The command's own timeout is far longer than the assertion window, so this
        // pins the *collection* bound: a grandchild holding the stdout pipe must not
        // keep `execute` blocked after the child it was spawned from has exited.
        let request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_orphan_pipe_child_fixture",
                "--nocapture",
            ])
            .timeout(Duration::from_secs(30));
        let (sender, receiver) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(ProcessAdapter.execute(&request).map(|output| output.stdout));
        });

        let collected = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("collection must not block on a descendant that still holds the pipe");

        assert!(collected
            .expect("child exited normally")
            .contains("parent-done"));
    }

    #[test]
    fn timed_out_command_terminates_its_descendants() {
        // A full parallel test run can take more than 500 ms to start the nested test
        // executable on Windows; keep the timeout well below the 30-second fixture lifetime.
        let request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_tree_parent_fixture",
                "--nocapture",
            ])
            .timeout(Duration::from_secs(5));

        let error = ProcessAdapter.execute(&request).expect_err("timeout");

        let ProcessError::TimedOut { stdout, .. } = &error else {
            panic!("expected a timeout, got {error:?}");
        };
        wait_until_process_stops(descendant_pid(stdout));
    }

    #[test]
    fn successful_command_leaves_its_descendant_running() {
        let request = ProcessRequest::new(std::env::current_exe().expect("test executable"))
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_tree_exiting_parent_fixture",
                "--nocapture",
            ])
            // Far longer than the command needs, so a failure here means the command was
            // misjudged as unfinished rather than that it genuinely ran long.
            .timeout(Duration::from_secs(30));

        let output = ProcessAdapter.execute(&request).expect("command completes");

        assert!(output.success(), "{}", output.stderr);
        let descendant = descendant_pid(&output.stdout);
        assert!(
            process_is_running(descendant),
            "a command that succeeded must keep whatever it started"
        );
        terminate_process(descendant);
    }

    fn descendant_pid(output: &str) -> u32 {
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix("DESCENDANT_PID "))
            .unwrap_or_else(|| panic!("fixture never reported its descendant: {output}"))
            .parse()
            .expect("descendant pid")
    }

    #[test]
    #[ignore = "spawned only by the process-tree termination test"]
    fn process_tree_parent_fixture() {
        spawn_reported_descendant();
        thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore = "spawned only by the surviving-descendant test"]
    fn process_tree_exiting_parent_fixture() {
        spawn_reported_descendant();
    }

    #[allow(clippy::zombie_processes)] // The descendant must outlive this process to be observable.
    fn spawn_reported_descendant() {
        let executable = std::env::current_exe().expect("test executable");
        let descendant = Command::new(executable)
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_tree_descendant_fixture",
            ])
            .stdin(Stdio::null())
            // Null rather than inherit: this fixture is about process-tree termination, not
            // about a descendant holding the output pipe open.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("descendant");
        println!("DESCENDANT_PID {}", descendant.id());
        std::io::stdout().flush().expect("flush pid");
    }

    #[test]
    #[ignore = "spawned only by the process-tree fixtures"]
    fn process_tree_descendant_fixture() {
        thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore = "spawned only by the orphaned-pipe collection test"]
    #[allow(clippy::zombie_processes)] // Must exit while its descendant still holds the inherited stdout pipe.
    fn process_orphan_pipe_child_fixture() {
        let executable = std::env::current_exe().expect("test executable");
        Command::new(executable)
            .args([
                "--ignored",
                "--exact",
                "platform::process::tests::process_orphan_pipe_grandchild_fixture",
            ])
            .stdin(Stdio::null())
            // Inheriting stdout hands the grandchild a live write handle to the pipe, so
            // the pipe cannot reach EOF when this process exits a moment from now.
            .stdout(Stdio::inherit())
            .stderr(Stdio::null())
            .spawn()
            .expect("grandchild");
        print!("parent-done");
        std::io::stdout().flush().expect("flush");
    }

    #[test]
    #[ignore = "spawned only by the orphaned-pipe collection fixture"]
    fn process_orphan_pipe_grandchild_fixture() {
        thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore = "spawned only by the timeout adapter test"]
    fn process_timeout_child_fixture() {
        thread::sleep(Duration::from_secs(5));
    }

    #[test]
    #[ignore = "spawned only by the bounded output adapter test"]
    fn process_output_child_fixture() {
        print!("{}", "x".repeat(4096));
    }

    #[cfg(windows)]
    fn process_is_running(process_id: u32) -> bool {
        use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let Ok(process) =
            (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) })
        else {
            return false;
        };
        let mut exit_code = 0;
        let queried = unsafe { GetExitCodeProcess(process, &mut exit_code) }.is_ok();
        let _ = unsafe { CloseHandle(process) };
        queried && exit_code == STILL_ACTIVE.0 as u32
    }

    #[cfg(unix)]
    fn process_is_running(process_id: u32) -> bool {
        unsafe { libc::kill(process_id as i32, 0) == 0 }
    }

    #[cfg(windows)]
    fn terminate_process(process_id: u32) {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        if let Ok(process) = unsafe { OpenProcess(PROCESS_TERMINATE, false, process_id) } {
            let _ = unsafe { TerminateProcess(process, 1) };
            let _ = unsafe { CloseHandle(process) };
        }
    }

    #[cfg(unix)]
    fn terminate_process(process_id: u32) {
        unsafe { libc::kill(process_id as i32, libc::SIGKILL) };
    }

    fn wait_until_process_stops(process_id: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while process_is_running(process_id) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !process_is_running(process_id),
            "descendant {process_id} survived the termination of its parent"
        );
    }

    #[test]
    fn rejects_empty_and_control_character_executables() {
        assert!(matches!(
            validate_executable("  "),
            Err(ProcessError::InvalidExecutable(_))
        ));
        assert!(matches!(
            validate_executable("node\nserver"),
            Err(ProcessError::InvalidExecutable(_))
        ));
    }

    #[test]
    fn detached_plan_keeps_special_arguments_separate() {
        let args = [OsString::from("D:/A & B/$(literal)")];
        assert_eq!(args[0], OsString::from("D:/A & B/$(literal)"));
    }
}
