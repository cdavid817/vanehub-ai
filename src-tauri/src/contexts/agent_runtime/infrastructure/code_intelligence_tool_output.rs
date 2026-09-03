use super::tools::{ToolExecutionOutcome, MAX_TOOL_OUTPUT_BYTES};
use crate::contexts::agent_runtime::application::{
    AgentCodeCallRelation, AgentCodeDiagnostic, AgentCodeHover, AgentCodeIntelligenceMetadata,
    AgentCodeIntelligenceOutcome, AgentCodeIntelligenceStatus, AgentCodeLocation, AgentCodeRange,
    AgentCodeSymbol,
};
use serde_json::{json, Value};

const MAX_IDENTITY_BYTES: usize = 128;
const MAX_REASON_BYTES: usize = 128;
const MAX_PREVIEW_BYTES: usize = 512;
const MAX_HOVER_SIGNATURE_BYTES: usize = 1_024;
const MAX_HOVER_DOCUMENTATION_BYTES: usize = 4_096;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4_096;
const MAX_SYMBOL_NAME_BYTES: usize = 256;
const MAX_CALL_SITES: usize = 20;

pub(super) fn locations_outcome(
    key: &str,
    mut outcome: AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>>,
    limit: usize,
) -> ToolExecutionOutcome {
    outcome.metadata.truncated |= outcome.value.as_ref().is_some_and(|locations| {
        locations.iter().any(|location| {
            location
                .preview
                .as_ref()
                .is_some_and(|preview| preview.len() > MAX_PREVIEW_BYTES)
        })
    });
    let mut values = outcome
        .value
        .take()
        .unwrap_or_default()
        .into_iter()
        .take(limit)
        .map(location_json)
        .collect::<Vec<_>>();
    outcome.metadata.truncated |= outcome.metadata.returned_count > values.len();
    bounded_collection(key, &mut outcome.metadata, &mut values)
}

pub(super) fn symbols_outcome(
    mut outcome: AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>>,
    limit: usize,
) -> ToolExecutionOutcome {
    let mut values = outcome
        .value
        .take()
        .unwrap_or_default()
        .into_iter()
        .take(limit)
        .map(symbol_json)
        .collect::<Vec<_>>();
    outcome.metadata.truncated |= outcome.metadata.returned_count > values.len();
    bounded_collection("symbols", &mut outcome.metadata, &mut values)
}

pub(super) fn call_relations_outcome(
    mut outcome: AgentCodeIntelligenceOutcome<Vec<AgentCodeCallRelation>>,
    limit: usize,
) -> ToolExecutionOutcome {
    let mut values = outcome
        .value
        .take()
        .unwrap_or_default()
        .into_iter()
        .take(limit)
        .map(|relation| {
            json!({
                "symbol": symbol_json(relation.symbol),
                "call_sites": relation
                    .call_sites
                    .into_iter()
                    .take(MAX_CALL_SITES)
                    .map(range_json)
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    outcome.metadata.truncated |= outcome.metadata.returned_count > values.len();
    bounded_collection("relations", &mut outcome.metadata, &mut values)
}

pub(super) fn diagnostics_outcome(
    mut outcome: AgentCodeIntelligenceOutcome<Vec<AgentCodeDiagnostic>>,
) -> ToolExecutionOutcome {
    outcome.metadata.truncated |= outcome.value.as_ref().is_some_and(|diagnostics| {
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES
                || diagnostic
                    .source
                    .as_ref()
                    .is_some_and(|source| source.len() > MAX_IDENTITY_BYTES)
                || diagnostic
                    .code
                    .as_ref()
                    .is_some_and(|code| code.len() > MAX_IDENTITY_BYTES)
        })
    });
    let mut values = outcome
        .value
        .take()
        .unwrap_or_default()
        .into_iter()
        .map(diagnostic_json)
        .collect::<Vec<_>>();
    bounded_collection("diagnostics", &mut outcome.metadata, &mut values)
}

pub(super) fn hover_outcome(
    mut outcome: AgentCodeIntelligenceOutcome<Option<AgentCodeHover>>,
) -> ToolExecutionOutcome {
    outcome.metadata.truncated |= outcome.value.as_ref().is_some_and(|hover| {
        hover.as_ref().is_some_and(|hover| {
            hover
                .signature
                .as_ref()
                .is_some_and(|signature| signature.len() > MAX_HOVER_SIGNATURE_BYTES)
                || hover.documentation.as_ref().is_some_and(|documentation| {
                    documentation.len() > MAX_HOVER_DOCUMENTATION_BYTES
                })
        })
    });
    let hover = outcome.value.take().flatten().map(|hover| {
        json!({
            "signature": bounded_option(hover.signature, MAX_HOVER_SIGNATURE_BYTES),
            "documentation": bounded_option(hover.documentation, MAX_HOVER_DOCUMENTATION_BYTES),
            "range": hover.range.map(range_json),
        })
    });
    let value = json!({
        "metadata": metadata_json(&outcome.metadata),
        "hover": hover,
    });
    serialize(value)
}

fn bounded_collection(
    key: &str,
    metadata: &mut AgentCodeIntelligenceMetadata,
    values: &mut Vec<Value>,
) -> ToolExecutionOutcome {
    loop {
        metadata.returned_count = values.len();
        let value = json!({
            "metadata": metadata_json(metadata),
            (key): values,
        });
        let output = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_owned());
        if output.len() <= MAX_TOOL_OUTPUT_BYTES {
            return ToolExecutionOutcome {
                output,
                is_error: false,
            };
        }
        if values.pop().is_none() {
            return ToolExecutionOutcome {
                output: serde_json::to_string(&json!({
                    "metadata": metadata_json(metadata),
                    (key): [],
                }))
                .unwrap_or_else(|_| "{}".to_owned()),
                is_error: false,
            };
        }
        metadata.truncated = true;
    }
}

fn metadata_json(metadata: &AgentCodeIntelligenceMetadata) -> Value {
    json!({
        "status": status_label(metadata.status),
        "server": bounded_ref(metadata.server.as_deref(), MAX_IDENTITY_BYTES),
        "language": bounded_ref(metadata.language.as_deref(), MAX_IDENTITY_BYTES),
        "document_version": metadata.document_version,
        "stale": metadata.stale,
        "returned_count": metadata.returned_count,
        "total": metadata.total,
        "truncated": metadata.truncated,
        "filtered_count": metadata.filtered_count,
        "reason_code": bounded_ref(metadata.reason_code.as_deref(), MAX_REASON_BYTES),
    })
}

fn location_json(location: AgentCodeLocation) -> Value {
    json!({
        "file": location.file,
        "range": range_json(location.range),
        "preview": bounded_option(location.preview, MAX_PREVIEW_BYTES),
    })
}

fn symbol_json(symbol: AgentCodeSymbol) -> Value {
    json!({
        "name": truncate_utf8(&symbol.name, MAX_SYMBOL_NAME_BYTES),
        "kind": truncate_utf8(&symbol.kind, MAX_IDENTITY_BYTES),
        "container": bounded_option(symbol.container, MAX_SYMBOL_NAME_BYTES),
        "file": symbol.file,
        "range": range_json(symbol.range),
        "preview": bounded_option(symbol.preview, MAX_PREVIEW_BYTES),
    })
}

fn diagnostic_json(diagnostic: AgentCodeDiagnostic) -> Value {
    json!({
        "file": diagnostic.file,
        "range": range_json(diagnostic.range),
        "severity": diagnostic.severity,
        "message": truncate_utf8(&diagnostic.message, MAX_DIAGNOSTIC_MESSAGE_BYTES),
        "source": bounded_option(diagnostic.source, MAX_IDENTITY_BYTES),
        "code": bounded_option(diagnostic.code, MAX_IDENTITY_BYTES),
    })
}

fn range_json(range: AgentCodeRange) -> Value {
    json!({
        "start_line": range.start_line,
        "start_column": range.start_column,
        "end_line": range.end_line,
        "end_column": range.end_column,
    })
}

fn serialize(value: Value) -> ToolExecutionOutcome {
    let output = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_owned());
    debug_assert!(output.len() <= MAX_TOOL_OUTPUT_BYTES);
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}

fn bounded_option(value: Option<String>, limit: usize) -> Option<String> {
    value.map(|value| truncate_utf8(&value, limit))
}

fn bounded_ref(value: Option<&str>, limit: usize) -> Option<String> {
    value.map(|value| truncate_utf8(value, limit))
}

fn truncate_utf8(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_owned();
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

const fn status_label(status: AgentCodeIntelligenceStatus) -> &'static str {
    match status {
        AgentCodeIntelligenceStatus::Ready => "ready",
        AgentCodeIntelligenceStatus::Warming => "warming",
        AgentCodeIntelligenceStatus::Timeout => "timeout",
        AgentCodeIntelligenceStatus::Unavailable => "unavailable",
        AgentCodeIntelligenceStatus::Failed => "failed",
    }
}
