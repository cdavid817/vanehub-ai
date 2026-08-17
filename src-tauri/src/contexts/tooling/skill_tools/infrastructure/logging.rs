use crate::contexts::operations::api::{LogSeverity, OperationLog, OperationLogPort};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolLogEvent, SkillToolLogLevel, SkillToolLoggingPort,
};
use crate::platform::logging::redact_text;
use std::collections::BTreeMap;
use std::sync::Arc;

const MAX_LOG_VALUE_CHARS: usize = 128;

#[derive(Clone)]
pub(crate) struct UnifiedSkillToolLoggingAdapter {
    logging: Arc<dyn OperationLogPort>,
}

impl UnifiedSkillToolLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn OperationLogPort>) -> Self {
        Self { logging }
    }
}

impl SkillToolLoggingPort for UnifiedSkillToolLoggingAdapter {
    fn record(&self, event: &SkillToolLogEvent) -> Result<(), SkillToolApplicationError> {
        let mut context =
            BTreeMap::from([("event".to_string(), event.action.as_str().to_string())]);
        insert(&mut context, "skillId", event.skill_id.as_deref());
        insert(&mut context, "toolId", event.tool_id.as_deref());
        insert(&mut context, "revision", event.revision.as_deref());
        for (key, value) in &event.context {
            context.insert(sanitize(key), sanitize(value));
        }
        self.logging
            .write_operation(OperationLog {
                operation_id: event
                    .revision
                    .as_deref()
                    .map(sanitize)
                    .unwrap_or_else(|| event.action.as_str().to_string()),
                severity: severity(event.level),
                category: "tooling.skill_tool".to_string(),
                message: sanitize(&event.message),
                context,
            })
            .map_err(|error| SkillToolApplicationError::Storage(error.to_string()))
    }
}

fn insert(context: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        context.insert(key.to_string(), sanitize(value));
    }
}

fn sanitize(value: &str) -> String {
    redact_text(value)
        .chars()
        .take(MAX_LOG_VALUE_CHARS)
        .collect()
}

const fn severity(level: SkillToolLogLevel) -> LogSeverity {
    match level {
        SkillToolLogLevel::Error => LogSeverity::Error,
        SkillToolLogLevel::Warn => LogSeverity::Warn,
        SkillToolLogLevel::Info => LogSeverity::Info,
        SkillToolLogLevel::Debug => LogSeverity::Debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::ApplicationError;
    use crate::contexts::tooling::skill_tools::application::SkillToolLogAction;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Capture(Mutex<Vec<OperationLog>>);

    impl OperationLogPort for Capture {
        fn write_operation(&self, log: OperationLog) -> Result<(), ApplicationError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[test]
    fn redacts_and_bounds_every_caller_supplied_field() {
        let capture = Arc::new(Capture::default());
        let adapter = UnifiedSkillToolLoggingAdapter::new(capture.clone());
        adapter
            .record(&SkillToolLogEvent {
                action: SkillToolLogAction::HostCall,
                level: SkillToolLogLevel::Warn,
                skill_id: Some("review".to_string()),
                tool_id: Some("scan".to_string()),
                revision: Some("a".repeat(64)),
                message: "token=secret-value".to_string(),
                context: BTreeMap::from([(
                    "path".to_string(),
                    format!("/private/{}", "x".repeat(300)),
                )]),
            })
            .expect("record");
        let logs = capture.0.lock().expect("logs");
        let encoded = format!("{:?}", logs[0]);
        assert!(!encoded.contains("secret-value"));
        assert!(!encoded.contains(&"x".repeat(129)));
        assert_eq!(logs[0].category, "tooling.skill_tool");
    }
}
