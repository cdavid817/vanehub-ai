use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::tooling::prompt_hooks::application::{
    PromptHookClockPort, PromptHookLogEvent, PromptHookLogLevel, PromptHookLoggingPort,
    PromptHookTraceIdPort,
};
use crate::platform::clock::SystemClock;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemPromptHookClock;

impl PromptHookClockPort for SystemPromptHookClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidPromptHookTraceIds;

impl PromptHookTraceIdPort for UuidPromptHookTraceIds {
    fn next_trace_id(&self) -> String {
        format!("prompt-hook-trace-{}", Uuid::new_v4())
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedPromptHookLoggingAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedPromptHookLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl PromptHookLoggingPort for UnifiedPromptHookLoggingAdapter {
    fn record(&self, event: &PromptHookLogEvent) {
        let mut context = BTreeMap::new();
        context.insert("action".to_string(), event.action.as_str().to_string());
        if let Some(hook_id) = &event.hook_id {
            context.insert("hookId".to_string(), hook_id.clone());
        }
        if let Some(agent_id) = &event.agent_id {
            context.insert("agentId".to_string(), agent_id.clone());
        }
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: match event.level {
                PromptHookLogLevel::Error => LogSeverity::Error,
                PromptHookLogLevel::Info => LogSeverity::Info,
            },
            category: format!("prompt-hook.{}", event.action.as_str()),
            message: event.message.clone(),
            context,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::contexts::tooling::prompt_hooks::application::PromptHookLogAction;
    use crate::platform::logging;
    use crate::test_support::TempDirectory;

    #[test]
    fn unified_logging_preserves_semantics_and_redacts_diagnostic_values() {
        let directory = TempDirectory::new("prompt-hook-unified-log");
        let adapter = UnifiedPromptHookLoggingAdapter::new(Arc::new(UnifiedLoggingAdapter::new(
            directory.path().to_path_buf(),
        )));

        adapter.record(&PromptHookLogEvent {
            action: PromptHookLogAction::Preview,
            level: PromptHookLogLevel::Error,
            hook_id: Some("fixture-hook".to_string()),
            agent_id: Some("codex-cli".to_string()),
            message: "token=private-token".to_string(),
        });

        let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
            .expect("unified log");
        assert!(raw.contains("prompt-hook.preview"));
        assert!(raw.contains("fixture-hook"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("private-token"));
    }

    #[test]
    fn trace_ids_keep_the_existing_prefix_and_are_unique() {
        let ids = UuidPromptHookTraceIds;
        let first = ids.next_trace_id();
        let second = ids.next_trace_id();
        assert!(first.starts_with("prompt-hook-trace-"));
        assert_ne!(first, second);
    }
}
