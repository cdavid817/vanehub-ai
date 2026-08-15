use crate::contexts::agent_runtime::application::{
    NativeToolErrorCode, NativeToolResultEnvelope, NativeToolResultStatus,
    IMAGE_ARTIFACT_METADATA_KEY, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::browser_automation::application::BrowserAction;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Clone, Copy)]
pub(super) enum AdapterError {
    InvalidInput,
    UnsafeTarget,
    StaleApproval,
    ExplicitUserResumeRequired,
    Cancelled,
    Deadline,
    BrowserFailure,
    UnsafeResult,
    Unavailable,
    Internal,
}

impl AdapterError {
    pub(super) const fn status(self) -> NativeToolResultStatus {
        match self {
            Self::Cancelled => NativeToolResultStatus::Cancelled,
            Self::ExplicitUserResumeRequired => NativeToolResultStatus::Denied,
            Self::Unavailable => NativeToolResultStatus::Unavailable,
            _ => NativeToolResultStatus::Failed,
        }
    }

    const fn code(self) -> NativeToolErrorCode {
        match self {
            Self::InvalidInput => NativeToolErrorCode::InvalidInput,
            Self::UnsafeTarget | Self::ExplicitUserResumeRequired => {
                NativeToolErrorCode::PermissionDenied
            }
            Self::StaleApproval => NativeToolErrorCode::StaleApproval,
            Self::Cancelled => NativeToolErrorCode::Cancelled,
            Self::Deadline => NativeToolErrorCode::DeadlineExceeded,
            Self::UnsafeResult => NativeToolErrorCode::IntegrityFailure,
            Self::Unavailable => NativeToolErrorCode::Unavailable,
            Self::BrowserFailure => NativeToolErrorCode::ExternalFailure,
            Self::Internal => NativeToolErrorCode::InternalFailure,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::ExplicitUserResumeRequired => "Browser control must be resumed by the user.",
            Self::UnsafeTarget => "The Browser target is not allowed.",
            Self::StaleApproval => "The Browser approval is stale.",
            Self::Cancelled => "The Browser operation was cancelled.",
            Self::Deadline => "The Browser operation deadline was reached.",
            Self::Unavailable => "The Browser dependency is unavailable.",
            Self::InvalidInput => "The Browser operation input is invalid.",
            Self::UnsafeResult => "The Browser returned an unsafe result.",
            Self::BrowserFailure => "The Browser operation failed safely.",
            Self::Internal => "The Browser operation could not be completed.",
        }
    }
}

pub(super) fn envelope(
    status: NativeToolResultStatus,
    output: Option<Value>,
    error: Option<AdapterError>,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output,
        error_code: error.map(AdapterError::code),
        safe_error: error.map(|value| value.message().to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}

/// Declares that a successful result's Artifact is an image the model may look at
/// (`add-onepiece-visual-tool-returns`). Carries the id the adapter already sealed, never bytes:
/// this metadata is persisted on the operation record.
pub(super) fn with_image_artifact(
    mut envelope: NativeToolResultEnvelope,
    artifact_id: &str,
) -> NativeToolResultEnvelope {
    envelope.metadata.insert(
        IMAGE_ARTIFACT_METADATA_KEY.to_owned(),
        Value::String(artifact_id.to_owned()),
    );
    envelope
}

pub(super) fn string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, AdapterError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or(AdapterError::InvalidInput)
}

pub(super) fn action(operation: &str) -> Result<BrowserAction, AdapterError> {
    match operation {
        "navigate" => Ok(BrowserAction::Navigate),
        "back" => Ok(BrowserAction::GoBack),
        "forward" => Ok(BrowserAction::GoForward),
        "inspect" => Ok(BrowserAction::Inspect),
        "click" => Ok(BrowserAction::Click),
        "type" => Ok(BrowserAction::Fill),
        "screenshot" => Ok(BrowserAction::Screenshot),
        "evaluate" => Ok(BrowserAction::Evaluate),
        "extract" => Ok(BrowserAction::Extract),
        _ => Err(AdapterError::InvalidInput),
    }
}

pub(super) fn action_input(operation: &str, object: &Map<String, Value>) -> Value {
    let mut input = object.clone();
    for field in ["operation", "page_id", "page_origin"] {
        input.remove(field);
    }
    if operation == "type" {
        return json!({"selector": input.get("selector"), "text": input.get("text")});
    }
    Value::Object(input)
}

pub(super) fn ensure_claimed_origin(
    object: &Map<String, Value>,
    current_url: Option<&str>,
) -> Result<(), AdapterError> {
    let claimed = Url::parse(string(object, "page_origin")?)
        .map_err(|_| AdapterError::InvalidInput)?
        .origin()
        .ascii_serialization();
    let actual = Url::parse(current_url.ok_or(AdapterError::StaleApproval)?)
        .map_err(|_| AdapterError::StaleApproval)?
        .origin()
        .ascii_serialization();
    if claimed != actual {
        return Err(AdapterError::StaleApproval);
    }
    Ok(())
}

#[cfg(test)]
mod image_declaration_tests {
    use super::*;
    use serde_json::json;

    fn succeeded(output: serde_json::Value) -> NativeToolResultEnvelope {
        envelope(NativeToolResultStatus::Succeeded, Some(output), None)
    }

    /// The declaration carries the id the adapter already sealed. Never bytes: this metadata is
    /// persisted on the operation record.
    #[test]
    fn declaring_an_image_adds_the_identifier_and_nothing_else() {
        let result = with_image_artifact(succeeded(json!({ "payload": {} })), "artifact-7");

        assert_eq!(
            result.metadata[IMAGE_ARTIFACT_METADATA_KEY],
            json!("artifact-7")
        );
        assert_eq!(result.metadata.len(), 1);
        let encoded = serde_json::to_string(&result.metadata).expect("metadata");
        assert!(!encoded.contains("base64"), "{encoded}");
    }

    #[test]
    fn an_undeclared_result_carries_no_image_key() {
        let result = succeeded(json!({ "payload": {} }));
        assert!(!result.metadata.contains_key(IMAGE_ARTIFACT_METADATA_KEY));
    }
}
