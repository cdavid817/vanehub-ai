use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillClockPort, SkillLogEvent, SkillLogLevel, SkillLoggingPort,
    SkillWorkspaceSelectionPort,
};
use crate::platform::clock::SystemClock;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemSkillClock;

impl SkillClockPort for SystemSkillClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CurrentWorkspaceSelection;

impl SkillWorkspaceSelectionPort for CurrentWorkspaceSelection {
    fn select_workspace_directory(&self) -> Result<Option<String>, SkillApplicationError> {
        crate::platform::filesystem::current_directory()
            .map(|path| Some(path.to_string_lossy().to_string()))
            .map_err(|error| SkillApplicationError::Selection(error.to_string()))
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedSkillLoggingAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedSkillLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl SkillLoggingPort for UnifiedSkillLoggingAdapter {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        let mut context = event.context.clone();
        context.insert("action".to_string(), event.action.as_str().to_string());
        context.insert("timestamp".to_string(), event.timestamp.clone());
        if let Some(skill_id) = &event.skill_id {
            context.insert("skillId".to_string(), skill_id.clone());
        }
        self.logging
            .write_diagnostic(DiagnosticLog {
                severity: match event.level {
                    SkillLogLevel::Error => LogSeverity::Error,
                    SkillLogLevel::Warn => LogSeverity::Warn,
                    SkillLogLevel::Info => LogSeverity::Info,
                },
                category: format!("skill.{}", event.action.as_str()),
                message: event.message.clone(),
                context,
            })
            .map_err(|error| SkillApplicationError::Logging(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::contexts::tooling::skills::application::{SkillLogAction, SkillLogEvent};
    use crate::platform::logging;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn unified_logging_preserves_semantics_and_redacts_skill_diagnostics() {
        let directory = TempDirectory::new("skill-unified-log");
        let adapter = UnifiedSkillLoggingAdapter::new(Arc::new(UnifiedLoggingAdapter::new(
            directory.path().to_path_buf(),
        )));
        let mut context = BTreeMap::new();
        context.insert("token".to_string(), "private-token".to_string());

        adapter
            .record(&SkillLogEvent {
                action: SkillLogAction::Import,
                level: SkillLogLevel::Warn,
                skill_id: Some("fixture-skill".to_string()),
                message: "password=private-password".to_string(),
                timestamp: "2026-07-18T00:00:00Z".to_string(),
                context,
            })
            .expect("Skill diagnostic");

        let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
            .expect("unified log");
        assert!(raw.contains("skill.import"));
        assert!(raw.contains("fixture-skill"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("private-token"));
        assert!(!raw.contains("private-password"));
    }
}
