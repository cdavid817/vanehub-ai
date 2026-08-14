use super::*;

fn absolute(name: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:/delegation/{name}"))
    } else {
        PathBuf::from(format!("/delegation/{name}"))
    }
}

fn build(mode: DelegationMode) -> ClaudeDelegationInvocation {
    ClaudeDelegationInvocationBuilder::build(ClaudeInvocationRequest {
        executable: absolute("claude.exe"),
        workspace: absolute("workspace"),
        settings_file: &absolute("control/settings.json"),
        empty_mcp_config: &absolute("control/mcp.json"),
        task_prompt: "Inspect the admitted repository.",
        schema_json: r#"{"type":"object"}"#,
        mode,
        maximum_turns: 12,
        maximum_budget_microusd: Some(2_500_000),
        profile: ClaudeInvocationProfile {
            model: Some("claude-sonnet-4-5".into()),
            effort: Some("high".into()),
        },
    })
    .expect("valid invocation")
}

fn value_after<'a>(args: &'a [String], flag: &str) -> &'a str {
    let index = args.iter().position(|item| item == flag).expect("flag");
    args.get(index + 1).expect("value")
}

#[test]
fn analyze_owns_fresh_isolated_noninteractive_flags() {
    let invocation = build(DelegationMode::Analyze);
    assert_eq!(
        Uuid::parse_str(&invocation.session_id)
            .expect("uuid")
            .get_version_num(),
        4
    );
    assert_eq!(
        value_after(&invocation.args, "--session-id"),
        invocation.session_id
    );
    assert_eq!(value_after(&invocation.args, "--permission-mode"), "plan");
    assert_eq!(value_after(&invocation.args, "--tools"), "Read,Glob,Grep");
    for flag in [
        "-p",
        "--include-partial-messages",
        "--verbose",
        "--no-session-persistence",
        "--no-chrome",
        "--safe-mode",
        "--disable-slash-commands",
        "--strict-mcp-config",
    ] {
        assert!(invocation.args.iter().any(|item| item == flag), "{flag}");
    }
    assert!(!invocation.args.iter().any(|item| {
        matches!(
            item.as_str(),
            "--resume" | "--continue" | "--dangerously-skip-permissions"
        )
    }));
    assert_eq!(
        value_after(&invocation.args, "--max-budget-usd"),
        "2.500000"
    );
    assert_eq!(invocation.stdin, b"Inspect the admitted repository.");
}

#[test]
fn edit_allows_only_reviewed_mutating_tools() {
    let invocation = build(DelegationMode::Edit);
    assert_eq!(
        value_after(&invocation.args, "--permission-mode"),
        "acceptEdits"
    );
    assert_eq!(
        value_after(&invocation.args, "--tools"),
        "Read,Glob,Grep,Edit,Write"
    );
    assert!(!invocation.args.iter().any(|value| value.contains("Bash")));
}

#[test]
fn unreviewed_profile_values_and_limits_fail_closed() {
    let result = ClaudeDelegationInvocationBuilder::build(ClaudeInvocationRequest {
        executable: absolute("claude.exe"),
        workspace: absolute("workspace"),
        settings_file: &absolute("settings.json"),
        empty_mcp_config: &absolute("mcp.json"),
        task_prompt: "task",
        schema_json: r#"{"type":"object"}"#,
        mode: DelegationMode::Analyze,
        maximum_turns: 65,
        maximum_budget_microusd: None,
        profile: ClaudeInvocationProfile {
            model: Some("model --resume prior".into()),
            effort: None,
        },
    });
    assert!(matches!(
        result,
        Err(ClaudeInvocationError::InvalidLimits | ClaudeInvocationError::InvalidProfile)
    ));
}
