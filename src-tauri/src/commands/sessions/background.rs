use super::events;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::retrieval::api::CodeIndexApi;
use crate::contexts::sessions::api::{
    PreparedNewSessionCreation, PreviewSessionDeletionRequest, SessionDeletionHandle,
    SessionDeletionPreview, SessionsApi, SessionsError,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::AppHandle;

pub(super) fn spawn_creation(
    app: AppHandle,
    api: SessionsApi,
    code_index: CodeIndexApi,
    log_directory: PathBuf,
    prepared: PreparedNewSessionCreation,
) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(session) = api.execute_creation(prepared) {
            events::emit_active_session_changed(&app, Some(session.id()));
            let folder = local_code_index_folder(
                session.workspace.remote_workspace.is_some(),
                session.workspace.worktree_path.as_deref(),
                session.workspace.folder.as_deref(),
                session.workspace.project_path.as_deref(),
            );
            if let Err(error) = code_index.discover_session_workspace(
                &session.agent_id,
                folder,
                session.workspace.remote_workspace.is_some(),
            ) {
                let logging = UnifiedLoggingAdapter::active(log_directory);
                let _ = logging.write_diagnostic(DiagnosticLog {
                    severity: LogSeverity::Warn,
                    category: "retrieval.code_index.discovery".to_string(),
                    message: "Automatic workspace code-index discovery failed".to_string(),
                    context: BTreeMap::from([
                        ("sessionId".to_string(), session.id().to_string()),
                        ("category".to_string(), error.category().to_string()),
                    ]),
                });
            }
        }
    });
}

/// Runs an accepted deletion to its recorded end off the main thread. Events are published by
/// the coordinator after each commit; the result is read back through the journal, so nothing
/// is lost if the window that started it has since closed.
pub(super) fn spawn_deletion(api: SessionsApi, operation_id: String) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = api.run_deletion(&operation_id);
    });
}

/// An idempotent replay hands back the operation it already accepted; only a newly journaled
/// request gets a runner, so one request never has two.
pub(super) fn spawn_deletion_unless_existing(api: SessionsApi, handle: &SessionDeletionHandle) {
    if !handle.existing {
        spawn_deletion(api, handle.operation_id.clone());
    }
}

fn join_error(error: tauri::Error) -> SessionsError {
    SessionsError::Runtime(error.to_string())
}

pub(super) async fn preview_deletion_off_thread(
    api: SessionsApi,
    input: PreviewSessionDeletionRequest,
) -> Result<SessionDeletionPreview, SessionsError> {
    tauri::async_runtime::spawn_blocking(move || api.preview_deletion(input))
        .await
        .map_err(join_error)?
}

/// Keep-only deletion. The active-session event is announced only when the deleted session was
/// the active one; deleting another session must not claim the selection was cleared.
pub(super) async fn delete_session_off_thread(
    app: AppHandle,
    api: SessionsApi,
    session_id: String,
) -> Result<(), SessionsError> {
    let was_active = api
        .active()?
        .is_some_and(|session| session.id() == session_id);
    let api_for_delete = api.clone();
    let target = session_id.clone();
    tauri::async_runtime::spawn_blocking(move || api_for_delete.delete(&target))
        .await
        .map_err(join_error)??;
    if was_active {
        events::emit_active_session_changed(&app, None);
    }
    Ok(())
}

fn local_code_index_folder<'a>(
    remote: bool,
    worktree: Option<&'a str>,
    folder: Option<&'a str>,
    project: Option<&'a str>,
) -> Option<&'a str> {
    (!remote).then_some(())?;
    worktree.or(folder).or(project)
}

#[cfg(test)]
mod tests {
    use super::local_code_index_folder;

    #[test]
    fn automatic_indexing_prefers_the_actual_worktree_folder() {
        assert_eq!(
            local_code_index_folder(
                false,
                Some("C:/worktree"),
                Some("C:/folder"),
                Some("C:/repo")
            ),
            Some("C:/worktree")
        );
    }

    #[test]
    fn automatic_indexing_excludes_remote_workspaces() {
        assert_eq!(
            local_code_index_folder(true, None, Some("ssh://host/repo"), None),
            None
        );
    }
}
