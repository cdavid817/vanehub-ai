use crate::contexts::tooling::mcp::application::{McpApplicationError, McpServerRepository};
use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, ConnectionStatus, Scope, ServerConfiguration, ServerConfigurationDraft,
    ServerName, ServerStatus, ToolDescriptor, TransportType,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, Row};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct SqliteMcpServerRepository {
    database: NativeDatabase,
}

impl SqliteMcpServerRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl McpServerRepository for SqliteMcpServerRepository {
    fn list_visible(
        &self,
        current_project_path: &str,
    ) -> Result<Vec<ServerConfiguration>, McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT name, transport_type, command, args, env, url, headers, description,
                       active, scope, project_path
                FROM mcp_servers
                WHERE scope = 'user' OR (scope = 'project' AND project_path = ?1)
                ORDER BY active DESC, name ASC
                "#,
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![current_project_path], McpServerRow::read)
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        rows.into_iter().map(McpServerRow::into_domain).collect()
    }

    fn find(&self, name: &str) -> Result<Option<ServerConfiguration>, McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .query_row(
                r#"
                SELECT name, transport_type, command, args, env, url, headers, description,
                       active, scope, project_path
                FROM mcp_servers WHERE name = ?1
                "#,
                params![name],
                McpServerRow::read,
            )
            .optional()
            .map_err(database_error)?
            .map(McpServerRow::into_domain)
            .transpose()
    }

    fn exists(&self, name: &ServerName) -> Result<bool, McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .query_row(
                "SELECT 1 FROM mcp_servers WHERE name = ?1",
                params![name.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map(|row| row.is_some())
            .map_err(database_error)
    }

    fn insert(
        &self,
        server: &ServerConfiguration,
        timestamp: &str,
    ) -> Result<(), McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                r#"
                INSERT INTO mcp_servers
                (name, transport_type, command, args, env, url, headers, description, active,
                 scope, project_path, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    server.name().as_str(),
                    server.transport_type().as_str(),
                    server.command(),
                    to_json(&server.args().map(<[String]>::to_vec))?,
                    to_json(&server.env().cloned())?,
                    server.url(),
                    to_json(&server.headers().cloned())?,
                    server.description(),
                    server.is_active() as i32,
                    server.scope().as_str(),
                    server.project_path(),
                    timestamp,
                    timestamp,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn replace(
        &self,
        original_name: &str,
        server: &ServerConfiguration,
        timestamp: &str,
    ) -> Result<(), McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let changed = connection
            .execute(
                r#"
                UPDATE mcp_servers
                SET name = ?1, transport_type = ?2, command = ?3, args = ?4, env = ?5,
                    url = ?6, headers = ?7, description = ?8, active = ?9, scope = ?10,
                    project_path = ?11, updated_at = ?12
                WHERE name = ?13
                "#,
                params![
                    server.name().as_str(),
                    server.transport_type().as_str(),
                    server.command(),
                    to_json(&server.args().map(<[String]>::to_vec))?,
                    to_json(&server.env().cloned())?,
                    server.url(),
                    to_json(&server.headers().cloned())?,
                    server.description(),
                    server.is_active() as i32,
                    server.scope().as_str(),
                    server.project_path(),
                    timestamp,
                    original_name,
                ],
            )
            .map_err(database_error)?;
        require_changed(changed, original_name)
    }

    fn remove(&self, name: &str) -> Result<(), McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let changed = connection
            .execute("DELETE FROM mcp_servers WHERE name = ?1", params![name])
            .map_err(database_error)?;
        require_changed(changed, name)
    }

    fn set_active(
        &self,
        name: &str,
        active: bool,
        timestamp: &str,
    ) -> Result<(), McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let changed = connection
            .execute(
                "UPDATE mcp_servers SET active = ?1, updated_at = ?2 WHERE name = ?3",
                params![active as i32, timestamp, name],
            )
            .map_err(database_error)?;
        require_changed(changed, name)
    }

    fn status(&self, name: &str) -> Result<ServerStatus, McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let row = connection
            .query_row(
                r#"
                SELECT name, active, last_connection_status, last_connected, last_error,
                       last_tools, last_test_duration_ms
                FROM mcp_servers WHERE name = ?1
                "#,
                params![name],
                McpStatusRow::read,
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| McpApplicationError::ServerNotFound(name.to_string()))?;
        row.into_domain()
    }

    fn record_connection_outcome(
        &self,
        name: &str,
        outcome: &ConnectionOutcome,
        timestamp: &str,
    ) -> Result<(), McpApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let last_connected = outcome.is_success().then_some(timestamp);
        let tools = outcome
            .tools()
            .iter()
            .cloned()
            .map(PersistedTool::from)
            .collect::<Vec<_>>();
        let changed = connection
            .execute(
                r#"
                UPDATE mcp_servers
                SET last_connection_status = ?1, last_connected = ?2, last_error = ?3,
                    last_tools = ?4, last_test_duration_ms = ?5, updated_at = ?6
                WHERE name = ?7
                "#,
                params![
                    outcome.status().as_str(),
                    last_connected,
                    outcome.error(),
                    to_json(&Some(tools))?,
                    outcome.duration_ms() as i64,
                    timestamp,
                    name,
                ],
            )
            .map_err(database_error)?;
        require_changed(changed, name)
    }
}

struct McpServerRow {
    name: String,
    transport_type: String,
    command: Option<String>,
    args: Option<String>,
    env: Option<String>,
    url: Option<String>,
    headers: Option<String>,
    description: Option<String>,
    active: bool,
    scope: String,
    project_path: Option<String>,
}

impl McpServerRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get(0)?,
            transport_type: row.get(1)?,
            command: row.get(2)?,
            args: row.get(3)?,
            env: row.get(4)?,
            url: row.get(5)?,
            headers: row.get(6)?,
            description: row.get(7)?,
            active: row.get::<_, i32>(8)? != 0,
            scope: row.get(9)?,
            project_path: row.get(10)?,
        })
    }

    fn into_domain(self) -> Result<ServerConfiguration, McpApplicationError> {
        ServerConfiguration::create(ServerConfigurationDraft {
            name: self.name,
            transport_type: TransportType::from_persisted(&self.transport_type),
            command: self.command,
            args: from_json(self.args.as_deref())?,
            env: from_json(self.env.as_deref())?,
            url: self.url,
            headers: from_json(self.headers.as_deref())?,
            description: self.description,
            active: self.active,
            scope: Scope::from_persisted(&self.scope),
            project_path: self.project_path,
        })
        .map_err(Into::into)
    }
}

struct McpStatusRow {
    name: String,
    active: bool,
    status: Option<String>,
    last_connected: Option<String>,
    error: Option<String>,
    tools: Option<String>,
    duration_ms: Option<i64>,
}

impl McpStatusRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get(0)?,
            active: row.get::<_, i32>(1)? != 0,
            status: row.get(2)?,
            last_connected: row.get(3)?,
            error: row.get(4)?,
            tools: row.get(5)?,
            duration_ms: row.get(6)?,
        })
    }

    fn into_domain(self) -> Result<ServerStatus, McpApplicationError> {
        Ok(ServerStatus {
            name: ServerName::parse(self.name)?,
            connection_status: ConnectionStatus::from_persisted(self.status.as_deref())
                .visible_for(self.active),
            tools: from_json::<Vec<PersistedTool>>(self.tools.as_deref())?
                .unwrap_or_default()
                .into_iter()
                .map(ToolDescriptor::from)
                .collect(),
            last_connected: self.last_connected,
            error: self.error,
            duration_ms: self.duration_ms.map(|value| value as u64),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTool {
    name: String,
    description: Option<String>,
    input_schema: Option<Value>,
}

impl From<ToolDescriptor> for PersistedTool {
    fn from(tool: ToolDescriptor) -> Self {
        Self {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
        }
    }
}

impl From<PersistedTool> for ToolDescriptor {
    fn from(tool: PersistedTool) -> Self {
        Self {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
        }
    }
}

fn require_changed(changed: usize, name: &str) -> Result<(), McpApplicationError> {
    if changed == 0 {
        Err(McpApplicationError::ServerNotFound(name.to_string()))
    } else {
        Ok(())
    }
}

fn to_json<T: Serialize>(value: &Option<T>) -> Result<Option<String>, McpApplicationError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| McpApplicationError::Validation(error.to_string()))
}

fn from_json<T: DeserializeOwned>(value: Option<&str>) -> Result<Option<T>, McpApplicationError> {
    value
        .filter(|json| !json.trim().is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| McpApplicationError::Validation(error.to_string()))
}

fn app_error(error: crate::platform::database::DatabaseError) -> McpApplicationError {
    match error {
        crate::platform::database::DatabaseError::Database(error) => {
            McpApplicationError::Database(error.to_string())
        }
        crate::platform::database::DatabaseError::Storage(message) => {
            McpApplicationError::Storage(message)
        }
    }
}

fn database_error(error: rusqlite::Error) -> McpApplicationError {
    McpApplicationError::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn repository(name: &str) -> (TempDirectory, SqliteMcpServerRepository) {
        let directory = TempDirectory::new(name);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteMcpServerRepository::new(database);
        (directory, repository)
    }

    fn server(name: &str, scope: Scope, project_path: Option<&str>) -> ServerConfiguration {
        ServerConfiguration::create(ServerConfigurationDraft {
            name: name.to_string(),
            transport_type: TransportType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: None,
            headers: None,
            description: Some("fixture".to_string()),
            active: true,
            scope,
            project_path: project_path.map(str::to_string),
        })
        .expect("server")
    }

    #[test]
    fn sqlite_adapter_round_trips_domain_configuration_and_filters_project_scope() {
        let (_directory, repository) = repository("mcp-sqlite-round-trip");
        let user = server("user-tools", Scope::User, None);
        let project = server("project-tools", Scope::Project, Some("D:\\code\\one"));
        repository.insert(&user, "100").expect("insert user");
        repository.insert(&project, "101").expect("insert project");

        let visible = repository
            .list_visible("D:\\code\\one")
            .expect("visible servers");
        assert_eq!(
            visible
                .iter()
                .map(|server| server.name().as_str())
                .collect::<Vec<_>>(),
            vec!["project-tools", "user-tools"]
        );
        assert_eq!(
            repository
                .list_visible("D:\\code\\other")
                .expect("other project")
                .iter()
                .map(|server| server.name().as_str())
                .collect::<Vec<_>>(),
            vec!["user-tools"]
        );
        let loaded = repository
            .find("user-tools")
            .expect("find")
            .expect("server");
        assert_eq!(loaded.args(), Some(["server.js".to_string()].as_slice()));
        assert_eq!(loaded.description(), Some("fixture"));
    }

    #[test]
    fn rename_preserves_cached_connection_status_tools_and_duration() {
        let (_directory, repository) = repository("mcp-sqlite-rename");
        repository
            .insert(&server("first-name", Scope::User, None), "100")
            .expect("insert");
        let outcome = ConnectionOutcome::connected(
            vec![ToolDescriptor {
                name: "search".to_string(),
                description: None,
                input_schema: Some(serde_json::json!({ "type": "object" })),
            }],
            17,
        );
        repository
            .record_connection_outcome("first-name", &outcome, "101")
            .expect("record");
        repository
            .replace(
                "first-name",
                &server("second-name", Scope::User, None),
                "102",
            )
            .expect("rename");

        let status = repository.status("second-name").expect("status");
        assert_eq!(status.connection_status, ConnectionStatus::Connected);
        assert_eq!(status.tools[0].name, "search");
        assert_eq!(status.duration_ms, Some(17));
        assert_eq!(status.last_connected.as_deref(), Some("101"));
    }

    #[test]
    fn row_mapping_applies_documented_legacy_fallbacks_through_domain_construction() {
        let (_directory, repository) = repository("mcp-sqlite-legacy-row");
        let connection = repository.database.connection().expect("connection");
        connection
            .execute(
                r#"
                INSERT INTO mcp_servers
                (name, transport_type, command, active, scope, created_at, updated_at)
                VALUES ('legacy-tools', 'unknown', 'legacy-mcp', 1, 'unknown', '100', '100')
                "#,
                [],
            )
            .expect("legacy row");

        let loaded = repository
            .find("legacy-tools")
            .expect("find")
            .expect("server");
        assert_eq!(loaded.transport_type(), TransportType::Stdio);
        assert_eq!(loaded.scope(), Scope::User);
        assert_eq!(loaded.command(), Some("legacy-mcp"));
    }
}
