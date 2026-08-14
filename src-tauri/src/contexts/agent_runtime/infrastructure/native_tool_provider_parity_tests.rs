use super::{
    anthropic_provider, api_process_adapter::GenerationOptions, openai_compatible_provider,
};
use crate::contexts::agent_runtime::application::{
    CanonicalToolResource, NativeToolDefinition, NativeToolErrorCode, NativeToolExecutionContext,
    NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolRegistry, NativeToolResultEnvelope,
    NativeToolResultStatus, ToolEligibility, ToolEligibilityContext, ToolResourceKind,
    ValidatedNativeToolInput, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

struct ParityHandler {
    definition: NativeToolDefinition,
}

impl ParityHandler {
    fn new() -> Self {
        Self {
            definition: NativeToolDefinition {
                contract_version: NATIVE_TOOL_CONTRACT_VERSION,
                name: "browser".to_owned(),
                description: "Bounded browser automation.".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "properties": {"operation": {"type": "string"}},
                    "required": ["operation"],
                    "additionalProperties": false
                }),
                operations: vec![NativeToolOperation::BrowserRead],
                plan_mode_compatible: false,
                limit_profile: NativeToolLimitProfile::bounded(1024, 4096, 10_000, 20),
            },
        }
    }
}

impl NativeToolHandler for ParityHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        if context.execution_mode == NativeToolExecutionMode::Execute {
            ToolEligibility::Eligible
        } else {
            ToolEligibility::Ineligible {
                reason: crate::contexts::agent_runtime::application::NativeToolReadinessReasonCode::PolicyUnavailable,
            }
        }
    }

    fn validate(&self, _input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        Err(NativeToolHandlerError::new(
            NativeToolErrorCode::InvalidInput,
            "Not used by provider translation tests.",
        ))
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            action: Action::new("browser.read"),
            resource: Resource::new("browser/session"),
            operation: input.operation,
            canonical_resource: CanonicalToolResource {
                kind: ToolResourceKind::BrowserOrigin,
                canonical_id: "browser/session".to_owned(),
                attributes: BTreeMap::new(),
            },
            input_hash: input.input_hash.clone(),
        }
    }

    fn execute(
        &self,
        _input: ValidatedNativeToolInput,
        _context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: None,
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn context(agent_id: &str, execution_mode: NativeToolExecutionMode) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent_id.to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: None,
        execution_mode,
        readiness: BTreeMap::new(),
    }
}

#[test]
fn registry_definitions_have_anthropic_and_openai_compatible_wire_parity() {
    let registry =
        NativeToolRegistry::try_new(vec![Arc::new(ParityHandler::new())]).expect("registry");
    let definitions =
        registry.eligible_tool_definitions(&context("onepiece", NativeToolExecutionMode::Execute));
    let options = GenerationOptions::disabled();
    let anthropic =
        anthropic_provider::build_request_body("model", &[], &definitions, None, &options);
    let openai =
        openai_compatible_provider::build_request_body("model", &[], &definitions, None, &options);

    assert_eq!(anthropic["tools"][0]["name"], "browser");
    assert_eq!(openai["tools"][0]["function"]["name"], "browser");
    assert_eq!(
        anthropic["tools"][0]["input_schema"],
        openai["tools"][0]["function"]["parameters"]
    );
    assert_eq!(
        anthropic["tools"][0]["description"],
        openai["tools"][0]["function"]["description"]
    );
    assert!(registry
        .eligible_tool_definitions(&context("onepiece", NativeToolExecutionMode::Plan))
        .is_empty());
    assert!(registry
        .eligible_tool_definitions(&context("custom-api", NativeToolExecutionMode::Execute))
        .is_empty());
}
