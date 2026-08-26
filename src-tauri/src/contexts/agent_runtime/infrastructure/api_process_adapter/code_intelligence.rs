//! The Agent-facing side of the code-intelligence tools: reading a tool call's arguments and
//! routing it to the port. Split out of `native_tools.rs` because nine tools' worth of argument
//! handling is a subject of its own, and that file is already at its recorded line budget.

use super::super::code_intelligence_tool_output::{
    call_relations_outcome, diagnostics_outcome, hover_outcome, locations_outcome, symbols_outcome,
};
use super::super::tools::ToolExecutionOutcome;
use crate::contexts::agent_runtime::application::{
    AgentCallDirection, AgentCallHierarchyInput, AgentCodeIntelligenceContext,
    AgentCodeIntelligencePort, AgentDocumentInput, AgentDocumentPositionInput,
    AgentWorkspaceSymbolInput, FIND_CALL_HIERARCHY_TOOL_NAME, FIND_DEFINITION_TOOL_NAME,
    FIND_IMPLEMENTATIONS_TOOL_NAME, FIND_REFERENCES_TOOL_NAME, FIND_TYPE_DEFINITION_TOOL_NAME,
    FIND_WORKSPACE_SYMBOLS_TOOL_NAME, GET_DIAGNOSTICS_TOOL_NAME, GET_DOCUMENT_SYMBOLS_TOOL_NAME,
    GET_HOVER_TOOL_NAME,
};
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Result caps, matching what the code-intelligence context already truncated to. Restated here
/// because this layer bounds a tool result independently of what produced it.
const MAX_WORKSPACE_SYMBOLS: usize = 50;
const MAX_DOCUMENT_SYMBOLS: usize = 200;
const MAX_CALL_RELATIONS: usize = 50;

pub(super) fn execute_code_intelligence_tool(
    name: &str,
    input: &Value,
    folder: &str,
    cancelled: Arc<AtomicBool>,
    code_intelligence: &dyn AgentCodeIntelligencePort,
) -> ToolExecutionOutcome {
    let relative_path = input
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if relative_path.is_empty() {
        return invalid_code_intelligence_input("path must be a non-empty relative string");
    }
    let context = AgentCodeIntelligenceContext::from_session_workspace(folder);
    if name == GET_DIAGNOSTICS_TOOL_NAME {
        return diagnostics_outcome(code_intelligence.get_diagnostics(
            &context,
            &AgentDocumentInput { relative_path },
            cancelled,
        ));
    }
    if name == GET_DOCUMENT_SYMBOLS_TOOL_NAME {
        return symbols_outcome(
            code_intelligence.get_document_symbols(
                &context,
                &AgentDocumentInput { relative_path },
                cancelled,
            ),
            MAX_DOCUMENT_SYMBOLS,
        );
    }
    if name == FIND_WORKSPACE_SYMBOLS_TOOL_NAME {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if query.is_empty() {
            return invalid_code_intelligence_input("query must be a non-empty string");
        }
        return symbols_outcome(
            code_intelligence.find_workspace_symbols(
                &context,
                &AgentWorkspaceSymbolInput {
                    relative_path,
                    query,
                },
                cancelled,
            ),
            MAX_WORKSPACE_SYMBOLS,
        );
    }
    let Some(line) = one_based_u32(input, "line") else {
        return invalid_code_intelligence_input("line must be a one-based integer");
    };
    let Some(column) = one_based_u32(input, "column") else {
        return invalid_code_intelligence_input("column must be a one-based integer");
    };
    let position = AgentDocumentPositionInput {
        relative_path,
        line,
        column,
    };
    match name {
        FIND_DEFINITION_TOOL_NAME => locations_outcome(
            "definitions",
            code_intelligence.find_definition(&context, &position, cancelled),
            20,
        ),
        FIND_REFERENCES_TOOL_NAME => locations_outcome(
            "references",
            code_intelligence.find_references(&context, &position, cancelled),
            50,
        ),
        GET_HOVER_TOOL_NAME => {
            hover_outcome(code_intelligence.get_hover(&context, &position, cancelled))
        }
        FIND_TYPE_DEFINITION_TOOL_NAME => locations_outcome(
            "definitions",
            code_intelligence.find_type_definition(&context, &position, cancelled),
            20,
        ),
        FIND_IMPLEMENTATIONS_TOOL_NAME => locations_outcome(
            "definitions",
            code_intelligence.find_implementations(&context, &position, cancelled),
            20,
        ),
        FIND_CALL_HIERARCHY_TOOL_NAME => {
            // Anything that is not "outgoing" reads as the default rather than as an error: the
            // choice is between two directions, and refusing a typo would cost a whole tool call
            // to say what the default already says.
            let direction = match input.get("direction").and_then(Value::as_str) {
                Some("outgoing") => AgentCallDirection::Outgoing,
                _ => AgentCallDirection::Incoming,
            };
            call_relations_outcome(
                code_intelligence.find_call_hierarchy(
                    &context,
                    &AgentCallHierarchyInput {
                        position,
                        direction,
                    },
                    cancelled,
                ),
                MAX_CALL_RELATIONS,
            )
        }
        _ => invalid_code_intelligence_input("unsupported code-intelligence operation"),
    }
}

fn one_based_u32(input: &Value, field: &str) -> Option<u32> {
    let value = input.get(field)?.as_u64()?;
    (value > 0).then(|| u32::try_from(value).ok()).flatten()
}

fn invalid_code_intelligence_input(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_owned(),
        is_error: true,
    }
}
