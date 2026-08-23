//! What an execution row may say, and which rows retention is allowed to remove.

use super::{
    all_hook_execution_errors, HookExecutionError, HookExecutionRetention, HookExecutionStatus,
    ALL_HOOK_EXECUTION_STATUSES, DEFAULT_HOOK_EXECUTION_RETENTION,
};

#[test]
fn only_a_finished_execution_is_terminal() {
    // The single predicate retention is allowed to prune by. A pending or running execution is not
    // old, it is unfinished: deleting one turns a Hook that is still going into a Hook that never
    // happened, and the completion that arrives afterwards has nothing to attach to.
    assert!(!HookExecutionStatus::Pending.is_terminal());
    assert!(!HookExecutionStatus::Running.is_terminal());

    for finished in [
        HookExecutionStatus::Succeeded,
        HookExecutionStatus::Failed,
        HookExecutionStatus::TimedOut,
        HookExecutionStatus::Denied,
    ] {
        assert!(finished.is_terminal(), "{finished:?}");
    }
}

#[test]
fn every_status_falls_on_one_side_of_the_terminal_line() {
    // `is_terminal` matches exhaustively, so a status added without deciding this is a compile
    // error rather than a row retention quietly refuses to ever remove. This asserts the list
    // itself has not drifted away from the enum.
    let terminal = ALL_HOOK_EXECUTION_STATUSES
        .iter()
        .filter(|status| status.is_terminal())
        .count();

    assert_eq!(ALL_HOOK_EXECUTION_STATUSES.len(), 6);
    assert_eq!(terminal, 4);
}

#[test]
fn every_status_round_trips_through_the_spelling_that_reaches_storage() {
    for status in ALL_HOOK_EXECUTION_STATUSES.iter().copied() {
        assert_eq!(HookExecutionStatus::parse(status.as_str()), Some(status));
    }
    assert_eq!(HookExecutionStatus::parse("cancelled"), None);
}

#[test]
fn every_status_spelling_is_distinct() {
    let mut spellings: Vec<&str> = ALL_HOOK_EXECUTION_STATUSES
        .iter()
        .map(|status| status.as_str())
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
}

#[test]
fn a_retention_window_of_zero_is_unconstructible() {
    // Sequence is assigned as MAX + 1. A window of zero would let retention empty a subject's
    // history, MAX would return to nothing, and the next execution would reissue a number a
    // previous one already used -- so "sequence is monotonic" would be true only until the first
    // prune. Refusing zero here is what makes the statement unconditional.
    assert_eq!(HookExecutionRetention::new(0), None);
    assert_eq!(
        HookExecutionRetention::new(1).map(HookExecutionRetention::keep),
        Some(1)
    );
}

#[test]
fn the_default_window_keeps_something() {
    // Stated through the constructor rather than as `DEFAULT >= 1`, which the compiler can fold to
    // `true` and therefore proves nothing. This asserts the default is a window the type itself
    // would accept, which is the property monotonicity actually rests on.
    assert_eq!(
        HookExecutionRetention::new(DEFAULT_HOOK_EXECUTION_RETENTION)
            .map(HookExecutionRetention::keep),
        Some(DEFAULT_HOOK_EXECUTION_RETENTION)
    );
    assert_eq!(
        HookExecutionRetention::default().keep(),
        DEFAULT_HOOK_EXECUTION_RETENTION
    );
}

#[test]
fn every_execution_failure_has_a_distinct_stable_code() {
    let errors = all_hook_execution_errors();
    let total = errors.len();

    let mut codes: Vec<&str> = errors.iter().map(HookExecutionError::code).collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), total);
}

#[test]
fn re_appending_an_execution_id_is_a_failure_rather_than_an_update() {
    // Rows are immutable. Treating a repeat as an update would let a finished execution be
    // rewritten after the fact, which is the one thing evidence must not permit.
    assert_eq!(
        HookExecutionError::DuplicateExecution.code(),
        "duplicate_hook_execution"
    );
}
