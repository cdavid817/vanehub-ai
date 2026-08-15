use super::*;

/// A command that finishes immediately on both supported shells (`cmd /C` and `bash -c`).
const QUICK_COMMAND: &str = "echo vanehub-background-probe";

/// A command that stays alive long enough to be observed as running, on either shell.
fn long_running_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "ping -n 30 127.0.0.1 > nul"
    } else {
        "sleep 30"
    }
}

fn workspace() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

fn wait_for<F: Fn() -> bool>(predicate: F) -> bool {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    predicate()
}

#[test]
fn output_buffer_drops_oldest_bytes_and_counts_them() {
    let mut buffer = OutputBuffer::default();
    buffer.append(&vec![b'a'; MAX_BUFFERED_OUTPUT_BYTES]);
    assert_eq!(buffer.dropped, 0);

    buffer.append(b"tail");
    assert_eq!(buffer.bytes.len(), MAX_BUFFERED_OUTPUT_BYTES);
    assert_eq!(buffer.dropped, 4);
    assert!(buffer.bytes.ends_with(b"tail"));
}

#[test]
fn take_front_is_lossless_and_leaves_the_remainder_buffered() {
    let mut buffer = OutputBuffer::default();
    let total = MAX_TOOL_OUTPUT_BYTES + 100;
    buffer.append(&vec![b'x'; total]);

    let (first, dropped, remaining) = buffer.take_front(MAX_TOOL_OUTPUT_BYTES);
    assert_eq!(first.len(), MAX_TOOL_OUTPUT_BYTES);
    assert_eq!(dropped, 0);
    assert_eq!(remaining, 100);

    let (second, _, remaining) = buffer.take_front(MAX_TOOL_OUTPUT_BYTES);
    assert_eq!(second.len(), 100);
    assert_eq!(remaining, 0);
}

#[test]
fn take_front_reports_dropped_bytes_once() {
    let mut buffer = OutputBuffer::default();
    buffer.append(&vec![b'a'; MAX_BUFFERED_OUTPUT_BYTES]);
    buffer.append(b"tail");

    let (_, dropped, _) = buffer.take_front(16);
    assert_eq!(dropped, 4);
    let (_, dropped_again, _) = buffer.take_front(16);
    assert_eq!(
        dropped_again, 0,
        "a drop is reported once, not on every read"
    );
}

#[test]
fn split_point_never_cuts_a_multibyte_character() {
    // "中" is three bytes; cutting at 1 or 2 would produce replacement characters on both sides.
    let bytes = "a中b".as_bytes();
    assert_eq!(safe_split_point(bytes, 1), 1);
    assert_eq!(safe_split_point(bytes, 2), 1);
    assert_eq!(safe_split_point(bytes, 3), 1);
    assert_eq!(safe_split_point(bytes, 4), 4);
    assert_eq!(safe_split_point(bytes, 99), bytes.len());
}

/// Retrieval is a cursor: every call consumes what it returns. A caller that polls for status
/// therefore has to accumulate as it goes, which is exactly what this does -- reading twice and
/// keeping only the second result would silently discard the first read's output.
fn drain_until(
    registry: &BackgroundShellRegistry,
    session_id: &str,
    handle: &str,
    done: impl Fn(&str, BackgroundStatus) -> bool,
) -> (String, BackgroundStatus) {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut seen = String::new();
    let mut status = BackgroundStatus::Running;
    while Instant::now() < deadline {
        let output = registry.take_output(session_id, handle).expect("output");
        seen.push_str(&output.text);
        status = output.status;
        if done(&seen, status) {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    (seen, status)
}

#[test]
fn background_command_runs_to_completion_and_reports_its_exit_code() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-a", QUICK_COMMAND, &workspace())
        .expect("start");

    let (seen, status) = drain_until(&registry, "session-a", &handle, |seen, status| {
        status.is_terminal() && seen.contains("vanehub-background-probe")
    });

    assert_eq!(status, BackgroundStatus::Exited(Some(0)));
    assert!(
        seen.contains("vanehub-background-probe"),
        "expected command output, got {seen:?}"
    );
}

#[test]
fn output_retrieval_returns_only_output_produced_since_the_previous_call() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-b", QUICK_COMMAND, &workspace())
        .expect("start");

    assert!(wait_for(|| registry
        .take_output("session-b", &handle)
        .is_ok_and(|output| output.status.is_terminal())));
    // Drain whatever is buffered, then confirm a second read returns nothing new.
    while registry
        .take_output("session-b", &handle)
        .expect("output")
        .remaining_bytes
        > 0
    {}

    let output = registry.take_output("session-b", &handle).expect("output");
    assert!(
        output.text.is_empty(),
        "expected no new output, got {:?}",
        output.text
    );
    assert!(output.status.is_terminal());
}

#[test]
fn a_running_command_survives_the_call_that_started_it_and_can_be_terminated() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-c", long_running_command(), &workspace())
        .expect("start");

    let output = registry.take_output("session-c", &handle).expect("output");
    assert_eq!(
        output.status,
        BackgroundStatus::Running,
        "a long-running command must still be running right after start"
    );

    let outcome = registry.kill("session-c", &handle).expect("kill");
    assert!(
        matches!(outcome, KillOutcome::Terminated(_)),
        "a running command must be reported as terminated by this call, got {outcome:?}"
    );
    assert!(
        outcome.status().is_terminal(),
        "kill must settle the status before returning, got {outcome:?}"
    );
}

#[test]
fn terminating_an_already_finished_command_reports_its_existing_status() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-d", QUICK_COMMAND, &workspace())
        .expect("start");
    assert!(wait_for(|| registry
        .take_output("session-d", &handle)
        .is_ok_and(|output| output.status.is_terminal())));

    let outcome = registry.kill("session-d", &handle).expect("kill");
    assert_eq!(
        outcome,
        KillOutcome::AlreadyFinished(BackgroundStatus::Exited(Some(0))),
        "an already-exited command keeps its exit status instead of reporting a termination"
    );
}

#[test]
fn session_concurrency_limit_rejects_the_next_start_without_touching_running_commands() {
    let registry = BackgroundShellRegistry::default();
    let mut handles = Vec::new();
    for _ in 0..MAX_BACKGROUND_COMMANDS_PER_SESSION {
        handles.push(
            registry
                .start("session-e", long_running_command(), &workspace())
                .expect("start"),
        );
    }

    assert_eq!(
        registry.start("session-e", long_running_command(), &workspace()),
        Err(BackgroundStartError::SessionLimitReached)
    );
    for handle in &handles {
        assert_eq!(
            registry
                .take_output("session-e", handle)
                .expect("output")
                .status,
            BackgroundStatus::Running,
            "a rejected start must not terminate an existing command to make room"
        );
    }

    for handle in &handles {
        let _ = registry.kill("session-e", handle);
    }
}

#[test]
fn the_limit_counts_running_commands_only() {
    let registry = BackgroundShellRegistry::default();
    let mut handles = Vec::new();
    for _ in 0..MAX_BACKGROUND_COMMANDS_PER_SESSION {
        handles.push(
            registry
                .start("session-f", QUICK_COMMAND, &workspace())
                .expect("start"),
        );
    }
    assert!(wait_for(|| handles.iter().all(|handle| registry
        .take_output("session-f", handle)
        .is_ok_and(|output| output.status.is_terminal()))));

    registry
        .start("session-f", QUICK_COMMAND, &workspace())
        .expect("finished commands must not occupy a concurrency slot");
}

#[test]
fn unknown_and_foreign_handles_are_rejected_rather_than_returning_another_sessions_output() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-owner", long_running_command(), &workspace())
        .expect("start");

    assert_eq!(
        registry.take_output("session-intruder", &handle),
        Err(UnknownHandle)
    );
    assert_eq!(
        registry.kill("session-intruder", &handle),
        Err(UnknownHandle)
    );
    assert_eq!(
        registry.take_output("session-owner", "bg_does_not_exist"),
        Err(UnknownHandle)
    );

    let _ = registry.kill("session-owner", &handle);
}

#[test]
fn reaping_a_session_terminates_and_forgets_only_that_sessions_commands() {
    let registry = BackgroundShellRegistry::default();
    let doomed = registry
        .start("session-ends", long_running_command(), &workspace())
        .expect("start");
    let survivor = registry
        .start("session-stays", long_running_command(), &workspace())
        .expect("start");

    registry.reap_session("session-ends");

    assert_eq!(
        registry.take_output("session-ends", &doomed),
        Err(UnknownHandle)
    );
    assert_eq!(
        registry
            .take_output("session-stays", &survivor)
            .expect("output")
            .status,
        BackgroundStatus::Running
    );

    let _ = registry.kill("session-stays", &survivor);
}

#[test]
fn retained_terminal_entries_are_pruned_so_the_registry_stays_bounded() {
    let registry = BackgroundShellRegistry::default();
    for _ in 0..(MAX_RETAINED_TERMINAL_PER_SESSION * 2 + 2) {
        let handle = registry
            .start("session-churn", QUICK_COMMAND, &workspace())
            .expect("start");
        assert!(wait_for(|| registry
            .take_output("session-churn", &handle)
            .is_ok_and(|output| output.status.is_terminal())));
    }

    assert!(
        registry.entry_count() <= MAX_RETAINED_TERMINAL_PER_SESSION + 1,
        "terminal entries accumulated without bound: {}",
        registry.entry_count()
    );
}

#[test]
fn status_labels_distinguish_every_terminal_outcome() {
    assert_eq!(BackgroundStatus::Running.label(), "running");
    assert!(!BackgroundStatus::Running.is_terminal());
    assert_eq!(BackgroundStatus::Exited(Some(2)).label(), "exited (code 2)");
    assert_eq!(BackgroundStatus::Killed.label(), "terminated");
    assert!(BackgroundStatus::LifetimeExceeded
        .label()
        .contains("lifetime"));
    assert!(BackgroundStatus::Killed.is_terminal());
    assert!(BackgroundStatus::LifetimeExceeded.is_terminal());
}

#[test]
fn a_command_exceeding_its_lifetime_is_terminated_and_reported_as_such() {
    let registry = BackgroundShellRegistry::with_lifetime(Duration::from_millis(300));
    let handle = registry
        .start("session-lifetime", long_running_command(), &workspace())
        .expect("start");

    assert!(wait_for(|| registry
        .take_output("session-lifetime", &handle)
        .is_ok_and(|output| output.status.is_terminal())));

    let status = registry
        .take_output("session-lifetime", &handle)
        .expect("output")
        .status;
    assert_eq!(
        status,
        BackgroundStatus::LifetimeExceeded,
        "a command killed by its lifetime must be distinguishable from a normal exit"
    );
    assert!(status.label().contains("lifetime"));
}

#[test]
fn reaping_everything_clears_the_registry_across_sessions() {
    let registry = BackgroundShellRegistry::default();
    let first = registry
        .start("session-one", long_running_command(), &workspace())
        .expect("start");
    let second = registry
        .start("session-two", long_running_command(), &workspace())
        .expect("start");

    registry.reap_all();

    assert_eq!(registry.entry_count(), 0);
    assert_eq!(
        registry.take_output("session-one", &first),
        Err(UnknownHandle)
    );
    assert_eq!(
        registry.take_output("session-two", &second),
        Err(UnknownHandle)
    );
}

#[test]
fn a_command_label_is_available_for_the_owning_session_only() {
    let registry = BackgroundShellRegistry::default();
    let handle = registry
        .start("session-label", long_running_command(), &workspace())
        .expect("start");

    assert_eq!(
        registry.command_label("session-label", &handle).as_deref(),
        Some(long_running_command())
    );
    assert_eq!(registry.command_label("session-other", &handle), None);

    let _ = registry.kill("session-label", &handle);
}
