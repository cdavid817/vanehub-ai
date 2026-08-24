use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::tooling::cli_parameters::application::ports::{
    CliParameterDiagnosticsPort, CliParameterDirectoryPort,
};
use crate::contexts::tooling::cli_parameters::domain::diagnostic::{
    CliParameterDiagnostic, CliParameterDiagnosticSeverity,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// Routes CLI parameter diagnostics through the unified logging service. Only stable ids, codes,
/// and the bounded details the domain already redacted are written.
#[derive(Clone)]
pub(crate) struct UnifiedCliParameterDiagnostics {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedCliParameterDiagnostics {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl CliParameterDiagnosticsPort for UnifiedCliParameterDiagnostics {
    fn emit(&self, diagnostic: &CliParameterDiagnostic) {
        let severity = match diagnostic.severity {
            CliParameterDiagnosticSeverity::Error => LogSeverity::Error,
            CliParameterDiagnosticSeverity::Warning => LogSeverity::Warn,
            CliParameterDiagnosticSeverity::Info => LogSeverity::Info,
        };
        let mut context = BTreeMap::from([
            ("agentId".to_string(), diagnostic.agent_id.clone()),
            ("code".to_string(), diagnostic.code.as_str().to_string()),
        ]);
        if let Some(parameter_id) = &diagnostic.parameter_id {
            context.insert("parameterId".to_string(), parameter_id.clone());
        }
        for (key, value) in &diagnostic.details {
            context.insert(key.clone(), value.clone());
        }
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: "cli.parameter".to_string(),
            message: diagnostic.code.as_str().to_string(),
            context,
        });
    }
}

/// Existence-only probe. It deliberately does not read, list, or walk the directory.
#[derive(Clone, Default)]
pub(crate) struct FilesystemDirectoryProbe;

impl CliParameterDirectoryPort for FilesystemDirectoryProbe {
    fn directory_exists(&self, path: &str) -> bool {
        !path.trim().is_empty() && Path::new(path).is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnosticCode;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingLog {
        entries: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for RecordingLog {
        fn write_diagnostic(
            &self,
            log: DiagnosticLog,
        ) -> Result<(), crate::contexts::operations::api::OperationsError> {
            self.entries.lock().expect("lock").push(log);
            Ok(())
        }
    }

    #[test]
    fn a_diagnostic_is_written_with_stable_ids_and_no_raw_value() {
        let log = Arc::new(RecordingLog::default());
        let adapter = UnifiedCliParameterDiagnostics::new(log.clone());
        let diagnostic = CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::LegacySelectionQuarantined,
            "codex-cli",
            Some("model".to_string()),
        )
        .with_redacted_detail("storedValue", "sk-live-secret");
        adapter.emit(&diagnostic);

        let entries = log.entries.lock().expect("lock");
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.category, "cli.parameter");
        assert_eq!(entry.message, "LEGACY_SELECTION_QUARANTINED");
        assert_eq!(
            entry.context.get("agentId").map(String::as_str),
            Some("codex-cli")
        );
        assert_eq!(
            entry.context.get("parameterId").map(String::as_str),
            Some("model")
        );
        assert!(!format!("{:?}", entry.context).contains("sk-live-secret"));
    }

    #[test]
    fn severity_maps_onto_the_unified_log_levels() {
        let log = Arc::new(RecordingLog::default());
        let adapter = UnifiedCliParameterDiagnostics::new(log.clone());
        for code in [
            CliParameterDiagnosticCode::ConflictingSelection,
            CliParameterDiagnosticCode::CliNotInstalled,
            CliParameterDiagnosticCode::LegacySelectionMigrated,
        ] {
            adapter.emit(&CliParameterDiagnostic::new(code, "claude-code", None));
        }
        let entries = log.entries.lock().expect("lock");
        assert_eq!(
            entries
                .iter()
                .map(|entry| format!("{:?}", entry.severity))
                .collect::<Vec<_>>(),
            ["Error", "Warn", "Info"]
        );
    }

    #[test]
    fn the_directory_probe_rejects_empty_and_missing_paths() {
        let probe = FilesystemDirectoryProbe;
        assert!(!probe.directory_exists(""));
        assert!(!probe.directory_exists("   "));
        assert!(!probe.directory_exists("/definitely/not/here/vanehub-cli-parameters"));
        assert!(probe.directory_exists(
            std::env::temp_dir()
                .to_str()
                .expect("temp dir is valid utf-8")
        ));
    }
}
