use super::invocation::ProviderInvocationError;
use super::{
    apply_configuration_overrides, build_interactive_invocation, build_invocation,
    output_parser_for, ProviderOutputEvent, ProviderPromptDelivery, ProviderToolEvent,
    ProviderToolPhase,
};
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, GenerationProcessFailureKind,
};
use crate::contexts::agent_runtime::domain::InteractionMode;
use crate::contexts::execution_observability::api::ExecutionFidelity;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const STABLE_AGENT_IDS: [&str; 4] = ["claude-code", "codex-cli", "gemini-cli", "opencode"];

fn running_tool(id: &str, name: &str, input: Value) -> ProviderOutputEvent {
    ProviderOutputEvent::ToolLifecycle(Box::new(ProviderToolEvent {
        call_id: Some(id.to_string()),
        name: Some(name.to_string()),
        input: Some(input),
        output: None,
        phase: ProviderToolPhase::Started,
        provider_timestamp: None,
        status: "running".to_string(),
        fidelity: ExecutionFidelity::Inferred,
        parent_run_id: None,
        parent_trace_id: None,
        parent_span_id: None,
        delegation_id: None,
        attempt: None,
    }))
}

fn completed_tool(id: &str, name: &str, output: Value) -> ProviderOutputEvent {
    ProviderOutputEvent::ToolLifecycle(Box::new(ProviderToolEvent {
        call_id: Some(id.to_string()),
        name: Some(name.to_string()),
        input: None,
        output: Some(output),
        phase: ProviderToolPhase::Completed,
        provider_timestamp: None,
        status: "completed".to_string(),
        fidelity: ExecutionFidelity::Inferred,
        parent_run_id: None,
        parent_trace_id: None,
        parent_span_id: None,
        delegation_id: None,
        attempt: None,
    }))
}

fn failed_tool(id: &str, name: &str) -> ProviderOutputEvent {
    ProviderOutputEvent::ToolLifecycle(Box::new(ProviderToolEvent {
        call_id: Some(id.to_string()),
        name: Some(name.to_string()),
        input: None,
        output: None,
        phase: ProviderToolPhase::Failed,
        provider_timestamp: None,
        status: "failed".to_string(),
        fidelity: ExecutionFidelity::Inferred,
        parent_run_id: None,
        parent_trace_id: None,
        parent_span_id: None,
        delegation_id: None,
        attempt: None,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvocationFixture {
    agent_id: String,
    executable: String,
    prompt: String,
    runtime_session_id: String,
    managed_args: Vec<String>,
    expected_args: Vec<String>,
    prompt_delivery: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParameterFixture {
    agent_id: String,
    model_id: Option<String>,
    reasoning_depth: Option<String>,
    permission_mode: String,
    thinking: bool,
    base: BTreeMap<String, Value>,
    expected: BTreeMap<String, Value>,
}

#[test]
fn invocation_fixtures_cover_every_stable_provider() {
    let fixtures: Vec<InvocationFixture> =
        serde_json::from_str(include_str!("fixtures/invocations.json")).expect("fixtures");
    assert_stable_agent_coverage(fixtures.iter().map(|fixture| fixture.agent_id.as_str()));

    for fixture in fixtures {
        let spec = build_invocation(
            &fixture.agent_id,
            fixture.executable.clone(),
            &fixture.prompt,
            Some(&fixture.runtime_session_id),
            &fixture.managed_args,
        )
        .expect("supported provider invocation");
        let expected_delivery = match fixture.prompt_delivery.as_str() {
            "stdin" => ProviderPromptDelivery::Stdin,
            "argument" => ProviderPromptDelivery::Argument,
            other => panic!("unknown prompt delivery fixture: {other}"),
        };

        assert_eq!(spec.executable, fixture.executable, "{}", fixture.agent_id);
        assert_eq!(spec.args, fixture.expected_args, "{}", fixture.agent_id);
        assert_eq!(
            spec.prompt_delivery, expected_delivery,
            "{}",
            fixture.agent_id
        );
        assert_eq!(
            spec.args.iter().any(|argument| argument == &fixture.prompt),
            expected_delivery == ProviderPromptDelivery::Argument,
            "prompt delivery leaked into the wrong channel for {}",
            fixture.agent_id
        );
    }
}

#[test]
fn parameter_mapping_fixtures_cover_every_stable_provider() {
    let fixtures: Vec<ParameterFixture> =
        serde_json::from_str(include_str!("fixtures/parameter-mappings.json")).expect("fixtures");
    assert_stable_agent_coverage(fixtures.iter().map(|fixture| fixture.agent_id.as_str()));

    for fixture in fixtures {
        let configuration = AgentChatConfiguration {
            agent_id: fixture.agent_id.clone(),
            interaction_mode: InteractionMode::Cli,
            permission_mode: fixture.permission_mode,
            provider_id: None,
            model_id: fixture.model_id,
            reasoning_depth: fixture.reasoning_depth,
            streaming: true,
            thinking: fixture.thinking,
            long_context: false,
        };
        let selections =
            apply_configuration_overrides(&fixture.agent_id, fixture.base, &configuration);

        assert_eq!(selections, fixture.expected, "{}", fixture.agent_id);
    }
}

#[test]
fn interactive_invocations_cover_fresh_and_resume_for_every_stable_provider() {
    let fixtures = [
        (
            "claude-code",
            vec!["--chrome".to_string()],
            vec!["--chrome".to_string()],
            vec![
                "--chrome".to_string(),
                "--resume".to_string(),
                "runtime-1".to_string(),
            ],
        ),
        (
            "codex-cli",
            vec!["--strict-config".to_string()],
            vec!["--strict-config".to_string()],
            vec![
                "--strict-config".to_string(),
                "resume".to_string(),
                "runtime-1".to_string(),
            ],
        ),
        (
            "gemini-cli",
            vec!["--sandbox".to_string()],
            vec!["--sandbox".to_string()],
            vec![
                "--sandbox".to_string(),
                "--resume".to_string(),
                "runtime-1".to_string(),
            ],
        ),
        (
            "opencode",
            vec!["--auto".to_string()],
            vec!["--auto".to_string()],
            vec![
                "--auto".to_string(),
                "--session".to_string(),
                "runtime-1".to_string(),
            ],
        ),
    ];
    assert_stable_agent_coverage(fixtures.iter().map(|(agent_id, _, _, _)| *agent_id));

    for (agent_id, managed_args, fresh_args, resume_args) in fixtures {
        let fresh = build_interactive_invocation(
            agent_id,
            format!("C:/bin/{agent_id}.exe"),
            None,
            &managed_args,
        )
        .expect("fresh interactive invocation");
        assert_eq!(fresh.args, fresh_args, "{agent_id} fresh");

        let resume = build_interactive_invocation(
            agent_id,
            format!("C:/bin/{agent_id}.exe"),
            Some("runtime-1"),
            &managed_args,
        )
        .expect("resume interactive invocation");
        assert_eq!(resume.args, resume_args, "{agent_id} resume");
    }
}

#[test]
fn output_fixtures_cover_every_stable_provider() {
    let fixtures = [
        (
            "claude-code",
            include_str!("fixtures/claude-code.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("claude-session".to_string()),
                ProviderOutputEvent::Token("hello from claude".to_string()),
                ProviderOutputEvent::Thinking("inspect first".to_string()),
                running_tool(
                    "claude-tool",
                    "Read",
                    serde_json::json!({"path":"src/main.rs"}),
                ),
                completed_tool("claude-tool", "Read", serde_json::json!({"bytes":12})),
                failed_tool("claude-failed", "Shell"),
                ProviderOutputEvent::RichBlock(serde_json::json!({
                    "id":"claude-card","kind":"card","v":1,"title":"Summary"
                })),
                ProviderOutputEvent::Completed,
            ],
        ),
        (
            "codex-cli",
            include_str!("fixtures/codex-cli.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("codex-session".to_string()),
                ProviderOutputEvent::Token("hello from codex".to_string()),
                ProviderOutputEvent::Thinking("checking files".to_string()),
                running_tool(
                    "codex-tool",
                    "read_file",
                    serde_json::json!({"path":"Cargo.toml"}),
                ),
                completed_tool("codex-tool", "read_file", serde_json::json!({"bytes":20})),
                failed_tool("codex-failed", "shell"),
                ProviderOutputEvent::Completed,
                ProviderOutputEvent::SessionId("codex-thread".to_string()),
                ProviderOutputEvent::Token("hello from current codex".to_string()),
            ],
        ),
        (
            "gemini-cli",
            include_str!("fixtures/gemini-cli.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("gemini-session".to_string()),
                ProviderOutputEvent::Token("hello from gemini".to_string()),
                running_tool(
                    "gemini-tool",
                    "read_file",
                    serde_json::json!({"path":"README.md"}),
                ),
                completed_tool("gemini-tool", "read_file", serde_json::json!({"bytes":30})),
                failed_tool("gemini-failed", "shell"),
                ProviderOutputEvent::Completed,
            ],
        ),
        (
            "opencode",
            include_str!("fixtures/opencode.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("opencode-session".to_string()),
                ProviderOutputEvent::Token("hello from opencode".to_string()),
                running_tool(
                    "opencode-tool",
                    "read",
                    serde_json::json!({"path":"src/lib.rs"}),
                ),
                completed_tool("opencode-tool", "read", serde_json::json!({"bytes":40})),
                failed_tool("opencode-failed", "shell"),
                ProviderOutputEvent::Completed,
                ProviderOutputEvent::SessionId("opencode-current-session".to_string()),
                ProviderOutputEvent::Token("hello from current opencode".to_string()),
                ProviderOutputEvent::Completed,
            ],
        ),
    ];
    assert_stable_agent_coverage(fixtures.iter().map(|(agent_id, _, _)| *agent_id));

    for (agent_id, fixture, expected) in fixtures {
        let parser = output_parser_for(agent_id);
        let parsed = fixture
            .lines()
            .map(|line| parser.parse_line(line))
            .collect::<Vec<_>>();
        assert_eq!(parsed, expected, "{agent_id}");
    }
}

#[test]
fn tool_lifecycle_fixture_preserves_ids_phases_timestamps_and_opaque_gaps() {
    let parser = output_parser_for("codex-cli");
    let tools = include_str!("fixtures/tool-lifecycle.output.jsonl")
        .lines()
        .map(|line| parser.parse_line(line))
        .map(|event| match event {
            ProviderOutputEvent::ToolLifecycle(tool) => tool,
            unexpected => panic!("expected tool lifecycle event, got {unexpected:?}"),
        })
        .collect::<Vec<_>>();

    assert_eq!(tools.len(), 10);
    assert_eq!(tools[0].phase, ProviderToolPhase::Started);
    assert_eq!(
        tools[0].provider_timestamp.as_deref(),
        Some("2026-07-23T00:00:00Z")
    );
    assert_eq!(tools[1].call_id, tools[0].call_id);
    assert_eq!(tools[2].phase, ProviderToolPhase::Completed);
    assert_eq!(tools[3].phase, ProviderToolPhase::Completed);
    assert_eq!(tools[4].phase, ProviderToolPhase::Started);
    assert_eq!(tools[5].phase, ProviderToolPhase::Failed);
    assert_eq!(tools[6].call_id, None);
    assert_eq!(tools[6].fidelity, ExecutionFidelity::Opaque);
    assert_eq!(tools[7].name, None);
    assert_ne!(tools[8].call_id, tools[9].call_id);
}

#[test]
fn provider_delegation_metadata_is_preserved_when_reported() {
    let event = output_parser_for("codex-cli").parse_line(
        r#"{"type":"tool_call","id":"call-1","name":"delegate","parent_run_id":"018f0f17-4d6a-7e20-b41d-66c5271a28d0","parent_trace_id":"4bf92f3577b34da6a3ce929d0e0e4736","parent_span_id":"00f067aa0ba902b7","delegation_id":"delegation-1","attempt":2}"#,
    );
    let ProviderOutputEvent::ToolLifecycle(tool) = event else {
        panic!("expected tool lifecycle event");
    };
    assert_eq!(tool.delegation_id.as_deref(), Some("delegation-1"));
    assert_eq!(tool.attempt, Some(2));
    assert_eq!(
        tool.parent_trace_id.as_deref(),
        Some("4bf92f3577b34da6a3ce929d0e0e4736")
    );
}

#[test]
fn unsupported_invocation_is_explicit_and_unknown_output_is_lossless() {
    assert_eq!(
        build_invocation("unknown", "unknown".to_string(), "prompt", None, &[]),
        Err(ProviderInvocationError::UnsupportedAgent(
            "unknown".to_string()
        ))
    );
    assert_eq!(
        output_parser_for("unknown").parse_line("unstructured output"),
        ProviderOutputEvent::Token("unstructured output".to_string())
    );
}

#[test]
fn structured_policy_failure_is_non_retryable_without_matching_diagnostic_text() {
    let event = output_parser_for("codex-cli")
        .parse_line(r#"{"type":"error","error":{"code":"permission_denied","message":"opaque"}}"#);
    let ProviderOutputEvent::Failed(failure) = event else {
        panic!("expected provider failure");
    };
    assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
    assert_eq!(failure.diagnostic, "opaque");

    let event = output_parser_for("codex-cli")
        .parse_line(r#"{"type":"error","error":{"code":"transport_error","message":"opaque"}}"#);
    let ProviderOutputEvent::Failed(failure) = event else {
        panic!("expected provider failure");
    };
    assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
}

fn assert_stable_agent_coverage<'a>(agent_ids: impl Iterator<Item = &'a str>) {
    assert_eq!(
        agent_ids.collect::<BTreeSet<_>>(),
        STABLE_AGENT_IDS.into_iter().collect::<BTreeSet<_>>()
    );
}
