use super::*;
use crate::contexts::agent_runtime::application::native_tools::{
    NativeToolExecutionMode, NativeToolResultStatus, NATIVE_TOOL_CONTRACT_VERSION,
};
use serde_json::json;

struct Port;

impl OcrInferencePort for Port {
    fn execute_ocr(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"text": "fixture"})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn context(mode: NativeToolExecutionMode) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_owned(),
        session_id: "session".to_owned(),
        generation_id: "generation".to_owned(),
        canonical_workspace: None,
        execution_mode: mode,
        readiness: BTreeMap::from([("ocr".to_owned(), None)]),
    }
}

#[test]
fn read_only_ocr_is_plan_compatible_but_publish_permission_is_effectful() {
    let handler = OcrNativeToolHandler::new(Arc::new(Port));
    assert_eq!(
        handler.eligibility(&context(NativeToolExecutionMode::Plan)),
        ToolEligibility::Eligible
    );
    let read = handler
        .validate(&json!({"artifact_id": "artifact-one", "languages": ["en"]}))
        .expect("read");
    assert_eq!(
        handler
            .permission_request(&read, &context(NativeToolExecutionMode::Plan))
            .operation,
        NativeToolOperation::OcrRead
    );
    let publish = handler
        .validate(&json!({"artifact_id": "artifact-one", "languages": ["en"], "publish": true}))
        .expect("publish");
    assert_eq!(publish.operation, NativeToolOperation::ArtifactPublish);
    assert_eq!(
        handler
            .permission_request(&publish, &context(NativeToolExecutionMode::Plan))
            .operation,
        NativeToolOperation::ArtifactPublish
    );
}

#[test]
fn arbitrary_paths_urls_and_execution_fields_are_not_in_the_contract() {
    let handler = OcrNativeToolHandler::new(Arc::new(Port));
    for input in [
        json!({"artifact_id": "artifact-one", "languages": ["en"], "path": "C:/secret.png"}),
        json!({"artifact_id": "artifact-one", "languages": ["en"], "url": "https://remote.invalid"}),
        json!({"artifact_id": "artifact-one", "languages": ["en"], "executable": "python"}),
        json!({"artifact_id": "artifact-one", "languages": ["en"], "pages": [2, 1]}),
    ] {
        assert!(handler.validate(&input).is_err());
    }
}
