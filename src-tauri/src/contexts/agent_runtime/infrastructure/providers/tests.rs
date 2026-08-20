use super::invocation::ProviderInvocationError;
use super::{
    apply_configuration_overrides, apply_policy_template_overrides, build_interactive_invocation,
    build_invocation, build_invocation_with_role, force_gemini_standard_approval_flag,
    output_parser_for, ProviderOutputEvent, ProviderPromptDelivery, ProviderReportedUsage,
    ProviderToolEvent, ProviderToolPhase, ProviderUsageOverlap, POLICY_TEMPLATE_GOVERNED_AGENT_IDS,
};
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, GenerationProcessFailureKind,
};
use crate::contexts::agent_runtime::domain::InteractionMode;
use crate::contexts::agent_runtime::infrastructure::process_adapter::{
    local_runner_launch_spec, provider_prompt_input,
};
use crate::contexts::execution_observability::api::ExecutionFidelity;
use crate::contexts::permissions::api::PolicyTemplateName;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const STABLE_AGENT_IDS: [&str; 5] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
];
const ALL_POLICY_TEMPLATES: [PolicyTemplateName; 4] = [
    PolicyTemplateName::Readonly,
    PolicyTemplateName::Standard,
    PolicyTemplateName::Trusted,
    PolicyTemplateName::Yolo,
];

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
    execution_mode: String,
    thinking: bool,
    base: BTreeMap<String, Value>,
    expected: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageEdgeCaseFixture {
    name: String,
    agent_id: String,
    line: String,
    expected: UsageEdgeCaseExpectation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageEdgeCaseExpectation {
    kind: String,
    input: Option<i64>,
    output: Option<i64>,
    cache_read: Option<i64>,
    cache_write: Option<i64>,
    reasoning: Option<i64>,
    total: Option<i64>,
    source_identity: Option<String>,
    source_revision: Option<String>,
    failure_kind: Option<String>,
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

        let runner_spec = local_runner_launch_spec(
            &spec,
            Some("session-fixture".to_string()),
            Some("C:/workspace/fixture".to_string()),
            "00-fixture-trace-fixture-span-01".to_string(),
        );
        assert_eq!(
            runner_spec.executable, fixture.executable,
            "{}",
            fixture.agent_id
        );
        assert_eq!(runner_spec.session_id.as_deref(), Some("session-fixture"));
        assert_eq!(
            runner_spec.arguments, fixture.expected_args,
            "{}",
            fixture.agent_id
        );
        assert_eq!(
            runner_spec.cwd.as_deref(),
            Some("C:/workspace/fixture"),
            "{}",
            fixture.agent_id
        );
        assert_eq!(
            runner_spec
                .environment
                .get("TRACEPARENT")
                .map(String::as_str),
            Some("00-fixture-trace-fixture-span-01"),
            "{}",
            fixture.agent_id
        );
        assert_eq!(
            runner_spec.pipe_stdin,
            expected_delivery == ProviderPromptDelivery::Stdin,
            "{}",
            fixture.agent_id
        );
        assert_eq!(
            provider_prompt_input(&spec, &fixture.prompt),
            (expected_delivery == ProviderPromptDelivery::Stdin).then(|| format!(
                "{}\n",
                fixture.prompt
            )
            .into_bytes()),
            "{}",
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
            execution_mode: fixture.execution_mode,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PolicyTemplateFixture {
    agent_id: String,
    template: String,
    base: BTreeMap<String, Value>,
    expected: BTreeMap<String, Value>,
}

#[test]
fn policy_template_override_fixtures_cover_every_combination() {
    let fixtures: Vec<PolicyTemplateFixture> =
        serde_json::from_str(include_str!("fixtures/policy-template-overrides.json"))
            .expect("fixtures");
    assert_eq!(
        fixtures.len(),
        POLICY_TEMPLATE_GOVERNED_AGENT_IDS.len() * ALL_POLICY_TEMPLATES.len(),
        "expected every (agent, template) combination to be covered exactly once"
    );
    let mut seen = BTreeSet::new();
    for fixture in &fixtures {
        assert!(
            POLICY_TEMPLATE_GOVERNED_AGENT_IDS.contains(&fixture.agent_id.as_str()),
            "unexpected agent id in fixture: {}",
            fixture.agent_id
        );
        seen.insert((fixture.agent_id.clone(), fixture.template.clone()));
    }
    assert_eq!(
        seen.len(),
        fixtures.len(),
        "fixture file must not repeat an (agent, template) combination"
    );

    for fixture in fixtures {
        let template =
            PolicyTemplateName::from_str(&fixture.template).expect("known policy template");
        let selections = apply_policy_template_overrides(&fixture.agent_id, fixture.base, template);
        assert_eq!(
            selections, fixture.expected,
            "{} / {}",
            fixture.agent_id, fixture.template
        );
    }
}

#[test]
fn policy_template_overrides_never_introduce_a_dangerous_flag() {
    for agent_id in POLICY_TEMPLATE_GOVERNED_AGENT_IDS {
        for template in ALL_POLICY_TEMPLATES {
            let selections = apply_policy_template_overrides(agent_id, BTreeMap::new(), template);
            for (key, value) in &selections {
                assert!(
                    !key.to_lowercase().contains("dangerously"),
                    "{agent_id} / {template:?} introduced a dangerous key: {key}"
                );
                if let Some(text) = value.as_str() {
                    assert!(
                        !text.to_lowercase().contains("dangerously"),
                        "{agent_id} / {template:?} introduced a dangerous value: {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn gemini_standard_force_emits_approval_mode_default() {
    let appended = force_gemini_standard_approval_flag(
        "gemini-cli",
        PolicyTemplateName::Standard,
        vec!["--sandbox".to_string()],
    );
    assert_eq!(
        appended,
        vec![
            "--sandbox".to_string(),
            "--approval-mode".to_string(),
            "default".to_string(),
        ]
    );

    let replaced = force_gemini_standard_approval_flag(
        "gemini-cli",
        PolicyTemplateName::Standard,
        vec![
            "--approval-mode".to_string(),
            "yolo".to_string(),
            "--sandbox".to_string(),
        ],
    );
    assert_eq!(
        replaced,
        vec![
            "--sandbox".to_string(),
            "--approval-mode".to_string(),
            "default".to_string(),
        ]
    );

    let untouched_other_template = force_gemini_standard_approval_flag(
        "gemini-cli",
        PolicyTemplateName::Trusted,
        vec!["--approval-mode".to_string(), "yolo".to_string()],
    );
    assert_eq!(
        untouched_other_template,
        vec!["--approval-mode".to_string(), "yolo".to_string()]
    );

    let untouched_other_agent = force_gemini_standard_approval_flag(
        "codex-cli",
        PolicyTemplateName::Standard,
        vec!["--sandbox".to_string(), "workspace-write".to_string()],
    );
    assert_eq!(
        untouched_other_agent,
        vec!["--sandbox".to_string(), "workspace-write".to_string()]
    );
}

#[test]
fn interactive_invocations_cover_fresh_and_resume_for_every_stable_provider() {
    let fixtures = [
        (
            "claude-code",
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
            vec![
                "--strict-config".to_string(),
                "resume".to_string(),
                "runtime-1".to_string(),
            ],
        ),
        (
            "gemini-cli",
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
            vec![
                "--auto".to_string(),
                "--session".to_string(),
                "runtime-1".to_string(),
            ],
        ),
        (
            "antigravity-cli",
            vec!["--sandbox".to_string()],
            vec![
                "--sandbox".to_string(),
                "--conversation".to_string(),
                "runtime-1".to_string(),
            ],
        ),
    ];
    assert_stable_agent_coverage(fixtures.iter().map(|(agent_id, _, _)| *agent_id));

    for (agent_id, managed_args, resume_args) in fixtures {
        let fresh = build_interactive_invocation(
            agent_id,
            format!("C:/bin/{agent_id}.exe"),
            None,
            &managed_args,
        )
        .expect("fresh interactive invocation");
        match agent_id {
            "claude-code" | "gemini-cli" => {
                let assigned = fresh
                    .assigned_runtime_session_id
                    .as_deref()
                    .expect("caller-assigned session id");
                uuid::Uuid::parse_str(assigned).expect("provider-valid UUID");
                assert_eq!(
                    fresh.args,
                    [
                        managed_args.clone(),
                        vec!["--session-id".to_string(), assigned.to_string()],
                    ]
                    .concat(),
                    "{agent_id} fresh"
                );
            }
            _ => {
                assert_eq!(fresh.args, managed_args, "{agent_id} fresh");
                assert_eq!(fresh.assigned_runtime_session_id, None);
            }
        }

        let resume = build_interactive_invocation(
            agent_id,
            format!("C:/bin/{agent_id}.exe"),
            Some("runtime-1"),
            &managed_args,
        )
        .expect("resume interactive invocation");
        assert_eq!(resume.args, resume_args, "{agent_id} resume");
        assert_eq!(resume.assigned_runtime_session_id, None);
    }
}

#[test]
fn empty_interactive_runtime_session_id_is_treated_as_fresh() {
    for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode"] {
        let invocation = build_interactive_invocation(
            agent_id,
            format!("C:/bin/{agent_id}.exe"),
            Some("  "),
            &[],
        )
        .expect("fresh interactive invocation");

        assert!(
            !invocation.args.iter().any(|argument| argument == "  "),
            "{agent_id}"
        );
        assert_eq!(
            invocation.assigned_runtime_session_id.is_some(),
            matches!(agent_id, "claude-code" | "gemini-cli"),
            "{agent_id}"
        );
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
                ProviderOutputEvent::Completed(None),
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
                ProviderOutputEvent::Completed(None),
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
                ProviderOutputEvent::Completed(None),
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
                ProviderOutputEvent::Completed(None),
                ProviderOutputEvent::SessionId("opencode-current-session".to_string()),
                ProviderOutputEvent::Token("hello from current opencode".to_string()),
                ProviderOutputEvent::Completed(None),
            ],
        ),
        (
            // Only the `result` line here is a verbatim capture. `init` carries the documented
            // `conversation_id`, and `step_update` stands in for "an event whose payload has not
            // been observed" — the parser must consume it without emitting invented increments.
            "antigravity-cli",
            include_str!("fixtures/antigravity-cli.output.jsonl"),
            vec![
                ProviderOutputEvent::SessionId("antigravity-conversation".to_string()),
                ProviderOutputEvent::Empty,
                ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
                    input_tokens: 12,
                    output_tokens: 5,
                    cache_read_tokens: 2,
                    cache_creation_tokens: 0,
                    reasoning_output_tokens: 3,
                    provider_total_tokens: Some(22),
                    cache_overlap: ProviderUsageOverlap::Exclusive,
                    reasoning_overlap: ProviderUsageOverlap::Exclusive,
                    normalization_version: "antigravity-result-usage-v2",
                    source_identity: Some("antigravity-conversation".to_string()),
                    ..ProviderReportedUsage::default()
                })),
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
fn claude_code_completion_line_reports_usage() {
    let event = output_parser_for("claude-code").parse_line(
        r#"{"type":"result","usage":{"input_tokens":120,"output_tokens":340,"cache_creation_input_tokens":50,"cache_read_input_tokens":900,"total_tokens":1410}}"#,
    );
    assert_eq!(
        event,
        ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
            input_tokens: 120,
            output_tokens: 340,
            cache_read_tokens: 900,
            cache_creation_tokens: 50,
            provider_total_tokens: Some(1410),
            cache_overlap: ProviderUsageOverlap::Exclusive,
            reasoning_overlap: ProviderUsageOverlap::Subset,
            normalization_version: "claude-code-result-usage-v1",
            ..ProviderReportedUsage::default()
        }))
    );
}

#[test]
fn managed_cli_usage_edge_fixture_covers_bounded_verified_shapes() {
    let fixtures: Vec<UsageEdgeCaseFixture> =
        serde_json::from_str(include_str!("fixtures/usage-edge-cases.json"))
            .expect("usage edge fixtures");
    assert_eq!(fixtures.len(), 10);
    let mut revisions = Vec::new();

    for fixture in fixtures {
        assert!(fixture.name.len() <= 64, "{}", fixture.name);
        assert!(fixture.line.len() <= 1_024, "{}", fixture.name);
        let event = output_parser_for(&fixture.agent_id).parse_line(&fixture.line);

        match fixture.expected.kind.as_str() {
            "usage" => {
                let ProviderOutputEvent::Completed(Some(usage)) = event else {
                    panic!("{} expected usage, got {event:?}", fixture.name);
                };
                assert_eq!(
                    usage.input_tokens,
                    fixture.expected.input.expect("expected input"),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.output_tokens,
                    fixture.expected.output.expect("expected output"),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.cache_read_tokens,
                    fixture.expected.cache_read.expect("expected cache read"),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.cache_creation_tokens,
                    fixture.expected.cache_write.expect("expected cache write"),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.reasoning_output_tokens,
                    fixture.expected.reasoning.expect("expected reasoning"),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.provider_total_tokens, fixture.expected.total,
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.source_identity.as_deref(),
                    fixture.expected.source_identity.as_deref(),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    usage.source_revision.as_deref(),
                    fixture.expected.source_revision.as_deref(),
                    "{}",
                    fixture.name
                );
                if fixture.name.starts_with("opencode-revision-") {
                    revisions.push((usage.source_identity, usage.source_revision));
                }
            }
            "no-usage" => assert_eq!(
                event,
                ProviderOutputEvent::Completed(None),
                "{}",
                fixture.name
            ),
            "failure" => {
                let ProviderOutputEvent::Failed(failure) = event else {
                    panic!("{} expected failure, got {event:?}", fixture.name);
                };
                let expected_kind = match fixture.expected.failure_kind.as_deref() {
                    Some("retryable") => GenerationProcessFailureKind::Retryable,
                    Some("non-retryable") => GenerationProcessFailureKind::NonRetryable,
                    other => panic!("{} has unknown failure kind: {other:?}", fixture.name),
                };
                assert_eq!(failure.kind, expected_kind, "{}", fixture.name);
            }
            other => panic!("{} has unknown expectation kind: {other}", fixture.name),
        }
    }

    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].0, revisions[1].0);
    assert_ne!(revisions[0].1, revisions[1].1);
}

#[test]
fn claude_code_all_zero_usage_is_treated_as_absent() {
    // The original payload here carried `is_error: true`, which now routes to a failure event.
    // This test is about usage normalization, so it exercises the success path deliberately;
    // the error path has its own tests below.
    let event = output_parser_for("claude-code").parse_line(
        r#"{"type":"result","usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}"#,
    );
    assert_eq!(event, ProviderOutputEvent::Completed(None));
}

#[test]
fn codex_cli_completion_line_preserves_non_additive_usage_dimensions() {
    let event = output_parser_for("codex-cli").parse_line(
        r#"{"type":"turn.completed","usage":{"input_tokens":500,"cached_input_tokens":200,"cache_write_input_tokens":30,"output_tokens":100,"reasoning_output_tokens":40,"total_tokens":600}}"#,
    );
    assert_eq!(
        event,
        ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
            input_tokens: 500,
            output_tokens: 100,
            cache_read_tokens: 200,
            cache_creation_tokens: 30,
            reasoning_output_tokens: 40,
            provider_total_tokens: Some(600),
            cache_overlap: ProviderUsageOverlap::Subset,
            reasoning_overlap: ProviderUsageOverlap::Subset,
            normalization_version: "codex-turn-completed-usage-v1",
            ..ProviderReportedUsage::default()
        }))
    );
}

#[test]
fn codex_cli_all_zero_usage_is_treated_as_absent() {
    let event = output_parser_for("codex-cli").parse_line(
        r#"{"type":"turn.completed","usage":{"input_tokens":0,"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0}}"#,
    );
    assert_eq!(event, ProviderOutputEvent::Completed(None));
}

#[test]
fn gemini_cli_completion_line_reports_usage() {
    let event = output_parser_for("gemini-cli").parse_line(
        r#"{"type":"result","stats":{"input_tokens":80,"output_tokens":200,"cached":15,"total_tokens":305,"models":{"gemini-2.5-pro":{"input_tokens":80,"output_tokens":200,"cached":15,"input":65,"total_tokens":305}}}}"#,
    );
    assert_eq!(
        event,
        ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
            input_tokens: 80,
            output_tokens: 200,
            cache_read_tokens: 15,
            cache_creation_tokens: 0,
            provider_total_tokens: Some(305),
            cache_overlap: ProviderUsageOverlap::Subset,
            reasoning_overlap: ProviderUsageOverlap::Exclusive,
            normalization_version: "gemini-result-stream-stats-v1",
            model_id: Some("gemini-2.5-pro".to_string()),
            ..ProviderReportedUsage::default()
        }))
    );
}

#[test]
fn gemini_cli_model_stats_preserve_explicit_thoughts_when_available() {
    let event = output_parser_for("gemini-cli").parse_line(
        r#"{"type":"result","stats":{"models":{"gemini-2.5-pro":{"input_tokens":80,"output_tokens":200,"cached":15,"thoughts":25,"total_tokens":305}}}}"#,
    );
    assert_eq!(
        event,
        ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
            input_tokens: 80,
            output_tokens: 200,
            cache_read_tokens: 15,
            reasoning_output_tokens: 25,
            provider_total_tokens: Some(305),
            cache_overlap: ProviderUsageOverlap::Subset,
            reasoning_overlap: ProviderUsageOverlap::Exclusive,
            normalization_version: "gemini-result-stream-stats-v1",
            model_id: Some("gemini-2.5-pro".to_string()),
            ..ProviderReportedUsage::default()
        }))
    );
}

#[test]
fn gemini_cli_all_zero_usage_is_treated_as_absent() {
    let event = output_parser_for("gemini-cli")
        .parse_line(r#"{"type":"result","stats":{"input_tokens":0,"output_tokens":0,"cached":0,"total_tokens":0}}"#);
    assert_eq!(event, ProviderOutputEvent::Completed(None));
}

#[test]
fn opencode_completion_line_preserves_usage_and_step_revision() {
    let event = output_parser_for("opencode").parse_line(
        r#"{"type":"step_finish","timestamp":1720000000123,"sessionID":"ses_one","part":{"id":"prt_step_one","sessionID":"ses_one","messageID":"msg_one","type":"step-finish","tokens":{"total":900,"input":600,"output":150,"reasoning":50,"cache":{"read":80,"write":20}},"cost":0.01}}"#,
    );
    assert_eq!(
        event,
        ProviderOutputEvent::Completed(Some(ProviderReportedUsage {
            input_tokens: 600,
            output_tokens: 150,
            cache_read_tokens: 80,
            cache_creation_tokens: 20,
            reasoning_output_tokens: 50,
            provider_total_tokens: Some(900),
            cache_overlap: ProviderUsageOverlap::Exclusive,
            reasoning_overlap: ProviderUsageOverlap::Exclusive,
            normalization_version: "opencode-step-finish-tokens-v1",
            source_identity: Some("prt_step_one".to_string()),
            source_revision: Some("1720000000123".to_string()),
            ..ProviderReportedUsage::default()
        }))
    );
}

#[test]
fn opencode_unsafe_step_identity_and_revision_are_not_retained() {
    let event = output_parser_for("opencode").parse_line(
        r#"{"type":"step_finish","revision":"../../unsafe value","part":{"id":"../private path","type":"step-finish","tokens":{"total":4,"input":1,"output":1,"reasoning":1,"cache":{"read":1,"write":0}}}}"#,
    );
    let ProviderOutputEvent::Completed(Some(usage)) = event else {
        panic!("expected reported usage");
    };
    assert_eq!(usage.source_identity, None);
    assert_eq!(usage.source_revision, None);
}

#[test]
fn opencode_all_zero_usage_is_treated_as_absent() {
    let event = output_parser_for("opencode").parse_line(
        r#"{"type":"step_finish","part":{"type":"step-finish","tokens":{"total":0,"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}},"cost":0}}"#,
    );
    assert_eq!(event, ProviderOutputEvent::Completed(None));
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

/// claude-code reports failures through a `result` event carrying `is_error`, not through an
/// `error` event, and writes nothing to stderr — so if the parser reads this as a completion the
/// user is left with only an exit code. Fixture is a real captured payload.
#[test]
fn claude_error_result_becomes_a_failure_carrying_the_cli_diagnostic() {
    let line = include_str!("fixtures/claude-code.error-result.jsonl");
    let event = output_parser_for("claude-code").parse_line(line.trim());

    match event {
        ProviderOutputEvent::Failed(failure) => {
            assert_eq!(
                failure.diagnostic, "Failed to authenticate. API Error: 403 Request not allowed",
                "the CLI's own text must reach the user"
            );
            assert_eq!(
                failure.kind,
                GenerationProcessFailureKind::NonRetryable,
                "a 403 is an authentication problem; retrying cannot fix it"
            );
        }
        other => panic!("expected a failure event, got {other:?}"),
    }
}

/// Captured verbatim from a real `agy -p ... --output-format stream-json` run (v1.1.11). The
/// envelope is `{"event":"<kind>","<kind>":{...}}`, not the flat `{"type":...}` the other CLIs
/// use, so this pins the shape a guess would have gotten wrong.
#[test]
fn antigravity_result_event_maps_status_and_diagnostic() {
    let line = include_str!("fixtures/antigravity-cli.auth-error-result.jsonl");
    let event = output_parser_for("antigravity-cli").parse_line(line.trim());

    match event {
        ProviderOutputEvent::Failed(failure) => {
            assert_eq!(failure.diagnostic, "authentication failed or timed out");
            assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
        }
        other => panic!("expected a failure event, got {other:?}"),
    }
}

#[test]
fn antigravity_success_preserves_verified_usage_dimensions() {
    let line = r#"{"event":"result","result":{"conversation_id":"c-1","status":"SUCCESS","response":"hi","error":"","usage":{"input_tokens":10,"output_tokens":4,"thinking_tokens":6,"cache_read_tokens":3,"total_tokens":23}}}"#;

    match output_parser_for("antigravity-cli").parse_line(line) {
        ProviderOutputEvent::Completed(Some(usage)) => {
            assert_eq!(usage.input_tokens, 10);
            assert_eq!(usage.output_tokens, 4);
            assert_eq!(usage.reasoning_output_tokens, 6);
            assert_eq!(usage.cache_read_tokens, 3);
            assert_eq!(usage.provider_total_tokens, Some(23));
            assert_eq!(usage.cache_overlap, ProviderUsageOverlap::Exclusive);
            assert_eq!(usage.reasoning_overlap, ProviderUsageOverlap::Exclusive);
            assert_eq!(usage.source_identity.as_deref(), Some("c-1"));
        }
        other => panic!("expected a completion carrying usage, got {other:?}"),
    }
}

#[test]
fn antigravity_success_with_all_zero_usage_is_treated_as_absent() {
    let line = r#"{"event":"result","result":{"conversation_id":"c-1","status":"SUCCESS","response":"","error":"","usage":{"input_tokens":0,"output_tokens":0,"thinking_tokens":0,"cache_read_tokens":0,"total_tokens":0}}}"#;

    assert_eq!(
        output_parser_for("antigravity-cli").parse_line(line),
        ProviderOutputEvent::Completed(None)
    );
}

#[test]
fn antigravity_init_yields_the_conversation_id_and_unknown_events_are_ignored() {
    let parser = output_parser_for("antigravity-cli");

    assert_eq!(
        parser.parse_line(r#"{"event":"init","init":{"conversation_id":"conv-7"}}"#),
        ProviderOutputEvent::SessionId("conv-7".to_string())
    );
    // `step_update` is consumed without emitting increments until its payload is captured from a
    // live authenticated run; an unrecognized event must never fail the turn.
    assert_eq!(
        parser.parse_line(r#"{"event":"step_update","step_update":{"unobserved":true}}"#),
        ProviderOutputEvent::Empty
    );
    assert_eq!(
        parser.parse_line(r#"{"event":"something_new","something_new":{}}"#),
        ProviderOutputEvent::Empty
    );
    assert_eq!(
        parser.parse_line("not json at all"),
        ProviderOutputEvent::Empty
    );
}

/// A non-terminal status on a terminal event means the contract moved; reporting it as success
/// would hand the user an empty reply and call the turn done.
#[test]
fn antigravity_non_terminal_result_status_fails_loudly() {
    let line = r#"{"event":"result","result":{"status":"RUNNING","error":""}}"#;

    match output_parser_for("antigravity-cli").parse_line(line) {
        ProviderOutputEvent::Failed(failure) => {
            assert!(
                failure.diagnostic.contains("non-terminal"),
                "{}",
                failure.diagnostic
            );
        }
        other => panic!("expected a failure event, got {other:?}"),
    }
}

#[test]
fn claude_successful_result_still_completes_with_its_usage() {
    let line = serde_json::json!({
        "type": "result",
        "subtype": "success",
        "usage": {"input_tokens": 11, "output_tokens": 5},
    })
    .to_string();

    match output_parser_for("claude-code").parse_line(&line) {
        ProviderOutputEvent::Completed(usage) => {
            let usage = usage.expect("successful results keep reporting usage");
            assert_eq!(usage.input_tokens, 11);
            assert_eq!(usage.output_tokens, 5);
        }
        other => panic!("expected a completed event, got {other:?}"),
    }
}

#[test]
fn claude_error_result_without_a_status_stays_retryable() {
    let line = serde_json::json!({
        "type": "result",
        "is_error": true,
        "result": "upstream timed out",
    })
    .to_string();

    match output_parser_for("claude-code").parse_line(&line) {
        ProviderOutputEvent::Failed(failure) => {
            assert_eq!(failure.diagnostic, "upstream timed out");
            assert_eq!(
                failure.kind,
                GenerationProcessFailureKind::Retryable,
                "without a classifying code the existing retryable default must hold"
            );
        }
        other => panic!("expected a failure event, got {other:?}"),
    }
}

/// Role briefings must ride the CLI's own system-prompt mechanism, which survives context
/// compaction. Passing them as ordinary prompt text would let a long session compact the role away
/// and the Agent would quietly stop being the reviewer.
#[test]
fn claude_role_briefing_uses_the_native_system_prompt_flag() {
    let spec = build_invocation_with_role(
        "claude-code",
        "claude".to_string(),
        "hello",
        None,
        &[],
        Some("你是架构师。"),
    )
    .expect("invocation");

    let index = spec
        .args
        .iter()
        .position(|argument| argument == "--append-system-prompt")
        .expect("claude takes the briefing through --append-system-prompt");
    assert_eq!(spec.args[index + 1], "你是架构师。");
}

#[test]
fn codex_role_briefing_uses_developer_instructions() {
    let spec = build_invocation_with_role(
        "codex-cli",
        "codex".to_string(),
        "hello",
        None,
        &[],
        Some("你是审查者。"),
    )
    .expect("invocation");

    let index = spec
        .args
        .iter()
        .position(|argument| argument == "-c")
        .expect("codex takes the briefing through -c");
    assert!(spec.args[index + 1].starts_with("developer_instructions="));
    assert!(spec.args[index + 1].contains("你是审查者。"));
}

/// A single-Agent session passes no briefing, and its command line must be byte-identical to what
/// it was before seats existed.
#[test]
fn no_role_briefing_leaves_the_invocation_untouched() {
    for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode"] {
        let plain = build_invocation(agent_id, agent_id.to_string(), "hello", None, &[])
            .expect("plain invocation");
        let with_none =
            build_invocation_with_role(agent_id, agent_id.to_string(), "hello", None, &[], None)
                .expect("invocation without a briefing");
        assert_eq!(
            plain, with_none,
            "{agent_id} must be unchanged without a briefing"
        );
    }
}

/// gemini-cli and opencode expose no native channel; the briefing must not be silently dropped, so
/// the builder reports that it could not place it.
#[test]
fn agents_without_a_native_channel_report_that_the_briefing_was_not_placed() {
    for agent_id in ["gemini-cli", "opencode"] {
        let spec = build_invocation_with_role(
            agent_id,
            agent_id.to_string(),
            "hello",
            None,
            &[],
            Some("角色"),
        )
        .expect("invocation");
        assert!(
            !spec.args.iter().any(|argument| argument.contains("角色")),
            "{agent_id} has no native channel, so the briefing must not be forced into args"
        );
    }
}

/// VaneHub's own argv passes `--include-partial-messages`, so a single claude-code turn emits
/// eight `stream_event` envelopes and a `rate_limit_event` alongside the one `assistant` event
/// that carries the reply. None of those envelope types has a parser arm, and the fallback treated
/// any unrecognised line as literal output -- so the persisted assistant message began with
/// `{"type":"stream_event",...}` instead of the answer.
///
/// A line that is valid JSON is a structured event whether or not this parser knows the type;
/// the raw-text fallback exists for output that is not JSON at all.
#[test]
fn claude_code_unrecognised_structured_events_are_not_emitted_as_text() {
    let parser = output_parser_for("claude-code");

    for line in [
        r#"{"type":"stream_event","event":{"type":"message_start","message":{"model":"claude-opus-5","content":[]}},"session_id":"s1","uuid":"u1","ttft_ms":1776}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"PO"}},"session_id":"s1"}"#,
        r#"{"type":"stream_event","event":{"type":"message_stop"},"session_id":"s1"}"#,
        r#"{"type":"rate_limit_event","rate_limit":{"status":"allowed"}}"#,
    ] {
        assert_eq!(
            parser.parse_line(line),
            ProviderOutputEvent::Empty,
            "a structured claude event must not be emitted as literal text: {line}"
        );
    }

    // The turn's actual reply still arrives through the `assistant` event.
    assert_eq!(
        parser.parse_line(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"PONG"}]}}"#
        ),
        ProviderOutputEvent::Token("PONG".to_string())
    );
    // Output that is not JSON at all is still the CLI talking to the user.
    assert_eq!(
        parser.parse_line("plain progress text"),
        ProviderOutputEvent::Token("plain progress text".to_string())
    );
}
