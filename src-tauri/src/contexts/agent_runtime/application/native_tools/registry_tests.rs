use super::{NativeToolRegistry, NativeToolRegistryError};
use crate::contexts::agent_runtime::application::{
    CanonicalToolResource, NativeToolDefinition, NativeToolErrorCode, NativeToolExecutionContext,
    NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile, NativeToolOperation,
    NativeToolPermissionRequest, NativeToolReadinessReasonCode, NativeToolResultEnvelope,
    NativeToolResultStatus, ToolEligibility, ToolEligibilityContext, ToolResourceKind,
    ValidatedNativeToolInput, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

struct FixtureHandler {
    definition: NativeToolDefinition,
    eligibility: ToolEligibility,
}

impl FixtureHandler {
    fn new(name: &str, version: u16, eligibility: ToolEligibility) -> Self {
        Self {
            definition: NativeToolDefinition {
                contract_version: version,
                name: name.to_string(),
                description: format!("{name} fixture"),
                input_schema: json!({"type": "object", "additionalProperties": false}),
                operations: vec![NativeToolOperation::ArtifactRead],
                plan_mode_compatible: true,
                limit_profile: NativeToolLimitProfile::bounded(100, 200, 300, 4),
            },
            eligibility,
        }
    }
}

impl NativeToolHandler for FixtureHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, _context: &ToolEligibilityContext) -> ToolEligibility {
        self.eligibility.clone()
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        if !input.is_object() {
            return Err(NativeToolHandlerError::new(
                NativeToolErrorCode::InvalidInput,
                "Input must be an object.",
            ));
        }
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: "sha256:fixture".to_string(),
            operation: NativeToolOperation::ArtifactRead,
            resource: fixture_resource(),
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            action: Action::new("artifact.read"),
            resource: Resource::new("artifact/fixture"),
            operation: input.operation,
            canonical_resource: input.resource.clone(),
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
            output: Some(json!({"ok": true})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn fixture_resource() -> CanonicalToolResource {
    CanonicalToolResource {
        kind: ToolResourceKind::Artifact,
        canonical_id: "artifact/fixture".to_string(),
        attributes: BTreeMap::new(),
    }
}

fn context() -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_string(),
        session_id: "session-1".to_string(),
        generation_id: "generation-1".to_string(),
        canonical_workspace: None,
        execution_mode:
            crate::contexts::agent_runtime::application::NativeToolExecutionMode::Execute,
        readiness: BTreeMap::new(),
    }
}

fn context_for_agent(agent_id: &str) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent_id.to_string(),
        ..context()
    }
}

#[test]
fn registry_is_fixed_after_construction_and_filters_catalog_by_eligibility() {
    let registry = NativeToolRegistry::try_new(vec![
        Arc::new(FixtureHandler::new(
            "artifact",
            NATIVE_TOOL_CONTRACT_VERSION,
            ToolEligibility::Eligible,
        )),
        Arc::new(FixtureHandler::new(
            "browser",
            NATIVE_TOOL_CONTRACT_VERSION,
            ToolEligibility::Ineligible {
                reason: crate::contexts::agent_runtime::application::NativeToolReadinessReasonCode::Disabled,
            },
        )),
    ])
    .expect("registry");

    assert_eq!(registry.len(), 2);
    assert!(!registry.is_empty());
    assert!(registry.handler("artifact").is_some());
    assert_eq!(
        registry
            .eligible_definitions(&context())
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>(),
        vec!["artifact"]
    );
    let provider_neutral = registry.eligible_tool_definitions(&context());
    assert_eq!(provider_neutral.len(), 1);
    assert_eq!(provider_neutral[0].name, "artifact");
    assert_eq!(provider_neutral[0].input_schema["type"], "object");
}

#[test]
fn registry_rejects_duplicate_empty_and_unknown_version_definitions() {
    let duplicate = NativeToolRegistry::try_new(vec![
        Arc::new(FixtureHandler::new(
            "artifact",
            NATIVE_TOOL_CONTRACT_VERSION,
            ToolEligibility::Eligible,
        )),
        Arc::new(FixtureHandler::new(
            "artifact",
            NATIVE_TOOL_CONTRACT_VERSION,
            ToolEligibility::Eligible,
        )),
    ]);
    assert!(matches!(
        duplicate,
        Err(NativeToolRegistryError::DuplicateName(name)) if name == "artifact"
    ));

    let empty = NativeToolRegistry::try_new(vec![Arc::new(FixtureHandler::new(
        " ",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    ))]);
    assert!(matches!(empty, Err(NativeToolRegistryError::EmptyName)));

    let incompatible = NativeToolRegistry::try_new(vec![Arc::new(FixtureHandler::new(
        "artifact",
        NATIVE_TOOL_CONTRACT_VERSION + 1,
        ToolEligibility::Eligible,
    ))]);
    assert!(matches!(
        incompatible,
        Err(NativeToolRegistryError::UnsupportedContractVersion { name, version })
            if name == "artifact" && version == NATIVE_TOOL_CONTRACT_VERSION + 1
    ));
}

#[test]
fn handler_contract_keeps_validation_permission_and_execution_separate() {
    let handler = FixtureHandler::new(
        "artifact",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    let validated = handler.validate(&json!({})).expect("validated");
    let permission = handler.permission_request(&validated, &context());
    assert_eq!(permission.action.as_str(), "artifact.read");
    assert_eq!(permission.resource.as_str(), "artifact/fixture");
    assert_eq!(permission.input_hash, "sha256:fixture");
    let error = handler.validate(&Value::Null).expect_err("invalid input");
    assert_eq!(error.code, NativeToolErrorCode::InvalidInput);
    assert_eq!(error.safe_message, "Input must be an object.");
}

#[test]
fn onepiece_only_names_fail_closed_even_when_handler_claims_eligibility() {
    let registry = NativeToolRegistry::try_new(vec![Arc::new(FixtureHandler::new(
        "artifact",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    ))])
    .expect("registry");

    assert_eq!(registry.eligible_definitions(&context()).len(), 1);
    for custom_agent in ["custom-api", "claude-code", "onepiece-copy"] {
        assert!(registry
            .eligible_definitions(&context_for_agent(custom_agent))
            .is_empty());
        assert!(matches!(
            registry.eligibility("artifact", &context_for_agent(custom_agent)),
            ToolEligibility::Ineligible { .. }
        ));
    }
}

#[test]
fn extended_tool_name_set_is_fixed_and_complete() {
    assert_eq!(
        crate::contexts::agent_runtime::application::ONEPIECE_ONLY_TOOL_NAMES,
        [
            "browser",
            "web_search",
            "web_fetch",
            "code_execution",
            "ocr",
            "artifact",
            "delegate_cli",
            "apply_delegation_changes",
            "delegate_subagent",
        ]
    );
    assert!(crate::contexts::agent_runtime::application::is_onepiece_only("browser"));
    assert!(!crate::contexts::agent_runtime::application::is_onepiece_only("file"));
}

#[test]
fn rollout_gates_hide_disabled_tools_and_project_artifact_read_only() {
    let mut artifact = FixtureHandler::new(
        "artifact",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    artifact.definition.operations = vec![
        NativeToolOperation::ArtifactRead,
        NativeToolOperation::ArtifactPublish,
    ];
    artifact.definition.input_schema = json!({
        "properties": {"operation": {"enum": ["list", "metadata", "read_text", "publish"]}}
    });
    let browser = FixtureHandler::new(
        "browser",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    let registry = NativeToolRegistry::try_new_with_feature_gates(
        vec![Arc::new(artifact), Arc::new(browser)],
        crate::contexts::agent_runtime::application::OnePieceToolFeatureGates::rollout_defaults(),
    )
    .expect("registry");

    let definitions = registry.eligible_definitions(&context());
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "artifact");
    assert_eq!(
        definitions[0].operations,
        vec![NativeToolOperation::ArtifactRead]
    );
    assert_eq!(
        definitions[0]
            .input_schema
            .pointer("/properties/operation/enum"),
        Some(&json!(["list", "metadata", "read_text"]))
    );
    assert!(!registry.is_operation_enabled(NativeToolOperation::ArtifactPublish));
    assert!(matches!(
        registry.eligibility("browser", &context()),
        ToolEligibility::Ineligible {
            reason:
                crate::contexts::agent_runtime::application::NativeToolReadinessReasonCode::Disabled
        }
    ));
}

#[test]
fn plan_catalog_projects_read_only_artifact_and_ocr_contracts() {
    let mut artifact = FixtureHandler::new(
        "artifact",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    artifact.definition.operations = vec![
        NativeToolOperation::ArtifactRead,
        NativeToolOperation::ArtifactPublish,
    ];
    artifact.definition.input_schema = json!({
        "type": "object",
        "properties": {"operation": {"enum": ["list", "metadata", "read_text", "publish"]}}
    });
    let mut ocr = FixtureHandler::new(
        "ocr",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    ocr.definition.operations = vec![
        NativeToolOperation::OcrRead,
        NativeToolOperation::ArtifactPublish,
    ];
    ocr.definition.input_schema = json!({
        "type": "object",
        "properties": {"artifact_id": {"type": "string"}, "publish": {"type": "boolean"}}
    });
    let mut effect = FixtureHandler::new(
        "code_execution",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    effect.definition.plan_mode_compatible = false;
    effect.definition.operations = vec![NativeToolOperation::CodeExecute];
    let registry =
        NativeToolRegistry::try_new(vec![Arc::new(artifact), Arc::new(ocr), Arc::new(effect)])
            .expect("registry");
    let mut plan = context();
    plan.execution_mode =
        crate::contexts::agent_runtime::application::NativeToolExecutionMode::Plan;

    let definitions = registry.eligible_definitions(&plan);
    assert_eq!(definitions.len(), 2);
    let artifact = definitions
        .iter()
        .find(|definition| definition.name == "artifact")
        .expect("artifact");
    assert_eq!(artifact.operations, vec![NativeToolOperation::ArtifactRead]);
    assert_eq!(
        artifact.input_schema.pointer("/properties/operation/enum"),
        Some(&json!(["list", "metadata", "read_text"]))
    );
    let ocr = definitions
        .iter()
        .find(|definition| definition.name == "ocr")
        .expect("ocr");
    assert_eq!(ocr.operations, vec![NativeToolOperation::OcrRead]);
    assert!(ocr.input_schema.pointer("/properties/publish").is_none());
}

#[test]
fn structural_registration_keeps_unavailable_handlers_out_of_the_catalog() {
    let handler = FixtureHandler::new(
        "delegate_cli",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    );
    let registry = NativeToolRegistry::try_new_with_feature_gates_and_readiness(
        vec![Arc::new(handler)],
        crate::contexts::agent_runtime::application::OnePieceToolFeatureGates::all_enabled(),
        BTreeMap::from([(
            "delegate_cli".to_owned(),
            NativeToolReadinessReasonCode::BackendUnavailable,
        )]),
    )
    .expect("registry");
    let mut eligibility = context();
    eligibility.readiness = registry.readiness_snapshot();

    assert!(registry.is_registered("delegate_cli"));
    assert_eq!(
        registry.readiness_reason("delegate_cli"),
        Some(NativeToolReadinessReasonCode::BackendUnavailable)
    );
    assert!(registry.eligible_definitions(&eligibility).is_empty());
}

#[test]
fn custom_api_and_cli_wrapped_agents_cannot_discover_extended_tools() {
    let registry = NativeToolRegistry::try_new(vec![Arc::new(FixtureHandler::new(
        "browser",
        NATIVE_TOOL_CONTRACT_VERSION,
        ToolEligibility::Eligible,
    ))])
    .expect("registry");

    for agent_id in [
        "custom-api-agent",
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "opencode-cli",
    ] {
        assert!(
            registry
                .eligible_tool_definitions(&context_for_agent(agent_id))
                .is_empty(),
            "{agent_id} discovered a OnePiece-only tool"
        );
    }
}
