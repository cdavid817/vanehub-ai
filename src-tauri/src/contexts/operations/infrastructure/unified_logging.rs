use crate::contexts::operations::application::{
    ApplicationError, DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity,
    OperationLog, OperationLogPort,
};
use crate::platform::logging;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub(crate) struct UnifiedLoggingAdapter {
    log_directory: LogDirectory,
    external: Arc<RwLock<Option<Arc<dyn ExternalLogExportPort>>>>,
}

#[derive(Debug, Clone)]
enum LogDirectory {
    #[cfg(test)]
    Fixed(PathBuf),
    Active {
        fallback: PathBuf,
    },
}

impl UnifiedLoggingAdapter {
    #[cfg(test)]
    pub(crate) fn new(log_dir: PathBuf) -> Self {
        Self {
            log_directory: LogDirectory::Fixed(log_dir),
            external: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn active(fallback: PathBuf) -> Self {
        Self {
            log_directory: LogDirectory::Active { fallback },
            external: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn attach_external_exporter(&self, exporter: Arc<dyn ExternalLogExportPort>) {
        if let Ok(mut current) = self.external.write() {
            *current = Some(exporter);
        }
    }

    pub(crate) fn write_legacy(
        &self,
        level: logging::LogLevel,
        category: &str,
        message: &str,
        context: BTreeMap<String, String>,
    ) -> Result<(), logging::LogStoreError> {
        logging::write_message_raw(&self.log_dir(), level, category, message, context)
    }

    fn log_dir(&self) -> PathBuf {
        match &self.log_directory {
            #[cfg(test)]
            LogDirectory::Fixed(log_dir) => log_dir.clone(),
            LogDirectory::Active { fallback } => logging::active_log_dir(fallback.clone()),
        }
    }

    fn write(
        &self,
        severity: LogSeverity,
        category: &str,
        message: &str,
        context: BTreeMap<String, String>,
    ) -> Result<(), ApplicationError> {
        self.write_legacy(
            to_legacy_level(severity),
            category,
            message,
            context.clone(),
        )
        .map_err(|_| {
            ApplicationError::infrastructure(
                "logging",
                "The diagnostic log could not be persisted.",
            )
        })?;
        if let Ok(exporter) = self.external.read() {
            if let Some(exporter) = exporter.as_ref() {
                let (message, context) = logging::redact_log_fields(message, context);
                let _ = exporter.export_log(&DiagnosticLog {
                    severity,
                    category: category.to_string(),
                    message,
                    context,
                });
            }
        }
        Ok(())
    }
}

impl DiagnosticLogPort for UnifiedLoggingAdapter {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
        self.write(log.severity, &log.category, &log.message, log.context)
    }
}

impl OperationLogPort for UnifiedLoggingAdapter {
    fn write_operation(&self, log: OperationLog) -> Result<(), ApplicationError> {
        let mut context = log.context;
        context.insert("operationId".to_string(), log.operation_id);
        self.write(log.severity, &log.category, &log.message, context)
    }
}

fn to_legacy_level(severity: LogSeverity) -> logging::LogLevel {
    match severity {
        LogSeverity::Error => logging::LogLevel::Error,
        LogSeverity::Warn => logging::LogLevel::Warn,
        LogSeverity::Info => logging::LogLevel::Info,
        LogSeverity::Debug => logging::LogLevel::Debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingExternalLogs {
        logs: Mutex<Vec<DiagnosticLog>>,
    }

    impl ExternalLogExportPort for CapturingExternalLogs {
        fn export_log(&self, log: &DiagnosticLog) -> Result<(), ApplicationError> {
            self.logs.lock().expect("logs").push(log.clone());
            Ok(())
        }
    }

    #[test]
    fn adapter_preserves_levels_and_operation_association_with_redaction_parity() {
        let directory = TempDirectory::new("unified-log-adapter");
        let adapter = UnifiedLoggingAdapter::new(directory.path().to_path_buf());
        let mut diagnostic_context = BTreeMap::new();
        diagnostic_context.insert("api_key".to_string(), "diagnostic-secret".to_string());
        adapter
            .write_diagnostic(DiagnosticLog {
                severity: LogSeverity::Warn,
                category: "runtime.health".to_string(),
                message: "Bearer diagnostic-token".to_string(),
                context: diagnostic_context,
            })
            .expect("diagnostic log");

        let mut operation_context = BTreeMap::new();
        operation_context.insert("operationId".to_string(), "spoofed".to_string());
        operation_context.insert("token".to_string(), "operation-secret".to_string());
        adapter
            .write_operation(OperationLog {
                operation_id: "op-fixture-17".to_string(),
                severity: LogSeverity::Info,
                category: "sdk.operation".to_string(),
                message: "installed password=operation-password".to_string(),
                context: operation_context,
            })
            .expect("operation log");

        let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
            .expect("unified log");
        let entries = raw
            .lines()
            .map(|line| serde_json::from_str::<logging::LogEntry>(line).expect("log entry"))
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, logging::LogLevel::Warn);
        assert_eq!(entries[1].level, logging::LogLevel::Info);
        assert_eq!(entries[1].category, "sdk.operation");
        assert_eq!(
            entries[1].context.get("operationId").map(String::as_str),
            Some("op-fixture-17")
        );
        assert!(raw.contains("[REDACTED]"));
        for secret in [
            "diagnostic-secret",
            "diagnostic-token",
            "operation-secret",
            "operation-password",
            "spoofed",
        ] {
            assert!(!raw.contains(secret), "redaction leaked {secret}");
        }
    }

    #[test]
    fn adapter_maps_storage_failures_to_a_command_safe_application_error() {
        let directory = TempDirectory::new("unified-log-adapter-error");
        let file = directory.write("not-a-directory", "fixture");
        let adapter = UnifiedLoggingAdapter::new(file);

        let error = adapter
            .write_diagnostic(DiagnosticLog {
                severity: LogSeverity::Error,
                category: "runtime.failure".to_string(),
                message: "failed".to_string(),
                context: BTreeMap::new(),
            })
            .expect_err("invalid log directory");

        assert_eq!(
            error.to_string(),
            "The diagnostic log could not be persisted."
        );
        assert!(!error.to_string().contains("not-a-directory"));
    }

    #[test]
    fn jsonl_and_external_export_receive_the_same_redacted_content() {
        let directory = TempDirectory::new("unified-log-external-redaction");
        let adapter = UnifiedLoggingAdapter::new(directory.path().to_path_buf());
        let external = Arc::new(CapturingExternalLogs::default());
        adapter.attach_external_exporter(external.clone());
        adapter
            .write_diagnostic(DiagnosticLog {
                severity: LogSeverity::Error,
                category: "execution.test".to_string(),
                message: "Bearer message-secret at C:\\Users\\developer\\private.json".to_string(),
                context: BTreeMap::from([
                    (
                        "authorization".to_string(),
                        "Bearer header-secret".to_string(),
                    ),
                    (
                        "detail".to_string(),
                        "password=context-secret /home/developer/private.env".to_string(),
                    ),
                ]),
            })
            .expect("write");

        let jsonl =
            std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME)).expect("jsonl");
        let exported = external.logs.lock().expect("logs");
        let external_rendered = format!("{:?}", exported[0]);
        for secret in [
            "message-secret",
            "header-secret",
            "context-secret",
            "developer",
            "private.json",
            "private.env",
        ] {
            assert!(!jsonl.contains(secret), "JSONL leaked {secret}");
            assert!(
                !external_rendered.contains(secret),
                "export leaked {secret}"
            );
        }
    }
}
