use crate::contexts::operations::application::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::contexts::workspaces::application::{
    WorkspaceApplicationService, WorkspaceQueryApplicationService, WorkspaceShellApplicationService,
};
use crate::contexts::workspaces::infrastructure::{
    PortablePtyShellRuntime, SessionWorkspaceQueryAdapter, SqliteShellWorkspaceAdapter,
    SqliteWorkspaceHistoryRepository, SystemWorkspaceClock, TauriProjectDirectorySelection,
    TauriWorkspaceShellEventPublisher, UuidWorkspaceShellId, WorkspaceFilesystemAdapter,
    WorkspaceGitAdapter, WorkspaceShellLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;

pub(crate) fn assemble_workspace_api(
    database: NativeDatabase,
    app: AppHandle,
    fallback_log_directory: PathBuf,
) -> WorkspaceApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let queries = WorkspaceQueryApplicationService::new(Arc::new(
        SessionWorkspaceQueryAdapter::new(database.clone(), app.clone()),
    ));
    let shell_events = Arc::new(TauriWorkspaceShellEventPublisher::new(app.clone()));
    let shell_logging = Arc::new(WorkspaceShellLoggingAdapter::new(logging.clone()));
    let shell = WorkspaceShellApplicationService::new(
        Arc::new(SqliteShellWorkspaceAdapter::new(database.clone())),
        Arc::new(PortablePtyShellRuntime::new(
            shell_events.clone(),
            shell_logging.clone(),
        )),
        Arc::new(UuidWorkspaceShellId),
        shell_events,
        shell_logging,
    );
    let service = WorkspaceApplicationService::new(
        Arc::new(SqliteWorkspaceHistoryRepository::new(database)),
        Arc::new(WorkspaceFilesystemAdapter::new(logging.clone())),
        Arc::new(WorkspaceGitAdapter::new(logging)),
        Arc::new(TauriProjectDirectorySelection::new(app)),
        Arc::new(SystemWorkspaceClock),
    );
    WorkspaceApi::new(service, queries, shell)
}
