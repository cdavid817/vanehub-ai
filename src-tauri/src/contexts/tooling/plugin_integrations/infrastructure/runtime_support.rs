use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::tooling::plugin_integrations::application::{
    PluginIntegrationApplicationError, PluginIntegrationClockPort, PluginIntegrationDiagnostic,
    PluginIntegrationDiagnosticLevel, PluginIntegrationLoggingPort,
};
use crate::platform::clock::SystemClock;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemPluginIntegrationClock;

impl PluginIntegrationClockPort for SystemPluginIntegrationClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedPluginIntegrationLoggingAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedPluginIntegrationLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl PluginIntegrationLoggingPort for UnifiedPluginIntegrationLoggingAdapter {
    fn record(
        &self,
        diagnostic: &PluginIntegrationDiagnostic,
    ) -> Result<(), PluginIntegrationApplicationError> {
        let mut context = diagnostic.context.clone();
        context.insert(
            "integrationId".to_string(),
            diagnostic.integration_id.as_str().to_string(),
        );
        context.insert("operation".to_string(), diagnostic.operation.to_string());
        context.insert(
            "safeStatus".to_string(),
            diagnostic.status.as_str().to_string(),
        );
        context.insert("checkedAt".to_string(), diagnostic.checked_at.clone());
        self.logging
            .write_diagnostic(DiagnosticLog {
                severity: severity(diagnostic.level),
                category: format!("plugin-integration.{}", diagnostic.integration_id.as_str()),
                message: diagnostic.message.clone(),
                context,
            })
            .map_err(|error| PluginIntegrationApplicationError::Logging(error.to_string()))
    }
}

fn severity(level: PluginIntegrationDiagnosticLevel) -> LogSeverity {
    match level {
        PluginIntegrationDiagnosticLevel::Info => LogSeverity::Info,
        PluginIntegrationDiagnosticLevel::Warn => LogSeverity::Warn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::contexts::tooling::plugin_integrations::domain::{
        PluginIntegrationId, PluginIntegrationStatus,
    };
    use crate::platform::logging;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn unified_adapter_preserves_category_safe_context_and_redaction() {
        let directory = TempDirectory::new("plugin-integration-log");
        let adapter = UnifiedPluginIntegrationLoggingAdapter::new(Arc::new(
            UnifiedLoggingAdapter::new(directory.path().to_path_buf()),
        ));
        let mut context = BTreeMap::new();
        context.insert("token".to_string(), "ghp_private".to_string());

        adapter
            .record(&PluginIntegrationDiagnostic {
                integration_id: PluginIntegrationId::Github,
                operation: "readiness-check",
                status: PluginIntegrationStatus::Error,
                level: PluginIntegrationDiagnosticLevel::Warn,
                message: "token=ghp_message".to_string(),
                checked_at: "2026-07-18T00:00:00Z".to_string(),
                context,
            })
            .expect("diagnostic");

        let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
            .expect("unified log");
        assert!(raw.contains("plugin-integration.github"));
        assert!(raw.contains("readiness-check"));
        assert!(raw.contains("safeStatus"));
        assert!(raw.contains("error"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("ghp_private"));
        assert!(!raw.contains("ghp_message"));
    }
}
