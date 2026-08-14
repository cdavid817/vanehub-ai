use super::{
    BrowserAutomationPort, CanonicalToolResource, NativeToolDefinition, NativeToolExecutionContext,
    NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPortRequest,
    NativeToolReadinessReasonCode, NativeToolResultEnvelope, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use url::Url;

const OPERATIONS: [&str; 13] = [
    "start",
    "navigate",
    "back",
    "forward",
    "inspect",
    "click",
    "type",
    "screenshot",
    "evaluate",
    "extract",
    "handoff",
    "resume",
    "close",
];

pub(crate) struct BrowserNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn BrowserAutomationPort>,
}

impl BrowserNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn BrowserAutomationPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for BrowserNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        if context.execution_mode == NativeToolExecutionMode::Plan {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        match context.readiness.get("browser") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ineligible(*reason),
            None => ineligible(NativeToolReadinessReasonCode::BackendUnavailable),
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(object)?;
        let operation = required_string(object, "operation", 32)?;
        if !OPERATIONS.contains(&operation) {
            return Err(invalid_input());
        }
        validate_operation_fields(operation, object)?;
        let hash = input_hash(input)?;
        let origin = canonical_origin(operation, object)?;
        let native_operation = risk_operation(operation);
        let mut attributes = BTreeMap::new();
        attributes.insert("browser_action".to_owned(), operation.to_owned());
        attributes.insert("safe_target".to_owned(), safe_target(operation, &hash));
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: hash,
            operation: native_operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::BrowserOrigin,
                canonical_id: origin,
                attributes,
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        let action = input
            .resource
            .attributes
            .get("browser_action")
            .map(String::as_str)
            .unwrap_or("unknown");
        let target = input
            .resource
            .attributes
            .get("safe_target")
            .map(String::as_str)
            .unwrap_or("target:unknown");
        let canonical_id = format!(
            "browser/{}/{}/{}/{}",
            context.session_id, input.resource.canonical_id, action, target
        );
        NativeToolPermissionRequest {
            action: Action::new(input.operation.as_str()),
            resource: Resource::new(canonical_id.clone()),
            operation: input.operation,
            canonical_resource: CanonicalToolResource {
                kind: ToolResourceKind::BrowserOrigin,
                canonical_id,
                attributes: input.resource.attributes.clone(),
            },
            input_hash: input.input_hash.clone(),
        }
    }

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        self.port
            .execute_browser(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "browser".to_owned(),
        description: "Control an isolated, policy-governed Playwright browser session.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": OPERATIONS},
                "page_id": {"type": "string", "maxLength": 128},
                "page_origin": {"type": "string", "maxLength": 4096},
                "url": {"type": "string", "maxLength": 4096},
                "selector": {"type": "string", "maxLength": 1024},
                "text": {"type": "string", "maxLength": 16384},
                "expression": {"type": "string", "maxLength": 16384},
                "full_page": {"type": "boolean"},
                "handoff_seconds": {"type": "integer", "minimum": 1, "maximum": 900}
            },
            "required": ["operation"],
            "additionalProperties": false
        }),
        operations: vec![
            NativeToolOperation::BrowserRead,
            NativeToolOperation::BrowserInteract,
        ],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile::bounded(64 * 1024, 256 * 1024, 45_000, 50),
    }
}

fn validate_operation_fields(
    operation: &str,
    object: &Map<String, Value>,
) -> Result<(), NativeToolHandlerError> {
    match operation {
        "start" => {}
        "navigate" => required_string(object, "url", 4096).map(|_| ())?,
        "click" | "extract" => required_string(object, "selector", 1024).map(|_| ())?,
        "type" => {
            required_string(object, "selector", 1024)?;
            required_string(object, "text", 16_384)?;
        }
        "evaluate" => required_string(object, "expression", 16_384).map(|_| ())?,
        "screenshot"
            if object
                .get("full_page")
                .is_some_and(|value| !value.is_boolean()) =>
        {
            return Err(invalid_input())
        }
        "handoff" => match object.get("handoff_seconds").and_then(Value::as_u64) {
            Some(1..=900) => {}
            _ => return Err(invalid_input()),
        },
        _ => {}
    }
    if !matches!(operation, "start" | "navigate") {
        required_string(object, "page_origin", 4096)?;
    }
    Ok(())
}

fn canonical_origin(
    operation: &str,
    object: &Map<String, Value>,
) -> Result<String, NativeToolHandlerError> {
    if operation == "start" {
        return Ok("new-context".to_owned());
    }
    let field = if operation == "navigate" {
        "url"
    } else {
        "page_origin"
    };
    let raw = required_string(object, field, 4096)?;
    let url = Url::parse(raw).map_err(|_| invalid_input())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.port_or_known_default(), Some(80 | 443))
        || url.host().is_none()
    {
        return Err(invalid_input());
    }
    Ok(url.origin().ascii_serialization())
}

fn risk_operation(operation: &str) -> NativeToolOperation {
    if matches!(
        operation,
        "click" | "type" | "screenshot" | "evaluate" | "handoff" | "resume"
    ) {
        NativeToolOperation::BrowserInteract
    } else {
        NativeToolOperation::BrowserRead
    }
}

fn safe_target(operation: &str, hash: &str) -> String {
    match operation {
        "click" | "type" | "extract" => format!("selector:{}", &hash[7..23]),
        "evaluate" => format!("script:{}", &hash[7..23]),
        _ => format!("action:{operation}"),
    }
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max_chars: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(invalid_input)?;
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(invalid_input());
    }
    Ok(value)
}

fn reject_unknown_fields(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let allowed = [
        "operation",
        "page_id",
        "page_origin",
        "url",
        "selector",
        "text",
        "expression",
        "full_page",
        "handoff_seconds",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if object.keys().any(|field| !allowed.contains(field.as_str())) {
        return Err(invalid_input());
    }
    Ok(())
}

fn input_hash(input: &Value) -> Result<String, NativeToolHandlerError> {
    let bytes = serde_json::to_vec(input).map_err(|_| invalid_input())?;
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(format!("sha256:{encoded}"))
}

fn ineligible(reason: NativeToolReadinessReasonCode) -> ToolEligibility {
    ToolEligibility::Ineligible { reason }
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The Browser tool input is invalid.",
    )
}

#[cfg(test)]
#[path = "browser_handler_tests.rs"]
mod tests;
