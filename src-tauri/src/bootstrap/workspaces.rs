use crate::contexts::operations::application::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::contexts::workspaces::application::{
    SessionShellPorts, SessionShellRegistry, ShellCapacities, ShellStore,
    WorkspaceApplicationService, WorkspaceInspectionRouter, WorkspaceInvalidationDispatcher,
    WorkspaceQueryApplicationService,
};
use crate::contexts::workspaces::infrastructure::{
    LocalWorkspaceInspectionProvider, RemoteWorkspaceInspectionProvider, RetainedLocalShellRuntime,
    RetainedRemoteShellRuntime, RoutedShellRuntime, SessionWorkspaceQueryAdapter,
    SessionWorkspaceTargetResolver, SqliteSessionShellWorkspace, SqliteShellWorkspaceAdapter,
    SqliteWorkspaceHistoryRepository, SshRemoteHelperSession, SshRemoteProfileSource,
    SshShellTransport, SystemShellClock, SystemWorkspaceClock, TauriProjectDirectorySelection,
    TauriSessionShellNotices, TauriWorkspaceInvalidationNotices, UnifiedLogShellDiagnostics,
    UuidShellIds, WorkspaceFilesystemAdapter, WorkspaceGitAdapter, WorkspaceInvalidationPoller,
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
    // Selection lives in one router rather than at each call site: the provider follows from the
    // session's registered binding, and a second place that decided it could disagree.
    let inspection = Arc::new(
        WorkspaceInspectionRouter::new(
            Arc::new(SessionWorkspaceTargetResolver::new(database.clone())),
            Arc::new(LocalWorkspaceInspectionProvider::new(
                review_adapter.clone(),
            )),
        )
        // The remote provider is registered unconditionally. Whether a *session* can use it is a
        // property of its binding, not of this build, and gating it here would make a missing
        // remote workspace and a missing feature indistinguishable.
        .with_remote(Arc::new(RemoteWorkspaceInspectionProvider::new(
            Arc::new(SshRemoteProfileSource::new(ssh.clone())),
            Arc::new(SshRemoteHelperSession::new(ssh.clone())),
        ))),
    );
    let shell_workspaces = Arc::new(SqliteShellWorkspaceAdapter::new(database.clone()));
    let shells = assemble_session_shell_registry(
        database.clone(),
        app.clone(),
        shell_workspaces,
        ssh,
        evidence,
    );
    let service = WorkspaceApplicationService::new(
        Arc::new(SqliteWorkspaceHistoryRepository::new(database)),
        Arc::new(WorkspaceFilesystemAdapter::new(logging.clone())),
        Arc::new(WorkspaceGitAdapter::new(logging)),
        Arc::new(TauriProjectDirectorySelection::new(app.clone())),
        Arc::new(SystemWorkspaceClock),
    );
    let invalidation = Arc::new(WorkspaceInvalidationDispatcher::new(Arc::new(
        TauriWorkspaceInvalidationNotices::new(app),
    )));
    start_workspace_invalidation_job(inspection.clone(), invalidation.clone());
    WorkspaceApi::new(
        service,
        queries,
        review_adapter,
        shells,
        inspection,
        invalidation,
    )
}

/// How often the driver wakes while a console has something open.
///
/// The coalescing window sets the floor: waking less often than it would hold a finished burst past
/// the point where publishing it still feels like a consequence of the change.
const INVALIDATION_TICK: Duration = Duration::from_millis(250);

/// How often it wakes when nothing is open.
///
/// Four wakeups a second for the life of the process is a real cost on a laptop, and when nothing is
/// observed there is nothing for any of them to do. The latency it buys back is only ever paid by
/// the first change after a quiet period, and a quiet period here means no console has read a
/// directory in a minute — so nobody is looking at the answer that arrives a beat late.
const IDLE_TICK: Duration = Duration::from_secs(1);

/// How many ticks pass between polls.
///
/// Polling is the expensive half — over SSH it is a round trip — and it answers a question that
/// changes on human timescales. Flushing is nearly free and answers one that does not.
const POLL_TICKS: u32 = 8;

/// Drives coalescing, polling, and expiry on one timer.
///
/// One loop rather than three, because all three depend on the same liveness condition: a session
/// with nothing open needs no poll, has nothing pending to flush, and has nothing left to expire.
/// Observations age out on their own, so a console that was hidden, closed, or crashed stops
/// costing anything without having to say so — which matters most for the clients least likely to.
fn start_workspace_invalidation_job(
    inspection: Arc<WorkspaceInspectionRouter>,
    dispatcher: Arc<WorkspaceInvalidationDispatcher>,
) {
    let poller = Arc::new(WorkspaceInvalidationPoller::new(
        inspection,
        dispatcher.clone(),
    ));
    tauri::async_runtime::spawn(async move {
        let mut tick: u32 = 0;
        let mut interval = IDLE_TICK;
        loop {
            sleep(interval).await;
            tick = tick.wrapping_add(1);
            let now = unix_milliseconds();
            if tick.is_multiple_of(POLL_TICKS) {
                poller.poll_observed(now).await;
            }
            // After the poll, so a change it just found is published in this cycle rather than
            // waiting for the next one.
            dispatcher.flush_due(now);
            interval = if dispatcher.expire(now) > 0 {
                INVALIDATION_TICK
            } else {
                IDLE_TICK
            };
        }
    });
}

/// Wall-clock milliseconds for the driver's own bookkeeping.
fn unix_milliseconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
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
    evidence: Arc<dyn crate::contexts::workspaces::api::WorkspaceEvidencePort>,
) -> Arc<SessionShellRegistry> {
    let clock = Arc::new(SystemShellClock::default());
    let store = Arc::new(ShellStore::new(
        Arc::new(TauriSessionShellNotices::new(app)),
        clock.clone(),
    ));
    let runtime = Arc::new(RoutedShellRuntime::new(
        Arc::new(RetainedLocalShellRuntime::new(Arc::new(
            UnifiedLogShellDiagnostics,
        ))),
        Arc::new(RetainedRemoteShellRuntime::new(Arc::new(
            SshShellTransport::new(ssh),
        ))),
    ));
    Arc::new(SessionShellRegistry::new(
        store,
        SessionShellPorts {
            runtime,
            workspaces: Arc::new(SqliteSessionShellWorkspace::new(database, shell_workspaces)),
            ids: Arc::new(UuidShellIds),
            clock,
            evidence,
            diagnostics: Arc::new(UnifiedLogShellDiagnostics),
        },
        ShellCapacities::default(),
    ))
}
