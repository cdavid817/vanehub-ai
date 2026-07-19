use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use std::collections::BTreeMap;

pub(super) fn record_runtime_error(
    logging: &dyn DiagnosticLogPort,
    category: &str,
    operation: &str,
    error: &str,
) {
    let mut context = BTreeMap::new();
    context.insert("operation".to_string(), operation.to_string());
    context.insert("error".to_string(), error.to_string());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Error,
        category: category.to_string(),
        message: "Desktop runtime operation failed".to_string(),
        context,
    });
}

pub(super) fn record_shutdown_warning(logging: &dyn DiagnosticLogPort) {
    let mut context = BTreeMap::new();
    context.insert("operation".to_string(), "explicit-quit".to_string());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Warn,
        category: "desktop.lifecycle".to_string(),
        message: "Connector shutdown exceeded its graceful boundary".to_string(),
        context,
    });
}

pub(super) fn record_exit_requested(logging: &dyn DiagnosticLogPort) {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Info,
        category: "floating-assistant.exit".to_string(),
        message: "application exit requested".to_string(),
        context: BTreeMap::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::platform::logging::LOG_FILE_NAME;
    use crate::test_support::TempDirectory;

    #[test]
    fn runtime_errors_use_the_unified_redaction_boundary() {
        let directory = TempDirectory::new("desktop-runtime-log");
        let logging = UnifiedLoggingAdapter::new(directory.path().to_path_buf());

        record_runtime_error(
            &logging,
            "floating-assistant.window",
            "ensure",
            "password=window-secret Bearer runtime-token",
        );

        let raw = std::fs::read_to_string(directory.path().join(LOG_FILE_NAME)).expect("log");
        assert!(raw.contains("floating-assistant.window"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("window-secret"));
        assert!(!raw.contains("runtime-token"));
    }
}
