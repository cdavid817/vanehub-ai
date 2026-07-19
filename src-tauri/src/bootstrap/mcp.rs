use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::mcp::api::McpApi;
use crate::contexts::tooling::mcp::application::McpApplicationService;
use crate::contexts::tooling::mcp::infrastructure::{
    CurrentProjectPathAdapter, McpOperationAdapter, RmcpConnectionAdapter,
    SqliteMcpServerRepository, SystemMcpClock, UnifiedMcpLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_mcp_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> McpApi {
    let operation_logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    McpApi::new(McpApplicationService::new(
        Arc::new(SqliteMcpServerRepository::new(database)),
        Arc::new(RmcpConnectionAdapter::default()),
        Arc::new(McpOperationAdapter::new(operations)),
        Arc::new(SystemMcpClock),
        Arc::new(UnifiedMcpLoggingAdapter::new(operation_logging)),
        Arc::new(CurrentProjectPathAdapter),
    ))
}
