#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use super::context_projection::{fingerprint, message_without_projected_blocks, ContextWireShape};
use crate::contexts::agent_runtime::application::ReinjectedContextValue;
use crate::contexts::agent_runtime::domain::{
    parse_structured_summary, ContextOptimizationPlan, OptimizationActionKind, OptimizationTarget,
    SafeFingerprint, SummaryBoundary, ToolResultReplacement, STRUCTURED_SUMMARY_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextReductionError {
    InvalidRequestShape,
    InvalidSummaryBoundary,
    InvalidToolReference,
    InvalidStructuredSummary,
    MissingStructuredSummary,
    UnmatchedAction,
}

pub(crate) fn reconstruct_candidate(
    original: &Value,
    shape: ContextWireShape,
    plan: &ContextOptimizationPlan,
    structured_summary: Option<&str>,
    reinjections: &[ReinjectedContextValue],
) -> Result<Value, ContextReductionError> {
    let mut candidate = apply_low_cost_reductions(original, shape, plan)?;
    let mut synthetic_parts = Vec::new();
    if let Some(boundary) = plan.summary_boundary.as_ref() {
        let summary = structured_summary.ok_or(ContextReductionError::MissingStructuredSummary)?;
        parse_structured_summary(summary)
            .map_err(|_| ContextReductionError::InvalidStructuredSummary)?;
        let selected = build_structured_summary_turns(original, shape, boundary)?;
        remove_selected_prefix(&mut candidate, selected.len())?;
        synthetic_parts.push(format!(
            "[OnePiece structured continuation summary: {STRUCTURED_SUMMARY_VERSION}]\n{summary}"
        ));
    } else if structured_summary.is_some() {
        return Err(ContextReductionError::InvalidStructuredSummary);
    }
    for value in reinjections {
        synthetic_parts.push(format!(
            "<onepiece-reinjected kind=\"{}\" revision=\"{}\" source=\"{}\">\n{}\n</onepiece-reinjected>",
            value.evidence.kind,
            value.evidence.revision,
            value.evidence.source_fingerprint,
            value.content,
        ));
    }
    if !synthetic_parts.is_empty() {
        insert_synthetic_context(&mut candidate, synthetic_parts.join("\n\n"))?;
    }
    Ok(candidate)
}

pub(crate) fn build_structured_summary_turns(
    original: &Value,
    shape: ContextWireShape,
    boundary: &SummaryBoundary,
) -> Result<Vec<Value>, ContextReductionError> {
    if boundary.first_round != 0
        || boundary.last_round < boundary.first_round
        || boundary.round_count as usize != boundary.last_round.saturating_add(1)
    {
        return Err(ContextReductionError::InvalidSummaryBoundary);
    }
    let messages = original
        .get("messages")
        .and_then(Value::as_array)
        .ok_or(ContextReductionError::InvalidRequestShape)?;
    let mut selected = Vec::new();
    let mut round = 0_usize;
    let mut current_has_assistant = false;
    for message in messages {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "system" {
            continue;
        }
        let tool_result_user = is_anthropic_tool_result_message(message, shape);
        if !selected.is_empty()
            && ((role == "assistant" && current_has_assistant)
                || (role == "user" && current_has_assistant && !tool_result_user))
        {
            round = round.saturating_add(1);
            current_has_assistant = false;
        }
        if round > boundary.last_round {
            break;
        }
        let mut sanitized = message.clone();
        strip_internal_generation_content(&mut sanitized);
        selected.push(sanitized);
        current_has_assistant |= role == "assistant";
    }
    if selected.is_empty() || round < boundary.last_round {
        return Err(ContextReductionError::InvalidSummaryBoundary);
    }
    Ok(selected)
}

pub(crate) fn apply_low_cost_reductions(
    original: &Value,
    shape: ContextWireShape,
    plan: &ContextOptimizationPlan,
) -> Result<Value, ContextReductionError> {
    let selected: HashMap<_, _> =
        plan.actions
            .iter()
            .filter_map(|action| match action.target {
                OptimizationTarget::Component(sequence)
                    if action.kind != OptimizationActionKind::SummarizeRound =>
                {
                    action.source_fingerprints.first().map(|fingerprint| {
                        (fingerprint.as_str().to_owned(), (sequence, action.kind))
                    })
                }
                _ => None,
            })
            .collect();
    let mut candidate = original.clone();
    let messages = candidate
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or(ContextReductionError::InvalidRequestShape)?;
    let mut applied = HashSet::new();
    let mut retained = Vec::with_capacity(messages.len());
    for mut message in messages.drain(..) {
        let base_fingerprint = fingerprint(&message_without_projected_blocks(&message, shape));
        if let Some((sequence, kind)) = selected.get(&base_fingerprint).copied() {
            match kind {
                OptimizationActionKind::DiscardTransient
                | OptimizationActionKind::ReplaceReinjectable => {
                    applied.insert(sequence);
                    continue;
                }
                OptimizationActionKind::MicrocompactToolResult
                    if shape == ContextWireShape::OpenAiCompatible =>
                {
                    replace_openai_tool_result(&mut message, &base_fingerprint)?;
                    applied.insert(sequence);
                }
                _ => {}
            }
        }
        if shape == ContextWireShape::Anthropic {
            apply_anthropic_blocks(&mut message, &selected, &mut applied)?;
        }
        retained.push(message);
    }
    *messages = retained;
    if applied.len() != selected.len() {
        return Err(ContextReductionError::UnmatchedAction);
    }
    Ok(candidate)
}

fn apply_anthropic_blocks(
    message: &mut Value,
    selected: &HashMap<String, (usize, OptimizationActionKind)>,
    applied: &mut HashSet<usize>,
) -> Result<(), ContextReductionError> {
    let Some(blocks) = message.get_mut("content").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    let mut retained = Vec::with_capacity(blocks.len());
    for mut block in blocks.drain(..) {
        let source_fingerprint = fingerprint(&block);
        let Some((sequence, kind)) = selected.get(&source_fingerprint).copied() else {
            retained.push(block);
            continue;
        };
        match kind {
            OptimizationActionKind::DiscardTransient
            | OptimizationActionKind::ReplaceReinjectable => {
                applied.insert(sequence);
            }
            OptimizationActionKind::MicrocompactToolResult => {
                replace_anthropic_tool_result(&mut block, &source_fingerprint)?;
                applied.insert(sequence);
                retained.push(block);
            }
            OptimizationActionKind::SummarizeRound => retained.push(block),
        }
    }
    *blocks = retained;
    Ok(())
}

fn replace_anthropic_tool_result(
    block: &mut Value,
    source_fingerprint: &str,
) -> Result<(), ContextReductionError> {
    let tool_reference = block
        .get("tool_use_id")
        .and_then(Value::as_str)
        .ok_or(ContextReductionError::InvalidToolReference)?;
    let failed = block
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replacement = replacement(tool_reference, failed, source_fingerprint)?;
    *block = json!({
        "type": "tool_result",
        "tool_use_id": replacement.tool_reference,
        "is_error": replacement.outcome == crate::contexts::agent_runtime::domain::ToolResultOutcome::Failed,
        "content": replacement.marker(),
    });
    Ok(())
}

fn replace_openai_tool_result(
    message: &mut Value,
    source_fingerprint: &str,
) -> Result<(), ContextReductionError> {
    let tool_reference = message
        .get("tool_call_id")
        .and_then(Value::as_str)
        .ok_or(ContextReductionError::InvalidToolReference)?;
    let failed = message
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("error"));
    let replacement = replacement(tool_reference, failed, source_fingerprint)?;
    let map = message
        .as_object_mut()
        .ok_or(ContextReductionError::InvalidRequestShape)?;
    map.insert("content".to_string(), Value::String(replacement.marker()));
    Ok(())
}

fn replacement(
    tool_reference: &str,
    failed: bool,
    source_fingerprint: &str,
) -> Result<ToolResultReplacement, ContextReductionError> {
    let fingerprint = SafeFingerprint::parse(source_fingerprint)
        .ok_or(ContextReductionError::InvalidToolReference)?;
    ToolResultReplacement::new(tool_reference, failed, fingerprint)
        .ok_or(ContextReductionError::InvalidToolReference)
}

fn remove_selected_prefix(
    candidate: &mut Value,
    selected_count: usize,
) -> Result<(), ContextReductionError> {
    let messages = candidate
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or(ContextReductionError::InvalidRequestShape)?;
    let mut removed = 0_usize;
    messages.retain(|message| {
        if removed < selected_count && message.get("role").and_then(Value::as_str) != Some("system")
        {
            removed = removed.saturating_add(1);
            false
        } else {
            true
        }
    });
    (removed == selected_count)
        .then_some(())
        .ok_or(ContextReductionError::InvalidSummaryBoundary)
}

fn insert_synthetic_context(
    candidate: &mut Value,
    content: String,
) -> Result<(), ContextReductionError> {
    let messages = candidate
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or(ContextReductionError::InvalidRequestShape)?;
    let insertion = messages
        .iter()
        .take_while(|message| message.get("role").and_then(Value::as_str) == Some("system"))
        .count();
    messages.insert(insertion, json!({ "role": "user", "content": content }));
    Ok(())
}

fn is_anthropic_tool_result_message(message: &Value, shape: ContextWireShape) -> bool {
    shape == ContextWireShape::Anthropic
        && message.get("role").and_then(Value::as_str) == Some("user")
        && message
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|blocks| {
                !blocks.is_empty()
                    && blocks.iter().all(|block| {
                        block.get("type").and_then(Value::as_str) == Some("tool_result")
                    })
            })
}

fn strip_internal_generation_content(message: &mut Value) {
    let Some(map) = message.as_object_mut() else {
        return;
    };
    map.remove("thinking");
    map.remove("reasoning");
    map.remove("reasoning_content");
    if let Some(blocks) = map.get_mut("content").and_then(Value::as_array_mut) {
        blocks.retain(|block| {
            !matches!(
                block.get("type").and_then(Value::as_str),
                Some("thinking" | "redacted_thinking")
            )
        });
    }
}
