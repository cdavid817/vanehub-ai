use crate::contexts::agent_runtime::application::{
    BrowserAutomationPort, ChangeSetApplyPort, CliDelegationPort, CodeExecutionPort,
    NativeToolErrorCode, NativeToolPortRequest, NativeToolResultEnvelope, NativeToolResultStatus,
    OcrInferencePort, SubagentPort, NATIVE_TOOL_CONTRACT_VERSION,
};
use std::collections::BTreeMap;

pub(crate) struct UnavailableNativeToolPort;

impl BrowserAutomationPort for UnavailableNativeToolPort {
    fn execute_browser(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

impl CodeExecutionPort for UnavailableNativeToolPort {
    fn execute_code(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

impl OcrInferencePort for UnavailableNativeToolPort {
    fn execute_ocr(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

impl CliDelegationPort for UnavailableNativeToolPort {
    fn execute_delegation(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

impl SubagentPort for UnavailableNativeToolPort {
    fn execute_subagent(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

impl ChangeSetApplyPort for UnavailableNativeToolPort {
    fn execute_change_set_apply(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        unavailable()
    }
}

fn unavailable() -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status: NativeToolResultStatus::Unavailable,
        output: None,
        error_code: Some(NativeToolErrorCode::Unavailable),
        safe_error: Some("The native tool backend is unavailable.".to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}
