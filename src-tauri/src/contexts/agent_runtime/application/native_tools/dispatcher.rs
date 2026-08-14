use super::{
    NativeToolErrorCode, NativeToolExecutionContext, NativeToolExecutionMode, NativeToolHandler,
    NativeToolRegistry, ToolEligibility, ToolEligibilityContext, ValidatedNativeToolInput,
};
use crate::contexts::agent_runtime::application::AgentPermissionPort;
use crate::contexts::permissions::api::{Action, Effect, Resource};
use serde_json::Value;
use std::sync::Arc;

use super::dispatcher_validation::revalidate_context;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolDispatchError {
    pub(crate) code: NativeToolErrorCode,
    pub(crate) safe_message: String,
}

impl NativeToolDispatchError {
    fn new(code: NativeToolErrorCode, safe_message: impl Into<String>) -> Self {
        Self {
            code,
            safe_message: safe_message.into(),
        }
    }
}

pub(crate) struct NativeToolDispatchRequest {
    pub(crate) tool_name: String,
    pub(crate) input: Value,
    pub(crate) authority: ToolEligibilityContext,
    pub(crate) execution: NativeToolExecutionContext,
}

pub(crate) struct PreparedNativeToolDispatch {
    handler: Arc<dyn NativeToolHandler>,
    validated: ValidatedNativeToolInput,
    authority: ToolEligibilityContext,
    execution: NativeToolExecutionContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolAuthorizationStatus {
    Allowed,
    AwaitingApproval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolApprovalWitness {
    pub(crate) status: NativeToolAuthorizationStatus,
    pub(crate) call_id: String,
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) workspace: Option<std::path::PathBuf>,
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) input_hash: String,
}

#[derive(Clone)]
pub(crate) struct NativeToolDispatcher {
    registry: NativeToolRegistry,
}

impl NativeToolDispatcher {
    pub(crate) fn new(registry: NativeToolRegistry) -> Self {
        Self { registry }
    }

    pub(crate) fn readiness_snapshot(
        &self,
    ) -> std::collections::BTreeMap<String, Option<super::NativeToolReadinessReasonCode>> {
        self.registry.readiness_snapshot()
    }

    pub(crate) fn prepare(
        &self,
        request: NativeToolDispatchRequest,
    ) -> Result<PreparedNativeToolDispatch, NativeToolDispatchError> {
        let handler = self.registry.handler(&request.tool_name).ok_or_else(|| {
            NativeToolDispatchError::new(
                NativeToolErrorCode::Ineligible,
                "The requested native tool is not registered.",
            )
        })?;
        if self
            .registry
            .eligibility(&request.tool_name, &request.authority)
            != ToolEligibility::Eligible
        {
            return Err(NativeToolDispatchError::new(
                NativeToolErrorCode::Ineligible,
                "The requested native tool is not eligible.",
            ));
        }
        revalidate_context(handler.as_ref(), &request)?;
        let input_bytes = serde_json::to_vec(&request.input).map_err(|_| {
            NativeToolDispatchError::new(
                NativeToolErrorCode::InvalidInput,
                "The native tool input is invalid.",
            )
        })?;
        if input_bytes.len() as u64 > handler.definition().limit_profile.max_input_bytes {
            return Err(NativeToolDispatchError::new(
                NativeToolErrorCode::LimitExceeded,
                "The native tool input exceeds its configured limit.",
            ));
        }
        let validated = handler
            .validate(&request.input)
            .map_err(|error| NativeToolDispatchError::new(error.code, error.safe_message))?;
        if !self.registry.is_operation_enabled(validated.operation) {
            return Err(NativeToolDispatchError::new(
                NativeToolErrorCode::Ineligible,
                "The requested native tool operation is disabled.",
            ));
        }
        if request.authority.execution_mode == NativeToolExecutionMode::Plan
            && !matches!(
                validated.operation,
                super::NativeToolOperation::ArtifactRead | super::NativeToolOperation::OcrRead
            )
        {
            return Err(NativeToolDispatchError::new(
                NativeToolErrorCode::PermissionDenied,
                "The native tool operation is unavailable in plan mode.",
            ));
        }
        if !handler
            .definition()
            .operations
            .contains(&validated.operation)
        {
            return Err(NativeToolDispatchError::new(
                NativeToolErrorCode::InvalidInput,
                "The requested operation is not supported by this native tool.",
            ));
        }
        Ok(PreparedNativeToolDispatch {
            handler,
            validated,
            authority: request.authority,
            execution: request.execution,
        })
    }

    pub(crate) fn authorize(
        &self,
        prepared: &PreparedNativeToolDispatch,
        permissions: &dyn AgentPermissionPort,
        project_key: &str,
    ) -> Result<NativeToolApprovalWitness, NativeToolDispatchError> {
        let request = prepared
            .handler
            .permission_request(&prepared.validated, &prepared.authority);
        let bound_resource = bind_resource_to_input(&request.resource, &request.input_hash);
        let mut effect = permissions.evaluate(
            &prepared.authority.agent_id,
            request.action.clone(),
            bound_resource.clone(),
            &prepared.authority.session_id,
            &prepared.authority.generation_id,
            project_key,
        );
        if prepared.validated.operation == super::NativeToolOperation::DelegationApply
            && effect == Effect::Allow
        {
            effect = Effect::Ask;
        }
        let status = match effect {
            Effect::Allow => NativeToolAuthorizationStatus::Allowed,
            Effect::Deny => {
                return Err(NativeToolDispatchError::new(
                    NativeToolErrorCode::PermissionDenied,
                    "The native tool operation was denied by policy.",
                ))
            }
            Effect::Ask => {
                permissions
                    .create_pending_approval(
                        &prepared.authority.agent_id,
                        request.action.clone(),
                        bound_resource.clone(),
                        &prepared.authority.session_id,
                        &prepared.authority.generation_id,
                        &prepared.execution.call_id,
                        project_key,
                    )
                    .map_err(|_| {
                        NativeToolDispatchError::new(
                            NativeToolErrorCode::InternalFailure,
                            "The native tool approval could not be created.",
                        )
                    })?;
                NativeToolAuthorizationStatus::AwaitingApproval
            }
        };
        Ok(NativeToolApprovalWitness {
            status,
            call_id: prepared.execution.call_id.clone(),
            agent_id: prepared.authority.agent_id.clone(),
            session_id: prepared.authority.session_id.clone(),
            generation_id: prepared.authority.generation_id.clone(),
            workspace: prepared.authority.canonical_workspace.clone(),
            action: request.action,
            resource: bound_resource,
            input_hash: request.input_hash,
        })
    }

    pub(crate) fn execute_authorized(
        &self,
        prepared: PreparedNativeToolDispatch,
        witness: &NativeToolApprovalWitness,
    ) -> Result<super::NativeToolResultEnvelope, NativeToolDispatchError> {
        revalidate_prepared_witness(&prepared, witness)?;
        let limits = prepared.handler.definition().limit_profile.clone();
        Ok(super::lifecycle::execute_with_lifecycle(
            prepared.handler.as_ref(),
            prepared.validated,
            prepared.execution,
            &limits,
        ))
    }
}

fn bind_resource_to_input(resource: &Resource, input_hash: &str) -> Resource {
    Resource::new(format!("{}#input={input_hash}", resource.as_str()))
}

fn revalidate_prepared_witness(
    prepared: &PreparedNativeToolDispatch,
    witness: &NativeToolApprovalWitness,
) -> Result<(), NativeToolDispatchError> {
    let request = prepared
        .handler
        .permission_request(&prepared.validated, &prepared.authority);
    let resource = bind_resource_to_input(&request.resource, &request.input_hash);
    if witness.call_id != prepared.execution.call_id
        || witness.agent_id != prepared.authority.agent_id
        || witness.session_id != prepared.authority.session_id
        || witness.generation_id != prepared.authority.generation_id
        || witness.workspace != prepared.authority.canonical_workspace
        || witness.action != request.action
        || witness.resource != resource
        || witness.input_hash != request.input_hash
    {
        return Err(NativeToolDispatchError::new(
            NativeToolErrorCode::StaleApproval,
            "The native tool approval is stale.",
        ));
    }
    if prepared.execution.is_cancelled() || prepared.execution.deadline_reached() {
        return Err(NativeToolDispatchError::new(
            NativeToolErrorCode::StaleApproval,
            "The native tool approval is no longer executable.",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "dispatcher_tests.rs"]
mod tests;
