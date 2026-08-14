use crate::contexts::agent_runtime::application::{
    NativeToolErrorCode, NativeToolResultEnvelope, NativeToolResultStatus,
    NATIVE_TOOL_CONTRACT_VERSION,
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
