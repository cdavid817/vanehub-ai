use crate::contexts::agent_runtime::domain::{
    classify_components, ContextComponent, ContextRound, ProtocolState, RetentionClass,
    SemanticClass,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextWireShape {
    Anthropic,
    OpenAiCompatible,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedContextProjection {
    pub(crate) request_fingerprint: String,
    pub(crate) characters: u64,
    pub(crate) components: Vec<ContextComponent>,
    pub(crate) rounds: Vec<ContextRound>,
    pub(crate) token_estimate_complete: bool,
    pub(crate) overflow_count: u32,
}

pub(crate) fn project_request(body: &Value, shape: ContextWireShape) -> PreparedContextProjection {
    let mut components = Vec::new();
    if shape == ContextWireShape::Anthropic {
        if let Some(system) = body.get("system") {
            push_component(
                &mut components,
                SemanticClass::SystemInstruction,
                None,
                system,
                None,
            );
        }
    }
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            push_component(&mut components, SemanticClass::ToolSchema, None, tool, None);
        }
    }
    let mut rounds = project_messages(body, shape, &mut components);
    mark_protocol_state(&mut rounds, &components);
    mark_current_user(&mut components);
    mark_repeated_tool_results(&mut components);
    classify_components(&mut components, &rounds);
    let characters = character_count(body);
    let covered = components.iter().fold(0_u64, |total, component| {
        total.saturating_add(component.characters)
    });
    if covered < characters {
        let remainder = characters - covered;
        let value = Value::String(format!("envelope:{remainder}"));
        push_component(&mut components, SemanticClass::Unknown, None, &value, None);
        if let Some(component) = components.last_mut() {
            component.characters = remainder;
            component.estimated_tokens = Some(estimate_tokens(remainder, remainder));
        }
    }
    PreparedContextProjection {
        request_fingerprint: fingerprint(body),
        characters,
        components,
        rounds,
        token_estimate_complete: !contains_unknown_native_block(body, shape),
        overflow_count: 0,
    }
}

fn project_messages(
    body: &Value,
    shape: ContextWireShape,
    components: &mut Vec<ContextComponent>,
) -> Vec<ContextRound> {
    let Some(messages) = body.get("messages").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut rounds = Vec::new();
    let mut round_index = 0;
    for message in messages {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "system" {
            push_component(
                components,
                SemanticClass::SystemInstruction,
                None,
                message,
                None,
            );
            continue;
        }
        let current_has_assistant = components.iter().any(|component| {
            component.round == Some(round_index)
                && component.semantic_class == SemanticClass::AssistantResponse
        });
        let tool_result_user = shape == ContextWireShape::Anthropic
            && role == "user"
            && message
                .get("content")
                .and_then(Value::as_array)
                .is_some_and(|blocks| {
                    !blocks.is_empty()
                        && blocks.iter().all(|block| {
                            block.get("type").and_then(Value::as_str) == Some("tool_result")
                        })
                });
        if !rounds.is_empty()
            && ((role == "assistant" && current_has_assistant)
                || (role == "user" && current_has_assistant && !tool_result_user))
        {
            round_index += 1;
        }
        while rounds.len() <= round_index {
            rounds.push(ContextRound {
                index: rounds.len(),
                protocol_state: ProtocolState::Complete,
                component_sequences: Vec::new(),
            });
        }
        project_message(message, role, round_index, shape, components, &mut rounds);
    }
    rounds
}

fn project_message(
    message: &Value,
    role: &str,
    round: usize,
    shape: ContextWireShape,
    components: &mut Vec<ContextComponent>,
    rounds: &mut [ContextRound],
) {
    let anthropic_tool_results = shape == ContextWireShape::Anthropic
        && role == "user"
        && message
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|blocks| {
                !blocks.is_empty()
                    && blocks.iter().all(|block| {
                        block.get("type").and_then(Value::as_str) == Some("tool_result")
                    })
            });
    let class = match role {
        "user" => SemanticClass::UserIntent,
        "assistant" => SemanticClass::AssistantResponse,
        "tool" => SemanticClass::ToolResult,
        _ => SemanticClass::Unknown,
    };
    if !anthropic_tool_results {
        let base_message = message_without_projected_blocks(message, shape);
        push_component(
            components,
            class,
            Some(round),
            &base_message,
            tool_result_id(message, role),
        );
        rounds[round].component_sequences.push(components.len() - 1);
    }
    if shape == ContextWireShape::Anthropic {
        project_anthropic_blocks(message, round, components, rounds);
    } else {
        project_openai_calls(message, round, components, rounds);
    }
}

pub(crate) fn message_without_projected_blocks(message: &Value, shape: ContextWireShape) -> Value {
    let mut base = message.clone();
    let Some(map) = base.as_object_mut() else {
        return base;
    };
    if shape == ContextWireShape::OpenAiCompatible {
        map.remove("tool_calls");
        return base;
    }
    if let Some(blocks) = map.get_mut("content").and_then(Value::as_array_mut) {
        blocks.retain(|block| {
            !matches!(
                block.get("type").and_then(Value::as_str),
                Some("tool_use" | "tool_result" | "image" | "document")
            )
        });
    }
    base
}

fn project_anthropic_blocks(
    message: &Value,
    round: usize,
    components: &mut Vec<ContextComponent>,
    rounds: &mut [ContextRound],
) {
    let Some(blocks) = message.get("content").and_then(Value::as_array) else {
        return;
    };
    for block in blocks {
        let (class, reference) = match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => (
                SemanticClass::ToolRequest,
                block.get("id").and_then(Value::as_str),
            ),
            Some("tool_result") => (
                SemanticClass::ToolResult,
                block.get("tool_use_id").and_then(Value::as_str),
            ),
            Some("image") | Some("document") => (SemanticClass::Attachment, None),
            _ => continue,
        };
        push_component(components, class, Some(round), block, reference);
        rounds[round].component_sequences.push(components.len() - 1);
    }
}

fn project_openai_calls(
    message: &Value,
    round: usize,
    components: &mut Vec<ContextComponent>,
    rounds: &mut [ContextRound],
) {
    let Some(calls) = message.get("tool_calls").and_then(Value::as_array) else {
        return;
    };
    for call in calls {
        push_component(
            components,
            SemanticClass::ToolRequest,
            Some(round),
            call,
            call.get("id").and_then(Value::as_str),
        );
        rounds[round].component_sequences.push(components.len() - 1);
    }
}

fn push_component(
    components: &mut Vec<ContextComponent>,
    semantic_class: SemanticClass,
    round: Option<usize>,
    value: &Value,
    tool_reference: Option<&str>,
) {
    let characters = character_count(value);
    let bytes = serde_json::to_vec(value).map_or(characters, |bytes| bytes.len() as u64);
    components.push(ContextComponent {
        sequence: components.len(),
        semantic_class,
        retention_class: RetentionClass::Protected,
        round,
        characters,
        estimated_tokens: Some(estimate_tokens(characters, bytes)),
        content_fingerprint: fingerprint(value),
        tool_reference: tool_reference.map(fingerprint_text),
        current_user_intent: false,
        correction: false,
        reinjectable: false,
        repeated_tool_result: false,
    });
}

fn mark_protocol_state(rounds: &mut [ContextRound], components: &[ContextComponent]) {
    for round in rounds {
        let members = components
            .iter()
            .filter(|component| component.round == Some(round.index));
        let requests: Vec<_> = members
            .clone()
            .filter(|component| component.semantic_class == SemanticClass::ToolRequest)
            .filter_map(|component| component.tool_reference.as_ref())
            .collect();
        let results: Vec<_> = members
            .filter(|component| component.semantic_class == SemanticClass::ToolResult)
            .filter_map(|component| component.tool_reference.as_ref())
            .collect();
        let request_set: HashSet<_> = requests.iter().copied().collect();
        let result_set: HashSet<_> = results.iter().copied().collect();
        if request_set != result_set
            || request_set.len() != requests.len()
            || result_set.len() != results.len()
        {
            round.protocol_state = ProtocolState::Incomplete;
        }
    }
}

fn mark_current_user(components: &mut [ContextComponent]) {
    if let Some(component) = components
        .iter_mut()
        .rev()
        .find(|component| component.semantic_class == SemanticClass::UserIntent)
    {
        component.current_user_intent = true;
    }
}

fn mark_repeated_tool_results(components: &mut [ContextComponent]) {
    let mut counts = HashMap::new();
    for component in components
        .iter()
        .filter(|component| component.semantic_class == SemanticClass::ToolResult)
    {
        *counts
            .entry(component.content_fingerprint.clone())
            .or_insert(0_u32) += 1;
    }
    for component in components.iter_mut() {
        component.repeated_tool_result = component.semantic_class == SemanticClass::ToolResult
            && counts
                .get(&component.content_fingerprint)
                .copied()
                .unwrap_or(0)
                > 1;
    }
}

pub(crate) fn character_count(value: &Value) -> u64 {
    match value {
        Value::String(text) => text.chars().count() as u64,
        Value::Array(values) => values
            .iter()
            .map(character_count)
            .fold(0, u64::saturating_add),
        Value::Object(map) => map
            .values()
            .map(character_count)
            .fold(0, u64::saturating_add),
        _ => 0,
    }
}

fn estimate_tokens(characters: u64, bytes: u64) -> u64 {
    let ascii_weight = characters.saturating_mul(3).saturating_add(11) / 12;
    let unicode_weight = bytes.saturating_sub(characters).saturating_add(2) / 3;
    ascii_weight
        .saturating_add(unicode_weight)
        .saturating_add(4)
}

pub(crate) fn fingerprint(value: &Value) -> String {
    fingerprint_text(&serde_json::to_string(value).unwrap_or_default())
}

fn fingerprint_text(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn tool_result_id<'a>(message: &'a Value, role: &str) -> Option<&'a str> {
    (role == "tool")
        .then(|| message.get("tool_call_id").and_then(Value::as_str))
        .flatten()
}

fn contains_unknown_native_block(body: &Value, shape: ContextWireShape) -> bool {
    body.get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|message| message.get("content").and_then(Value::as_array))
        .flatten()
        .filter_map(|block| block.get("type").and_then(Value::as_str))
        .any(|kind| match shape {
            ContextWireShape::Anthropic => !matches!(
                kind,
                "text" | "tool_use" | "tool_result" | "image" | "document"
            ),
            ContextWireShape::OpenAiCompatible => {
                !matches!(kind, "text" | "input_text" | "image_url")
            }
        })
}
