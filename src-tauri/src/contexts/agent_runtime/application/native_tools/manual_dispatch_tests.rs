use super::{
    CanonicalToolResource, ManualNativeToolAuthorityPort, ManualNativeToolOperationPort,
    ManualNativeToolRequest, ManualNativeToolService, NativeToolDefinition, NativeToolDispatcher,
    NativeToolExecutionContext, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolRegistry, NativeToolResultEnvelope,
    NativeToolResultStatus, StoredToolOperation, StoredToolOperationStatus, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::agent_runtime::application::{
    AgentPermissionPort, AgentRuntimeApplicationError, ToolApprovalDecision,
};
use crate::contexts::permissions::api::{Action, Effect, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct ApplyHandler {
    definition: NativeToolDefinition,
    executions: Arc<AtomicUsize>,
}

impl ApplyHandler {
    fn new(executions: Arc<AtomicUsize>) -> Self {
        Self {
            definition: NativeToolDefinition {
                contract_version: NATIVE_TOOL_CONTRACT_VERSION,
                name: "apply_delegation_changes".to_owned(),
                description: "fixture".to_owned(),
                input_schema: json!({"type": "object"}),
                operations: vec![NativeToolOperation::DelegationApply],
                plan_mode_compatible: false,
                limit_profile: NativeToolLimitProfile::bounded(4096, 4096, 60_000, 10),
            },
            executions,
        }
    }
}

impl NativeToolHandler for ApplyHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, _context: &ToolEligibilityContext) -> ToolEligibility {
        ToolEligibility::Eligible
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: "sha256:exact-input".to_owned(),
            operation: NativeToolOperation::DelegationApply,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::ChangeSet,
                canonical_id: "changeset/artifact-1/repo-1/base-1".to_owned(),
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
            action: Action::new(NativeToolOperation::DelegationApply.as_str()),
            resource: Resource::new(input.resource.canonical_id.clone()),
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
        self.executions.fetch_add(1, Ordering::AcqRel);
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"artifactId": "artifact-1"})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

struct AllowPermissions {
    pending: Mutex<Option<Sender<(String, String)>>>,
}

impl AgentPermissionPort for AllowPermissions {
    fn evaluate(
        &self,
        _agent_id: &str,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _project_key: &str,
    ) -> Effect {
        Effect::Allow
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
    ) -> Result<(), AgentRuntimeApplicationError> {
        if let Ok(pending) = self.pending.lock() {
            if let Some(sender) = pending.as_ref() {
                let _ = sender.send((call_id.to_owned(), resource.as_str().to_owned()));
            }
        }
        Ok(())
    }
}

struct Authority {
    artifacts: Arc<Mutex<Vec<Option<String>>>>,
}

impl ManualNativeToolAuthorityPort for Authority {
    fn resolve(
        &self,
        session_id: &str,
        agent_id: &str,
        artifact_id: Option<&str>,
    ) -> Result<PathBuf, &'static str> {
        if session_id != "session-1" || agent_id != "onepiece" {
            return Err("session_authority_mismatch");
        }
        self.artifacts
            .lock()
            .map_err(|_| "authority_unavailable")?
            .push(artifact_id.map(str::to_owned));
        Ok(PathBuf::from("C:/canonical-workspace"))
    }
}

#[derive(Default)]
struct Operations {
    records: Mutex<Vec<StoredToolOperation>>,
}

impl ManualNativeToolOperationPort for Operations {
    fn save(&self, operation: &StoredToolOperation) -> Result<(), ()> {
        self.records.lock().map_err(|_| ())?.push(operation.clone());
        Ok(())
    }
}

struct Fixture {
    service: ManualNativeToolService,
    pending: Receiver<(String, String)>,
    executions: Arc<AtomicUsize>,
    operations: Arc<Operations>,
    artifacts: Arc<Mutex<Vec<Option<String>>>>,
}

fn fixture() -> Fixture {
    let executions = Arc::new(AtomicUsize::new(0));
    let registry =
        NativeToolRegistry::try_new(vec![Arc::new(ApplyHandler::new(executions.clone()))])
            .expect("registry");
    let (sender, receiver) = channel();
    let operations = Arc::new(Operations::default());
    let artifacts = Arc::new(Mutex::new(Vec::new()));
    let service = ManualNativeToolService::new(
        NativeToolDispatcher::new(registry),
        Arc::new(AllowPermissions {
            pending: Mutex::new(Some(sender)),
        }),
        Arc::new(Authority {
            artifacts: artifacts.clone(),
        }),
        operations.clone(),
    );
    Fixture {
        service,
        pending: receiver,
        executions,
        operations,
        artifacts,
    }
}

#[test]
fn manual_apply_forces_once_approval_and_reuses_bound_authority() {
    let Fixture {
        service,
        pending,
        executions,
        operations,
        artifacts,
    } = fixture();
    let worker = service.clone();
    let handle = std::thread::spawn(move || {
        worker.execute(ManualNativeToolRequest {
            agent_id: "onepiece".to_owned(),
            session_id: "session-1".to_owned(),
            tool_name: "apply_delegation_changes".to_owned(),
            input: json!({"acknowledged": true}),
            authority_artifact_id: Some("artifact-1".to_owned()),
        })
    });
    let (call_id, resource) = pending
        .recv_timeout(Duration::from_secs(2))
        .expect("forced pending approval");
    assert!(resource.ends_with("#input=sha256:exact-input"));
    assert!(!service.resolve_approval("wrong-session", &call_id, ToolApprovalDecision::Approved));
    assert!(service.resolve_approval("session-1", &call_id, ToolApprovalDecision::Approved));
    let result = handle.join().expect("worker").expect("dispatch");
    assert_eq!(result.operation_id, call_id);
    assert_eq!(result.result.status, NativeToolResultStatus::Succeeded);
    assert_eq!(executions.load(Ordering::Acquire), 1);
    assert_eq!(
        artifacts.lock().expect("artifacts").as_slice(),
        &[Some("artifact-1".to_owned())]
    );
    let statuses = operations
        .records
        .lock()
        .expect("operations")
        .iter()
        .map(|record| record.status)
        .collect::<Vec<_>>();
    assert!(statuses.contains(&StoredToolOperationStatus::AwaitingApproval));
    assert_eq!(statuses.last(), Some(&StoredToolOperationStatus::Succeeded));
}

#[test]
fn manual_apply_denial_is_terminal_without_invoking_the_backend() {
    let Fixture {
        service,
        pending,
        executions,
        operations,
        ..
    } = fixture();
    let worker = service.clone();
    let handle = std::thread::spawn(move || {
        worker.execute(ManualNativeToolRequest {
            agent_id: "onepiece".to_owned(),
            session_id: "session-1".to_owned(),
            tool_name: "apply_delegation_changes".to_owned(),
            input: json!({"acknowledged": true}),
            authority_artifact_id: Some("artifact-1".to_owned()),
        })
    });
    let (call_id, _) = pending
        .recv_timeout(Duration::from_secs(2))
        .expect("pending approval");
    assert!(service.resolve_approval("session-1", &call_id, ToolApprovalDecision::Denied));
    let result = handle.join().expect("worker").expect("dispatch");
    assert_eq!(result.result.status, NativeToolResultStatus::Denied);
    assert_eq!(executions.load(Ordering::Acquire), 0);
    assert_eq!(
        operations
            .records
            .lock()
            .expect("operations")
            .last()
            .map(|record| record.status),
        Some(StoredToolOperationStatus::Failed)
    );
}
