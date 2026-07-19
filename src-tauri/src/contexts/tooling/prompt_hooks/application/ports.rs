use super::{
    PromptHookApplicationError, PromptHookLogEvent, PromptHookOverride, PromptHookRecord,
    PromptHookTrace,
};
use crate::contexts::tooling::prompt_hooks::domain::{PromptHookBindings, PromptHookId};

pub(crate) trait PromptHookRepository: Send + Sync {
    fn list_user_hooks(&self) -> Result<Vec<PromptHookRecord>, PromptHookApplicationError>;

    fn list_builtin_overrides(&self)
        -> Result<Vec<PromptHookOverride>, PromptHookApplicationError>;

    fn create_user_hook(&self, record: &PromptHookRecord)
        -> Result<(), PromptHookApplicationError>;

    fn update_user_hook(&self, record: &PromptHookRecord)
        -> Result<(), PromptHookApplicationError>;

    fn delete_user_hook(&self, hook_id: &PromptHookId) -> Result<(), PromptHookApplicationError>;

    fn set_user_enabled(
        &self,
        hook_id: &PromptHookId,
        enabled: bool,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError>;

    fn set_user_bindings(
        &self,
        hook_id: &PromptHookId,
        bindings: &PromptHookBindings,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError>;

    fn save_builtin_override(
        &self,
        override_record: &PromptHookOverride,
    ) -> Result<(), PromptHookApplicationError>;

    fn save_traces(
        &self,
        traces: &[PromptHookTrace],
        retained_limit: usize,
    ) -> Result<(), PromptHookApplicationError>;

    fn list_traces(&self, limit: usize)
        -> Result<Vec<PromptHookTrace>, PromptHookApplicationError>;
}

pub(crate) trait PromptHookClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait PromptHookTraceIdPort: Send + Sync {
    fn next_trace_id(&self) -> String;
}

pub(crate) trait PromptHookLoggingPort: Send + Sync {
    fn record(&self, event: &PromptHookLogEvent);
}
