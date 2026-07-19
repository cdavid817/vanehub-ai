use crate::contexts::workspaces::application::{
    ShellWorkspace, WorkspaceApplicationError, WorkspaceShellContextPort,
};
use crate::platform::{database::NativeDatabase, filesystem};
use rusqlite::{params, OptionalExtension};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct SqliteShellWorkspaceAdapter {
    database: NativeDatabase,
}

impl SqliteShellWorkspaceAdapter {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl WorkspaceShellContextPort for SqliteShellWorkspaceAdapter {
    fn load_shell_workspace(
        &self,
        session_id: &str,
    ) -> Result<ShellWorkspace, WorkspaceApplicationError> {
        let connection = self
            .database
            .connection()
            .map_err(|error| WorkspaceApplicationError::Repository(error.to_string()))?;
        let workspace = connection
            .query_row(
                "SELECT agent_id, folder, project_path, worktree_path, remote_workspace_host, \
                 remote_workspace_path, remote_workspace_display_name, remote_workspace_uri \
                 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    let agent_id = row.get(0)?;
                    let folder = row.get::<_, Option<String>>(1)?;
                    let project_path = row.get::<_, Option<String>>(2)?;
                    let worktree_path = row.get::<_, Option<String>>(3)?;
                    let remote_host = row.get::<_, Option<String>>(4)?;
                    let remote_path = row.get::<_, Option<String>>(5)?;
                    let remote_display_name = row.get::<_, Option<String>>(6)?;
                    let remote_uri = row.get::<_, Option<String>>(7)?;
                    Ok((
                        agent_id,
                        worktree_path.or(folder).or(project_path),
                        remote_host.is_some()
                            && remote_path.is_some()
                            && remote_display_name.is_some()
                            && remote_uri.is_some(),
                    ))
                },
            )
            .optional()
            .map_err(|error| WorkspaceApplicationError::Repository(error.to_string()))?
            .ok_or_else(|| WorkspaceApplicationError::SessionNotFound(session_id.to_string()))?;
        let root = if workspace.2 {
            None
        } else {
            filesystem::canonical_directory_if_available(workspace.1.as_deref().map(Path::new))
                .map_err(|error| WorkspaceApplicationError::Storage(error.to_string()))?
                .map(|path| path.to_string_lossy().to_string())
        };
        Ok(ShellWorkspace {
            agent_id: workspace.0,
            root,
            remote: workspace.2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use rusqlite::params;

    fn insert_session(database: &NativeDatabase, id: &str, folder: Option<&str>, remote: bool) {
        let connection = database.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO sessions \
                 (id, title, agent_id, interaction_mode, lifecycle_state, folder, \
                  remote_workspace_host, remote_workspace_path, remote_workspace_display_name, \
                  remote_workspace_uri, pinned, archived, created_at, updated_at) \
                 VALUES (?1, 'Shell fixture', 'codex-cli', 'cli', 'idle', ?2, ?3, ?4, ?5, ?6, \
                         0, 0, '2026-07-18T12:00:00Z', '2026-07-18T12:00:00Z')",
                params![
                    id,
                    folder,
                    remote.then_some("example.com"),
                    remote.then_some("/work/app"),
                    remote.then_some("Remote app"),
                    remote.then_some("ssh://example.com/work/app"),
                ],
            )
            .expect("insert session");
    }

    #[test]
    fn sqlite_shell_projection_distinguishes_local_remote_and_missing_sessions() {
        let fixture = TempDirectory::new("workspace-shell-projection");
        let root = fixture.path().join("workspace");
        std::fs::create_dir_all(&root).expect("workspace");
        let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
        insert_session(
            &database,
            "session-local",
            Some(&root.to_string_lossy()),
            false,
        );
        insert_session(&database, "session-remote", None, true);
        let adapter = SqliteShellWorkspaceAdapter::new(database);

        let local = adapter
            .load_shell_workspace("session-local")
            .expect("local workspace");
        let remote = adapter
            .load_shell_workspace("session-remote")
            .expect("remote workspace");
        let missing = adapter
            .load_shell_workspace("missing")
            .expect_err("missing session");
        let canonical_root = root
            .canonicalize()
            .expect("canonical")
            .to_string_lossy()
            .to_string();

        assert_eq!(local.agent_id, "codex-cli");
        assert_eq!(local.root.as_deref(), Some(canonical_root.as_str()));
        assert!(!local.remote);
        assert!(remote.remote);
        assert_eq!(remote.root, None);
        assert_eq!(
            missing,
            WorkspaceApplicationError::SessionNotFound("missing".to_string())
        );
    }
}
