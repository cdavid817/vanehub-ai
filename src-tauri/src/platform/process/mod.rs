//! Explicit-argument external process construction and bounded execution.
#![allow(dead_code)]

use crate::platform::network;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
        let mut command = request.command()?;
        output_with_control(
            &mut command,
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

pub(crate) fn std_command(executable: &str) -> Result<Command, ProcessError> {
    validate_executable(executable)?;
    let mut command = Command::new(OsStr::new(executable));
    network::apply_to_std_command(&mut command);
    Ok(command)
}

pub(crate) fn tokio_command(executable: &str) -> Result<tokio::process::Command, ProcessError> {
    validate_executable(executable)?;
    let mut command = tokio::process::Command::new(OsStr::new(executable));
    network::apply_to_tokio_command(&mut command);
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

pub(crate) fn output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<ProcessOutput, ProcessError> {
    output_with_control(command, timeout, None, None)
}

fn output_with_control(
    command: &mut Command,
    timeout: Duration,
    cancellation: Option<&ProcessCancellation>,
    output_limit: Option<usize>,
) -> Result<ProcessOutput, ProcessError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ProcessError::Spawn(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProcessError::Wait("stdout pipe is unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProcessError::Wait("stderr pipe is unavailable".to_string()))?;
    let limit = output_limit.unwrap_or(usize::MAX);
    let stdout_reader = thread::spawn(move || read_pipe(stdout, limit));
    let stderr_reader = thread::spawn(move || read_pipe(stderr, limit));
    let start = Instant::now();

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancellation.is_some_and(ProcessCancellation::is_cancelled) => {
                let _ = child.kill();
                let _ = child.wait();
                let (stdout, stdout_truncated) = join_reader(stdout_reader)?;
                let (stderr, stderr_truncated) = join_reader(stderr_reader)?;
                return Err(ProcessError::Cancelled {
                    stdout: decode_output(stdout),
                    stderr: decode_output(stderr),
                    output_truncated: stdout_truncated || stderr_truncated,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let (stdout, stdout_truncated) = join_reader(stdout_reader)?;
                let (stderr, stderr_truncated) = join_reader(stderr_reader)?;
                return Err(ProcessError::TimedOut {
                    timeout_seconds: timeout.as_secs(),
                    stdout: decode_output(stdout),
                    stderr: decode_output(stderr),
                    output_truncated: stdout_truncated || stderr_truncated,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(ProcessError::Wait(error.to_string())),
        }
    };

    let (stdout_bytes, stdout_truncated) = join_reader(stdout_reader)?;
    let (stderr_bytes, stderr_truncated) = join_reader(stderr_reader)?;
    Ok(ProcessOutput {
        status,
        stdout: decode_output(stdout_bytes.clone()),
        stderr: decode_output(stderr_bytes.clone()),
        stdout_bytes,
        stderr_bytes,
        output_truncated: stdout_truncated || stderr_truncated,
    })
}

fn read_pipe(mut pipe: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut truncated = false;
    loop {
        let count = pipe.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    Ok((retained, truncated))
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<(Vec<u8>, bool)>>,
) -> Result<(Vec<u8>, bool), ProcessError> {
    reader
        .join()
        .map_err(|_| ProcessError::Wait("process output reader panicked".to_string()))?
        .map_err(|error| ProcessError::Wait(error.to_string()))
}

fn decode_output(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[ignore = "spawned only by the timeout adapter test"]
    fn process_timeout_child_fixture() {
        thread::sleep(Duration::from_secs(5));
    }

    #[test]
    #[ignore = "spawned only by the bounded output adapter test"]
    fn process_output_child_fixture() {
        print!("{}", "x".repeat(4096));
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
