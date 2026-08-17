use super::*;
use serde::Serialize;
use std::time::Instant;

fn run(max_retries: u32) -> AgentRun {
    run_with_id("018f0f17-4d6a-7e20-b41d-66c5271a28d0", max_retries)
}

fn run_with_id(id: &str, max_retries: u32) -> AgentRun {
    AgentRun::create(RunCreation {
        id: id.into(),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunPerformanceEvidence {
    dataset_id: &'static str,
    dataset_version: u32,
    run_count: usize,
    retained_events: usize,
    cancelled_runs: usize,
    duplicate_terminal_deliveries: usize,
    rejected_terminal_conflicts: usize,
    peak_concurrent_runs: usize,
    transition_microseconds: u128,
    cancellation_microseconds: u128,
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

#[test]
fn thousand_run_fixture_has_bounded_lifecycle_and_cancellation_work() {
    const RUNS: usize = 1_000;
    const MAX_CONCURRENT: usize = 64;
    const MAX_RETAINED_EVENTS: usize = 8_000;
    let started = Instant::now();
    let mut retained_events = 0_usize;
    let mut cancelled_runs = 0_usize;
    let mut duplicate_terminal_deliveries = 0_usize;
    let mut rejected_terminal_conflicts = 0_usize;
    let mut peak_concurrent_runs = 0_usize;
    let mut cancellation_microseconds = 0_u128;

    for batch_start in (0..RUNS).step_by(MAX_CONCURRENT) {
        let batch_end = (batch_start + MAX_CONCURRENT).min(RUNS);
        let mut active = Vec::with_capacity(MAX_CONCURRENT);
        for index in batch_start..batch_end {
            let id = uuid::Uuid::from_u128(index as u128 + 1).to_string();
            active.push(run_with_id(&id, 0));
            retained_events += 1;
        }
        peak_concurrent_runs = peak_concurrent_runs.max(active.len());
        for (offset, value) in active.iter_mut().enumerate() {
            let index = batch_start + offset;
            retained_events += usize::from(
                move_run(value, RunTrigger::Prepare, &format!("prepare-{index}"))
                    .expect("prepare")
                    .is_some(),
            );
            retained_events += usize::from(
                move_run(value, RunTrigger::Start, &format!("start-{index}"))
                    .expect("start")
                    .is_some(),
            );
            if index % 10 == 0 {
                let cancellation_started = Instant::now();
                let witness = format!("cancel-{index}");
                retained_events += usize::from(
                    move_run(value, RunTrigger::CancelUser, &witness)
                        .expect("cancel")
                        .is_some(),
                );
                cancellation_microseconds = cancellation_microseconds
                    .saturating_add(cancellation_started.elapsed().as_micros());
                duplicate_terminal_deliveries += usize::from(
                    move_run(value, RunTrigger::CancelUser, &witness)
                        .expect("idempotent cancellation")
                        .is_none(),
                );
                cancelled_runs += 1;
            } else {
                for (trigger, label) in [
                    (RunTrigger::RequestApproval, "approval"),
                    (RunTrigger::ApprovalGranted, "approved"),
                    (RunTrigger::AskUser, "question"),
                    (RunTrigger::UserAnswered, "answered"),
                    (RunTrigger::Complete, "complete"),
                ] {
                    retained_events += usize::from(
                        move_run(value, trigger, &format!("{label}-{index}"))
                            .expect("valid lifecycle")
                            .is_some(),
                    );
                }
                rejected_terminal_conflicts += usize::from(matches!(
                    move_run(value, RunTrigger::CancelUser, &format!("late-{index}")),
                    Err(RunDomainError::TerminalConflict)
                ));
            }
        }
    }

    assert_eq!(cancelled_runs, 100);
    assert_eq!(duplicate_terminal_deliveries, cancelled_runs);
    assert_eq!(rejected_terminal_conflicts, RUNS - cancelled_runs);
    assert!(retained_events <= MAX_RETAINED_EVENTS);
    assert!(peak_concurrent_runs <= MAX_CONCURRENT);
    let evidence = RunPerformanceEvidence {
        dataset_id: "runs-1000",
        dataset_version: 1,
        run_count: RUNS,
        retained_events,
        cancelled_runs,
        duplicate_terminal_deliveries,
        rejected_terminal_conflicts,
        peak_concurrent_runs,
        transition_microseconds: started.elapsed().as_micros(),
        cancellation_microseconds,
    };
    eprintln!(
        "RUN_PERFORMANCE {}",
        serde_json::to_string(&evidence).expect("performance evidence")
    );
}
