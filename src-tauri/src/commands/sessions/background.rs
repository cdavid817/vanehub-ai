use super::events;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::retrieval::api::CodeIndexApi;
use crate::contexts::sessions::api::{PreparedNewSessionCreation, SessionsApi};
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
