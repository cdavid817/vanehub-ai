use super::{tokio_command, ProcessError};
use process_wrap::{std as process_std, tokio as process_tokio};
use std::collections::BTreeMap;
use std::process::{ChildStderr, ChildStdin, ChildStdout, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tokio::process::{
    ChildStderr as TokioChildStderr, ChildStdin as TokioChildStdin, ChildStdout as TokioChildStdout,
};

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COOPERATIVE_SHUTDOWN_GRACE: Duration = Duration::from_millis(250);
#[cfg(unix)]
const SHUTDOWN_PHASE_COUNT: u32 = 3;
#[cfg(not(unix))]
const SHUTDOWN_PHASE_COUNT: u32 = 2;
#[cfg(unix)]
const UNIX_TERMINATE_SIGNAL: i32 = 15;

#[derive(Debug)]
pub(crate) struct ManagedChild {
    child: Option<Box<dyn process_std::ChildWrapper>>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    status: Option<ExitStatus>,
}

impl ManagedChild {
    pub(crate) fn spawn(
        executable: &str,
        args: &[String],
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, ProcessError> {
        let mut command = super::std_command(executable)?;
        command
            .args(args)
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut command = process_std::CommandWrap::from(command);
        add_std_containment(&mut command);
        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::Spawn(error.to_string()))?;
        let stdin = child
            .stdin()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdin"))?;
        let stdout = child
            .stdout()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdout"))?;
        let stderr = child
            .stderr()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stderr"))?;
        Ok(Self {
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(stdout),
            stderr: Some(stderr),
            status: None,
        })
    }

    pub(crate) fn id(&self) -> Option<u32> {
        self.child.as_ref().map(|child| child.id())
    }

    pub(crate) fn take_stdin(&mut self) -> Result<ChildStdin, ProcessError> {
        self.stdin
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdin"))
    }

    pub(crate) fn take_stdout(&mut self) -> Result<ChildStdout, ProcessError> {
        self.stdout
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdout"))
    }

    pub(crate) fn take_stderr(&mut self) -> Result<ChildStderr, ProcessError> {
        self.stderr
            .take()
            .ok_or(ProcessError::PipeUnavailable("stderr"))
    }

    pub(crate) fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<ExitStatus>, ProcessError> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        loop {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?;
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.status = Some(status);
                    return Ok(Some(status));
                }
                Ok(None) if Instant::now() >= deadline => return Ok(None),
                Ok(None) => thread::sleep(
                    WAIT_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                ),
                Err(error) => return Err(ProcessError::Wait(error.to_string())),
            }
        }
    }

    pub(crate) fn shutdown(&mut self, deadline: Instant) -> Result<ExitStatus, ProcessError> {
        self.stdin.take();
        let grace_deadline = phase_deadline(deadline, SHUTDOWN_PHASE_COUNT);
        if let Some(status) = self.wait_until(grace_deadline)? {
            return Ok(status);
        }
        #[cfg(unix)]
        {
            let signal_result = self
                .child
                .as_ref()
                .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?
                .signal(UNIX_TERMINATE_SIGNAL);
            if let Err(error) = signal_result {
                if let Some(status) = self.wait_until(Instant::now())? {
                    return Ok(status);
                }
                return Err(ProcessError::Wait(error.to_string()));
            }
            let terminate_deadline = phase_deadline(deadline, 2);
            if let Some(status) = self.wait_until(terminate_deadline)? {
                return Ok(status);
            }
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?;
        child
            .start_kill()
            .map_err(|error| ProcessError::Wait(error.to_string()))?;
        if let Some(status) = self.wait_until(deadline)? {
            return Ok(status);
        }
        Err(ProcessError::ShutdownTimedOut)
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.status.is_none() {
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct ManagedTokioChild {
    child: Option<Box<dyn process_tokio::ChildWrapper>>,
    stdin: Option<TokioChildStdin>,
    stdout: Option<TokioChildStdout>,
    stderr: Option<TokioChildStderr>,
    status: Option<ExitStatus>,
}

impl ManagedTokioChild {
    pub(crate) fn spawn(
        executable: &str,
        args: &[String],
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, ProcessError> {
        let mut command = tokio_command(executable)?;
        command
            .args(args)
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut command = process_tokio::CommandWrap::from(command);
        command.wrap(process_tokio::KillOnDrop);
        add_tokio_containment(&mut command);
        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::Spawn(error.to_string()))?;
        let stdin = child
            .stdin()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdin"))?;
        let stdout = child
            .stdout()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdout"))?;
        let stderr = child
            .stderr()
            .take()
            .ok_or(ProcessError::PipeUnavailable("stderr"))?;
        Ok(Self {
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(stdout),
            stderr: Some(stderr),
            status: None,
        })
    }

    pub(crate) fn id(&self) -> Option<u32> {
        self.child.as_ref().and_then(|child| child.id())
    }

    pub(crate) fn take_stdin(&mut self) -> Result<TokioChildStdin, ProcessError> {
        self.stdin
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdin"))
    }

    pub(crate) fn take_stdout(&mut self) -> Result<TokioChildStdout, ProcessError> {
        self.stdout
            .take()
            .ok_or(ProcessError::PipeUnavailable("stdout"))
    }

    pub(crate) fn take_stderr(&mut self) -> Result<TokioChildStderr, ProcessError> {
        self.stderr
            .take()
            .ok_or(ProcessError::PipeUnavailable("stderr"))
    }

    pub(crate) async fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<ExitStatus>, ProcessError> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return self.try_wait();
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?;
        match tokio::time::timeout(remaining, child.wait()).await {
            Ok(Ok(status)) => {
                self.status = Some(status);
                Ok(Some(status))
            }
            Ok(Err(error)) => Err(ProcessError::Wait(error.to_string())),
            Err(_) => Ok(None),
        }
    }

    pub(crate) async fn shutdown(&mut self, deadline: Instant) -> Result<ExitStatus, ProcessError> {
        self.stdin.take();
        let grace_deadline = phase_deadline(deadline, SHUTDOWN_PHASE_COUNT);
        if let Some(status) = self.wait_until(grace_deadline).await? {
            return Ok(status);
        }
        #[cfg(unix)]
        {
            let signal_result = self
                .child
                .as_ref()
                .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?
                .signal(UNIX_TERMINATE_SIGNAL);
            if let Err(error) = signal_result {
                if let Some(status) = self.wait_until(Instant::now()).await? {
                    return Ok(status);
                }
                return Err(ProcessError::Wait(error.to_string()));
            }
            let terminate_deadline = phase_deadline(deadline, 2);
            if let Some(status) = self.wait_until(terminate_deadline).await? {
                return Ok(status);
            }
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?;
        child
            .start_kill()
            .map_err(|error| ProcessError::Wait(error.to_string()))?;
        if let Some(status) = self.wait_until(deadline).await? {
            return Ok(status);
        }
        Err(ProcessError::ShutdownTimedOut)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, ProcessError> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| ProcessError::Wait("managed child is unavailable".to_string()))?;
        match child.try_wait() {
            Ok(Some(status)) => {
                self.status = Some(status);
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(ProcessError::Wait(error.to_string())),
        }
    }
}

impl Drop for ManagedTokioChild {
    fn drop(&mut self) {
        if self.status.is_some() {
            return;
        }
        if let Some(child) = self.child.take() {
            schedule_tokio_reap(child);
        }
    }
}

fn schedule_tokio_reap(mut child: Box<dyn process_tokio::ChildWrapper>) {
    let _ = child.start_kill();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        drop(handle.spawn(async move {
            let _ = child.wait().await;
        }));
        return;
    }
    let _ = thread::Builder::new()
        .name("vanehub-managed-child-reaper".to_string())
        .spawn(move || {
            if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                let _ = runtime.block_on(child.wait());
            }
        });
}

fn phase_deadline(deadline: Instant, remaining_phases: u32) -> Instant {
    let now = Instant::now();
    let phase_budget = deadline.saturating_duration_since(now) / remaining_phases;
    deadline.min(now + COOPERATIVE_SHUTDOWN_GRACE.min(phase_budget))
}

#[cfg(windows)]
fn add_std_containment(command: &mut process_std::CommandWrap) {
    command.wrap(super::windows_job::KillOnCloseJobObject::new());
}

#[cfg(unix)]
fn add_std_containment(command: &mut process_std::CommandWrap) {
    command.wrap(process_std::ProcessGroup::leader());
}

#[cfg(not(any(windows, unix)))]
fn add_std_containment(_command: &mut process_std::CommandWrap) {}

#[cfg(windows)]
fn add_tokio_containment(command: &mut process_tokio::CommandWrap) {
    command.wrap(process_tokio::JobObject);
}

#[cfg(unix)]
fn add_tokio_containment(command: &mut process_tokio::CommandWrap) {
    command.wrap(process_tokio::ProcessGroup::leader());
}

#[cfg(not(any(windows, unix)))]
fn add_tokio_containment(_command: &mut process_tokio::CommandWrap) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[cfg(any(windows, unix))]
    use std::process::{Command, Stdio};
    #[cfg(windows)]
    use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
    #[cfg(windows)]
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    #[cfg(windows)]
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    fn fixture_args(name: &str) -> Vec<String> {
        vec![
            "--ignored".to_string(),
            "--exact".to_string(),
            format!("platform::process::managed_child::tests::{name}"),
            "--nocapture".to_string(),
        ]
    }

    #[test]
    fn managed_child_owns_all_pipes_and_reaps_after_shutdown() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_echo_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let mut stdin = child.take_stdin().expect("stdin");
        let mut stdout = child.take_stdout().expect("stdout");
        let stderr = child.take_stderr().expect("stderr");
        stdin.write_all(b"hello managed child\n").expect("write");
        drop(stdin);
        let mut output = String::new();
        stdout.read_to_string(&mut output).expect("stdout");
        let status = child
            .wait_until(Instant::now() + Duration::from_secs(2))
            .expect("bounded wait")
            .expect("normal exit");
        let mut error_output = String::new();
        BufReader::new(stderr)
            .read_to_string(&mut error_output)
            .expect("stderr");
        assert!(status.success(), "{status:?}: {error_output}");
        assert!(output.contains("hello managed child"), "{output}");
        assert_eq!(
            child
                .shutdown(Instant::now() + Duration::from_secs(1))
                .expect("already reaped"),
            status
        );
    }

    #[test]
    fn managed_child_wait_is_bounded_and_shutdown_forces_exit() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_hang_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let _stdin = child.take_stdin().expect("stdin");
        let _stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        assert!(child
            .wait_until(Instant::now() + Duration::from_millis(50))
            .expect("bounded wait")
            .is_none());
        let status = child
            .shutdown(Instant::now() + Duration::from_secs(2))
            .expect("forced shutdown");
        assert!(!status.success());
        assert!(child.wait_until(Instant::now()).expect("reaped").is_some());
    }

    #[tokio::test]
    async fn managed_tokio_child_wait_is_bounded_and_shutdown_reaps() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedTokioChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_hang_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        assert!(child.id().is_some());
        let _stdin = child.take_stdin().expect("stdin");
        let _stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        assert!(child
            .wait_until(Instant::now() + Duration::from_millis(50))
            .await
            .expect("bounded wait")
            .is_none());
        let status = child
            .shutdown(Instant::now() + Duration::from_secs(2))
            .await
            .expect("forced shutdown");
        assert!(!status.success());
        assert!(child
            .wait_until(Instant::now())
            .await
            .expect("reaped")
            .is_some());
    }

    #[cfg(windows)]
    #[test]
    fn managed_child_job_shutdown_terminates_launcher_descendants() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_descendant_launcher_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let parent_id = child.id().expect("parent pid");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let descendant_id = loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).expect("launcher output"), 0);
            if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                break value.parse::<u32>().expect("descendant pid");
            }
        };
        assert!(process_is_running(parent_id));
        assert!(process_is_running(descendant_id));

        let status = child
            .shutdown(Instant::now() + Duration::from_secs(3))
            .expect("job shutdown");

        assert!(!status.success());
        assert!(!process_is_running(parent_id));
        assert!(!process_is_running(descendant_id));
    }

    #[cfg(windows)]
    #[test]
    fn managed_child_job_close_terminates_launcher_descendants() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_descendant_launcher_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let parent_id = child.id().expect("parent pid");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let descendant_id = loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).expect("launcher output"), 0);
            if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                break value.parse::<u32>().expect("descendant pid");
            }
        };
        assert!(process_is_running(parent_id));
        assert!(process_is_running(descendant_id));

        let job_child = child.child.take().expect("job child");
        drop(job_child);
        wait_until_process_stops(parent_id);
        wait_until_process_stops(descendant_id);
    }

    #[cfg(windows)]
    #[test]
    fn managed_child_wait_does_not_finish_while_a_descendant_survives() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_exiting_launcher_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let descendant_id = loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).expect("launcher output"), 0);
            if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                break value.parse::<u32>().expect("descendant pid");
            }
        };
        assert!(process_is_running(descendant_id));
        assert!(child
            .wait_until(Instant::now() + Duration::from_millis(100))
            .expect("tree wait")
            .is_none());

        child
            .shutdown(Instant::now() + Duration::from_secs(3))
            .expect("job shutdown");

        assert!(!process_is_running(descendant_id));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn managed_tokio_job_shutdown_terminates_launcher_descendants() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedTokioChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_descendant_launcher_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let parent_id = child.id().expect("parent pid");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut lines = TokioBufReader::new(stdout).lines();
        let descendant_id = loop {
            let line = lines
                .next_line()
                .await
                .expect("launcher output")
                .expect("launcher line");
            if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                break value.parse::<u32>().expect("descendant pid");
            }
        };
        assert!(process_is_running(parent_id));
        assert!(process_is_running(descendant_id));

        let status = child
            .shutdown(Instant::now() + Duration::from_secs(3))
            .await
            .expect("job shutdown");

        assert!(!status.success());
        assert!(!process_is_running(parent_id));
        assert!(!process_is_running(descendant_id));
    }

    #[cfg(unix)]
    #[test]
    fn managed_child_process_group_uses_graceful_termination_first() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_graceful_termination_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).expect("fixture output"), 0);
            if line.trim() == "READY" {
                break;
            }
        }

        let status = child
            .shutdown(Instant::now() + Duration::from_secs(3))
            .expect("graceful shutdown");

        assert!(status.success());
    }

    #[cfg(unix)]
    #[test]
    fn managed_child_process_group_forces_ignored_signal_descendants() {
        let executable = std::env::current_exe().expect("test executable");
        let mut child = ManagedChild::spawn(
            &executable.to_string_lossy(),
            &fixture_args("managed_child_signal_ignoring_launcher_fixture"),
            &BTreeMap::new(),
        )
        .expect("managed child");
        let parent_id = child.id().expect("parent pid");
        let _stdin = child.take_stdin().expect("stdin");
        let stdout = child.take_stdout().expect("stdout");
        let _stderr = child.take_stderr().expect("stderr");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let descendant_id = loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).expect("launcher output"), 0);
            if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                break value.parse::<u32>().expect("descendant pid");
            }
        };

        let status = child
            .shutdown(Instant::now() + Duration::from_secs(3))
            .expect("forced group shutdown");

        assert!(!status.success());
        wait_until_process_stops(parent_id);
        wait_until_process_stops(descendant_id);
    }

    #[test]
    #[ignore = "spawned only by managed child tests"]
    fn managed_child_echo_fixture() {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).expect("stdin");
        print!("{line}");
    }

    #[test]
    #[ignore = "spawned only by managed child tests"]
    fn managed_child_hang_fixture() {
        thread::sleep(Duration::from_secs(30));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "spawned only by the Windows Job Object descendant test"]
    fn managed_child_descendant_launcher_fixture() {
        let executable = std::env::current_exe().expect("test executable");
        let mut descendant = Command::new(executable)
            .args(fixture_args("managed_child_descendant_fixture"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("descendant");
        println!("DESCENDANT_PID {}", descendant.id());
        std::io::stdout().flush().expect("flush pid");
        thread::sleep(Duration::from_secs(30));
        let _ = descendant.kill();
        let _ = descendant.wait();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "spawned only by the Windows Job Object tree-wait test"]
    #[allow(clippy::zombie_processes)] // This fixture must exit before its Job-owned descendant to exercise tree-aware wait semantics.
    fn managed_child_exiting_launcher_fixture() {
        let executable = std::env::current_exe().expect("test executable");
        let descendant = Command::new(executable)
            .args(fixture_args("managed_child_descendant_fixture"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("descendant");
        println!("DESCENDANT_PID {}", descendant.id());
        std::io::stdout().flush().expect("flush pid");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "spawned only by the Windows Job Object descendant launcher"]
    fn managed_child_descendant_fixture() {
        thread::sleep(Duration::from_secs(30));
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "spawned only by the Unix graceful-termination test"]
    fn managed_child_graceful_termination_fixture() {
        let previous = unsafe {
            libc::signal(
                libc::SIGTERM,
                graceful_termination_handler as *const () as libc::sighandler_t,
            )
        };
        assert_ne!(previous, libc::SIG_ERR);
        println!("READY");
        std::io::stdout().flush().expect("flush ready");
        thread::sleep(Duration::from_secs(30));
    }

    #[cfg(unix)]
    extern "C" fn graceful_termination_handler(_signal: i32) {
        unsafe { libc::_exit(0) }
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "spawned only by the Unix forced process-group test"]
    fn managed_child_signal_ignoring_launcher_fixture() {
        let previous = unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
        assert_ne!(previous, libc::SIG_ERR);
        let executable = std::env::current_exe().expect("test executable");
        let mut descendant = Command::new(executable)
            .args(fixture_args("managed_child_descendant_fixture"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("descendant");
        println!("DESCENDANT_PID {}", descendant.id());
        std::io::stdout().flush().expect("flush pid");
        thread::sleep(Duration::from_secs(30));
        let _ = descendant.kill();
        let _ = descendant.wait();
    }

    #[cfg(windows)]
    fn process_is_running(process_id: u32) -> bool {
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

    #[cfg(any(windows, unix))]
    fn wait_until_process_stops(process_id: u32) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while process_is_running(process_id) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !process_is_running(process_id),
            "process {process_id} survived"
        );
    }
}
