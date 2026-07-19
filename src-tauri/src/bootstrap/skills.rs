use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::skills::api::SkillApi;
use crate::contexts::tooling::skills::application::SkillApplicationService;
use crate::contexts::tooling::skills::infrastructure::{
    CurrentWorkspaceSelection, ManagedSkillFilesystem, SqliteSkillRepository, SystemSkillClock,
    UnifiedSkillLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_skill_api(database: NativeDatabase, fallback_log_dir: PathBuf) -> SkillApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    SkillApi::new(SkillApplicationService::new(
        Arc::new(SqliteSkillRepository::new(database)),
        Arc::new(ManagedSkillFilesystem::new()),
        Arc::new(CurrentWorkspaceSelection),
        Arc::new(SystemSkillClock),
        Arc::new(UnifiedSkillLoggingAdapter::new(logging)),
    ))
}
