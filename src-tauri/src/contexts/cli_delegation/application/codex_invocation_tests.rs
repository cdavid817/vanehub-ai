use super::*;

fn absolute(name: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:/delegation/{name}"))
    } else {
        PathBuf::from(format!("/delegation/{name}"))
    }
}

fn build(mode: DelegationMode) -> CodexDelegationInvocation {
    CodexDelegationInvocationBuilder::build(CodexInvocationRequest {
        executable: absolute("codex.exe"),
        workspace: absolute("workspace"),
        schema_file: &absolute("control/report.schema.json"),
        final_output: &absolute("control/final.json"),
        task_prompt: "Inspect the admitted repository.",
        mode,
        profile: CodexInvocationProfile {
            model: Some("gpt-5.4".into()),
            reasoning_effort: Some("high".into()),
        },
    })
    .expect("valid invocation")
}

fn value_after<'a>(args: &'a [String], flag: &str) -> &'a str {
    let index = args.iter().position(|item| item == flag).expect("flag");
    args.get(index + 1).expect("value")
}

#[test]
fn analyze_is_ephemeral_read_only_and_owns_final_capture() {
    let invocation = build(DelegationMode::Analyze);
    assert_eq!(invocation.args.first().map(String::as_str), Some("exec"));
    assert_eq!(invocation.args.last().map(String::as_str), Some("-"));
    assert_eq!(value_after(&invocation.args, "--sandbox"), "read-only");
    for flag in [
        "--json",
        "--ephemeral",
        "--ignore-user-config",
        "--ignore-rules",
        "--strict-config",
        "--output-schema",
        "--output-last-message",
    ] {
        assert!(invocation.args.iter().any(|item| item == flag), "{flag}");
    }
    assert!(invocation
        .args
        .iter()
        .any(|item| item == "approval_policy=\"never\""));
    assert!(!invocation
        .args
        .iter()
        .any(|item| item.contains("dangerously") || item == "--yolo"));
}

#[test]
fn edit_is_workspace_write_without_bypass() {
    let invocation = build(DelegationMode::Edit);
    assert_eq!(
        value_after(&invocation.args, "--sandbox"),
        "workspace-write"
    );
    assert_eq!(invocation.stdin, b"Inspect the admitted repository.");
}

#[test]
fn profile_cannot_inject_owned_arguments() {
    let result = CodexDelegationInvocationBuilder::build(CodexInvocationRequest {
        executable: absolute("codex.exe"),
        workspace: absolute("workspace"),
        schema_file: &absolute("schema.json"),
        final_output: &absolute("final.json"),
        task_prompt: "task",
        mode: DelegationMode::Edit,
        profile: CodexInvocationProfile {
            model: Some("gpt --yolo".into()),
            reasoning_effort: None,
        },
    });
    assert_eq!(result, Err(CodexInvocationError::InvalidProfile));
}
