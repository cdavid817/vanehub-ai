use super::{
    CanonicalToolResource, NativeToolDefinition, NativeToolExecutionContext,
    NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPortRequest,
    NativeToolReadinessReasonCode, NativeToolResultEnvelope, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput, WebResearchPort,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::web_handler_input::{
    canonical_web_url, input_hash, invalid_fetch_input, invalid_search_input, optional_string,
    optional_u64, reject_unknown_fields, required_string,
};

pub(crate) struct WebSearchNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn WebResearchPort>,
}

pub(crate) fn web_native_tool_handlers(
    port: Arc<dyn WebResearchPort>,
) -> Vec<Arc<dyn NativeToolHandler>> {
    vec![
        Arc::new(WebSearchNativeToolHandler::new(port.clone())),
        Arc::new(WebFetchNativeToolHandler::new(port)),
    ]
}

impl WebSearchNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn WebResearchPort>) -> Self {
        Self {
            definition: search_definition(),
            port,
        }
    }
}

impl NativeToolHandler for WebSearchNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        web_eligibility(context, "web_search")
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_search_input)?;
        reject_unknown_fields(object, &["query", "locale", "safe_search", "count"])?;
        let query = required_string(object, "query", invalid_search_input)?;
        if query.trim() != query || query.is_empty() || query.chars().count() > 500 {
            return Err(invalid_search_input());
        }
        optional_string(object, "locale", 16, invalid_search_input)?;
        if let Some(value) = object.get("safe_search") {
            if !matches!(value.as_str(), Some("strict" | "moderate" | "off")) {
                return Err(invalid_search_input());
            }
        }
        optional_u64(object, "count", 1, 10, invalid_search_input)?;
        let hash = input_hash(input, invalid_search_input)?;
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: hash.clone(),
            operation: NativeToolOperation::WebSearch,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::WebUrl,
                canonical_id: format!("web-search/{hash}"),
                attributes: BTreeMap::new(),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        permission(input)
    }

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        self.port
            .execute_web(NativeToolPortRequest { input, context })
    }
}

pub(crate) struct WebFetchNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn WebResearchPort>,
}

impl WebFetchNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn WebResearchPort>) -> Self {
        Self {
            definition: fetch_definition(),
            port,
        }
    }
}

impl NativeToolHandler for WebFetchNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        web_eligibility(context, "web_fetch")
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_fetch_input)?;
        reject_unknown_fields(object, &["url", "max_text_chars", "persist_binary"])?;
        let raw_url = required_string(object, "url", invalid_fetch_input)?;
        let canonical_url = canonical_web_url(raw_url)?;
        optional_u64(object, "max_text_chars", 1, 100_000, invalid_fetch_input)?;
        if object
            .get("persist_binary")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err(invalid_fetch_input());
        }
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: input_hash(input, invalid_fetch_input)?,
            operation: NativeToolOperation::WebFetch,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::WebUrl,
                canonical_id: canonical_url,
                attributes: BTreeMap::new(),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        permission(input)
    }

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        self.port
            .execute_web(NativeToolPortRequest { input, context })
    }
}

fn web_eligibility(context: &ToolEligibilityContext, readiness_key: &str) -> ToolEligibility {
    if context.execution_mode == NativeToolExecutionMode::Plan {
        return ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::PolicyUnavailable,
        };
    }
    match context.readiness.get(readiness_key) {
        Some(None) => ToolEligibility::Eligible,
        Some(Some(reason)) => ToolEligibility::Ineligible { reason: *reason },
        None => ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::BackendUnavailable,
        },
    }
}

fn search_definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "web_search".to_string(),
        description: "Search the public Web through bounded DuckDuckGo results.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "minLength": 1, "maxLength": 500},
                "locale": {"type": "string", "maxLength": 16},
                "safe_search": {"type": "string", "enum": ["strict", "moderate", "off"]},
                "count": {"type": "integer", "minimum": 1, "maximum": 10}
            },
            "required": ["query"],
            "additionalProperties": false
        }),
        operations: vec![NativeToolOperation::WebSearch],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile::bounded(4_096, 131_072, 20_000, 20),
    }
}

fn fetch_definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "web_fetch".to_string(),
        description: "Fetch and extract a public HTTP(S) page through guarded retrieval."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "minLength": 1, "maxLength": 4096},
                "max_text_chars": {"type": "integer", "minimum": 1, "maximum": 100000},
                "persist_binary": {"type": "boolean"}
            },
            "required": ["url"],
            "additionalProperties": false
        }),
        operations: vec![NativeToolOperation::WebFetch],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile::bounded(8_192, 262_144, 30_000, 30),
    }
}

fn permission(input: &ValidatedNativeToolInput) -> NativeToolPermissionRequest {
    NativeToolPermissionRequest {
        action: Action::new(input.operation.as_str()),
        resource: Resource::new(input.resource.canonical_id.clone()),
        operation: input.operation,
        canonical_resource: input.resource.clone(),
        input_hash: input.input_hash.clone(),
    }
}

#[cfg(test)]
#[path = "web_handler_tests.rs"]
mod tests;
