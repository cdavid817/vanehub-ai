use crate::contexts::workspaces::application::{
    KnownProject, KnownRemoteWorkspace, WorkspaceApplicationError, WorkspaceHistoryRepository,
};
use crate::contexts::workspaces::domain::{ProjectInspection, RemoteWorkspace};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};

#[derive(Clone)]
pub(crate) struct SqliteWorkspaceHistoryRepository {
    database: NativeDatabase,
}

impl SqliteWorkspaceHistoryRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl WorkspaceHistoryRepository for SqliteWorkspaceHistoryRepository {
    fn list_projects(&self) -> Result<Vec<KnownProject>, WorkspaceApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                "SELECT path, display_name, is_git, last_opened_at FROM known_projects ORDER BY last_opened_at DESC",
            )
            .map_err(database_error)?;
        let projects = statement
            .query_map([], read_known_project)
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(projects)
    }

    fn list_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                "SELECT uri, host, COALESCE(port, 22), user, path, display_name, last_opened_at FROM known_remote_workspaces ORDER BY last_opened_at DESC",
            )
            .map_err(database_error)?;
        let workspaces = statement
            .query_map([], read_known_remote_workspace)
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(workspaces)
    }

    fn remember_project(
        &self,
        inspection: &ProjectInspection,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.database
            .connection()
            .map_err(app_error)?
            .execute(
                r#"
                INSERT INTO known_projects (path, display_name, is_git, last_opened_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(path) DO UPDATE SET
                    display_name = excluded.display_name,
                    is_git = excluded.is_git,
                    last_opened_at = excluded.last_opened_at
                "#,
                params![
                    inspection.path(),
                    inspection.display_name(),
                    inspection.is_git() as i32,
                    opened_at,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.database
            .connection()
            .map_err(app_error)?
            .execute(
                r#"
                INSERT INTO known_remote_workspaces
                    (uri, host, port, user, path, display_name, last_opened_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(uri) DO UPDATE SET
                    host = excluded.host,
                    port = excluded.port,
                    user = excluded.user,
                    path = excluded.path,
                    display_name = excluded.display_name,
                    last_opened_at = excluded.last_opened_at
                "#,
                params![
                    workspace.uri(),
                    workspace.host(),
                    i64::from(workspace.port()),
                    workspace.user(),
                    workspace.path(),
                    workspace.display_name(),
                    opened_at,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

fn read_known_project(row: &Row<'_>) -> Result<KnownProject, rusqlite::Error> {
    Ok(KnownProject {
        path: row.get(0)?,
        display_name: row.get(1)?,
        is_git: row.get::<_, i64>(2)? != 0,
        last_opened_at: row.get(3)?,
    })
}

fn read_known_remote_workspace(row: &Row<'_>) -> Result<KnownRemoteWorkspace, rusqlite::Error> {
    Ok(KnownRemoteWorkspace {
        uri: row.get(0)?,
        host: row.get(1)?,
        port: row.get::<_, i64>(2)? as u16,
        user: row.get(3)?,
        path: row.get(4)?,
        display_name: row.get(5)?,
        last_opened_at: row.get(6)?,
    })
}

fn app_error(error: crate::platform::database::DatabaseError) -> WorkspaceApplicationError {
    WorkspaceApplicationError::Repository(error.to_string())
}

fn database_error(error: rusqlite::Error) -> WorkspaceApplicationError {
    WorkspaceApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn histories_round_trip_through_existing_tables_and_order_by_last_opened() {
        let directory = TempDirectory::new("workspace-history-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteWorkspaceHistoryRepository::new(database);
        let inspection =
            ProjectInspection::from_probe("C:\\code\\app", Some("C:\\code\\app".to_string()))
                .expect("project");
        let remote = RemoteWorkspace::new("example.com", None, Some("dev"), "/work/app", None)
            .expect("remote workspace");

        repository
            .remember_project(&inspection, "2026-07-18T12:00:00Z")
            .expect("remember project");
        repository
            .remember_remote_workspace(&remote, "2026-07-18T13:00:00Z")
            .expect("remember remote");

        let projects = repository.list_projects().expect("projects");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, "C:\\code\\app");
        assert!(projects[0].is_git);
        let remotes = repository
            .list_remote_workspaces()
            .expect("remote workspaces");
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].uri, "ssh://dev@example.com/work/app");
    }
}
