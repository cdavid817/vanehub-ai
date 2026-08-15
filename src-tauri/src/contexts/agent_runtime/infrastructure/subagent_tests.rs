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
    let names: Vec<String> = child_tool_catalog()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names, vec!["file", "grep", "glob"]);
}

#[test]
fn the_child_catalog_excludes_every_effectful_and_delegating_tool() {
    let names: Vec<String> = child_tool_catalog()
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
