use crate::contexts::operations::api::{
    LogSeverity, OperationKind, OperationLog, OperationLogPort, OperationsApi,
};
use crate::contexts::tooling::sdk::application::{
    SdkApplicationError, SdkClockPort, SdkLogEvent, SdkLogLevel, SdkLoggingPort, SdkOperationPort,
    SdkOperationResult, StartedSdkOperation,
};
use crate::contexts::tooling::sdk::domain::{SdkId, SdkOperationType};
use crate::platform::clock::SystemClock;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemSdkClock;

impl SdkClockPort for SystemSdkClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct SdkOperationAdapter {
    operations: OperationsApi,
}

impl SdkOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl SdkOperationPort for SdkOperationAdapter {
    fn start(
        &self,
        sdk_id: SdkId,
        _operation: SdkOperationType,
        message: String,
    ) -> Result<StartedSdkOperation, SdkApplicationError> {
        self.operations
            .start(
                OperationKind::Sdk,
                Some(sdk_id.as_str().to_string()),
                Some(message),
            )
            .map(|operation| StartedSdkOperation {
                id: operation.id,
                related_entity_id: operation.related_entity_id,
                message: operation.message,
                created_at: operation.created_at,
                updated_at: operation.updated_at,
            })
            .map_err(operation_error)
    }

    fn append_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        self.operations
            .append_log(&event.operation_id, event.line.clone())
            .map(|_| ())
            .map_err(operation_error)
    }

    fn complete(&self, result: &SdkOperationResult) -> Result<(), SdkApplicationError> {
        let payload = serde_json::to_value(OperationPayload::from(result))
            .map_err(|error| SdkApplicationError::Validation(error.to_string()))?;
        self.operations
            .complete(&result.operation_id, Some(payload))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), SdkApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedSdkLoggingAdapter {
    logging: Arc<dyn OperationLogPort>,
}

impl UnifiedSdkLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn OperationLogPort>) -> Self {
        Self { logging }
    }
}

impl SdkLoggingPort for UnifiedSdkLoggingAdapter {
    fn record(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        let mut context = event.context.clone();
        context.insert("sdkId".to_string(), event.sdk_id.as_str().to_string());
        context.insert(
            "operation".to_string(),
            operation_str(event.operation).to_string(),
        );
        self.logging
            .write_operation(OperationLog {
                operation_id: event.operation_id.clone(),
                severity: severity(event.level),
                category: "sdk.operation".to_string(),
                message: event.line.clone(),
                context,
            })
            .map_err(logging_error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationPayload<'a> {
    success: bool,
    operation_id: &'a str,
    sdk_id: &'a str,
    operation: &'a str,
    installed_version: &'a Option<String>,
    requested_version: &'a Option<String>,
    logs: Vec<OperationLogPayload<'a>>,
    error: &'a Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationLogPayload<'a> {
    sdk_id: &'a str,
    operation: &'a str,
    line: &'a str,
    timestamp: &'a str,
}

impl<'a> From<&'a SdkOperationResult> for OperationPayload<'a> {
    fn from(result: &'a SdkOperationResult) -> Self {
        Self {
            success: result.success,
            operation_id: &result.operation_id,
            sdk_id: result.sdk_id.as_str(),
            operation: operation_str(result.operation),
            installed_version: &result.installed_version,
            requested_version: &result.requested_version,
            logs: result
                .logs
                .iter()
                .map(|log| OperationLogPayload {
                    sdk_id: log.sdk_id.as_str(),
                    operation: operation_str(log.operation),
                    line: &log.line,
                    timestamp: &log.timestamp,
                })
                .collect(),
            error: &result.error,
        }
    }
}

fn severity(level: SdkLogLevel) -> LogSeverity {
    match level {
        SdkLogLevel::Error => LogSeverity::Error,
        SdkLogLevel::Warn => LogSeverity::Warn,
        SdkLogLevel::Info => LogSeverity::Info,
        SdkLogLevel::Debug => LogSeverity::Debug,
    }
}

fn operation_str(operation: SdkOperationType) -> &'static str {
    match operation {
        SdkOperationType::Install => "install",
        SdkOperationType::Update => "update",
        SdkOperationType::Rollback => "rollback",
        SdkOperationType::Uninstall => "uninstall",
    }
}

fn operation_error(error: impl std::fmt::Display) -> SdkApplicationError {
    SdkApplicationError::Operation(error.to_string())
}

fn logging_error(error: impl std::fmt::Display) -> SdkApplicationError {
    SdkApplicationError::Logging(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::OperationStatus;
    use crate::contexts::operations::infrastructure::{operation_service, UnifiedLoggingAdapter};
    use crate::contexts::tooling::sdk::application::SdkOperationLog;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn operation_adapter_preserves_sdk_task_and_result_payload() {
        let service = operation_service();
        let adapter = SdkOperationAdapter::new(OperationsApi::new(service.clone()));
        let started = adapter
            .start(
                SdkId::ClaudeSdk,
                SdkOperationType::Rollback,
                "Rollback SDK operation".to_string(),
            )
            .expect("start");
        adapter
            .complete(&SdkOperationResult {
                success: true,
                operation_id: started.id.clone(),
                sdk_id: SdkId::ClaudeSdk,
                operation: SdkOperationType::Rollback,
                installed_version: Some("0.2.58".to_string()),
                requested_version: Some("0.2.58".to_string()),
                logs: vec![SdkOperationLog {
                    sdk_id: SdkId::ClaudeSdk,
                    operation: SdkOperationType::Rollback,
                    line: "installed".to_string(),
                    timestamp: "now".to_string(),
                }],
                error: None,
            })
            .expect("complete");

        let operation = service.get(&started.id).expect("operation");
        assert_eq!(operation.status, OperationStatus::Succeeded);
        assert_eq!(
            operation.result,
            Some(serde_json::json!({
                "success": true,
                "operationId": started.id,
                "sdkId": "claude-sdk",
                "operation": "rollback",
                "installedVersion": "0.2.58",
                "requestedVersion": "0.2.58",
                "logs": [{
                    "sdkId": "claude-sdk",
                    "operation": "rollback",
                    "line": "installed",
                    "timestamp": "now"
                }],
                "error": null
            }))
        );
    }

    #[test]
    fn logging_adapter_associates_operation_and_redacts_output() {
        let directory = TempDirectory::new("sdk-runtime-logging");
        let adapter = UnifiedSdkLoggingAdapter::new(Arc::new(UnifiedLoggingAdapter::new(
            directory.path().to_path_buf(),
        )));
        adapter
            .record(&SdkLogEvent {
                operation_id: "sdk-op-1".to_string(),
                sdk_id: SdkId::CodexSdk,
                operation: SdkOperationType::Install,
                level: SdkLogLevel::Info,
                line: "installed token=operation-secret".to_string(),
                timestamp: "now".to_string(),
                context: BTreeMap::new(),
            })
            .expect("log");

        let raw = std::fs::read_to_string(
            directory
                .path()
                .join(crate::platform::logging::LOG_FILE_NAME),
        )
        .expect("log file");
        assert!(raw.contains("\"category\":\"sdk.operation\""));
        assert!(raw.contains("\"operationId\":\"sdk-op-1\""));
        assert!(raw.contains("\"sdkId\":\"codex-sdk\""));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("operation-secret"));
    }
}
