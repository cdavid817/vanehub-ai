use super::*;
use crate::contexts::workspaces::domain::{ShellRuntimeDescriptor, TerminalDimensions};
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
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
        read_only: false,
    });

    let session = service
        .create_shell(&CreateShellRequest {
            session_id: "session-1".to_string(),
            rows: 0,
            cols: 900,
            seat_id: None,
        })
        .expect("shell");

    assert_eq!(session.shell_id, "shell-fixture");
    assert_eq!(session.state, "connected");
    assert_eq!(session.runtime, ShellRuntimeDescriptor::Native);
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
                policy: ShellWorkspacePolicy {
                    requires_host_trust: false,
                },
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
                policy: ShellWorkspacePolicy {
                    requires_host_trust: false,
                },
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
                seat_id: None,
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
fn a_remote_workspace_describes_its_connection_witnesses() {
    let (service, runtime, _, _, _) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: None,
        remote: true,
        remote_endpoint: Some(ShellRemoteEndpoint {
            host: "build-host".to_string(),
            port: 22,
            user: "builder".to_string(),
            path: "/srv/app".to_string(),
            display_name: "build-host".to_string(),
            uri: "ssh://builder@build-host/srv/app".to_string(),
        }),
        ssh_binding: Some(ShellSshBinding {
            connection_id: "connection-7".to_string(),
            revision: 3,
        }),
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
        read_only: false,
    });

    let session = service
        .create_shell(&CreateShellRequest {
            session_id: "session-remote".to_string(),
            rows: 24,
            cols: 80,
            seat_id: None,
        })
        .expect("remote shell");

    assert_eq!(
        session.runtime,
        ShellRuntimeDescriptor::Remote {
            connection_id: "connection-7".to_string(),
            profile_revision: 3,
            supports_reconnect: false,
        }
    );
    assert_eq!(runtime.launches.lock().expect("launches").len(), 1);
}

#[test]
fn a_remote_workspace_without_a_binding_is_refused_before_any_runtime_effect() {
    // Without the binding there is nothing truthful to put in the descriptor, and the local PTY
    // path would happily open a shell at the remote path and call it `remote`.
    let (service, runtime, _, _, _) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: None,
        remote: true,
        remote_endpoint: Some(ShellRemoteEndpoint {
            host: "build-host".to_string(),
            port: 22,
            user: "builder".to_string(),
            path: "/srv/app".to_string(),
            display_name: "build-host".to_string(),
            uri: "ssh://builder@build-host/srv/app".to_string(),
        }),
        ssh_binding: None,
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
        read_only: false,
    });

    let error = service
        .create_shell(&CreateShellRequest {
            session_id: "session-remote".to_string(),
            rows: 24,
            cols: 80,
            seat_id: None,
        })
        .expect_err("missing binding is refused");

    assert_eq!(
        error,
        WorkspaceApplicationError::Validation(
            "Remote session workspace has no current SSH binding.".to_string()
        )
    );
    assert!(runtime.launches.lock().expect("launches").is_empty());
}

#[test]
fn shell_routes_and_cleanup_preserve_idempotence_events_and_bounds() {
    let (service, _, events, logs, calls) = shell_service(ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: Some("C:\\code\\app".to_string()),
        remote: false,
        remote_endpoint: None,
        ssh_binding: None,
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
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
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
        read_only: true,
    });

    let error = service
        .create_shell(&CreateShellRequest {
            session_id: "verifier-session".to_string(),
            rows: 24,
            cols: 80,
            seat_id: Some("seat-verifier".to_string()),
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
    // The seat travels with the record so a seat-filtered Logs query can find it.
    assert_eq!(log.seat_id.as_deref(), Some("seat-verifier"));
}

/// An evidence publisher that behaves the way a full queue and an unavailable recorder both do
/// from the producer's side: it accepts the call, keeps the signal, and reports nothing back.
#[derive(Clone, Default)]
struct RefusingEvidence {
    seen: Arc<Mutex<Vec<WorkspaceEvidenceSignal>>>,
}

impl WorkspaceEvidencePort for RefusingEvidence {
    fn try_publish(&self, signal: WorkspaceEvidenceSignal) {
        self.seen.lock().expect("seen").push(signal);
    }
}

/// Fails if consulted before the shell is open.
///
/// The ordering matters: an observation published first would exist for a shell that then failed
/// to open, and a reader cannot tell that record from one that succeeded.
struct EvidenceAfterOpenOnly {
    opened: Arc<Mutex<Vec<String>>>,
}

impl WorkspaceEvidencePort for EvidenceAfterOpenOnly {
    fn try_publish(&self, _signal: WorkspaceEvidenceSignal) {
        assert!(
            self.opened
                .lock()
                .expect("calls")
                .iter()
                .any(|call| call.starts_with("runtime:open:")),
            "evidence was published before the shell opened"
        );
    }
}

fn open_shell_workspace() -> ShellWorkspace {
    ShellWorkspace {
        agent_id: "codex-cli".to_string(),
        root: Some("C:\\code\\app".to_string()),
        remote: false,
        remote_endpoint: None,
        ssh_binding: None,
        policy: ShellWorkspacePolicy {
            requires_host_trust: false,
        },
        read_only: false,
    }
}

fn open_request() -> CreateShellRequest {
    CreateShellRequest {
        session_id: "session-1".to_string(),
        seat_id: Some("seat-builder".to_string()),
        rows: 24,
        cols: 80,
    }
}

/// The owning operation's result is identical whether or not the journal took the observation.
///
/// Both halves run the same request against the same doubles; only the publisher differs. A
/// refused publish that changed the returned session, or failed the call, would make observation a
/// precondition of the work being observed.
#[test]
fn a_refused_evidence_publish_does_not_change_the_shell_result() {
    let (baseline, ..) = shell_service(open_shell_workspace());
    let expected = baseline.create_shell(&open_request()).expect("baseline");

    let refusing = RefusingEvidence::default();
    let (service, ..) = shell_service(open_shell_workspace());
    let observed = service
        .with_evidence(Arc::new(refusing.clone()))
        .create_shell(&open_request())
        .expect("shell opens while evidence is refused");

    assert_eq!(observed.shell_id, expected.shell_id);
    assert_eq!(observed.session_id, expected.session_id);
    assert_eq!(observed.state, expected.state);
    assert_eq!(refusing.seen.lock().expect("seen").len(), 1);
}

#[test]
fn evidence_is_published_only_after_the_shell_is_open() {
    let (service, _runtime, _events, _logs, calls) = shell_service(open_shell_workspace());
    service
        .with_evidence(Arc::new(EvidenceAfterOpenOnly {
            opened: calls.clone(),
        }))
        .create_shell(&open_request())
        .expect("shell");
}

/// A denied request opens no shell, so it files no observation. A record of a shell that never
/// opened is worse than none: it reads as one that opened and then vanished.
#[test]
fn a_denied_shell_publishes_no_evidence() {
    let refusing = RefusingEvidence::default();
    let (service, ..) = shell_service(ShellWorkspace {
        read_only: true,
        ..open_shell_workspace()
    });

    let denied = service
        .with_evidence(Arc::new(refusing.clone()))
        .create_shell(&open_request());

    assert!(denied.is_err());
    assert!(refusing.seen.lock().expect("seen").is_empty());
}

/// The signal carries the shell's identity and which runtime opened it, never where it opened. A
/// path or a hostname would turn an identity record into a location record.
#[test]
fn a_shell_signal_carries_identifiers_and_a_runtime_kind_only() {
    let refusing = RefusingEvidence::default();
    let (service, ..) = shell_service(open_shell_workspace());
    service
        .with_evidence(Arc::new(refusing.clone()))
        .create_shell(&open_request())
        .expect("shell");

    let seen = refusing.seen.lock().expect("seen");
    let WorkspaceEvidenceSignal::ShellOpened {
        session_id,
        seat_id,
        runtime,
        ..
    } = &seen[0];
    assert_eq!(session_id, "session-1");
    assert_eq!(seat_id.as_deref(), Some("seat-builder"));
    assert_eq!(*runtime, WorkspaceShellRuntimeKind::Local);
    let rendered = format!("{:?}", seen[0]);
    assert!(
        !rendered.contains("C:"),
        "the workspace root reached the signal"
    );
    assert!(
        !rendered.contains("codex-cli"),
        "the agent id is not this signal's subject"
    );
}
