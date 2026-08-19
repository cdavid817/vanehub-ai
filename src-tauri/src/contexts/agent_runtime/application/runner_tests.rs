use super::{
    AgentRunner, PreparedRunnerLaunch, RunnerCapabilities, RunnerDescriptor, RunnerError,
    RunnerErrorKind, RunnerEvent, RunnerHandle, RunnerInspection, RunnerKind, RunnerLaunchSpec,
    RunnerRecoveryMode, RunnerReference, RunnerSelection,
};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

struct ContractRunner {
    events: Mutex<VecDeque<RunnerEvent>>,
}

impl ContractRunner {
    fn new(events: Vec<RunnerEvent>) -> Self {
        Self {
            events: Mutex::new(events.into()),
        }
    }
}

impl AgentRunner for ContractRunner {
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
        spec.validate()?;
        Ok(PreparedRunnerLaunch {
            reference: reference(),
            spec,
            preparation_id: None,
            admission_id: None,
        })
    }

    fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
        Ok(RunnerHandle {
            id: "runner-handle-1".into(),
            reference: prepared.reference,
            process_reference: Some("opaque-process-1".into()),
        })
    }

    fn send_input(&self, _: &RunnerHandle, _: &[u8]) -> Result<(), RunnerError> {
        Ok(())
    }

    fn next_event(&self, _: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
        Ok(self.events.lock().expect("events").pop_front())
    }

    fn cancel(&self, _: &RunnerHandle) -> Result<bool, RunnerError> {
        Ok(true)
    }

    fn inspect(&self, _: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
        Ok(RunnerInspection::Running)
    }

    fn cleanup(&self, _: &RunnerHandle) -> Result<(), RunnerError> {
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

#[test]
fn shared_contract_exercises_prepare_spawn_stream_cancel_and_cleanup() {
    let runner = ContractRunner::new(vec![
        RunnerEvent::Stdout(b"ready".to_vec()),
        RunnerEvent::Exited(Some(0)),
    ]);
    assert_eq!(runner.kind(), RunnerKind::Local);
    assert!(runner.capabilities().cancellation);
    let prepared = runner
        .prepare(&RunnerSelection::default(), launch_spec())
        .expect("prepare");
    let handle = runner.spawn(prepared).expect("spawn");
    runner.send_input(&handle, b"hello\n").expect("input");
    assert_eq!(
        runner.next_event(&handle).expect("event"),
        Some(RunnerEvent::Stdout(b"ready".to_vec()))
    );
    assert_eq!(
        runner.inspect(&handle).expect("inspect"),
        RunnerInspection::Running
    );
    assert!(runner.cancel(&handle).expect("cancel"));
    runner.cleanup(&handle).expect("cleanup");
    assert_eq!(
        runner.recover(&reference(), None).expect("recover"),
        RunnerInspection::Unknown
    );
}

#[test]
fn selections_and_launches_reject_unsafe_or_unsupported_values() {
    assert_eq!(
        RunnerSelection::ssh("bad target".into(), 1)
            .expect_err("target")
            .kind,
        RunnerErrorKind::InvalidSelection
    );
    let mut invalid_local = RunnerSelection::local();
    invalid_local.target_id = Some("ssh-1".into());
    assert!(invalid_local.validate().is_err());
    assert_eq!(
        RunnerSelection {
            kind: RunnerKind::Docker,
            target_id: None,
            target_revision: None,
        }
        .validate()
        .expect_err("unsupported")
        .code(),
        "runner_unsupported_capability"
    );
    let mut spec = launch_spec();
    spec.environment.insert("BAD-NAME".into(), "secret".into());
    assert_eq!(
        spec.validate().expect_err("environment").kind,
        RunnerErrorKind::InvalidLaunch
    );
    let mut unapproved_environment = launch_spec();
    unapproved_environment
        .environment
        .insert("API_TOKEN".into(), "secret-value".into());
    assert_eq!(
        unapproved_environment
            .validate_for(RunnerKind::Local)
            .expect_err("unapproved environment")
            .kind,
        RunnerErrorKind::PermissionDenied
    );
    let mut remote_secret = launch_spec();
    remote_secret.arguments = vec!["--api-key=secret-value".into()];
    assert_eq!(
        remote_secret
            .validate_for(RunnerKind::Ssh)
            .expect_err("remote secret")
            .kind,
        RunnerErrorKind::PermissionDenied
    );
    assert!(remote_secret.validate_for(RunnerKind::Local).is_ok());
}

#[test]
fn descriptors_events_recovery_and_error_codes_cover_the_stable_vocabulary() {
    for (kind, label) in [
        (RunnerKind::Local, "local"),
        (RunnerKind::Ssh, "ssh"),
        (RunnerKind::Docker, "docker"),
        (RunnerKind::Cloud, "cloud"),
    ] {
        assert_eq!(kind.as_str(), label);
    }
    for (mode, label) in [
        (RunnerRecoveryMode::None, "none"),
        (RunnerRecoveryMode::InspectOnly, "inspect_only"),
        (RunnerRecoveryMode::Reattach, "reattach"),
    ] {
        assert_eq!(mode.as_str(), label);
    }
    let descriptor = RunnerDescriptor {
        selection: RunnerSelection::local(),
        label: "Local".into(),
        host_label: Some("This device".into()),
        available: true,
        unavailable_reason: None,
        simulated: false,
        capabilities: ContractRunner::new(Vec::new()).capabilities(),
    };
    descriptor.validate().expect("descriptor");
    assert!(!descriptor.simulated);
    assert!(descriptor.capabilities.interactive_input);
    assert_eq!(
        [RunnerEvent::Stderr(Vec::new()), RunnerEvent::Disconnected].len(),
        2
    );
    assert_eq!(
        [
            RunnerInspection::Exited(Some(0)),
            RunnerInspection::Disconnected
        ]
        .len(),
        2
    );
    let kinds = [
        RunnerErrorKind::AuthorityStale,
        RunnerErrorKind::PermissionDenied,
        RunnerErrorKind::Preparation,
        RunnerErrorKind::Spawn,
        RunnerErrorKind::Input,
        RunnerErrorKind::Disconnected,
        RunnerErrorKind::ReconnectExhausted,
        RunnerErrorKind::Cancellation,
        RunnerErrorKind::Inspection,
        RunnerErrorKind::Cleanup,
        RunnerErrorKind::ResourceExhausted,
    ];
    assert!(kinds.iter().all(|kind| kind.code().starts_with("runner_")));
}

fn launch_spec() -> RunnerLaunchSpec {
    RunnerLaunchSpec {
        session_id: Some("session-1".into()),
        executable: "fixture-cli".into(),
        arguments: vec!["--json".into()],
        cwd: Some("workspace".into()),
        environment: BTreeMap::from([("TRACEPARENT".into(), "00-safe".into())]),
        pipe_stdin: true,
    }
}

fn reference() -> RunnerReference {
    RunnerReference {
        kind: RunnerKind::Local,
        target_id: "local".into(),
        target_revision: None,
        recovery: RunnerRecoveryMode::None,
        authority_witness: "local-policy-v1".into(),
    }
}

/// gemini-cli, opencode and antigravity-cli receive the prompt as an argv entry rather than on
/// stdin, and VaneHub composes that prompt from several sections, so it always contains newlines.
/// Rejecting every control character therefore made a chat turn impossible for three of the five
/// managed CLI Agents -- `runner_invalid_launch` before the process was ever spawned.
///
/// Newlines are not an injection vector here: arguments are handed to the OS as an array, never
/// through a shell. NUL is different -- it terminates a C string and would silently truncate the
/// argument the caller believes it passed -- so it stays rejected.
#[test]
fn a_multi_line_prompt_argument_is_accepted_but_a_nul_is_not() {
    let mut multi_line = launch_spec();
    multi_line.arguments = vec![
        "run".into(),
        "System context\n\nUser: reply with PONG\n".into(),
    ];
    multi_line
        .validate()
        .expect("a composed prompt delivered through argv must launch");

    let mut tabbed = launch_spec();
    tabbed.arguments = vec!["--format".into(), "a\tb\r\nc".into()];
    tabbed
        .validate()
        .expect("tabs and carriage returns are ordinary prompt text");

    let mut nul = launch_spec();
    nul.arguments = vec!["run".into(), "before\u{0}after".into()];
    assert_eq!(
        nul.validate().expect_err("NUL truncates the argument").kind,
        RunnerErrorKind::InvalidLaunch
    );

    let mut escape = launch_spec();
    escape.arguments = vec!["run".into(), "\u{1b}[31mred\u{1b}[0m".into()];
    assert_eq!(
        escape
            .validate()
            .expect_err("terminal escape sequences are not prompt text")
            .kind,
        RunnerErrorKind::InvalidLaunch
    );

    // The executable itself is an identifier, not free text, and stays strict.
    let mut executable = launch_spec();
    executable.executable = "fixture\ncli".into();
    assert_eq!(
        executable.validate().expect_err("executable name").kind,
        RunnerErrorKind::InvalidLaunch
    );
}
