//! Resolving a workspace target, and inspecting a local one.
//!
//! The resolver is the only thing in the process that produces a `WorkspaceTarget`, and its only
//! input is a session id. That is the whole enforcement of "a caller cannot name a root": there is
//! no other constructor a command could reach, so an absolute path arriving from the frontend has
//! nowhere to go.
//!
//! The local provider delegates to the confined implementations that already exist rather than
//! reimplementing them against the target's root. Re-deriving the root inside those functions is
//! deliberate: a second entry point that took a root from its caller would be a second path into
//! the confinement code whose boundary is whatever it was handed, which is the failure the resolver
//! exists to prevent. The root on the target is a witness that resolution happened, not an input.

use super::session_queries::resolve_session_root;
use crate::contexts::workspaces::application::{
    CapabilityState, DirectoryListing, DocumentListing, FileContent, FileSearchListing,
    GitDiffRequest, GitDiffResult, GitStatusResult, ListDirectoryRequest, LocalWorkspaceTarget,
    ReadTextFileRequest, RemoteWorkspaceTarget, WatchMode, WorkspaceApplicationError as AppError,
    WorkspaceInspectionCapabilities, WorkspaceInspectionError, WorkspaceInspectionProvider,
    WorkspaceSearchRequest, WorkspaceSessionQueryPort, WorkspaceTarget, WorkspaceTargetResolver,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use async_trait::async_trait;
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

/// What the session row says about where its workspace is.
struct SessionWorkspaceBinding {
    remote_path: Option<String>,
    remote_display_name: Option<String>,
    connection_id: Option<String>,
    connection_revision: Option<i64>,
}

#[derive(Clone)]
pub(crate) struct SessionWorkspaceTargetResolver {
    database: NativeDatabase,
}

impl SessionWorkspaceTargetResolver {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, WorkspaceInspectionError> {
        self.database
            .connection()
            .map_err(|error| WorkspaceInspectionError::Storage(error.to_string()))
    }
}

impl WorkspaceTargetResolver for SessionWorkspaceTargetResolver {
    fn resolve(&self, session_id: &str) -> Result<WorkspaceTarget, WorkspaceInspectionError> {
        let connection = self.connection()?;
        let binding: SessionWorkspaceBinding = connection
            .query_row(
                "SELECT remote_workspace_path, remote_workspace_display_name, \
                 remote_ssh_connection_id, remote_ssh_connection_revision \
                 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(SessionWorkspaceBinding {
                        remote_path: row.get(0)?,
                        remote_display_name: row.get(1)?,
                        connection_id: row.get(2)?,
                        connection_revision: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|error| WorkspaceInspectionError::Storage(error.to_string()))?
            .ok_or(WorkspaceInspectionError::TargetUnavailable(
                "workspace_session_not_found",
            ))?;

        if let Some(remote_path) = binding.remote_path {
            // A remote workspace with no binding is not a remote workspace anybody can reach, and
            // silently falling back to a local root would inspect this machine while the reader
            // believed they were looking at the host.
            let connection_id =
                binding
                    .connection_id
                    .ok_or(WorkspaceInspectionError::TargetUnavailable(
                        "workspace_remote_binding_missing",
                    ))?;
            let connection_revision =
                binding
                    .connection_revision
                    .ok_or(WorkspaceInspectionError::TargetUnavailable(
                        "workspace_remote_binding_missing",
                    ))?;
            return Ok(WorkspaceTarget::Remote(RemoteWorkspaceTarget {
                session_id: session_id.to_string(),
                connection_id,
                connection_revision,
                display_name: binding
                    .remote_display_name
                    .unwrap_or_else(|| remote_path.clone()),
                root: remote_path,
            }));
        }

        let root = resolve_session_root(&connection, session_id)
            .map_err(WorkspaceInspectionError::from)?
            .ok_or(WorkspaceInspectionError::TargetUnavailable(
                "workspace_provider_unavailable",
            ))?;
        Ok(WorkspaceTarget::Local(LocalWorkspaceTarget {
            session_id: session_id.to_string(),
            root,
        }))
    }
}

/// Refuses a remote target rather than answering about this machine.
///
/// A free function so the rule can be tested on its own: the alternative is worse than an error —
/// a local provider that quietly inspected its own filesystem for a remote session would show real
/// files under a remote host's name — and proving that needs no adapter, no database, and no app.
pub(super) fn require_local(
    target: &WorkspaceTarget,
) -> Result<&LocalWorkspaceTarget, WorkspaceInspectionError> {
    match target {
        WorkspaceTarget::Local(local) => Ok(local),
        WorkspaceTarget::Remote(_) => Err(WorkspaceInspectionError::Unsupported(
            "workspace_provider_local_only",
        )),
    }
}

/// Whether a requested path is an escape attempt, before anything touches the filesystem.
///
/// Not a second boundary - the confinement stays where it is, resolving against a canonical root.
/// This is a *classification*: the deeper code refuses an escape and a missing file the same way,
/// and a reader needs those told apart. An absolute path or a `..` component is an escape by
/// inspection alone, and everything the deeper code then refuses is a path that is simply gone.
fn classify_relative_path(path: &str) -> Result<(), WorkspaceInspectionError> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') || normalized.split('/').any(|part| part == "..") {
        return Err(WorkspaceInspectionError::PathEscaped);
    }
    Ok(())
}

/// The local half, over the confined implementations that already exist.
#[derive(Clone)]
pub(crate) struct LocalWorkspaceInspectionProvider {
    /// The port the application already declares, not the concrete adapter. Narrower, and it is
    /// what lets the provider be built without an app handle — which the adapter needs only for
    /// the log export, an operation no inspection performs.
    queries: Arc<dyn WorkspaceSessionQueryPort>,
}

impl LocalWorkspaceInspectionProvider {
    pub(crate) fn new(queries: Arc<dyn WorkspaceSessionQueryPort>) -> Self {
        Self { queries }
    }

    /// Runs one confined read on the blocking pool.
    ///
    /// SQLite and the filesystem are both blocking, and this is called from an async command
    /// handler: doing the work inline would hold an executor thread for as long as a directory walk
    /// takes, which on a cold cache is long enough to stall unrelated commands.
    async fn blocking<T, F>(&self, work: F) -> Result<T, WorkspaceInspectionError>
    where
        T: Send + 'static,
        F: FnOnce(&dyn WorkspaceSessionQueryPort) -> Result<T, AppError> + Send + 'static,
    {
        let queries = self.queries.clone();
        tauri::async_runtime::spawn_blocking(move || work(queries.as_ref()))
            .await
            .map_err(|_| WorkspaceInspectionError::Storage("inspection task failed".to_string()))?
            .map_err(WorkspaceInspectionError::from)
    }
}

#[async_trait]
impl WorkspaceInspectionProvider for LocalWorkspaceInspectionProvider {
    async fn capabilities(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        require_local(target)?;
        Ok(WorkspaceInspectionCapabilities {
            provider: "local",
            list_files: CapabilityState::available(),
            read_text_files: CapabilityState::available(),
            search_files: CapabilityState::available(),
            git_status: CapabilityState::available(),
            git_diff: CapabilityState::available(),
            // Nothing watches the local tree yet. `EventDerived` rather than `Native` is the honest
            // answer: the panel refreshes on the file-mutation notices this application already
            // emits, which means a change made outside it is invisible until an explicit refresh —
            // and a reader who believed a watcher was running would not know to press it.
            watch_mode: WatchMode::EventDerived,
        })
    }

    async fn list_directory(
        &self,
        target: &WorkspaceTarget,
        request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        classify_relative_path(&request.path)?;
        self.blocking(move |queries| queries.list_directory(&session_id, &request.path))
            .await
    }

    async fn list_documents(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<DocumentListing, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        self.blocking(move |queries| queries.list_documents(&session_id))
            .await
    }

    async fn read_text_file(
        &self,
        target: &WorkspaceTarget,
        request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        classify_relative_path(&request.path)?;
        // `read_file`, not `read_text_file`: the latter refuses anything that is not text,
        // which is right for prompt assembly and wrong for a preview. A binary file is
        // something a panel shows as binary, not something it fails on.
        self.blocking(move |queries| queries.read_file(&session_id, &request.path))
            .await
    }

    async fn search(
        &self,
        target: &WorkspaceTarget,
        request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        self.blocking(move |queries| {
            queries.search_files(&session_id, &request.query, request.max_results)
        })
        .await
    }

    async fn git_status(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<GitStatusResult, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        self.blocking(move |queries| queries.git_status(&session_id))
            .await
    }

    async fn git_diff(
        &self,
        target: &WorkspaceTarget,
        request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError> {
        let session_id = require_local(target)?.session_id.clone();
        classify_relative_path(&request.path)?;
        self.blocking(move |queries| queries.git_diff(&session_id, &request.path, request.source))
            .await
    }
}
