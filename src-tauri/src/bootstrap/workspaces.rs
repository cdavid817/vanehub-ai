use crate::contexts::operations::application::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::contexts::workspaces::application::{
    SessionShellRegistry, ShellCapacities, ShellStore, WorkspaceApplicationService,
    WorkspaceQueryApplicationService, WorkspaceShellApplicationService,
};
use crate::contexts::workspaces::infrastructure::{
    PortablePtyShellRuntime, RetainedLocalShellRuntime, RetainedRemoteShellRuntime,
    RoutedShellRuntime, SessionWorkspaceQueryAdapter, SqliteSessionShellWorkspace,
    SqliteShellWorkspaceAdapter, SqliteWorkspaceHistoryRepository, SystemShellClock,
    SystemWorkspaceClock, TauriProjectDirectorySelection, TauriSessionShellNotices,
    TauriWorkspaceShellEventPublisher, UuidShellIds, UuidWorkspaceShellId,
    WorkspaceFilesystemAdapter, WorkspaceGitAdapter, WorkspaceShellLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::time::{sleep, Duration};

pub(crate) fn assemble_workspace_api(
    database: NativeDatabase,
    app: AppHandle,
    fallback_log_directory: PathBuf,
    evidence: Arc<dyn crate::contexts::workspaces::api::WorkspaceEvidencePort>,
    ssh: SshConnectionsApi,
) -> WorkspaceApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let review_adapter = Arc::new(SessionWorkspaceQueryAdapter::new(
        database.clone(),
        app.clone(),
    ));
    let queries = WorkspaceQueryApplicationService::new(review_adapter.clone());
    let shell_events = Arc::new(TauriWorkspaceShellEventPublisher::new(app.clone()));
    let shell_logging = Arc::new(WorkspaceShellLoggingAdapter::new(logging.clone()));
    let shell_workspaces = Arc::new(SqliteShellWorkspaceAdapter::new(database.clone()));
    let shell = WorkspaceShellApplicationService::new(
        shell_workspaces.clone(),
        Arc::new(PortablePtyShellRuntime::new(
            shell_events.clone(),
            shell_logging.clone(),
        )),
        Arc::new(UuidWorkspaceShellId),
        shell_events,
        shell_logging,
        evidence,
    );
    let shells =
        assemble_session_shell_registry(database.clone(), app.clone(), shell_workspaces, ssh);
    let service = WorkspaceApplicationService::new(
        Arc::new(SqliteWorkspaceHistoryRepository::new(database)),
        Arc::new(WorkspaceFilesystemAdapter::new(logging.clone())),
        Arc::new(WorkspaceGitAdapter::new(logging)),
        Arc::new(TauriProjectDirectorySelection::new(app)),
        Arc::new(SystemWorkspaceClock),
    );
    WorkspaceApi::new(service, queries, shell, review_adapter, shells)
}

/// How often idle Shells are considered for reclamation.
///
/// Well below the idle window itself, so a Shell is reclaimed some minutes after it qualifies
/// rather than at some arbitrary point in the next window.
const SHELL_IDLE_SWEEP_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Reclaims detached, quiet Shells on a timer.
///
/// The sweep runs on the blocking pool because closing a Shell kills a process and joins its reader
/// thread; doing that on the async executor would park a runtime worker on a PTY that is taking its
/// time to die. Each cycle is bounded by the registry to a handful of Shells, so a long-idle
/// application reclaims steadily rather than in one stall.
pub(crate) fn start_session_shell_idle_job(workspaces: WorkspaceApi) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(SHELL_IDLE_SWEEP_INTERVAL).await;
            let workspaces = workspaces.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                workspaces.sweep_idle_session_shells()
            })
            .await;
        }
    });
}

/// The retained Shell registry and everything it owns.
///
/// The store is built first and handed to the registry, because the store is what the runtime's
/// worker threads write into: a runtime that had to reach back through the registry to record a
/// frame would be a PTY read taking the registry's lock.
fn assemble_session_shell_registry(
    database: NativeDatabase,
    app: AppHandle,
    shell_workspaces: Arc<SqliteShellWorkspaceAdapter>,
    ssh: SshConnectionsApi,
) -> Arc<SessionShellRegistry> {
    let clock = Arc::new(SystemShellClock::default());
    let store = Arc::new(ShellStore::new(
        Arc::new(TauriSessionShellNotices::new(app)),
        clock.clone(),
    ));
    let runtime = Arc::new(RoutedShellRuntime::new(
        Arc::new(RetainedLocalShellRuntime::default()),
        Arc::new(RetainedRemoteShellRuntime::new(ssh)),
    ));
    Arc::new(SessionShellRegistry::new(
        store,
        runtime,
        Arc::new(SqliteSessionShellWorkspace::new(database, shell_workspaces)),
        Arc::new(UuidShellIds),
        clock,
        ShellCapacities::default(),
    ))
}
