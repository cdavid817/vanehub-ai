use super::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize};

struct Sessions {
    target: Option<AgentSessionRunnerTarget>,
}

impl SshSessionTargetPort for Sessions {
    fn current_target(&self, _session_id: &str) -> Result<Option<AgentSessionRunnerTarget>, ()> {
        Ok(self.target.clone())
    }
}

struct Channel {
    events: Mutex<VecDeque<SshExecutionChannelEvent>>,
    closed: Arc<AtomicBool>,
    inputs: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl SshRunnerChannelPort for Channel {
    fn write(&self, content: &[u8]) -> SshRunnerResult<()> {
        self.inputs.lock().expect("inputs").push(content.to_vec());
        Ok(())
    }

    fn next_event(&self) -> SshRunnerResult<Option<SshExecutionChannelEvent>> {
        Ok(self.events.lock().expect("events").pop_front())
    }

    fn close(&self) -> SshRunnerResult<()> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

struct Lease {
    probe_succeeds: bool,
    closed: Arc<AtomicBool>,
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
    process_events: Arc<Mutex<Option<VecDeque<SshExecutionChannelEvent>>>>,
    inputs: Arc<Mutex<Vec<Vec<u8>>>>,
    control_results: Arc<Mutex<VecDeque<bool>>>,
    keepalives: Arc<AtomicUsize>,
}

impl SshRunnerLeasePort for Lease {
    fn is_healthy(&self) -> bool {
        true
    }

    fn keepalive(&self) -> SshRunnerResult<()> {
        self.keepalives.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn open_exec(&self, command: &[u8]) -> SshRunnerResult<Arc<dyn SshRunnerChannelPort>> {
        self.commands
            .lock()
            .expect("commands")
            .push(command.to_vec());
        let events = if command.starts_with(b"command -v") {
            VecDeque::from([
                SshExecutionChannelEvent::ExitStatus(u32::from(!self.probe_succeeds)),
                SshExecutionChannelEvent::Closed,
            ])
        } else if command
            .windows(b"setsid env".len())
            .any(|part| part == b"setsid env")
        {
            self.process_events
                .lock()
                .expect("process events")
                .take()
                .unwrap_or_default()
        } else {
            let success = self
                .control_results
                .lock()
                .expect("control results")
                .pop_front()
                .unwrap_or(false);
            VecDeque::from([SshExecutionChannelEvent::ExitStatus(u32::from(!success))])
        };
        Ok(Arc::new(Channel {
            events: Mutex::new(events),
            closed: self.closed.clone(),
            inputs: self.inputs.clone(),
        }))
    }
}

struct Gateway {
    profile: SshExecutionProfile,
    pool: Vec<SshExecutionPoolSnapshot>,
    probe_succeeds: bool,
    acquisitions: AtomicUsize,
    closed: Arc<AtomicBool>,
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
    process_events: Arc<Mutex<Option<VecDeque<SshExecutionChannelEvent>>>>,
    inputs: Arc<Mutex<Vec<Vec<u8>>>>,
    control_results: Arc<Mutex<VecDeque<bool>>>,
    keepalives: Arc<AtomicUsize>,
}

impl SshRunnerGateway for Gateway {
    fn profile(&self, _connection_id: &str) -> SshRunnerResult<SshExecutionProfile> {
        Ok(self.profile.clone())
    }

    fn pool_snapshot(&self) -> Vec<SshExecutionPoolSnapshot> {
        self.pool.clone()
    }

    fn acquire(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> SshRunnerResult<Arc<dyn SshRunnerLeasePort>> {
        self.acquisitions.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(Lease {
            probe_succeeds: self.probe_succeeds,
            closed: self.closed.clone(),
            commands: self.commands.clone(),
            process_events: self.process_events.clone(),
            inputs: self.inputs.clone(),
            control_results: self.control_results.clone(),
            keepalives: self.keepalives.clone(),
        }))
    }
}

fn target() -> AgentSessionRunnerTarget {
    AgentSessionRunnerTarget {
        session_id: "session-1".to_string(),
        connection_id: "ssh-1".to_string(),
        connection_revision: 7,
        host: "host.example".to_string(),
        port: 22,
        user: "runner".to_string(),
        workspace_path: "/srv/workspace".to_string(),
        display_name: "Build host".to_string(),
    }
}

fn profile() -> SshExecutionProfile {
    SshExecutionProfile {
        connection_id: "ssh-1".to_string(),
        revision: 7,
        host: "host.example".to_string(),
        port: 22,
        user: "runner".to_string(),
        host_trusted: true,
        credential_configured: true,
    }
}

fn launch() -> RunnerLaunchSpec {
    RunnerLaunchSpec {
        session_id: Some("session-1".to_string()),
        executable: "codex".to_string(),
        arguments: vec!["exec".to_string()],
        cwd: Some("C:/caller-must-not-control-remote-cwd".to_string()),
        environment: BTreeMap::new(),
        pipe_stdin: true,
    }
}

fn runner(
    target: Option<AgentSessionRunnerTarget>,
    profile: SshExecutionProfile,
    pool: Vec<SshExecutionPoolSnapshot>,
    probe_succeeds: bool,
) -> (SshRunner, Arc<Gateway>) {
    let gateway = Arc::new(Gateway {
        profile,
        pool,
        probe_succeeds,
        acquisitions: AtomicUsize::new(0),
        closed: Arc::new(AtomicBool::new(false)),
        commands: Arc::new(Mutex::new(Vec::new())),
        process_events: Arc::new(Mutex::new(Some(VecDeque::from([
            SshExecutionChannelEvent::Output(b"\x1eVANEHUB_PID:42\n".to_vec()),
            SshExecutionChannelEvent::Output(b"provider output\n".to_vec()),
            SshExecutionChannelEvent::ExtendedOutput {
                stream: 1,
                content: b"safe stderr".to_vec(),
            },
            SshExecutionChannelEvent::Output(b"\x1eVANEHUB_EXIT:0\n".to_vec()),
        ])))),
        inputs: Arc::new(Mutex::new(Vec::new())),
        control_results: Arc::new(Mutex::new(VecDeque::from([true, true, true, true]))),
        keepalives: Arc::new(AtomicUsize::new(0)),
    });
    (
        SshRunner::with_ports(Arc::new(Sessions { target }), gateway.clone()),
        gateway,
    )
}

#[test]
fn spawn_streams_provider_bytes_maps_exit_and_closes_only_its_channel() {
    let (runner, gateway) = runner(Some(target()), profile(), Vec::new(), true);
    let prepared = runner
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            launch(),
        )
        .expect("prepared");
    let handle = runner.spawn(prepared).expect("spawned");
    assert!(handle.process_reference.is_some());
    assert!(!handle.process_reference.as_deref().unwrap().contains("42"));

    runner.send_input(&handle, b"prompt\n").expect("input");
    assert_eq!(
        *gateway.inputs.lock().expect("inputs"),
        [b"prompt\n".to_vec()]
    );
    assert_eq!(
        runner.next_event(&handle).expect("stdout"),
        Some(RunnerEvent::Stdout(b"provider output\n".to_vec()))
    );
    assert_eq!(
        runner.next_event(&handle).expect("stderr"),
        Some(RunnerEvent::Stderr(b"safe stderr".to_vec()))
    );
    assert_eq!(
        runner.next_event(&handle).expect("exit"),
        Some(RunnerEvent::Exited(Some(0)))
    );
    assert_eq!(
        runner.inspect(&handle).expect("inspection"),
        RunnerInspection::Exited(Some(0))
    );
    runner.cleanup(&handle).expect("cleanup");
    runner.cleanup(&handle).expect("idempotent cleanup");
}

#[test]
fn disconnect_is_explicit_and_inspect_only_recovery_never_replays_input() {
    let (runner, gateway) = runner(Some(target()), profile(), Vec::new(), true);
    *gateway.process_events.lock().expect("process events") = Some(VecDeque::from([
        SshExecutionChannelEvent::Output(b"\x1eVANEHUB_PID:42\n".to_vec()),
        SshExecutionChannelEvent::Closed,
    ]));
    let prepared = runner
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            launch(),
        )
        .expect("prepared");
    let handle = runner.spawn(prepared).expect("spawned");

    assert_eq!(
        runner.next_event(&handle).expect("disconnect event"),
        Some(RunnerEvent::Disconnected)
    );
    assert_eq!(
        runner.inspect(&handle).expect("disconnected inspection"),
        RunnerInspection::Disconnected
    );
    assert_eq!(
        runner
            .recover(&handle.reference, handle.process_reference.as_deref())
            .expect("bounded inspect-only recovery"),
        RunnerInspection::Running
    );
    assert!(gateway.inputs.lock().expect("inputs").is_empty());
    assert_eq!(
        runner
            .recover(&handle.reference, Some("not-an-opaque-reference"))
            .expect_err("invalid process reference")
            .kind,
        RunnerErrorKind::Inspection
    );
    assert_eq!(gateway.keepalives.load(Ordering::SeqCst), 2);
}

#[test]
fn cancellation_targets_only_the_owned_group_and_escalates_within_the_budget() {
    let (runner, gateway) = runner(Some(target()), profile(), Vec::new(), true);
    let prepared = runner
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            launch(),
        )
        .expect("prepared");
    let handle = runner.spawn(prepared).expect("spawned");

    assert!(runner.cancel(&handle).expect("cancelled"));
    assert!(!runner.cancel(&handle).expect("idempotent cancellation"));
    assert_eq!(
        runner.inspect(&handle).expect("inspection"),
        RunnerInspection::Exited(None)
    );
    let commands = gateway.commands.lock().expect("commands");
    let controls = commands
        .iter()
        .filter_map(|command| String::from_utf8(command.clone()).ok())
        .filter(|command| command.starts_with("kill -"))
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        [
            "kill -0 -- -42 >/dev/null 2>&1",
            "kill -TERM -- -42 >/dev/null 2>&1",
            "kill -0 -- -42 >/dev/null 2>&1",
            "kill -0 -- -42 >/dev/null 2>&1",
            "kill -KILL -- -42 >/dev/null 2>&1",
        ]
    );
    assert!(commands
        .iter()
        .all(|command| !String::from_utf8_lossy(command).contains("ssh-2")));
}

#[test]
fn preparation_revalidates_binding_endpoint_trust_credential_and_command() {
    let (runner, gateway) = runner(Some(target()), profile(), Vec::new(), true);
    let prepared = runner
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            launch(),
        )
        .expect("prepared");

    assert_eq!(prepared.spec.cwd.as_deref(), Some("/srv/workspace"));
    assert_eq!(prepared.reference.kind, RunnerKind::Ssh);
    assert_eq!(prepared.reference.recovery, RunnerRecoveryMode::InspectOnly);
    assert!(prepared.preparation_id.is_some());
    assert_eq!(gateway.acquisitions.load(Ordering::SeqCst), 1);
    assert_eq!(gateway.keepalives.load(Ordering::SeqCst), 1);
    assert!(gateway.closed.load(Ordering::SeqCst));
    let commands = gateway.commands.lock().expect("commands");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0], b"command -v 'codex' >/dev/null 2>&1");
    let stored = runner.prepared.lock().expect("prepared launches");
    let stored = stored.values().next().expect("stored preparation");
    assert!(String::from_utf8_lossy(&stored.command).contains("/srv/workspace"));
    assert!(stored.lease.is_healthy());
}

#[test]
fn the_same_provider_launch_contract_is_accepted_by_local_and_ssh_runners() {
    use crate::contexts::agent_runtime::infrastructure::LocalRunner;

    let provider_launch = launch();
    let local = LocalRunner::new()
        .prepare(&RunnerSelection::local(), provider_launch.clone())
        .expect("Local preparation");
    let (ssh, _) = runner(Some(target()), profile(), Vec::new(), true);
    let remote = ssh
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            provider_launch,
        )
        .expect("SSH preparation");

    assert_eq!(local.spec.executable, remote.spec.executable);
    assert_eq!(local.spec.arguments, remote.spec.arguments);
    assert_eq!(local.reference.kind, RunnerKind::Local);
    assert_eq!(remote.reference.kind, RunnerKind::Ssh);
}

#[test]
fn preparation_fails_closed_without_local_fallback_or_authentication() {
    let mut stale_profile = profile();
    stale_profile.host = "other.example".to_string();
    let (runner, gateway) = runner(Some(target()), stale_profile, Vec::new(), true);
    assert_eq!(
        runner
            .prepare(
                &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
                launch()
            )
            .expect_err("stale endpoint")
            .kind,
        RunnerErrorKind::AuthorityStale
    );
    assert_eq!(gateway.acquisitions.load(Ordering::SeqCst), 0);
    assert_eq!(
        runner
            .prepare(&RunnerSelection::local(), launch())
            .expect_err("no Local fallback")
            .kind,
        RunnerErrorKind::InvalidSelection
    );
}

#[test]
fn preparation_rejects_untrusted_missing_credentials_unavailable_command_and_bad_pool() {
    let mut unsafe_profile = profile();
    unsafe_profile.host_trusted = false;
    assert_eq!(
        runner(Some(target()), unsafe_profile, Vec::new(), true)
            .0
            .prepare(
                &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
                launch()
            )
            .expect_err("untrusted")
            .kind,
        RunnerErrorKind::Preparation
    );

    let mut missing_credential = profile();
    missing_credential.credential_configured = false;
    assert!(runner(Some(target()), missing_credential, Vec::new(), true)
        .0
        .prepare(
            &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
            launch()
        )
        .is_err());

    assert_eq!(
        runner(Some(target()), profile(), Vec::new(), false)
            .0
            .prepare(
                &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
                launch()
            )
            .expect_err("command missing")
            .kind,
        RunnerErrorKind::Preparation
    );

    let bad_pool = vec![SshExecutionPoolSnapshot {
        connection_id: "ssh-1".to_string(),
        revision: 7,
        leases: 1,
        health: SshExecutionPoolHealth::Draining,
    }];
    assert_eq!(
        runner(Some(target()), profile(), bad_pool, true)
            .0
            .prepare(
                &RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection"),
                launch()
            )
            .expect_err("draining pool")
            .kind,
        RunnerErrorKind::Disconnected
    );
}

#[test]
fn unapproved_secret_or_environment_is_rejected_before_transport_acquisition() {
    let (runner, gateway) = runner(Some(target()), profile(), Vec::new(), true);
    let selection = RunnerSelection::ssh("ssh-1".to_string(), 7).expect("selection");
    let mut secret_argument = launch();
    secret_argument.arguments = vec!["exec".into(), "--api-key=do-not-forward".into()];
    assert_eq!(
        runner
            .prepare(&selection, secret_argument)
            .expect_err("secret argument")
            .kind,
        RunnerErrorKind::PermissionDenied
    );
    let mut secret_environment = launch();
    secret_environment
        .environment
        .insert("ACCESS_TOKEN".into(), "do-not-forward".into());
    assert_eq!(
        runner
            .prepare(&selection, secret_environment)
            .expect_err("secret environment")
            .kind,
        RunnerErrorKind::PermissionDenied
    );
    assert_eq!(gateway.acquisitions.load(Ordering::SeqCst), 0);
    assert!(gateway.commands.lock().expect("commands").is_empty());
}
