use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::task_orchestration::api::PlanApplicationError;
use crate::contexts::task_orchestration::api::TaskOrchestrationApi;
use crate::contexts::task_orchestration::infrastructure::UnifiedPlanDiagnosticsAdapter;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_task_orchestration_api(
    database: NativeDatabase,
    sessions: SessionsApi,
    agents: AgentRuntimeApi,
    workspaces: WorkspaceApi,
    operations: OperationsApi,
    fallback_log_directory: PathBuf,
) -> Result<TaskOrchestrationApi, PlanApplicationError> {
    let diagnostics = Arc::new(UnifiedPlanDiagnosticsAdapter::new(Arc::new(
        UnifiedLoggingAdapter::active(fallback_log_directory),
    )));
    TaskOrchestrationApi::native(
        database,
        sessions,
        agents,
        workspaces,
        operations,
        diagnostics,
    )
}
