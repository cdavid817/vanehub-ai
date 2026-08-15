use super::{NativeToolExecutionContext, NativeToolResultEnvelope, ValidatedNativeToolInput};
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct NativeToolPortRequest {
    pub(crate) input: ValidatedNativeToolInput,
    pub(crate) context: NativeToolExecutionContext,
}

pub(crate) trait BrowserAutomationPort: Send + Sync {
    fn execute_browser(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait WebResearchPort: Send + Sync {
    fn execute_web(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait CodeExecutionPort: Send + Sync {
    fn execute_code(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait OcrInferencePort: Send + Sync {
    fn execute_ocr(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait ArtifactPort: Send + Sync {
    fn execute_artifact(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

/// Runs one bounded child OnePiece attempt. Lives in infrastructure because a child needs
/// provider access, which the application layer deliberately does not have
/// (`add-onepiece-subagents`).
pub(crate) trait SubagentPort: Send + Sync {
    fn execute_subagent(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait CliDelegationPort: Send + Sync {
    fn execute_delegation(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait ChangeSetApplyPort: Send + Sync {
    fn execute_change_set_apply(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope;
}

pub(crate) trait BrowserHandoffControlPort: Send + Sync {
    fn get_handoff(&self, operation_id: &str) -> Result<Value, ()>;
    fn begin_handoff(&self, operation_id: &str) -> Result<Value, ()>;
    fn resume_automation(&self, operation_id: &str, ownership_token: &str) -> Result<Value, ()>;
}
