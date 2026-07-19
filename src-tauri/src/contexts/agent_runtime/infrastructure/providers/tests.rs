use super::invocation::ProviderInvocationError;
use super::{
    apply_configuration_overrides, build_invocation, output_parser_for, ProviderOutputEvent,
    ProviderPromptDelivery,
};
use crate::contexts::agent_runtime::application::{AgentChatConfiguration, ToolUseBlock};
use crate::contexts::agent_runtime::domain::InteractionMode;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const STABLE_AGENT_IDS: [&str; 4] = ["claude-code", "codex-cli", "gemini-cli", "opencode"];

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
fn output_fixtures_cover_every_stable_provider() {
    let fixtures = [
        (
            "claude-code",
            include_str!("fixtures/claude-code.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("claude-session".to_string()),
                ProviderOutputEvent::Token("hello from claude".to_string()),
                ProviderOutputEvent::Thinking("inspect first".to_string()),
                ProviderOutputEvent::ToolUse(ToolUseBlock {
                    id: "claude-tool".to_string(),
                    name: "Read".to_string(),
                    input: Some(serde_json::json!({"path":"src/main.rs"})),
                    output: None,
                    status: "running".to_string(),
                }),
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
                ProviderOutputEvent::ToolUse(ToolUseBlock {
                    id: "codex-tool".to_string(),
                    name: "read_file".to_string(),
                    input: Some(serde_json::json!({"path":"Cargo.toml"})),
                    output: None,
                    status: "running".to_string(),
                }),
                ProviderOutputEvent::Completed,
            ],
        ),
        (
            "gemini-cli",
            include_str!("fixtures/gemini-cli.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("gemini-session".to_string()),
                ProviderOutputEvent::Token("hello from gemini".to_string()),
                ProviderOutputEvent::ToolUse(ToolUseBlock {
                    id: "gemini-tool".to_string(),
                    name: "read_file".to_string(),
                    input: Some(serde_json::json!({"path":"README.md"})),
                    output: None,
                    status: "running".to_string(),
                }),
                ProviderOutputEvent::Completed,
            ],
        ),
        (
            "opencode",
            include_str!("fixtures/opencode.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("opencode-session".to_string()),
                ProviderOutputEvent::Token("hello from opencode".to_string()),
                ProviderOutputEvent::ToolUse(ToolUseBlock {
                    id: "opencode-tool".to_string(),
                    name: "read".to_string(),
                    input: Some(serde_json::json!({"path":"src/lib.rs"})),
                    output: None,
                    status: "running".to_string(),
                }),
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

fn assert_stable_agent_coverage<'a>(agent_ids: impl Iterator<Item = &'a str>) {
    assert_eq!(
        agent_ids.collect::<BTreeSet<_>>(),
        STABLE_AGENT_IDS.into_iter().collect::<BTreeSet<_>>()
    );
}
