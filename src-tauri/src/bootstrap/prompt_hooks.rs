use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::prompt_hooks::application::PromptHookApplicationService;
use crate::contexts::tooling::prompt_hooks::infrastructure::{
    SqlitePromptHookRepository, SystemPromptHookClock, UnifiedPromptHookLoggingAdapter,
    UuidPromptHookTraceIds,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_prompt_hook_api(
    database: NativeDatabase,
    fallback_log_dir: PathBuf,
) -> PromptHookApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    PromptHookApi::new(PromptHookApplicationService::new(
        Arc::new(SqlitePromptHookRepository::new(database)),
        Arc::new(SystemPromptHookClock),
        Arc::new(UuidPromptHookTraceIds),
        Arc::new(UnifiedPromptHookLoggingAdapter::new(logging)),
    ))
}
