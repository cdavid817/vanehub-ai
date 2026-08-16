use super::*;

fn run(max_retries: u32) -> AgentRun {
    AgentRun::create(RunCreation {
        id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0".into(),
        owner: RunOwner {
            owner_type: "session_generation".into(),
            owner_id: "generation-1".into(),
        },
        links: vec![],
        parent_run_id: None,
        recovery_policy: RunRecoveryPolicy::NotRecoverable,
        max_retries,
        timestamp: "2026-08-16T00:00:00Z".into(),
        witness: "created-1".into(),
    })
    .expect("valid run")
    .0
}

fn move_run(
    run: &mut AgentRun,
    trigger: RunTrigger,
    witness: &str,
) -> Result<Option<RunEvent>, RunDomainError> {
    run.transition(RunTransition {
        trigger,
        timestamp: format!("2026-08-16T00:00:{:02}Z", run.version),
        reason_code: None,
        witness: witness.into(),
    })
}

#[test]
fn normal_and_waiting_paths_are_guarded() {
    let mut value = run(2);
    for (trigger, expected) in [
        (RunTrigger::Prepare, RunState::Preparing),
        (RunTrigger::Start, RunState::Running),
        (RunTrigger::RequestApproval, RunState::WaitingApproval),
        (RunTrigger::ApprovalGranted, RunState::Running),
        (RunTrigger::AskUser, RunState::WaitingUser),
        (RunTrigger::UserAnswered, RunState::Running),
        (RunTrigger::Verify, RunState::Verifying),
        (RunTrigger::Complete, RunState::Completed),
    ] {
        move_run(&mut value, trigger, &format!("{trigger:?}")).expect("allowed");
        assert_eq!(value.state, expected);
    }
}

#[test]
fn every_state_has_expected_terminal_classification() {
    let terminal = [RunState::Completed, RunState::Failed, RunState::Cancelled];
    for state in [
        RunState::Created,
        RunState::Preparing,
        RunState::Running,
        RunState::WaitingApproval,
        RunState::WaitingUser,
        RunState::Paused,
        RunState::Retrying,
        RunState::Blocked,
        RunState::Stuck,
        RunState::Verifying,
        RunState::Completed,
        RunState::Failed,
        RunState::Cancelled,
    ] {
        assert_eq!(state.is_terminal(), terminal.contains(&state));
    }
}

#[test]
fn terminal_delivery_is_idempotent_only_for_same_witness_and_outcome() {
    let mut value = run(0);
    move_run(&mut value, RunTrigger::CancelUser, "cancel-1").expect("cancel");
    assert_eq!(
        move_run(&mut value, RunTrigger::CancelUser, "cancel-1").expect("duplicate"),
        None
    );
    assert_eq!(
        move_run(&mut value, RunTrigger::Complete, "complete-1"),
        Err(RunDomainError::TerminalConflict)
    );
}

#[test]
fn retry_policy_and_invalid_transitions_fail_closed() {
    let mut value = run(1);
    assert!(matches!(
        move_run(&mut value, RunTrigger::Start, "bad"),
        Err(RunDomainError::InvalidTransition { .. })
    ));
    move_run(&mut value, RunTrigger::Prepare, "prepare").expect("prepare");
    move_run(&mut value, RunTrigger::Start, "start").expect("start");
    move_run(&mut value, RunTrigger::Retry, "retry-1").expect("retry");
    move_run(&mut value, RunTrigger::RetryReady, "ready").expect("ready");
    assert_eq!(
        move_run(&mut value, RunTrigger::Retry, "retry-2"),
        Err(RunDomainError::RetryExhausted)
    );
}

#[test]
fn identities_and_safe_metadata_are_bounded() {
    let result = AgentRun::create(RunCreation {
        id: " ".into(),
        owner: RunOwner {
            owner_type: "session".into(),
            owner_id: "one".into(),
        },
        links: vec![],
        parent_run_id: None,
        recovery_policy: RunRecoveryPolicy::NotRecoverable,
        max_retries: 0,
        timestamp: "now".into(),
        witness: "witness".into(),
    });
    assert_eq!(result, Err(RunDomainError::InvalidField("id")));
    let unsafe_witness = AgentRun::create(RunCreation {
        id: "018f0f17-4d6a-7e20-b41d-66c5271a28d1".into(),
        owner: RunOwner {
            owner_type: "session".into(),
            owner_id: "one".into(),
        },
        links: vec![],
        parent_run_id: None,
        recovery_policy: RunRecoveryPolicy::NotRecoverable,
        max_retries: 0,
        timestamp: "2026-08-16T00:00:00Z".into(),
        witness: "raw prompt must not persist".into(),
    });
    assert_eq!(unsafe_witness, Err(RunDomainError::InvalidField("witness")));
    let mut value = run(0);
    assert_eq!(
        value.transition(RunTransition {
            trigger: RunTrigger::CancelUser,
            timestamp: "2026-08-16T00:00:01Z".into(),
            reason_code: Some("x".repeat(MAX_REASON_LENGTH + 1)),
            witness: "cancel".into()
        }),
        Err(RunDomainError::InvalidField("reason_code"))
    );
}
