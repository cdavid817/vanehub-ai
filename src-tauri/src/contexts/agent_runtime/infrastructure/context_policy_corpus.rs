#![allow(dead_code)]

use super::context_projection::ContextWireShape;
use crate::contexts::agent_runtime::application::ContextReinjectionKind;
use serde_json::{json, Value};

pub(crate) const CONTEXT_POLICY_CORPUS_VERSION: &str = "onepiece-context-regression-corpus-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CorpusAdjustment {
    None,
    MarkDiscardable,
    MarkReinjectable,
    TokensUnavailable,
    ArithmeticBoundary,
}

#[derive(Debug, Clone)]
pub(crate) struct ContextPolicyRegressionCase {
    pub(crate) id: &'static str,
    pub(crate) shape: ContextWireShape,
    pub(crate) body: Value,
    pub(crate) adjustment: CorpusAdjustment,
    pub(crate) reinjection_kind: Option<ContextReinjectionKind>,
}

pub(crate) fn context_policy_regression_corpus() -> Vec<ContextPolicyRegressionCase> {
    vec![
        ContextPolicyRegressionCase {
            id: "protocol-rounds",
            shape: ContextWireShape::OpenAiCompatible,
            body: protocol_rounds(),
            adjustment: CorpusAdjustment::None,
            reinjection_kind: None,
        },
        ContextPolicyRegressionCase {
            id: "retention-classes",
            shape: ContextWireShape::OpenAiCompatible,
            body: ordinary_rounds("retention", 1_600),
            adjustment: CorpusAdjustment::MarkDiscardable,
            reinjection_kind: None,
        },
        ContextPolicyRegressionCase {
            id: "reinjection",
            shape: ContextWireShape::OpenAiCompatible,
            body: ordinary_rounds("reinjection", 1_600),
            adjustment: CorpusAdjustment::MarkReinjectable,
            reinjection_kind: Some(ContextReinjectionKind::Memory),
        },
        ContextPolicyRegressionCase {
            id: "large-tool-result",
            shape: ContextWireShape::OpenAiCompatible,
            body: large_tool_result(),
            adjustment: CorpusAdjustment::None,
            reinjection_kind: None,
        },
        ContextPolicyRegressionCase {
            id: "multilingual-sizes",
            shape: ContextWireShape::Anthropic,
            body: multilingual_rounds(),
            adjustment: CorpusAdjustment::None,
            reinjection_kind: None,
        },
        ContextPolicyRegressionCase {
            id: "unavailable-tokens",
            shape: ContextWireShape::OpenAiCompatible,
            body: ordinary_rounds("characters-only", 1_600),
            adjustment: CorpusAdjustment::TokensUnavailable,
            reinjection_kind: None,
        },
        ContextPolicyRegressionCase {
            id: "arithmetic-boundary",
            shape: ContextWireShape::OpenAiCompatible,
            body: ordinary_rounds("saturation", 128),
            adjustment: CorpusAdjustment::ArithmeticBoundary,
            reinjection_kind: None,
        },
    ]
}

fn fixture_text(label: &str, characters: usize) -> String {
    let marker = format!("[fixture-{label}]");
    marker.repeat(characters.div_ceil(marker.len()))
}

fn ordinary_rounds(label: &str, characters: usize) -> Value {
    json!({
        "model": "fixture-model",
        "messages": [
            { "role": "system", "content": "[fixture-system-policy]" },
            { "role": "user", "content": fixture_text(label, characters) },
            { "role": "assistant", "content": fixture_text("assistant", characters) },
            { "role": "user", "content": "[fixture-current-intent]" }
        ]
    })
}

fn protocol_rounds() -> Value {
    json!({
        "model": "fixture-model",
        "messages": [
            { "role": "system", "content": "[fixture-system-policy]" },
            { "role": "user", "content": fixture_text("protocol-request", 1_200) },
            {
                "role": "assistant",
                "content": "[fixture-tool-request]",
                "tool_calls": [{
                    "id": "fixture-call-1",
                    "type": "function",
                    "function": { "name": "fixture_read", "arguments": "{}" }
                }]
            },
            { "role": "tool", "tool_call_id": "fixture-call-1", "content": fixture_text("protocol-result", 1_200) },
            { "role": "assistant", "content": fixture_text("protocol-answer", 1_200) },
            { "role": "user", "content": "[fixture-current-intent]" }
        ]
    })
}

fn large_tool_result() -> Value {
    json!({
        "model": "fixture-model",
        "messages": [
            { "role": "system", "content": "[fixture-system-policy]" },
            { "role": "user", "content": "[fixture-tool-intent]" },
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "fixture-large-call",
                    "type": "function",
                    "function": { "name": "fixture_read", "arguments": "{}" }
                }]
            },
            { "role": "tool", "tool_call_id": "fixture-large-call", "content": fixture_text("large-result", 8_000) },
            { "role": "assistant", "content": "[fixture-tool-complete]" },
            { "role": "user", "content": "[fixture-current-intent]" }
        ]
    })
}

fn multilingual_rounds() -> Value {
    json!({
        "model": "fixture-model",
        "system": "[fixture-system-policy]",
        "messages": [
            { "role": "user", "content": [{ "type": "text", "text": fixture_text("中文上下文", 1_600) }] },
            { "role": "assistant", "content": [{ "type": "text", "text": fixture_text("日本語맥락", 1_600) }] },
            { "role": "user", "content": [{ "type": "text", "text": "[fixture-current-intent]" }] }
        ]
    })
}
