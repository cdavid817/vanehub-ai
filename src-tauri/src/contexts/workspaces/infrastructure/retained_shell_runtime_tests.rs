use super::retained_remote_shell::RoutedShellRuntime;
use super::retained_shell_runtime::RetainedLocalShellRuntime;
use crate::contexts::workspaces::application::{
    SessionShellRuntimePort, ShellOutputSink, ShellRemoteTarget, ShellRuntimeOpen,
    ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellForegroundProcessState, ShellId,
    ShellRuntimeDescriptor, ShellStream, TerminalDimensions,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct RecordingRuntime {
    label: &'static str,
    calls: Mutex<Vec<String>>,
    fail_open: bool,
}

impl RecordingRuntime {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            calls: Mutex::new(Vec::new()),
            fail_open: false,
        }
    }

    fn failing(label: &'static str) -> Self {
        Self {
            label,
            calls: Mutex::new(Vec::new()),
            fail_open: true,
        }
    }

    fn record(&self, call: &str) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("{}:{call}", self.label));
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }
}

impl SessionShellRuntimePort for RecordingRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        _sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        if self.fail_open {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: crate::contexts::workspaces::domain::shell_reason("unavailable"),
            });
        }
        self.record(&format!("open:{}", request.shell_id.as_str()));
        Ok(ShellRuntimeOpened {
            runtime: ShellRuntimeDescriptor::Native,
            state: SessionShellState::Running,
        })
    }

    fn write(&self, shell_id: &ShellId, _content: &str) -> Result<(), SessionShellError> {
        self.record(&format!("write:{}", shell_id.as_str()));
        Ok(())
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        _dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        self.record(&format!("resize:{}", shell_id.as_str()));
        Ok(())
    }

    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        self.record(&format!("close:{}", shell_id.as_str()));
        Ok(())
    }

    fn foreground_process(&self, _shell_id: &ShellId) -> ShellForegroundProcessState {
        ShellForegroundProcessState::Unknown
    }
}

#[derive(Default)]
struct SilentSink;

impl ShellOutputSink for SilentSink {
    fn on_output(&self, _shell_id: &ShellId, _stream: ShellStream, _bytes: &[u8]) {}

    fn on_state(&self, _shell_id: &ShellId, _state: SessionShellState) {}
}

fn shell(id: &str) -> ShellId {
    ShellId::parse(id).expect("shell id")
}

fn open_request(id: &str, remote: bool) -> ShellRuntimeOpen {
    ShellRuntimeOpen {
        shell_id: shell(id),
        session_id: "session-1".to_string(),
        root: "D:/project".to_string(),
        dimensions: TerminalDimensions::bounded(24, 80),
        remote: remote.then(|| ShellRemoteTarget {
            connection_id: "connection-1".to_string(),
            profile_revision: 3,
            path: "/srv/project".to_string(),
        }),
    }
}

#[test]
fn a_shell_stays_with_the_runtime_that_opened_it() {
    let local = Arc::new(RecordingRuntime::new("local"));
    let remote = Arc::new(RecordingRuntime::new("remote"));
    let routed = RoutedShellRuntime::new(local.clone(), remote.clone());

    routed
        .open(&open_request("shell-local", false), Arc::new(SilentSink))
        .expect("local open");
    routed
        .open(&open_request("shell-remote", true), Arc::new(SilentSink))
        .expect("remote open");
    routed.write(&shell("shell-local"), "ls\n").expect("write");
    routed.write(&shell("shell-remote"), "ls\n").expect("write");
    routed.close(&shell("shell-remote")).expect("close");

    // A Shell that could change which runtime it belongs to midway would be two shells sharing an
    // id, so the route is decided once at open and read back for every later call.
    assert_eq!(
        local.calls(),
        vec!["local:open:shell-local", "local:write:shell-local"]
    );
    assert_eq!(
        remote.calls(),
        vec![
            "remote:open:shell-remote",
            "remote:write:shell-remote",
            "remote:close:shell-remote"
        ]
    );
}

#[test]
fn a_failed_open_records_no_route() {
    let local = Arc::new(RecordingRuntime::failing("local"));
    let remote = Arc::new(RecordingRuntime::new("remote"));
    let routed = RoutedShellRuntime::new(local.clone(), remote.clone());

    routed
        .open(&open_request("shell-1", false), Arc::new(SilentSink))
        .expect_err("open fails");
    // A route to a Shell that does not exist would send a later write into a runtime that never
    // opened it.
    routed
        .close(&shell("shell-1"))
        .expect("close is idempotent");
    assert!(remote.calls().is_empty());
}

/// Closing a Shell the runtime does not hold is a success: a registry entry can outlive its
/// process, and a close that failed on that would make cleanup unreliable exactly where it matters.
#[test]
fn closing_an_unknown_local_shell_succeeds() {
    let runtime = RetainedLocalShellRuntime::default();
    runtime.close(&shell("shell-missing")).expect("close");
    assert_eq!(
        runtime.foreground_process(&shell("shell-missing")),
        ShellForegroundProcessState::Absent
    );
}

/// A local PTY exposes no reliable foreground marker, and guessing one from terminal text would be
/// parsing output to invent a fact.
#[test]
fn a_local_runtime_refuses_a_remote_open_rather_than_opening_here() {
    let runtime = RetainedLocalShellRuntime::default();

    let error = runtime
        .open(&open_request("shell-1", true), Arc::new(SilentSink))
        .expect_err("remote refused");

    // Opening a local PTY at a remote path would open a shell on this machine and label it remote.
    assert_eq!(error.code(), "shell_runtime_unavailable");
}
