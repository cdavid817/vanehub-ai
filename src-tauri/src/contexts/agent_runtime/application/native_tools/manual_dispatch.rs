use super::manual_operation::ManualOperationRecorder;
use super::manual_waits::ManualApprovalWaits;
use super::{
    NativeToolAuthorizationStatus, NativeToolDispatchError, NativeToolDispatchRequest,
    NativeToolDispatcher, NativeToolErrorCode, NativeToolExecutionContext, NativeToolExecutionMode,
    NativeToolResultEnvelope, NativeToolResultStatus, StoredToolOperation,
    StoredToolOperationStatus, ToolEligibilityContext,
};
use crate::contexts::agent_runtime::application::{AgentPermissionPort, ToolApprovalDecision};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const MANUAL_TOOL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManualStartDelegationRequest {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) provider: String,
    pub(crate) mode: String,
    pub(crate) prompt: String,
    pub(crate) artifact_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManualApplyDelegationRequest {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) artifact_id: String,
    pub(crate) expected_content_hash: String,
    pub(crate) expected_diff_hash: String,
    pub(crate) repository_identity: String,
    pub(crate) base_commit: String,
    pub(crate) acknowledgement: bool,
}

pub(crate) trait ManualNativeToolAuthorityPort: Send + Sync {
    fn resolve(
        &self,
        session_id: &str,
        agent_id: &str,
        artifact_id: Option<&str>,
    ) -> Result<PathBuf, &'static str>;
}

pub(crate) trait ManualNativeToolOperationPort: Send + Sync {
    fn save(&self, operation: &StoredToolOperation) -> Result<(), ()>;
}

pub(crate) struct ManualNativeToolRequest {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) tool_name: String,
    pub(crate) input: Value,
    pub(crate) authority_artifact_id: Option<String>,
}

pub(crate) struct ManualNativeToolResult {
    pub(crate) operation_id: String,
    pub(crate) result: NativeToolResultEnvelope,
}

#[derive(Clone)]
pub(crate) struct ManualNativeToolService {
    dispatcher: NativeToolDispatcher,
    permissions: Arc<dyn AgentPermissionPort>,
    authority: Arc<dyn ManualNativeToolAuthorityPort>,
    operations: Arc<dyn ManualNativeToolOperationPort>,
    waits: Arc<ManualApprovalWaits>,
}

impl ManualNativeToolService {
    pub(crate) fn new(
        dispatcher: NativeToolDispatcher,
        permissions: Arc<dyn AgentPermissionPort>,
        authority: Arc<dyn ManualNativeToolAuthorityPort>,
        operations: Arc<dyn ManualNativeToolOperationPort>,
    ) -> Self {
        Self {
            dispatcher,
            permissions,
            authority,
            operations,
            waits: Arc::new(ManualApprovalWaits::default()),
        }
    }

    pub(crate) fn execute(
        &self,
        request: ManualNativeToolRequest,
    ) -> Result<ManualNativeToolResult, NativeToolDispatchError> {
        let workspace = self
            .authority
            .resolve(
                &request.session_id,
                &request.agent_id,
                request.authority_artifact_id.as_deref(),
            )
            .map_err(|code| dispatch_error(NativeToolErrorCode::Ineligible, code))?;
        let operation_id = format!("manual-tool-{}", Uuid::new_v4());
        let generation_id = format!("manual-generation-{}", Uuid::new_v4());
        let cancelled = Arc::new(AtomicBool::new(false));
        let recorder = Arc::new(
            ManualOperationRecorder::new(
                self.operations.clone(),
                &operation_id,
                &request,
                &generation_id,
            )
            .map_err(|()| {
                dispatch_error(
                    NativeToolErrorCode::InternalFailure,
                    "native tool operation persistence failed",
                )
            })?,
        );
        self.waits
            .register(&operation_id, &request.session_id, cancelled.clone())?;
        let authority = ToolEligibilityContext {
            agent_id: request.agent_id.clone(),
            session_id: request.session_id.clone(),
            generation_id: generation_id.clone(),
            canonical_workspace: Some(workspace.clone()),
            execution_mode: NativeToolExecutionMode::Execute,
            readiness: self.dispatcher.readiness_snapshot(),
        };
        let execution = NativeToolExecutionContext {
            call_id: operation_id.clone(),
            session_id: request.session_id,
            generation_id,
            agent_id: request.agent_id,
            canonical_workspace: Some(workspace),
            deadline: Instant::now() + MANUAL_TOOL_TIMEOUT,
            cancelled,
            progress: recorder.clone(),
        };
        let outcome = self.execute_prepared(
            request.tool_name,
            request.input,
            authority,
            execution,
            &recorder,
        );
        match &outcome {
            Ok(result) => recorder.complete(result),
            Err(error) => recorder.transition(
                StoredToolOperationStatus::Failed,
                Some(error.code.as_str().to_owned()),
                Vec::new(),
            ),
        }
        self.waits.remove(&operation_id);
        outcome.map(|result| ManualNativeToolResult {
            operation_id,
            result,
        })
    }

    pub(crate) fn resolve_approval(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> bool {
        self.waits.resolve(session_id, call_id, decision)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> bool {
        self.waits.cancel(operation_id)
    }

    fn execute_prepared(
        &self,
        tool_name: String,
        input: Value,
        authority: ToolEligibilityContext,
        execution: NativeToolExecutionContext,
        recorder: &Arc<ManualOperationRecorder>,
    ) -> Result<NativeToolResultEnvelope, NativeToolDispatchError> {
        let prepared = self.dispatcher.prepare(NativeToolDispatchRequest {
            tool_name,
            input,
            authority,
            execution: execution.clone(),
        })?;
        let project_key = execution
            .canonical_workspace
            .as_deref()
            .and_then(std::path::Path::to_str)
            .unwrap_or("");
        let mut witness =
            self.dispatcher
                .authorize(&prepared, self.permissions.as_ref(), project_key)?;
        if witness.status == NativeToolAuthorizationStatus::AwaitingApproval {
            recorder.transition(
                StoredToolOperationStatus::AwaitingApproval,
                None,
                Vec::new(),
            );
            match self.waits.wait(&execution.call_id, execution.deadline) {
                Some(ToolApprovalDecision::Approved) => {
                    witness.status = NativeToolAuthorizationStatus::Allowed;
                }
                // An answer delivered to a call that asked for permission means the two
                // resolution paths were crossed; deny rather than treat it as consent.
                Some(ToolApprovalDecision::Denied | ToolApprovalDecision::Answered(_)) => {
                    return Ok(terminal_result(
                        NativeToolResultStatus::Denied,
                        NativeToolErrorCode::PermissionDenied,
                    ));
                }
                None => {
                    let code = if execution.is_cancelled() {
                        NativeToolErrorCode::Cancelled
                    } else {
                        NativeToolErrorCode::DeadlineExceeded
                    };
                    return Ok(terminal_result(
                        if code == NativeToolErrorCode::Cancelled {
                            NativeToolResultStatus::Cancelled
                        } else {
                            NativeToolResultStatus::Failed
                        },
                        code,
                    ));
                }
            }
        }
        recorder.transition(StoredToolOperationStatus::Running, None, Vec::new());
        self.dispatcher.execute_authorized(prepared, &witness)
    }
}

fn dispatch_error(code: NativeToolErrorCode, message: &str) -> NativeToolDispatchError {
    NativeToolDispatchError {
        code,
        safe_message: message.to_owned(),
    }
}

fn terminal_result(
    status: NativeToolResultStatus,
    code: NativeToolErrorCode,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: 1,
        status,
        output: None,
        error_code: Some(code),
        safe_error: Some(code.as_str().to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}
