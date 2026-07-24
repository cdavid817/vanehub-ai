use crate::contexts::workspaces::application::{
    ShellRemoteEndpoint, ShellSshBinding, ShellWorkspace, ShellWorkspacePolicy,
    WorkspaceApplicationError, WorkspaceShellContextPort,
};
use crate::contexts::workspaces::domain::normalize_windows_extended_length_path;
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
                 remote_workspace_port, remote_workspace_user, remote_workspace_path, \
                 remote_workspace_display_name, remote_workspace_uri, remote_ssh_connection_id, \
                 remote_ssh_connection_revision, loop_role \
                 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    let agent_id = row.get(0)?;
                    let folder = row.get::<_, Option<String>>(1)?;
                    let project_path = row.get::<_, Option<String>>(2)?;
                    let worktree_path = row.get::<_, Option<String>>(3)?;
                    let remote_host = row.get::<_, Option<String>>(4)?;
                    let remote_port = row.get::<_, Option<u16>>(5)?;
                    let remote_user = row.get::<_, Option<String>>(6)?;
                    let remote_path = row.get::<_, Option<String>>(7)?;
                    let remote_display_name = row.get::<_, Option<String>>(8)?;
                    let remote_uri = row.get::<_, Option<String>>(9)?;
                    let binding_id = row.get::<_, Option<String>>(10)?;
                    let binding_revision = row
                        .get::<_, Option<String>>(11)?
                        .and_then(|value| value.parse::<i64>().ok());
                    let loop_role = row.get::<_, Option<String>>(12)?;
                    let endpoint = match (
                        remote_host,
                        remote_port,
                        remote_user,
                        remote_path,
                        remote_display_name,
                        remote_uri,
                    ) {
                        (
                            Some(host),
                            Some(port),
                            Some(user),
                            Some(path),
                            Some(display_name),
                            Some(uri),
                        ) => Some(ShellRemoteEndpoint {
                            host,
                            port,
                            user,
                            path,
                            display_name,
                            uri,
                        }),
                        _ => None,
                    };
                    Ok((
                        agent_id,
                        worktree_path.or(folder).or(project_path),
                        endpoint,
                        binding_id
                            .zip(binding_revision)
                            .map(|(connection_id, revision)| ShellSshBinding {
                                connection_id,
                                revision,
                            }),
                        loop_role.as_deref() == Some("verifier"),
                    ))
                },
            )
            .optional()
            .map_err(|error| WorkspaceApplicationError::Repository(error.to_string()))?
            .ok_or_else(|| WorkspaceApplicationError::SessionNotFound(session_id.to_string()))?;
        let root = if workspace.2.is_some() {
            None
        } else {
            filesystem::canonical_directory_if_available(workspace.1.as_deref().map(Path::new))
                .map_err(|error| WorkspaceApplicationError::Storage(error.to_string()))?
                .map(|path| normalize_windows_extended_length_path(&path.to_string_lossy()))
        };
        Ok(ShellWorkspace {
            agent_id: workspace.0,
            root,
            remote: workspace.2.is_some(),
            remote_endpoint: workspace.2,
            ssh_binding: workspace.3,
            policy: ShellWorkspacePolicy {
                requires_host_trust: false,
            },
            read_only: workspace.4,
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
                  remote_workspace_host, remote_workspace_port, remote_workspace_user, remote_workspace_path, remote_workspace_display_name, \
                  remote_workspace_uri, pinned, archived, created_at, updated_at) \
                 VALUES (?1, 'Shell fixture', 'codex-cli', 'cli', 'idle', ?2, ?3, 22, 'developer', ?4, ?5, ?6, \
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
        let canonical_root = normalize_windows_extended_length_path(
            &root.canonicalize().expect("canonical").to_string_lossy(),
        );

        assert_eq!(local.agent_id, "codex-cli");
        assert_eq!(local.root.as_deref(), Some(canonical_root.as_str()));
        assert!(!local.remote);
        assert!(remote.remote);
        assert_eq!(remote.root, None);
        assert_eq!(
            remote
                .remote_endpoint
                .as_ref()
                .map(|endpoint| endpoint.host.as_str()),
            Some("example.com")
        );
        assert_eq!(
            missing,
            WorkspaceApplicationError::SessionNotFound("missing".to_string())
        );
    }
}
