use crate::contexts::tooling::sdk::domain::{SdkId, SdkLifecyclePlan, SdkOperationType};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkEnvironmentStatus {
    pub(crate) available: bool,
    pub(crate) node_path: Option<String>,
    pub(crate) node_version: Option<String>,
    pub(crate) npm_path: Option<String>,
    pub(crate) npm_version: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdkLogLevel {
    Error,
    Warn,
    Info,
    #[expect(
        dead_code,
        reason = "SDK logging preserves the four-level log contract"
    )]
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkLogEvent {
    pub(crate) operation_id: String,
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) level: SdkLogLevel,
    pub(crate) line: String,
    pub(crate) timestamp: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkPackageLog {
    pub(crate) level: SdkLogLevel,
    pub(crate) line: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkOperationLog {
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) line: String,
    pub(crate) timestamp: String,
}

impl From<&SdkLogEvent> for SdkOperationLog {
    fn from(event: &SdkLogEvent) -> Self {
        Self {
            sdk_id: event.sdk_id,
            operation: event.operation,
            line: event.line.clone(),
            timestamp: event.timestamp.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkOperationRequest {
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedSdkOperation {
    pub(crate) id: String,
    pub(crate) related_entity_id: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkOperationResult {
    pub(crate) success: bool,
    pub(crate) operation_id: String,
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) installed_version: Option<String>,
    pub(crate) requested_version: Option<String>,
    pub(crate) logs: Vec<SdkOperationLog>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedSdkOperation {
    pub(crate) operation: StartedSdkOperation,
    pub(super) plan: SdkLifecyclePlan,
}
