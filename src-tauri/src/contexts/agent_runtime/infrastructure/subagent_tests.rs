use super::*;
use serde_json::json;

fn call(name: &str, input: Value) -> ToolUseBlock {
    ToolUseBlock {
        id: format!("call-{name}"),
        name: name.to_owned(),
        input: Some(input),
        output: None,
        status: "pending".to_owned(),
    }
}

/// The child's authority is structural: these three definitions are all it is offered, and
/// `execute_child_tool` is the only dispatcher. There is no allowlist to get wrong.
#[test]
fn the_child_catalog_is_exactly_three_read_only_tools() {
    let names: Vec<String> = child_tool_catalog(false)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names, vec!["file", "grep", "glob"]);
}

/// A mutating child gains exactly `edit` and a writable `file`, and nothing else. In particular it
/// still cannot run a command, reach the network, ask a question, or delegate.
#[test]
fn a_mutating_child_gains_only_write_tools() {
    let names: Vec<String> = child_tool_catalog(true)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec!["edit", "file", "glob", "grep"]);

    // The writable `file` definition replaced the plan-mode one, which cannot write.
    let file = child_tool_catalog(true)
        .into_iter()
        .find(|tool| tool.name == "file")
        .expect("file tool");
    assert_eq!(
        file.input_schema["properties"]["operation"]["enum"],
        json!(["read", "write"])
    );

    for forbidden in [
        "shell",
        "shell_kill",
        "ask_user_question",
        "delegate_subagent",
        "delegate_cli",
        "apply_delegation_changes",
    ] {
        assert!(!names.contains(&forbidden.to_owned()), "{forbidden}");
    }
}

/// The read-only dispatcher is a separate function, not a branch inside the mutating one, so a
/// read-only child has no code path to a write regardless of any flag.
#[test]
fn the_mutating_dispatcher_writes_and_the_read_only_one_still_cannot() {
    let directory = crate::test_support::TempDirectory::new("subagent-mutating-dispatch");
    let root = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("target.txt"), "before\n").expect("fixture");

    let write = call(
        "file",
        json!({"operation": "write", "path": "target.txt", "content": "after\n"}),
    );

    let (_output, is_error) = execute_child_tool(&write, &root);
    assert!(!is_error);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("target.txt")).expect("read"),
        "before\n",
        "the read-only dispatcher must not write, whatever the operation says"
    );

    let (_output, is_error) = execute_mutating_child_tool(&write, &root);
    assert!(!is_error);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("target.txt")).expect("read"),
        "after\n",
        "the mutating dispatcher writes"
    );
}

#[test]
fn the_mutating_dispatcher_delegates_reads_to_the_read_only_one() {
    let directory = crate::test_support::TempDirectory::new("subagent-mutating-read");
    let root = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("a.txt"), "needle\n").expect("fixture");

    let (output, is_error) =
        execute_mutating_child_tool(&call("file", json!({"path": "a.txt"})), &root);
    assert!(!is_error, "{output}");
    assert!(output.contains("needle"), "{output}");

    // Still refuses anything outside the child surface.
    let (refused, is_error) =
        execute_mutating_child_tool(&call("shell", json!({"command": "echo hi"})), &root);
    assert!(is_error);
    assert!(refused.contains("not available to a subagent"), "{refused}");
}

/// A captured status letter must land on the right change kind, or a reviewer reads "modified"
/// for a file that was deleted.
#[test]
fn captured_status_letters_map_to_change_kinds() {
    let kind = |status: char| {
        change_set_file(&CapturedFile {
            path: "a.txt".to_owned(),
            status,
            new_hash: None,
            binary: false,
        })
        .change_kind
    };
    assert_eq!(kind('?'), FileChangeKind::Add);
    assert_eq!(kind('A'), FileChangeKind::Add);
    assert_eq!(kind('D'), FileChangeKind::Delete);
    assert_eq!(kind('R'), FileChangeKind::Rename);
    assert_eq!(kind('M'), FileChangeKind::Modify);
}

#[test]
fn the_child_catalog_excludes_every_effectful_and_delegating_tool() {
    let names: Vec<String> = child_tool_catalog(false)
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    for forbidden in [
        "shell",
        "shell_output",
        "shell_kill",
        "edit",
        "remember",
        "todo_write",
        "ask_user_question",
        "delegate_subagent",
        "delegate_cli",
        "apply_delegation_changes",
        "browser",
        "web_search",
        "web_fetch",
        "code_execution",
    ] {
        assert!(!names.contains(&forbidden.to_owned()), "{forbidden}");
    }
}

/// The file tool is dispatched with a hardcoded "read". A model asking for a write reaches the
/// read path, not the write path -- the write is unreachable rather than rejected.
#[test]
fn a_child_write_request_still_reaches_only_the_read_path() {
    let directory = crate::test_support::TempDirectory::new("subagent-write-attempt");
    let workspace = directory.path().to_string_lossy().to_string();
    let target = directory.path().join("untouched.txt");
    std::fs::write(&target, "original").expect("fixture");

    let (_output, _is_error) = execute_child_tool(
        &call(
            "file",
            json!({"operation": "write", "path": "untouched.txt", "content": "overwritten"}),
        ),
        &workspace,
    );

    assert_eq!(
        std::fs::read_to_string(&target).expect("read back"),
        "original",
        "a child must not be able to write, whatever it puts in `operation`"
    );
}

#[test]
fn a_child_can_read_search_and_glob_inside_its_workspace() {
    let directory = crate::test_support::TempDirectory::new("subagent-read");
    let workspace = directory.path().to_string_lossy().to_string();
    std::fs::write(directory.path().join("a.txt"), "needle here\n").expect("fixture");

    let (read, read_error) =
        execute_child_tool(&call("file", json!({"path": "a.txt"})), &workspace);
    assert!(!read_error, "{read}");
    assert!(read.contains("needle"), "{read}");

    let (found, grep_error) =
        execute_child_tool(&call("grep", json!({"pattern": "needle"})), &workspace);
    assert!(!grep_error, "{found}");
    assert!(found.contains("a.txt"), "{found}");

    let (listed, glob_error) =
        execute_child_tool(&call("glob", json!({"pattern": "*.txt"})), &workspace);
    assert!(!glob_error, "{listed}");
    assert!(listed.contains("a.txt"), "{listed}");
}

#[test]
fn a_tool_outside_the_child_surface_is_refused_by_the_dispatcher() {
    let directory = crate::test_support::TempDirectory::new("subagent-unknown-tool");
    let workspace = directory.path().to_string_lossy().to_string();

    for name in ["shell", "edit", "delegate_subagent", "ask_user_question"] {
        let (output, is_error) =
            execute_child_tool(&call(name, json!({"command": "echo hi"})), &workspace);
        assert!(is_error, "{name} must be refused");
        assert!(output.contains("not available to a subagent"), "{output}");
    }
}

#[test]
fn a_child_read_cannot_escape_its_workspace() {
    let directory = crate::test_support::TempDirectory::new("subagent-escape");
    let workspace = directory.path().to_string_lossy().to_string();

    let (output, is_error) =
        execute_child_tool(&call("file", json!({"path": "../outside.txt"})), &workspace);
    assert!(is_error, "{output}");
}

#[test]
fn an_answer_is_trimmed_and_capped() {
    assert_eq!(bounded("  the answer  ").as_deref(), Some("the answer"));
    assert_eq!(bounded("   ").as_deref(), None);
    assert_eq!(bounded("").as_deref(), None);

    let long = "x".repeat(MAX_CHILD_RESULT_CHARS + 500);
    assert_eq!(
        bounded(&long).expect("capped").chars().count(),
        MAX_CHILD_RESULT_CHARS
    );
}

/// An empty answer is a failure, not an empty success: returning one would read to the parent as
/// "investigated, found nothing".
#[test]
fn an_empty_answer_is_reported_as_a_failure() {
    let envelope = succeeded("   ", 3);
    assert_eq!(envelope.status, NativeToolResultStatus::Failed);
    assert_eq!(
        envelope.error_code,
        Some(NativeToolErrorCode::ExternalFailure)
    );
}

#[test]
fn a_capped_answer_reports_truncation() {
    let long = "y".repeat(MAX_CHILD_RESULT_CHARS + 1);
    let envelope = succeeded(&long, 5);
    assert_eq!(envelope.status, NativeToolResultStatus::Succeeded);
    assert!(envelope.truncated);
    assert_eq!(envelope.metadata["tool_calls"], json!(5));

    let short = succeeded("brief", 1);
    assert!(!short.truncated);
    assert_eq!(short.output.expect("output")["summary"], json!("brief"));
}

/// Counts and timing only: the child's turns, tool inputs, and tool outputs never reach anywhere
/// the parent can read them.
#[test]
fn result_metadata_carries_counts_and_nothing_else() {
    let envelope = succeeded("done", 7);
    assert_eq!(
        envelope.metadata.keys().cloned().collect::<Vec<_>>(),
        vec!["tool_calls".to_owned()]
    );
    assert_eq!(envelope.metadata["tool_calls"], json!(7));
}

#[test]
fn terminal_outcomes_map_to_their_error_codes() {
    assert_eq!(
        terminal(NativeToolResultStatus::LimitExceeded, None, 0).error_code,
        Some(NativeToolErrorCode::LimitExceeded)
    );
    assert_eq!(
        terminal(NativeToolResultStatus::Cancelled, None, 0).error_code,
        Some(NativeToolErrorCode::Cancelled)
    );
}

/// A refused claim never terminates a running child to make room.
#[test]
fn the_concurrency_cap_is_per_session_and_releases_slots() {
    let slots = ConcurrencySlots::default();

    for index in 0..MAX_CONCURRENT_CHILDREN {
        assert!(slots.claim("session-a"), "claim {index}");
    }
    assert!(
        !slots.claim("session-a"),
        "a session at its cap must be refused"
    );
    assert!(
        slots.claim("session-b"),
        "another session has its own budget"
    );

    slots.release("session-a");
    assert!(
        slots.claim("session-a"),
        "a released slot becomes available again"
    );
}

/// Child spend must be recorded, but a caller with no accounting handle (Web/mock, tests) must
/// degrade rather than panic.
#[test]
fn a_child_invocation_without_accounting_declines_instead_of_failing() {
    let config = crate::contexts::agent_runtime::application::ApiProviderConfig {
        source_provider_id: Some("openai".to_owned()),
        model_id: "gpt-5.4".to_owned(),
        interface_format: "openai-compatible".to_owned(),
        base_url: Some("https://api.openai.com/v1".to_owned()),
        auto_approve_tools: false,
    };
    let invocation = super::super::api_process_adapter::begin_child_invocation(
        None,
        super::super::api_process_adapter::ChildInvocationIdentity {
            call_id: "call-1",
            session_id: "session-1",
            agent_id: "onepiece",
            operation_id: "operation-1",
        },
        &config,
        0,
        &super::super::SystemAgentRuntimeClock,
    );
    assert!(invocation.is_none());
}

/// The handler advertises a duration ceiling; the dispatcher enforces the native-tool request
/// timeout. If they drift apart, the limit profile promises time a child never gets.
#[test]
fn the_declared_duration_matches_the_enforced_deadline() {
    use crate::contexts::agent_runtime::application::MAX_SUBAGENT_DURATION_MS;

    assert_eq!(
        u128::from(MAX_SUBAGENT_DURATION_MS),
        super::super::api_process_adapter::REQUEST_TIMEOUT.as_millis(),
        "the advertised subagent duration must be the deadline actually applied"
    );
}

#[derive(Debug, Default)]
struct CapturingProgress {
    events: Mutex<Vec<NativeToolProgress>>,
}

impl crate::contexts::agent_runtime::application::NativeToolProgressSink for CapturingProgress {
    fn publish(&self, progress: NativeToolProgress) {
        self.events.lock().expect("events").push(progress);
    }
}

fn execution_context(
    cancelled: Arc<AtomicBool>,
    progress: Arc<CapturingProgress>,
) -> crate::contexts::agent_runtime::application::NativeToolExecutionContext {
    crate::contexts::agent_runtime::application::NativeToolExecutionContext {
        call_id: "call-1".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        agent_id: "onepiece".to_owned(),
        canonical_workspace: Some(std::path::PathBuf::from("C:/work")),
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
        cancelled,
        progress,
    }
}

/// The child shares the generation's cancellation flag, so cancelling the generation -- which is
/// what ending or archiving a session does -- stops the child at its next turn boundary. Pinned
/// because it is inherited rather than implemented here, and inherited guarantees are the ones
/// that quietly disappear.
#[test]
fn the_child_observes_the_generations_cancellation_flag() {
    let cancelled = Arc::new(AtomicBool::new(true));
    let context = execution_context(cancelled.clone(), Arc::new(CapturingProgress::default()));

    assert!(context.is_cancelled());
    cancelled.store(false, std::sync::atomic::Ordering::Release);
    assert!(!context.is_cancelled());
}

/// Progress carries counts and a fixed phrase. A file path or search pattern here would leak the
/// child's reading into a user-facing signal, which is the one thing its own context is for.
#[test]
fn progress_events_carry_counts_and_no_content() {
    let sink = Arc::new(CapturingProgress::default());
    let context = execution_context(Arc::new(AtomicBool::new(false)), sink.clone());

    publish_progress(
        &context,
        2,
        NativeToolProgressPhase::Updated,
        "Reading 3 more sources.".to_owned(),
        7,
    );

    let events = sink.events.lock().expect("events");
    let event = events.first().expect("a progress event");
    assert_eq!(event.sequence, 2);
    assert_eq!(event.metadata["tool_calls"], json!(7));
    let message = event.message.as_deref().expect("message");
    assert!(message.contains('3'), "{message}");
    for leaked in ["C:/", ".rs", "pattern", "needle"] {
        assert!(!message.contains(leaked), "{message}");
    }
}
