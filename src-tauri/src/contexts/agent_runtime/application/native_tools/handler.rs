use super::{
    CanonicalToolResource, NativeToolDefinition, NativeToolErrorCode, NativeToolExecutionContext,
    NativeToolOperation, NativeToolResultEnvelope, ToolEligibility, ToolEligibilityContext,
    ValidatedNativeToolInput,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolHandlerError {
    pub(crate) code: NativeToolErrorCode,
    pub(crate) safe_message: String,
}

impl NativeToolHandlerError {
    pub(crate) fn new(code: NativeToolErrorCode, safe_message: impl Into<String>) -> Self {
        Self {
            code,
            safe_message: safe_message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolPermissionRequest {
    pub(crate) action: Action,
    pub(crate) resource: Resource,
    pub(crate) operation: NativeToolOperation,
    pub(crate) canonical_resource: CanonicalToolResource,
    pub(crate) input_hash: String,
}

pub(crate) trait NativeToolHandler: Send + Sync {
    fn definition(&self) -> &NativeToolDefinition;

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility;

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError>;

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest;

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope;

    fn cleanup(&self, _context: &NativeToolExecutionContext) -> Result<(), NativeToolHandlerError> {
        Ok(())
    }
}
