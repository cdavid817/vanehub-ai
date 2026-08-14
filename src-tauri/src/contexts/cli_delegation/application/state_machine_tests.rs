use super::*;

fn snapshot() -> DelegationRequestSnapshot {
    DelegationRequestSnapshot {
        task: "analyze repository".to_owned(),
        context_summary: None,
        artifact_hashes: vec!["sha256:input".to_owned()],
        repository_identity: "repo-1".to_owned(),
        base_commit: "a".repeat(40),
        instruction_hashes: Vec::new(),
        provider_configuration_hash: "sha256:provider".to_owned(),
        limits_hash: "sha256:limits".to_owned(),
        adapter_version: "1".to_owned(),
    }
}

#[test]
fn one_logical_delegation_allows_three_explicit_attempts_and_one_terminal_result() {
    let frozen = snapshot();
    let mut delegation = Delegation::queued(
        "delegation-1".to_owned(),
        "session-1".to_owned(),
        frozen.clone(),
    )
    .expect("delegation");
    for number in 1..=3 {
        let id = format!("attempt-{number}");
        assert_eq!(
            delegation
                .queue_attempt(id.clone(), DelegationTarget::CodexCli, DelegationMode::Edit)
                .expect("queue")
                .number,
            number
        );
        delegation.start_attempt(&id).expect("start");
        if number < 3 {
            delegation.fail_attempt(&id).expect("fail");
        } else {
            delegation
                .complete(
                    DelegationStatus::Succeeded,
                    DelegationResult {
                        attempt_id: id,
                        report_artifact_id: Some("artifact-report".to_owned()),
                        change_set_artifact_id: Some("artifact-changes".to_owned()),
                        error_code: None,
                    },
                )
                .expect("complete");
        }
    }
    assert_eq!(delegation.snapshot, frozen);
    assert_eq!(delegation.attempts.len(), MAX_DELEGATION_ATTEMPTS);
    assert_eq!(delegation.status, DelegationStatus::Succeeded);
    assert_eq!(
        delegation.queue_attempt(
            "attempt-4".to_owned(),
            DelegationTarget::ClaudeCode,
            DelegationMode::Analyze
        ),
        Err(DelegationError::AlreadyTerminal)
    );
    assert_eq!(
        delegation.complete(
            DelegationStatus::Failed,
            DelegationResult {
                attempt_id: "attempt-3".to_owned(),
                report_artifact_id: None,
                change_set_artifact_id: None,
                error_code: Some("late".to_owned())
            }
        ),
        Err(DelegationError::AlreadyTerminal)
    );
}

#[test]
fn queue_and_attempt_transitions_reject_parallel_or_out_of_order_work() {
    let mut delegation = Delegation::queued(
        "delegation-1".to_owned(),
        "session-1".to_owned(),
        snapshot(),
    )
    .expect("delegation");
    delegation
        .queue_attempt(
            "attempt-1".to_owned(),
            DelegationTarget::ClaudeCode,
            DelegationMode::Analyze,
        )
        .expect("queue");
    assert_eq!(
        delegation.queue_attempt(
            "attempt-2".to_owned(),
            DelegationTarget::CodexCli,
            DelegationMode::Analyze
        ),
        Err(DelegationError::AttemptAlreadyActive)
    );
    assert_eq!(
        delegation.fail_attempt("attempt-1"),
        Err(DelegationError::AttemptNotActive)
    );
    delegation.start_attempt("attempt-1").expect("start");
    assert_eq!(
        delegation.start_attempt("attempt-1"),
        Err(DelegationError::AttemptNotActive)
    );
}

#[test]
fn restart_interrupts_non_terminal_work_without_replay_or_terminal_regression() {
    let mut running = Delegation::queued(
        "delegation-1".to_owned(),
        "session-1".to_owned(),
        snapshot(),
    )
    .expect("delegation");
    running
        .queue_attempt(
            "attempt-1".to_owned(),
            DelegationTarget::ClaudeCode,
            DelegationMode::Analyze,
        )
        .expect("queue");
    running.start_attempt("attempt-1").expect("start");
    assert!(running.interrupt_after_restart());
    assert_eq!(running.status, DelegationStatus::Interrupted);
    assert_eq!(
        running.attempts[0].status,
        DelegationAttemptStatus::Interrupted
    );
    assert!(running.result.is_none());
    assert_eq!(
        running.queue_attempt(
            "attempt-2".to_owned(),
            DelegationTarget::CodexCli,
            DelegationMode::Analyze
        ),
        Err(DelegationError::AlreadyTerminal)
    );
    assert!(!running.interrupt_after_restart());
}
