use crate::contexts::operations::api::{
    LogSeverity, OperationKind, OperationLog, OperationLogPort, OperationsApi,
};
use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError, ExtensionClockPort, ExtensionLogEvent, ExtensionLogLevel,
    ExtensionLoggingPort, ExtensionOperationPort, ExtensionOperationResult,
    StartedExtensionOperation,
};
use crate::contexts::tooling::extensions::domain::{ExtensionAction, ExtensionFrameworkId};
use crate::platform::clock::SystemClock;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemExtensionClock;

impl ExtensionClockPort for SystemExtensionClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct ExtensionOperationAdapter {
    operations: OperationsApi,
}

impl ExtensionOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl ExtensionOperationPort for ExtensionOperationAdapter {
    fn start(
        &self,
        framework_id: ExtensionFrameworkId,
        _action: ExtensionAction,
        message: String,
    ) -> Result<StartedExtensionOperation, ExtensionApplicationError> {
        self.operations
            .start(
                OperationKind::Extension,
                Some(framework_id.as_str().to_string()),
                Some(message),
            )
            .map(|operation| StartedExtensionOperation {
                id: operation.id,
                related_entity_id: operation.related_entity_id,
                message: operation.message,
                created_at: operation.created_at,
                updated_at: operation.updated_at,
            })
            .map_err(operation_error)
    }

    fn append_log(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError> {
        self.operations
            .append_log(&event.operation_id, event.line.clone())
            .map(|_| ())
            .map_err(operation_error)
    }

    fn complete(&self, result: &ExtensionOperationResult) -> Result<(), ExtensionApplicationError> {
        let payload = serde_json::to_value(OperationPayload::from(result))
            .map_err(|error| ExtensionApplicationError::Operation(error.to_string()))?;
        self.operations
            .complete(&result.operation_id, Some(payload))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), ExtensionApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedExtensionLoggingAdapter {
    logging: Arc<dyn OperationLogPort>,
}

impl UnifiedExtensionLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn OperationLogPort>) -> Self {
        Self { logging }
    }
}

impl ExtensionLoggingPort for UnifiedExtensionLoggingAdapter {
    fn record(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError> {
        let mut context = event.context.clone();
        context.insert(
            "frameworkId".to_string(),
            event.framework_id.as_str().to_string(),
        );
        context.insert("action".to_string(), event.action.as_str().to_string());
        context.insert("timestamp".to_string(), event.timestamp.clone());
        self.logging
            .write_operation(OperationLog {
                operation_id: event.operation_id.clone(),
                severity: severity(event.level),
                category: "extension.operation".to_string(),
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
    framework_id: &'a str,
    action: &'a str,
    message: &'a str,
    logs: &'a [String],
    error: &'a Option<String>,
}

impl<'a> From<&'a ExtensionOperationResult> for OperationPayload<'a> {
    fn from(result: &'a ExtensionOperationResult) -> Self {
        Self {
            success: result.success,
            framework_id: result.framework_id.as_str(),
            action: result.action.as_str(),
            message: &result.message,
            logs: &result.logs,
            error: &result.error,
        }
    }
}

fn severity(level: ExtensionLogLevel) -> LogSeverity {
    match level {
        ExtensionLogLevel::Error => LogSeverity::Error,
        ExtensionLogLevel::Warn => LogSeverity::Warn,
        ExtensionLogLevel::Info => LogSeverity::Info,
        ExtensionLogLevel::Debug => LogSeverity::Debug,
    }
}

fn operation_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Operation(error.to_string())
}

fn logging_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Logging(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::OperationStatus;
    use crate::contexts::operations::infrastructure::{operation_service, UnifiedLoggingAdapter};
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn operation_adapter_preserves_extension_task_and_result_payload() {
        let service = operation_service();
        let adapter = ExtensionOperationAdapter::new(OperationsApi::new(service.clone()));
        let started = adapter
            .start(
                ExtensionFrameworkId::Paddleocr,
                ExtensionAction::Install,
                "Install local extension".to_string(),
            )
            .expect("start");
        adapter
            .complete(&ExtensionOperationResult {
                success: true,
                operation_id: started.id.clone(),
                framework_id: ExtensionFrameworkId::Paddleocr,
                action: ExtensionAction::Install,
                message: "Framework installed".to_string(),
                logs: vec!["Framework installation verified".to_string()],
                error: None,
            })
            .expect("complete");

        let operation = service.get(&started.id).expect("operation");
        assert_eq!(operation.status, OperationStatus::Succeeded);
        assert_eq!(
            operation.result,
            Some(serde_json::json!({
                "success": true,
                "frameworkId": "paddleocr",
                "action": "install",
                "message": "Framework installed",
                "logs": ["Framework installation verified"],
                "error": null
            }))
        );
    }

    #[test]
    fn logging_adapter_associates_and_redacts_without_feature_log_file() {
        let directory = TempDirectory::new("extension-runtime-logging");
        let adapter = UnifiedExtensionLoggingAdapter::new(Arc::new(UnifiedLoggingAdapter::new(
            directory.path().to_path_buf(),
        )));
        adapter
            .record(&ExtensionLogEvent {
                operation_id: "extension-op-1".to_string(),
                framework_id: ExtensionFrameworkId::SherpaOnnx,
                action: ExtensionAction::SelfTest,
                level: ExtensionLogLevel::Info,
                line: "checked token=operation-secret".to_string(),
                timestamp: "now".to_string(),
                context: BTreeMap::new(),
            })
            .expect("log");

        let raw = std::fs::read_to_string(
            directory
                .path()
                .join(crate::platform::logging::LOG_FILE_NAME),
        )
        .expect("unified log");
        assert!(raw.contains("\"category\":\"extension.operation\""));
        assert!(raw.contains("\"operationId\":\"extension-op-1\""));
        assert!(raw.contains("\"frameworkId\":\"sherpa-onnx\""));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("operation-secret"));
        assert!(!directory.path().join("extensions.log").exists());
    }
}
