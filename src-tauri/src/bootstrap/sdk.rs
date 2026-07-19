use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::sdk::api::SdkApi;
use crate::contexts::tooling::sdk::application::SdkApplicationService;
use crate::contexts::tooling::sdk::infrastructure::{
    SdkOperationAdapter, SdkPackageAdapter, SqliteSdkRepository, SystemSdkClock,
    UnifiedSdkLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_sdk_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> SdkApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    SdkApi::new(SdkApplicationService::new(
        Arc::new(SqliteSdkRepository::new(database)),
        Arc::new(SdkPackageAdapter::new()),
        Arc::new(SdkOperationAdapter::new(operations)),
        Arc::new(UnifiedSdkLoggingAdapter::new(logging)),
        Arc::new(SystemSdkClock),
    ))
}
