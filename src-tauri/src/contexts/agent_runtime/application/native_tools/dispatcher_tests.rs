use super::NativeToolDispatcher;
use crate::contexts::agent_runtime::application::{
    AgentPermissionPort, CanonicalToolResource, NativeToolAuthorizationStatus,
    NativeToolDefinition, NativeToolDispatchRequest, NativeToolErrorCode,
    NativeToolExecutionContext, NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError,
    NativeToolLimitProfile, NativeToolOperation, NativeToolPermissionRequest, NativeToolProgress,
    NativeToolProgressSink, NativeToolRegistry, NativeToolResultEnvelope, NativeToolResultStatus,
    ToolEligibility, ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Effect, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct NoopProgress;

impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}

struct Handler {
    definition: NativeToolDefinition,
}

impl Handler {
    fn new(plan_mode_compatible: bool, max_input_bytes: u64) -> Self {
        Self {
            definition: NativeToolDefinition {
                contract_version: NATIVE_TOOL_CONTRACT_VERSION,
                name: "artifact".to_string(),
                description: "fixture".to_string(),
                input_schema: json!({"type": "object"}),
                operations: vec![
                    NativeToolOperation::ArtifactRead,
                    NativeToolOperation::ArtifactPublish,
                    NativeToolOperation::DelegationApply,
                ],
                plan_mode_compatible,
                limit_profile: NativeToolLimitProfile::bounded(max_input_bytes, 1024, 1000, 10),
            },
        }
    }
}

impl NativeToolHandler for Handler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, _context: &ToolEligibilityContext) -> ToolEligibility {
        ToolEligibility::Eligible
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let operation = match input.get("operation").and_then(Value::as_str) {
            Some("publish") => NativeToolOperation::ArtifactPublish,
            Some("apply") => NativeToolOperation::DelegationApply,
            _ => NativeToolOperation::ArtifactRead,
        };
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: "sha256:fixture".to_string(),
            operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: "artifact/1".to_string(),
                attributes: BTreeMap::new(),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            action: Action::new(input.operation.as_str()),
            resource: Resource::new("artifact/1"),
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

struct Permissions {
    effect: Effect,
    pending: Mutex<Vec<(String, String)>>,
}

impl Permissions {
    fn new(effect: Effect) -> Self {
        Self {
            effect,
            pending: Mutex::new(Vec::new()),
        }
    }
}

impl AgentPermissionPort for Permissions {
    fn evaluate(
        &self,
        _agent_id: &str,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _project_key: &str,
    ) -> Effect {
        self.effect
    }

    fn create_pending_approval(
        &self,
        _agent_id: &str,
        _action: Action,
        resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        call_id: &str,
        _project_key: &str,
    ) -> Result<(), crate::contexts::agent_runtime::application::AgentRuntimeApplicationError> {
        self.pending
            .lock()
            .expect("pending")
            .push((call_id.to_string(), resource.as_str().to_string()));
        Ok(())
    }
}

fn authority(agent_id: &str, mode: NativeToolExecutionMode) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent_id.to_string(),
        session_id: "session-1".to_string(),
        generation_id: "generation-1".to_string(),
        canonical_workspace: Some(PathBuf::from("C:/workspace")),
        execution_mode: mode,
        readiness: BTreeMap::new(),
    }
}

fn request(
    authority: ToolEligibilityContext,
    cancelled: bool,
    deadline: Instant,
) -> NativeToolDispatchRequest {
    NativeToolDispatchRequest {
        tool_name: "artifact".to_string(),
        input: json!({"operation": "metadata"}),
        execution: NativeToolExecutionContext {
            call_id: "call-1".to_string(),
            session_id: authority.session_id.clone(),
            generation_id: authority.generation_id.clone(),
            agent_id: authority.agent_id.clone(),
            canonical_workspace: authority.canonical_workspace.clone(),
            deadline,
            cancelled: Arc::new(AtomicBool::new(cancelled)),
            progress: Arc::new(NoopProgress),
        },
        authority,
    }
}

fn dispatcher(plan_mode_compatible: bool, max_input_bytes: u64) -> NativeToolDispatcher {
    NativeToolDispatcher::new(
        NativeToolRegistry::try_new(vec![Arc::new(Handler::new(
            plan_mode_compatible,
            max_input_bytes,
        ))])
        .expect("registry"),
    )
}

fn rollout_dispatcher() -> NativeToolDispatcher {
    NativeToolDispatcher::new(
        NativeToolRegistry::try_new_with_feature_gates(
            vec![Arc::new(Handler::new(true, 1024))],
            crate::contexts::agent_runtime::application::OnePieceToolFeatureGates::rollout_defaults(
            ),
        )
        .expect("registry"),
    )
}

#[test]
fn preparation_revalidates_identity_mode_and_lifecycle_before_execution() {
    let execute = authority("onepiece", NativeToolExecutionMode::Execute);
    assert!(dispatcher(false, 1024)
        .prepare(request(
            execute.clone(),
            false,
            Instant::now() + Duration::from_secs(10)
        ))
        .is_ok());

    let cases = [
        (
            request(
                authority("custom-api", NativeToolExecutionMode::Execute),
                false,
                Instant::now() + Duration::from_secs(10),
            ),
            NativeToolErrorCode::Ineligible,
        ),
        (
            request(
                authority("onepiece", NativeToolExecutionMode::Plan),
                false,
                Instant::now() + Duration::from_secs(10),
            ),
            NativeToolErrorCode::Ineligible,
        ),
        (
            request(
                execute.clone(),
                true,
                Instant::now() + Duration::from_secs(10),
            ),
            NativeToolErrorCode::Cancelled,
        ),
        (
            request(execute, false, Instant::now() - Duration::from_secs(1)),
            NativeToolErrorCode::DeadlineExceeded,
        ),
    ];
    for (request, code) in cases {
        assert_eq!(
            dispatcher(false, 1024)
                .prepare(request)
                .err()
                .expect("rejected")
                .code,
            code
        );
    }
}

#[test]
fn preparation_rejects_ownership_workspace_and_input_limit_changes() {
    let authority = authority("onepiece", NativeToolExecutionMode::Execute);
    let mut ownership = request(
        authority.clone(),
        false,
        Instant::now() + Duration::from_secs(10),
    );
    ownership.execution.session_id = "forged-session".to_string();
    assert_eq!(
        dispatcher(false, 1024)
            .prepare(ownership)
            .err()
            .expect("ownership")
            .code,
        NativeToolErrorCode::Ineligible
    );

    let mut workspace = request(
        authority.clone(),
        false,
        Instant::now() + Duration::from_secs(10),
    );
    workspace.execution.canonical_workspace = Some(PathBuf::from("C:/other"));
    assert_eq!(
        dispatcher(false, 1024)
            .prepare(workspace)
            .err()
            .expect("workspace")
            .code,
        NativeToolErrorCode::Conflict
    );

    assert_eq!(
        dispatcher(false, 2)
            .prepare(request(
                authority,
                false,
                Instant::now() + Duration::from_secs(10),
            ))
            .err()
            .expect("input limit")
            .code,
        NativeToolErrorCode::LimitExceeded
    );
}

#[test]
fn permission_evaluation_binds_input_hash_and_routes_ask_through_unified_port() {
    let dispatcher = dispatcher(false, 1024);
    let prepared = dispatcher
        .prepare(request(
            authority("onepiece", NativeToolExecutionMode::Execute),
            false,
            Instant::now() + Duration::from_secs(10),
        ))
        .expect("prepared");
    let permissions = Permissions::new(Effect::Ask);
    let witness = dispatcher
        .authorize(&prepared, &permissions, "C:/workspace")
        .expect("witness");

    assert_eq!(
        witness.status,
        NativeToolAuthorizationStatus::AwaitingApproval
    );
    assert_eq!(witness.input_hash, "sha256:fixture");
    assert_eq!(
        permissions.pending.lock().expect("pending").as_slice(),
        &[(
            "call-1".to_string(),
            "artifact/1#input=sha256:fixture".to_string()
        )]
    );
    let result = dispatcher
        .execute_authorized(prepared, &witness)
        .expect("execution");
    assert_eq!(result.status, NativeToolResultStatus::Succeeded);
}

#[test]
fn delegation_apply_always_asks_even_when_a_remembered_policy_allows() {
    let dispatcher = dispatcher(false, 1024);
    let mut apply = request(
        authority("onepiece", NativeToolExecutionMode::Execute),
        false,
        Instant::now() + Duration::from_secs(10),
    );
    apply.input = json!({"operation": "apply"});
    let prepared = dispatcher.prepare(apply).expect("prepared");
    let permissions = Permissions::new(Effect::Allow);

    let witness = dispatcher
        .authorize(&prepared, &permissions, "C:/workspace")
        .expect("witness");

    assert_eq!(
        witness.status,
        NativeToolAuthorizationStatus::AwaitingApproval
    );
    assert_eq!(permissions.pending.lock().expect("pending").len(), 1);
}

#[test]
fn denied_and_stale_permissions_never_execute() {
    let dispatcher = dispatcher(false, 1024);
    let prepared = dispatcher
        .prepare(request(
            authority("onepiece", NativeToolExecutionMode::Execute),
            false,
            Instant::now() + Duration::from_secs(10),
        ))
        .expect("prepared");
    assert_eq!(
        dispatcher
            .authorize(&prepared, &Permissions::new(Effect::Deny), "C:/workspace")
            .expect_err("denied")
            .code,
        NativeToolErrorCode::PermissionDenied
    );

    let mut witness = dispatcher
        .authorize(&prepared, &Permissions::new(Effect::Allow), "C:/workspace")
        .expect("witness");
    witness.input_hash = "sha256:changed".to_string();
    assert_eq!(
        dispatcher
            .execute_authorized(prepared, &witness)
            .expect_err("stale")
            .code,
        NativeToolErrorCode::StaleApproval
    );
}

#[test]
fn forged_cli_wrapped_agent_dispatches_fail_closed() {
    let dispatcher = dispatcher(false, 1024);
    for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode-cli"] {
        let error = dispatcher
            .prepare(request(
                authority(agent_id, NativeToolExecutionMode::Execute),
                false,
                Instant::now() + Duration::from_secs(10),
            ))
            .err()
            .expect("forged dispatch must fail");
        assert_eq!(error.code, NativeToolErrorCode::Ineligible, "{agent_id}");
    }
}

#[test]
fn plan_mode_rejects_artifact_publication_even_for_plan_compatible_handler() {
    let mut publish = request(
        authority("onepiece", NativeToolExecutionMode::Plan),
        false,
        Instant::now() + Duration::from_secs(10),
    );
    publish.input = json!({"operation": "publish"});
    let error = dispatcher(true, 1024)
        .prepare(publish)
        .err()
        .expect("publication must be rejected");
    assert_eq!(error.code, NativeToolErrorCode::PermissionDenied);
}

#[test]
fn disabled_effect_is_rejected_after_validation_even_when_read_tool_is_visible() {
    let mut publish = request(
        authority("onepiece", NativeToolExecutionMode::Execute),
        false,
        Instant::now() + Duration::from_secs(10),
    );
    publish.input = json!({"operation": "publish"});

    let error = rollout_dispatcher()
        .prepare(publish)
        .err()
        .expect("disabled publication");
    assert_eq!(error.code, NativeToolErrorCode::Ineligible);
}

#[test]
fn policy_matrix_keeps_identity_mode_gate_and_approval_decisions_fail_closed() {
    let cases = [
        (
            "custom-api",
            NativeToolExecutionMode::Execute,
            "metadata",
            NativeToolErrorCode::Ineligible,
        ),
        (
            "onepiece",
            NativeToolExecutionMode::Plan,
            "publish",
            NativeToolErrorCode::Ineligible,
        ),
        (
            "onepiece",
            NativeToolExecutionMode::Execute,
            "publish",
            NativeToolErrorCode::Ineligible,
        ),
    ];
    for (agent_id, mode, operation, expected) in cases {
        let mut candidate = request(
            authority(agent_id, mode),
            false,
            Instant::now() + Duration::from_secs(10),
        );
        candidate.input = json!({"operation": operation});
        assert_eq!(
            rollout_dispatcher()
                .prepare(candidate)
                .err()
                .expect("matrix rejection")
                .code,
            expected,
            "{agent_id}/{mode:?}/{operation}"
        );
    }

    let dispatcher = dispatcher(true, 1024);
    for (effect, allowed) in [
        (Effect::Allow, true),
        (Effect::Ask, true),
        (Effect::Deny, false),
    ] {
        let prepared = dispatcher
            .prepare(request(
                authority("onepiece", NativeToolExecutionMode::Execute),
                false,
                Instant::now() + Duration::from_secs(10),
            ))
            .expect("prepared");
        assert_eq!(
            dispatcher
                .authorize(&prepared, &Permissions::new(effect), "C:/workspace")
                .is_ok(),
            allowed,
            "{effect:?}"
        );
    }
}
