use super::api_process_adapter::context_snapshot_diagnostic;
use super::context_projection::{character_count, project_request, ContextWireShape};
use crate::contexts::agent_runtime::application::{ContextAnalysisInput, ContextAnalysisService};
use crate::contexts::agent_runtime::domain::{ProtocolState, RetentionClass, SemanticClass};
use serde_json::json;

#[test]
fn groups_plain_and_consecutive_assistant_messages_into_safe_rounds() {
    let body = json!({
        "model": "test",
        "messages": [
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "answer"},
            {"role": "assistant", "content": "continued"}
        ]
    });
    let projected = project_request(&body, ContextWireShape::OpenAiCompatible);
    assert_eq!(projected.rounds.len(), 2);
    assert_eq!(projected.components[0].round, Some(0));
    assert_eq!(projected.components[1].round, Some(0));
    assert_eq!(projected.components[2].round, Some(1));
}

#[test]
fn keeps_multiple_tool_pairs_atomic_and_marks_protocol_damage() {
    let complete = json!({
        "messages": [
            {"role": "assistant", "content": null, "tool_calls": [
                {"id": "a", "type": "function", "function": {"name": "read", "arguments": "{}"}},
                {"id": "b", "type": "function", "function": {"name": "read", "arguments": "{}"}}
            ]},
            {"role": "tool", "tool_call_id": "a", "content": "one"},
            {"role": "tool", "tool_call_id": "b", "content": "two"}
        ]
    });
    let projection = project_request(&complete, ContextWireShape::OpenAiCompatible);
    assert_eq!(projection.rounds.len(), 1);
    assert_eq!(projection.rounds[0].protocol_state, ProtocolState::Complete);

    for malformed in [
        json!({"messages": [{"role":"assistant","tool_calls":[{"id":"a"}]}]}),
        json!({"messages": [{"role":"tool","tool_call_id":"orphan","content":"x"}]}),
        json!({"messages": [
            {"role":"assistant","tool_calls":[{"id":"a"},{"id":"a"}]},
            {"role":"tool","tool_call_id":"a","content":"x"}
        ]}),
    ] {
        let projection = project_request(&malformed, ContextWireShape::OpenAiCompatible);
        assert_eq!(
            projection.rounds[0].protocol_state,
            ProtocolState::Incomplete
        );
        assert!(projection
            .components
            .iter()
            .all(|component| component.retention_class == RetentionClass::Protected));
    }
}

#[test]
fn equivalent_wire_requests_have_equivalent_semantic_tool_protocol() {
    let anthropic = json!({
        "model": "m", "system": "safe", "tools": [{"name":"read","input_schema":{}}],
        "messages": [
            {"role":"user","content":"go"},
            {"role":"assistant","content":[{"type":"tool_use","id":"call","name":"read","input":{}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"call","content":"ok"}]}
        ]
    });
    let openai = json!({
        "model": "m", "tools": [{"type":"function","function":{"name":"read","parameters":{}}}],
        "messages": [
            {"role":"system","content":"safe"},
            {"role":"user","content":"go"},
            {"role":"assistant","content":null,"tool_calls":[{"id":"call","type":"function","function":{"name":"read","arguments":"{}"}}]},
            {"role":"tool","tool_call_id":"call","content":"ok"}
        ]
    });
    for projection in [
        project_request(&anthropic, ContextWireShape::Anthropic),
        project_request(&openai, ContextWireShape::OpenAiCompatible),
    ] {
        assert_eq!(
            projection
                .components
                .iter()
                .map(|component| component.characters)
                .sum::<u64>(),
            projection.characters
        );
        assert_eq!(projection.rounds.len(), 1);
        assert_eq!(projection.rounds[0].protocol_state, ProtocolState::Complete);
        for class in [
            SemanticClass::SystemInstruction,
            SemanticClass::ToolSchema,
            SemanticClass::UserIntent,
            SemanticClass::AssistantResponse,
            SemanticClass::ToolRequest,
            SemanticClass::ToolResult,
        ] {
            assert!(projection
                .components
                .iter()
                .any(|component| component.semantic_class == class));
        }
        assert!(projection.components.iter().all(|component| {
            component.round.is_some()
                || matches!(
                    component.semantic_class,
                    SemanticClass::SystemInstruction
                        | SemanticClass::ToolSchema
                        | SemanticClass::Unknown
                )
        }));
    }
}

#[test]
fn estimator_is_deterministic_monotonic_multilingual_and_degrades_unknown_blocks() {
    let short = json!({"messages":[{"role":"user","content":"hello 世界"}]});
    let long = json!({"messages":[{"role":"user","content":"hello 世界 and more text"}]});
    let first = project_request(&short, ContextWireShape::OpenAiCompatible);
    let repeated = project_request(&short, ContextWireShape::OpenAiCompatible);
    let larger = project_request(&long, ContextWireShape::OpenAiCompatible);
    assert_eq!(first.request_fingerprint, repeated.request_fingerprint);
    assert_eq!(
        short,
        json!({"messages":[{"role":"user","content":"hello 世界"}]})
    );
    assert_eq!(first.characters, character_count(&short));
    assert!(larger.characters > first.characters);
    assert!(
        larger
            .components
            .iter()
            .map(|part| part.estimated_tokens.unwrap_or(0))
            .sum::<u64>()
            >= first
                .components
                .iter()
                .map(|part| part.estimated_tokens.unwrap_or(0))
                .sum::<u64>()
    );

    let unknown =
        json!({"messages":[{"role":"user","content":[{"type":"future_block","data":"x"}]}]});
    assert!(!project_request(&unknown, ContextWireShape::OpenAiCompatible).token_estimate_complete);
}

#[test]
fn diagnostics_are_bounded_and_never_project_request_content() {
    let body = json!({
        "model": "secret-model-payload",
        "messages": [{"role":"user","content":"PROMPT_SECRET"}],
        "tools": [{"type":"function","function":{"name":"tool","parameters":{"secret":"TOOL_SECRET"}}}],
        "authorization": "CREDENTIAL_SECRET",
        "raw_frame": "FRAME_SECRET"
    });
    let projection = project_request(&body, ContextWireShape::OpenAiCompatible);
    let snapshot = ContextAnalysisService::analyze(
        ContextAnalysisInput {
            provider_id: None,
            model_id: "custom".to_string(),
            request_fingerprint: projection.request_fingerprint,
            characters: projection.characters,
            components: projection.components,
            rounds: projection.rounds,
            token_estimate_complete: projection.token_estimate_complete,
            capacity: None,
            active_character_compaction: false,
            invocation_sequence: 0,
            overflow_count: projection.overflow_count,
        },
        None,
    );
    let diagnostic = context_snapshot_diagnostic(0, &snapshot);
    assert!(diagnostic.len() < 2_000);
    for secret in [
        "PROMPT_SECRET",
        "TOOL_SECRET",
        "CREDENTIAL_SECRET",
        "FRAME_SECRET",
        "secret-model-payload",
    ] {
        assert!(!diagnostic.contains(secret));
    }
}
