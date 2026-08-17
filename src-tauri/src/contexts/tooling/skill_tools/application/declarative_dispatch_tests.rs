use super::*;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolDeclarativeValidator, SkillToolSchemaValidationPort, SkillToolSchemaViolation,
    SkillToolTargetCatalogPort,
};
use crate::contexts::tooling::skill_tools::domain::{
    parse_manifest_bytes, SkillToolImplementation, SkillToolKey, SkillToolRevision,
    SkillToolSourceScope, DEFAULT_MANIFEST_LIMITS, DEFAULT_SKILL_TOOL_LIMITS,
};
use std::sync::Mutex;

struct Catalog;

impl SkillToolTargetCatalogPort for Catalog {
    fn contains_operation(&self, operation: &str) -> bool {
        operation == "read_file"
    }
}

struct Modes(bool);

impl SkillToolCapabilityModePort for Modes {
    fn allows(&self, _mode: SkillToolExecutionMode, _capability: &SkillToolCapability) -> bool {
        self.0
    }
}

struct Budget(bool);

impl SkillToolInvocationBudgetPort for Budget {
    fn reserve_host_call(&self) -> Result<(), SkillToolApplicationError> {
        self.0
            .then_some(())
            .ok_or_else(|| SkillToolApplicationError::ResourceLimit("host-calls".to_string()))
    }

    fn consume_output(&self, _bytes: u64) -> Result<(), SkillToolApplicationError> {
        self.reserve_host_call()
    }
}

struct Schemas;

impl SkillToolSchemaValidationPort for Schemas {
    fn validate_instance(
        &self,
        _schema: &BoundedJsonSchema,
        _instance: &Value,
    ) -> Result<(), Vec<SkillToolSchemaViolation>> {
        Ok(())
    }
}

static SCHEMAS: Schemas = Schemas;

struct DirectionalSchemas;

impl SkillToolSchemaValidationPort for DirectionalSchemas {
    fn validate_instance(
        &self,
        _schema: &BoundedJsonSchema,
        instance: &Value,
    ) -> Result<(), Vec<SkillToolSchemaViolation>> {
        if instance.get("path").is_some() {
            Ok(())
        } else {
            Err(vec![SkillToolSchemaViolation {
                pointer: "/summary".to_string(),
                code: "fixture-output".to_string(),
            }])
        }
    }
}

#[derive(Default)]
struct Host {
    arguments: Mutex<Vec<Value>>,
    cancelled: bool,
}

impl SkillToolHostDispatchPort for Host {
    fn dispatch(
        &self,
        _principal: &SkillToolPrincipal,
        _capability: &SkillToolCapability,
        arguments: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        self.arguments
            .lock()
            .expect("arguments")
            .push(arguments.clone());
        if self.cancelled {
            Ok(SkillToolDispatchOutcome::Cancelled)
        } else {
            Ok(SkillToolDispatchOutcome::Completed(serde_json::json!({
                "summary": "done"
            })))
        }
    }
}

struct DenyingGateway<'a> {
    reason: &'a str,
    calls: Mutex<Vec<(SkillToolPrincipal, SkillToolCapability, Value)>>,
}

impl SkillToolHostDispatchPort for DenyingGateway<'_> {
    fn dispatch(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        self.calls.lock().expect("calls").push((
            principal.clone(),
            capability.clone(),
            arguments.clone(),
        ));
        Ok(denied(self.reason))
    }
}

fn fixture() -> (
    Vec<SkillToolCapability>,
    ValidatedDeclarativeTemplate,
    SkillToolPrincipal,
    BoundedJsonSchema,
    BoundedJsonSchema,
) {
    let manifest = parse_manifest_bytes(
        include_bytes!("../../../../../tests/fixtures/skill-tools/valid-declarative.json"),
        &DEFAULT_MANIFEST_LIMITS,
    )
    .expect("manifest");
    let declaration = manifest.tools[0].clone();
    let key = SkillToolKey::new(
        manifest.owner,
        SkillToolSourceScope::global(),
        declaration.id.clone(),
        SkillToolRevision::parse(&"0".repeat(64)).expect("revision"),
    );
    let SkillToolImplementation::Declarative(implementation) = declaration.implementation else {
        panic!("declarative")
    };
    let template = SkillToolDeclarativeValidator::new(&Catalog)
        .validate(&implementation)
        .expect("template");
    let principal = SkillToolPrincipal {
        parent_agent_id: "agent".to_string(),
        key,
        workspace_path: Some("/workspace".to_string()),
        session_id: Some("session".to_string()),
        generation_id: "generation".to_string(),
        delegation_chain: Vec::new(),
    };
    (
        declaration.capabilities,
        template,
        principal,
        declaration.input,
        declaration.output,
    )
}

#[test]
fn allowed_dispatch_projects_and_routes_through_the_host_port() {
    let (capabilities, template, principal, input_schema, output_schema) = fixture();
    let host = Host::default();
    let dispatcher = SkillToolDeclarativeDispatcher::new(
        &host,
        &Modes(true),
        SkillToolPayloadValidator::new(&SCHEMAS),
    );
    let outcome = dispatcher
        .dispatch(
            &principal,
            SkillToolExecutionMode::Execute,
            &capabilities,
            &template,
            &input_schema,
            &output_schema,
            &serde_json::json!({"path": "src/main.rs"}),
            DEFAULT_SKILL_TOOL_LIMITS,
            &Budget(true),
        )
        .expect("dispatch");
    assert!(matches!(outcome, SkillToolDispatchOutcome::Completed(_)));
    assert_eq!(
        host.arguments.lock().expect("arguments").as_slice(),
        &[serde_json::json!({"encoding": "utf-8", "path": "src/main.rs"})]
    );
}

#[test]
fn mode_capability_cycle_depth_and_budget_fail_before_host_dispatch() {
    let (capabilities, template, mut principal, input_schema, output_schema) = fixture();
    let host = Host::default();
    let denied_mode = SkillToolDeclarativeDispatcher::new(
        &host,
        &Modes(false),
        SkillToolPayloadValidator::new(&SCHEMAS),
    );
    assert!(matches!(
        denied_mode
            .dispatch(
                &principal,
                SkillToolExecutionMode::Plan,
                &capabilities,
                &template,
                &input_schema,
                &output_schema,
                &serde_json::json!({"path": "src/main.rs"}),
                DEFAULT_SKILL_TOOL_LIMITS,
                &Budget(true),
            )
            .expect("mode"),
        SkillToolDispatchOutcome::Denied { .. }
    ));
    principal.delegation_chain = vec![template.target().as_declaration()];
    let allowed = SkillToolDeclarativeDispatcher::new(
        &host,
        &Modes(true),
        SkillToolPayloadValidator::new(&SCHEMAS),
    );
    assert!(matches!(
        allowed
            .dispatch(
                &principal,
                SkillToolExecutionMode::Execute,
                &capabilities,
                &template,
                &input_schema,
                &output_schema,
                &serde_json::json!({"path": "src/main.rs"}),
                DEFAULT_SKILL_TOOL_LIMITS,
                &Budget(true),
            )
            .expect("cycle"),
        SkillToolDispatchOutcome::Denied { .. }
    ));
    principal.delegation_chain = vec!["tool:other".to_string(); 4];
    assert!(matches!(
        allowed
            .dispatch(
                &principal,
                SkillToolExecutionMode::Execute,
                &capabilities,
                &template,
                &input_schema,
                &output_schema,
                &serde_json::json!({"path": "src/main.rs"}),
                DEFAULT_SKILL_TOOL_LIMITS,
                &Budget(true),
            )
            .expect("depth"),
        SkillToolDispatchOutcome::Denied { .. }
    ));
    principal.delegation_chain.clear();
    assert!(matches!(
        allowed
            .dispatch(
                &principal,
                SkillToolExecutionMode::Execute,
                &[],
                &template,
                &input_schema,
                &output_schema,
                &serde_json::json!({"path": "src/main.rs"}),
                DEFAULT_SKILL_TOOL_LIMITS,
                &Budget(true),
            )
            .expect("capability"),
        SkillToolDispatchOutcome::Denied { .. }
    ));
    assert!(matches!(
        allowed.dispatch(
            &principal,
            SkillToolExecutionMode::Execute,
            &capabilities,
            &template,
            &input_schema,
            &output_schema,
            &serde_json::json!({"path": "src/main.rs"}),
            DEFAULT_SKILL_TOOL_LIMITS,
            &Budget(false),
        ),
        Err(SkillToolApplicationError::ResourceLimit(_))
    ));
    assert!(host.arguments.lock().expect("arguments").is_empty());
}

#[test]
fn host_cancellation_is_propagated_without_becoming_a_failure() {
    let (capabilities, template, principal, input_schema, output_schema) = fixture();
    let host = Host {
        arguments: Mutex::new(Vec::new()),
        cancelled: true,
    };
    let dispatcher = SkillToolDeclarativeDispatcher::new(
        &host,
        &Modes(true),
        SkillToolPayloadValidator::new(&SCHEMAS),
    );
    assert_eq!(
        dispatcher
            .dispatch(
                &principal,
                SkillToolExecutionMode::Execute,
                &capabilities,
                &template,
                &input_schema,
                &output_schema,
                &serde_json::json!({"path": "src/main.rs"}),
                DEFAULT_SKILL_TOOL_LIMITS,
                &Budget(true),
            )
            .expect("cancelled"),
        SkillToolDispatchOutcome::Cancelled
    );
}

#[test]
fn invalid_input_stops_before_host_and_invalid_output_is_not_returned() {
    let (capabilities, template, principal, input_schema, output_schema) = fixture();
    let host = Host::default();
    let reject_all = SkillToolPayloadValidator::new(&DirectionalSchemas);
    let dispatcher = SkillToolDeclarativeDispatcher::new(&host, &Modes(true), reject_all);
    let invalid_input = dispatcher
        .dispatch(
            &principal,
            SkillToolExecutionMode::Execute,
            &capabilities,
            &template,
            &input_schema,
            &output_schema,
            &serde_json::json!({"unexpected": true}),
            DEFAULT_SKILL_TOOL_LIMITS,
            &Budget(true),
        )
        .expect("invalid input");
    assert_eq!(
        invalid_input,
        SkillToolDispatchOutcome::Failed {
            code: "invalid-input".to_string()
        }
    );
    assert!(host.arguments.lock().expect("arguments").is_empty());

    let invalid_output = dispatcher
        .dispatch(
            &principal,
            SkillToolExecutionMode::Execute,
            &capabilities,
            &template,
            &input_schema,
            &output_schema,
            &serde_json::json!({"path": "src/main.rs"}),
            DEFAULT_SKILL_TOOL_LIMITS,
            &Budget(true),
        )
        .expect("invalid output");
    assert_eq!(
        invalid_output,
        SkillToolDispatchOutcome::Failed {
            code: "invalid-output".to_string()
        }
    );
    assert_eq!(host.arguments.lock().expect("arguments").len(), 1);
}

#[test]
fn native_gateway_denials_are_final_and_receive_unmodified_security_context() {
    let (capabilities, template, principal, input_schema, output_schema) = fixture();
    for reason in [
        "risk-classification",
        "permission-policy",
        "approval-denied",
    ] {
        let host = DenyingGateway {
            reason,
            calls: Mutex::new(Vec::new()),
        };
        let dispatcher = SkillToolDeclarativeDispatcher::new(
            &host,
            &Modes(true),
            SkillToolPayloadValidator::new(&SCHEMAS),
        );
        assert_eq!(
            dispatcher
                .dispatch(
                    &principal,
                    SkillToolExecutionMode::Execute,
                    &capabilities,
                    &template,
                    &input_schema,
                    &output_schema,
                    &serde_json::json!({"path": "src/main.rs"}),
                    DEFAULT_SKILL_TOOL_LIMITS,
                    &Budget(true),
                )
                .expect("gateway denial"),
            denied(reason)
        );
        let calls = host.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, principal);
        assert_eq!(&calls[0].1, template.target());
        assert_eq!(
            calls[0].2,
            serde_json::json!({"encoding": "utf-8", "path": "src/main.rs"})
        );
    }
}
