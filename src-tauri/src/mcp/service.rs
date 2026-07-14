use crate::mcp::models::*;
use crate::AppError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_project_path() -> Result<String, AppError> {
    let path = std::env::current_dir().map_err(|error| AppError::Storage(error.to_string()))?;
    absolutize(path)
}

fn absolutize(path: PathBuf) -> Result<String, AppError> {
    match path.canonicalize() {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(_) => Ok(path.to_string_lossy().to_string()),
    }
}

pub fn list_servers(conn: &Connection) -> Result<Vec<McpServerConfig>, AppError> {
    let project_path = current_project_path()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT name, transport_type, command, args, env, url, headers, description, active, scope, project_path
        FROM mcp_servers
        WHERE scope = 'user' OR (scope = 'project' AND project_path = ?1)
        ORDER BY active DESC, name ASC
        "#,
    )?;
    let rows = stmt.query_map(params![project_path], row_to_config)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::Database)
}

pub fn add_server(conn: &Connection, mut config: McpServerConfig) -> Result<(), AppError> {
    validate_new_name(conn, &config.name, None)?;
    apply_scope_project_path(&mut config)?;
    validate_config(&config)?;
    let now = now_string();
    conn.execute(
        r#"
        INSERT INTO mcp_servers
        (name, transport_type, command, args, env, url, headers, description, active, scope, project_path, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            config.name,
            config.transport_type.as_str(),
            config.command,
            to_json(&config.args)?,
            to_json(&config.env)?,
            config.url,
            to_json(&config.headers)?,
            config.description,
            config.active as i32,
            config.scope.as_str(),
            config.project_path,
            now,
            now,
        ],
    )?;
    Ok(())
}

pub fn update_server(
    conn: &Connection,
    name: &str,
    patch: PartialMcpServerConfig,
) -> Result<(), AppError> {
    let mut config = get_server_from_db(conn, name)?;
    if let Some(next_name) = patch.name {
        if next_name != name {
            validate_new_name(conn, &next_name, Some(name))?;
            config.name = next_name;
        }
    }
    if let Some(value) = patch.transport_type {
        config.transport_type = value;
    }
    if let Some(value) = patch.command {
        config.command = value;
    }
    if let Some(value) = patch.args {
        config.args = value;
    }
    if let Some(value) = patch.env {
        config.env = value;
    }
    if let Some(value) = patch.url {
        config.url = value;
    }
    if let Some(value) = patch.headers {
        config.headers = value;
    }
    if let Some(value) = patch.description {
        config.description = value;
    }
    if let Some(value) = patch.active {
        config.active = value;
    }
    if let Some(value) = patch.scope {
        config.scope = value;
    }
    apply_scope_project_path(&mut config)?;
    validate_config(&config)?;
    conn.execute(
        r#"
        UPDATE mcp_servers
        SET name = ?1, transport_type = ?2, command = ?3, args = ?4, env = ?5, url = ?6,
            headers = ?7, description = ?8, active = ?9, scope = ?10, project_path = ?11, updated_at = ?12
        WHERE name = ?13
        "#,
        params![
            config.name,
            config.transport_type.as_str(),
            config.command,
            to_json(&config.args)?,
            to_json(&config.env)?,
            config.url,
            to_json(&config.headers)?,
            config.description,
            config.active as i32,
            config.scope.as_str(),
            config.project_path,
            now_string(),
            name,
        ],
    )?;
    Ok(())
}

pub fn remove_server(conn: &Connection, name: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM mcp_servers WHERE name = ?1", params![name])?;
    if changed == 0 {
        return Err(AppError::McpServerNotFound(name.to_string()));
    }
    Ok(())
}

pub fn toggle_server(conn: &Connection, name: &str, active: bool) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE mcp_servers SET active = ?1, updated_at = ?2 WHERE name = ?3",
        params![active as i32, now_string(), name],
    )?;
    if changed == 0 {
        return Err(AppError::McpServerNotFound(name.to_string()));
    }
    Ok(())
}

pub fn get_server_status(conn: &Connection, name: &str) -> Result<McpServerStatus, AppError> {
    let row = conn
        .query_row(
            "SELECT name, active, last_connection_status, last_connected, last_error, last_tools, last_test_duration_ms FROM mcp_servers WHERE name = ?1",
            params![name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::McpServerNotFound(name.to_string()))?;
    let tools = from_json_opt::<Vec<McpToolInfo>>(row.5.as_deref())?.unwrap_or_default();
    let cached_status = row
        .2
        .as_deref()
        .and_then(McpConnectionStatus::parse)
        .unwrap_or(McpConnectionStatus::Disconnected);
    Ok(McpServerStatus {
        name: row.0,
        connection_status: if row.1 == 0 {
            McpConnectionStatus::Disabled
        } else {
            cached_status
        },
        tools,
        last_connected: row.3,
        error: row.4,
        duration_ms: row.6.map(|value| value as u64),
    })
}

pub fn get_server_from_db(conn: &Connection, name: &str) -> Result<McpServerConfig, AppError> {
    conn.query_row(
        r#"
        SELECT name, transport_type, command, args, env, url, headers, description, active, scope, project_path
        FROM mcp_servers WHERE name = ?1
        "#,
        params![name],
        row_to_config,
    )
    .optional()?
    .ok_or_else(|| AppError::McpServerNotFound(name.to_string()))
}

pub fn record_test_result(
    conn: &Connection,
    name: &str,
    result: &McpTestResult,
) -> Result<(), AppError> {
    let changed = conn.execute(
        r#"
        UPDATE mcp_servers
        SET last_connection_status = ?1, last_connected = ?2, last_error = ?3,
            last_tools = ?4, last_test_duration_ms = ?5, updated_at = ?6
        WHERE name = ?7
        "#,
        params![
            if result.success { "connected" } else { "error" },
            if result.success { Some(now_string()) } else { None },
            result.error,
            to_json(&Some(result.tools.clone()))?,
            result.duration_ms.map(|value| value as i64),
            now_string(),
            name,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::McpServerNotFound(name.to_string()));
    }
    Ok(())
}

pub fn import_servers(
    conn: &Connection,
    data: McpImportExport,
    scope: McpScope,
) -> Result<McpImportResult, AppError> {
    let mut result = McpImportResult::default();
    for (name, entry) in data.mcp_servers {
        if server_exists(conn, &name)? || !is_kebab_case(&name) {
            result.skipped.push(name);
            continue;
        }
        let transport_type = if entry.command.as_deref().unwrap_or("").trim().is_empty() {
            McpTransportType::Sse
        } else {
            McpTransportType::Stdio
        };
        let config = McpServerConfig {
            name: name.clone(),
            transport_type,
            command: entry.command,
            args: entry.args,
            env: entry.env,
            url: entry.url,
            headers: entry.headers,
            description: None,
            active: true,
            scope: scope.clone(),
            project_path: None,
        };
        match add_server(conn, config) {
            Ok(()) => result.imported.push(name),
            Err(_) => result.skipped.push(name),
        }
    }
    Ok(result)
}

pub fn export_servers(conn: &Connection, names: Vec<String>) -> Result<McpImportExport, AppError> {
    let mut mcp_servers = BTreeMap::new();
    for name in names {
        let config = get_server_from_db(conn, &name)?;
        let entry = match config.transport_type {
            McpTransportType::Stdio => McpImportServerEntry {
                command: config.command,
                args: config.args,
                env: config.env,
                ..Default::default()
            },
            McpTransportType::Sse | McpTransportType::StreamableHttp => McpImportServerEntry {
                url: config.url,
                headers: config.headers,
                ..Default::default()
            },
        };
        mcp_servers.insert(config.name, entry);
    }
    Ok(McpImportExport { mcp_servers })
}

fn row_to_config(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerConfig> {
    let transport_type = row.get::<_, String>(1)?;
    let scope = row.get::<_, String>(9)?;
    Ok(McpServerConfig {
        name: row.get(0)?,
        transport_type: McpTransportType::parse(&transport_type).unwrap_or(McpTransportType::Stdio),
        command: row.get(2)?,
        args: from_json_opt(row.get::<_, Option<String>>(3)?.as_deref())
            .map_err(json_to_sql_error)?,
        env: from_json_opt(row.get::<_, Option<String>>(4)?.as_deref()).map_err(json_to_sql_error)?,
        url: row.get(5)?,
        headers: from_json_opt(row.get::<_, Option<String>>(6)?.as_deref())
            .map_err(json_to_sql_error)?,
        description: row.get(7)?,
        active: row.get::<_, i32>(8)? != 0,
        scope: McpScope::parse(&scope).unwrap_or(McpScope::User),
        project_path: row.get(10)?,
    })
}

fn validate_config(config: &McpServerConfig) -> Result<(), AppError> {
    if !is_kebab_case(&config.name) {
        return Err(AppError::Validation(
            "MCP server name must be kebab-case letters, digits, and hyphens".to_string(),
        ));
    }
    match config.transport_type {
        McpTransportType::Stdio => {
            if config.command.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::Validation(
                    "stdio MCP server requires command".to_string(),
                ));
            }
        }
        McpTransportType::Sse | McpTransportType::StreamableHttp => {
            if config.url.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::Validation("URL MCP server requires url".to_string()));
            }
        }
    }
    Ok(())
}

fn validate_new_name(
    conn: &Connection,
    name: &str,
    existing_name: Option<&str>,
) -> Result<(), AppError> {
    if !is_kebab_case(name) {
        return Err(AppError::Validation(
            "MCP server name must be kebab-case letters, digits, and hyphens".to_string(),
        ));
    }
    if server_exists(conn, name)? && existing_name != Some(name) {
        return Err(AppError::Validation(format!(
            "MCP server name already exists: {name}"
        )));
    }
    Ok(())
}

fn apply_scope_project_path(config: &mut McpServerConfig) -> Result<(), AppError> {
    match config.scope {
        McpScope::User => config.project_path = None,
        McpScope::Project => config.project_path = Some(current_project_path()?),
    }
    Ok(())
}

fn server_exists(conn: &Connection, name: &str) -> Result<bool, AppError> {
    let exists = conn.query_row(
        "SELECT 1 FROM mcp_servers WHERE name = ?1",
        params![name],
        |_| Ok(()),
    );
    match exists {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(error) => Err(AppError::Database(error)),
    }
}

fn is_kebab_case(value: &str) -> bool {
    if value.is_empty() || value.starts_with('-') || value.ends_with('-') {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn to_json<T: Serialize>(value: &Option<T>) -> Result<Option<String>, AppError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn from_json_opt<T: DeserializeOwned>(value: Option<&str>) -> Result<Option<T>, AppError> {
    value
        .filter(|json| !json.trim().is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn json_to_sql_error(error: AppError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn now_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            r#"
            CREATE TABLE mcp_servers (
                name TEXT PRIMARY KEY,
                transport_type TEXT NOT NULL DEFAULT 'stdio',
                command TEXT,
                args TEXT,
                env TEXT,
                url TEXT,
                headers TEXT,
                description TEXT,
                active INTEGER NOT NULL DEFAULT 1,
                scope TEXT NOT NULL DEFAULT 'user',
                project_path TEXT,
                last_connection_status TEXT,
                last_connected TEXT,
                last_error TEXT,
                last_tools TEXT,
                last_test_duration_ms INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .expect("schema");
        conn
    }

    fn stdio_config(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            transport_type: McpTransportType::Stdio,
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            env: None,
            url: None,
            headers: None,
            description: None,
            active: true,
            scope: McpScope::User,
            project_path: None,
        }
    }

    #[test]
    fn rejects_invalid_names() {
        let conn = conn();
        let mut config = stdio_config("Bad_Name");
        assert!(add_server(&conn, config.clone()).is_err());
        config.name = "good-name".to_string();
        assert!(add_server(&conn, config).is_ok());
    }

    #[test]
    fn rename_preserves_status_cache() {
        let conn = conn();
        add_server(&conn, stdio_config("first-name")).expect("add");
        record_test_result(
            &conn,
            "first-name",
            &McpTestResult {
                success: true,
                tools: vec![McpToolInfo {
                    name: "search".to_string(),
                    description: None,
                    input_schema: Some(Value::Object(Default::default())),
                }],
                error: None,
                duration_ms: Some(10),
            },
        )
        .expect("record");
        update_server(
            &conn,
            "first-name",
            PartialMcpServerConfig {
                name: Some("second-name".to_string()),
                ..Default::default()
            },
        )
        .expect("rename");
        let status = get_server_status(&conn, "second-name").expect("status");
        assert!(matches!(status.connection_status, McpConnectionStatus::Connected));
        assert_eq!(status.tools.len(), 1);
    }

    #[test]
    fn import_skips_conflicts_and_defaults_url_to_sse() {
        let conn = conn();
        add_server(&conn, stdio_config("existing")).expect("add");
        let mut data = McpImportExport::default();
        data.mcp_servers.insert(
            "existing".to_string(),
            McpImportServerEntry {
                command: Some("node".to_string()),
                ..Default::default()
            },
        );
        data.mcp_servers.insert(
            "remote-tools".to_string(),
            McpImportServerEntry {
                url: Some("http://localhost:8000/mcp".to_string()),
                ..Default::default()
            },
        );
        let result = import_servers(&conn, data, McpScope::User).expect("import");
        assert_eq!(result.imported, vec!["remote-tools"]);
        assert_eq!(result.skipped, vec!["existing"]);
        let config = get_server_from_db(&conn, "remote-tools").expect("config");
        assert!(matches!(config.transport_type, McpTransportType::Sse));
    }

    #[test]
    fn export_excludes_internal_fields() {
        let conn = conn();
        add_server(&conn, stdio_config("stdio-server")).expect("add");
        let data = export_servers(&conn, vec!["stdio-server".to_string()]).expect("export");
        let entry = data.mcp_servers.get("stdio-server").expect("entry");
        assert_eq!(entry.command.as_deref(), Some("node"));
        assert!(entry.url.is_none());
    }
}
