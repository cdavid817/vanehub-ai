use super::*;
use crate::contexts::workspaces::domain::TerminalDimensions;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct FakeShellContext {
    workspace: ShellWorkspace,
    calls: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceShellContextPort for FakeShellContext {
    fn load_shell_workspace(
        &self,
        session_id: &str,
    ) -> Result<ShellWorkspace, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("context:{session_id}"));
        Ok(self.workspace.clone())
    }
}

#[derive(Clone, Default)]
struct FakeShellRuntime {
    calls: Arc<Mutex<Vec<String>>>,
    launches: Arc<Mutex<Vec<ShellLaunch>>>,
}

impl WorkspaceShellRuntimePort for FakeShellRuntime {
    fn open_shell(&self, launch: &ShellLaunch) -> Result<(), WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("runtime:open:{}", launch.shell_id));
        self.launches.lock().expect("launches").push(launch.clone());
        Ok(())
    }

    fn write_input(&self, shell_id: &str, content: &str) -> Result<(), WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("runtime:input:{shell_id}:{content}"));
        Ok(())
    }

    fn reset_directory(&self, shell_id: &str) -> Result<(), WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("runtime:cd:{shell_id}"));
        Ok(())
    }

    fn resize(
        &self,
        shell_id: &str,
        dimensions: TerminalDimensions,
    ) -> Result<(), WorkspaceApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "runtime:resize:{shell_id}:{}:{}",
            dimensions.rows(),
            dimensions.cols()
        ));
        Ok(())
    }

    fn stop(&self, shell_id: &str) -> Result<Option<String>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("runtime:stop:{shell_id}"));
        Ok((shell_id != "missing").then(|| "session-1".to_string()))
    }

    fn stop_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<(String, String)>, WorkspaceApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("runtime:stop-session:{session_id}"));
        Ok(vec![("shell-session".to_string(), session_id.to_string())])
    }
}

#[derive(Clone, Copy)]
struct FixedShellId;

impl WorkspaceShellIdPort for FixedShellId {
    fn next_shell_id(&self) -> String {
        "shell-fixture".to_string()
    }
}

#[derive(Clone, Default)]
struct CapturingShellEvents {
    events: Arc<Mutex<Vec<ShellEvent>>>,
}

impl WorkspaceShellEventPort for CapturingShellEvents {
    fn publish(&self, event: ShellEvent) {
        self.events.lock().expect("events").push(event);
    }
}

#[derive(Clone, Default)]
struct CapturingShellLogs {
    logs: Arc<Mutex<Vec<ShellLog>>>,
}

impl WorkspaceShellLogPort for CapturingShellLogs {
    fn write(&self, log: ShellLog) {
        self.logs.lock().expect("logs").push(log);
    }
}

fn shell_service(
    workspace: ShellWorkspace,
) -> (
    WorkspaceShellApplicationService,
    FakeShellRuntime,
    CapturingShellEvents,
    CapturingShellLogs,
    Arc<Mutex<Vec<String>>>,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = FakeShellRuntime {
        calls: calls.clone(),
        ..FakeShellRuntime::default()
    };
    let events = CapturingShellEvents::default();
    let logs = CapturingShellLogs::default();
    (
        WorkspaceShellApplicationService::new(
            Arc::new(FakeShellContext {
                workspace,
                calls: calls.clone(),
            }),
            Arc::new(runtime.clone()),
            Arc::new(FixedShellId),
            Arc::new(events.clone()),
            Arc::new(logs.clone()),
        ),
        runtime,
        events,
        logs,
        calls,
    )
}

#[test]
fn shell_creation_validates_workspace_bounds_dimensions_and_logs_after_open() {
    let (service, runtime, _, logs, calls) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: Some("C:\\code\\app".to_string()),
        remote: false,
        remote_endpoint: None,
        ssh_binding: None,
        policy: ShellWorkspacePolicy { requires_host_trust: false },
        read_only: false,
    });

    let session = service
        .create_shell(&CreateShellRequest {
            session_id: "session-1".to_string(),
            rows: 0,
            cols: 900,
        })
        .expect("shell");

    assert_eq!(session.shell_id, "shell-fixture");
    assert_eq!(session.state, "connected");
    let launch = runtime.launches.lock().expect("launches")[0].clone();
    assert_eq!(launch.root, "C:\\code\\app");
    assert_eq!(launch.dimensions.rows(), 1);
    assert_eq!(launch.dimensions.cols(), 500);
    assert_eq!(
        *calls.lock().expect("calls"),
        vec!["context:session-1", "runtime:open:shell-fixture"]
    );
    assert_eq!(
        logs.logs.lock().expect("logs")[0].message,
        "Shell connected for agent codex-cli."
    );
}

#[test]
fn remote_and_unavailable_workspaces_stop_before_runtime_effects() {
    for (workspace, expected) in [
        (
            ShellWorkspace {
                agent_id: "codex-cli".to_string(),
                root: None,
                remote: true,
                remote_endpoint: None,
                ssh_binding: None,
                policy: ShellWorkspacePolicy { requires_host_trust: false },
                read_only: false,
            },
            "Session workspace is unavailable.",
        ),
        (
            ShellWorkspace {
                agent_id: "codex-cli".to_string(),
                root: None,
                remote: false,
                remote_endpoint: None,
                ssh_binding: None,
                policy: ShellWorkspacePolicy { requires_host_trust: false },
                read_only: false,
            },
            "Session workspace is unavailable.",
        ),
    ] {
        let (service, runtime, _, logs, _) = shell_service(workspace);
        let error = service
            .create_shell(&CreateShellRequest {
                session_id: "session-1".to_string(),
                rows: 24,
                cols: 80,
            })
            .expect_err("validation error");
        assert_eq!(
            error,
            WorkspaceApplicationError::Validation(expected.to_string())
        );
        assert!(runtime.launches.lock().expect("launches").is_empty());
        assert!(logs.logs.lock().expect("logs").is_empty());
    }
}

#[test]
fn shell_routes_and_cleanup_preserve_idempotence_events_and_bounds() {
    let (service, _, events, logs, calls) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: Some("C:\\code\\app".to_string()),
        remote: false,
        remote_endpoint: None,
        ssh_binding: None,
        policy: ShellWorkspacePolicy { requires_host_trust: false },
        read_only: false,
    });

    service
        .write_input("shell-one", "echo fixture")
        .expect("input");
    service.reset_directory("shell-one").expect("cd");
    service
        .resize_shell(&ResizeShellRequest {
            shell_id: "shell-one".to_string(),
            rows: 800,
            cols: 0,
        })
        .expect("resize");
    service.kill_shell("missing").expect("idempotent kill");
    service.kill_shell("shell-one").expect("kill");
    service
        .kill_for_session("session-two")
        .expect("session cleanup");

    assert_eq!(
        *calls.lock().expect("calls"),
        vec![
            "runtime:input:shell-one:echo fixture",
            "runtime:cd:shell-one",
            "runtime:resize:shell-one:500:1",
            "runtime:stop:missing",
            "runtime:stop:shell-one",
            "runtime:stop-session:session-two",
        ]
    );
    assert_eq!(events.events.lock().expect("events").len(), 2);
    assert_eq!(logs.logs.lock().expect("logs").len(), 2);
    assert!(logs
        .logs
        .lock()
        .expect("logs")
        .iter()
        .all(|log| log.message == "Shell disconnected."));
}

#[test]
fn verifier_shell_is_rejected_before_runtime_open_and_logged() {
    let (service, runtime, _, logs, calls) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: Some("C:\\code\\app".to_string()),
        remote: false,
        remote_endpoint: None,
        ssh_binding: None,
        policy: ShellWorkspacePolicy { requires_host_trust: false },
        read_only: true,
    });

    let error = service
        .create_shell(&CreateShellRequest {
            session_id: "verifier-session".to_string(),
            rows: 24,
            cols: 80,
        })
        .expect_err("verifier shell rejected");

    assert_eq!(
        error,
        WorkspaceApplicationError::PolicyDenied {
            session_id: "verifier-session".to_string(),
            action: "create-shell".to_string(),
        }
    );
    assert!(runtime.launches.lock().expect("launches").is_empty());
    assert_eq!(
        *calls.lock().expect("calls"),
        vec!["context:verifier-session"]
    );
    let log = &logs.logs.lock().expect("logs")[0];
    assert_eq!(log.level, WorkspaceLogLevel::Warn);
    assert!(log.message.contains("read-only policy"));
}
