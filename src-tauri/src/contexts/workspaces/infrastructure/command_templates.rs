use crate::contexts::workspaces::domain::{CommandTemplate, CommandTemplateError, CommandTemplateScope};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub(crate) struct SqliteCommandTemplateRepository { database: NativeDatabase }

impl SqliteCommandTemplateRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self { Self { database } }
    pub(crate) fn save(&self, template: &CommandTemplate) -> Result<(), String> {
        template.validate().map_err(|error: CommandTemplateError| error.to_string())?;
        let connection = self.database.connection().map_err(|error| error.to_string())?;
        connection.execute("INSERT OR REPLACE INTO terminal_command_templates (id,name,command,scope,connection_id,workspace_uri,working_directory,tags,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![template.id, template.name, template.command, scope_name(template.scope), template.connection_id, template.workspace_uri, template.working_directory, template.tags_json, template.created_at, template.updated_at]).map_err(|error| error.to_string())?;
        Ok(())
    }
    pub(crate) fn find(&self, id: &str) -> Result<Option<CommandTemplate>, String> {
        let connection = self.database.connection().map_err(|error| error.to_string())?;
        connection.query_row("SELECT id,name,command,scope,connection_id,workspace_uri,working_directory,tags,created_at,updated_at FROM terminal_command_templates WHERE id=?1", params![id], read_template).optional().map_err(|error| error.to_string())
    }

    pub(crate) fn list(&self, scope: Option<CommandTemplateScope>, connection_id: Option<&str>, workspace_uri: Option<&str>) -> Result<Vec<CommandTemplate>, String> {
        let connection = self.database.connection().map_err(|error| error.to_string())?;
        let mut statement = connection.prepare("SELECT id,name,command,scope,connection_id,workspace_uri,working_directory,tags,created_at,updated_at FROM terminal_command_templates WHERE (?1 IS NULL OR scope=?1) AND (?2 IS NULL OR connection_id=?2) AND (?3 IS NULL OR workspace_uri=?3) ORDER BY updated_at DESC").map_err(|error| error.to_string())?;
        let rows = statement.query_map(params![scope.map(scope_name), connection_id, workspace_uri], read_template).map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }

    pub(crate) fn delete(&self, id: &str) -> Result<(), String> {
        let connection = self.database.connection().map_err(|error| error.to_string())?;
        connection.execute("DELETE FROM terminal_command_templates WHERE id=?1", params![id]).map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn scope_name(scope: CommandTemplateScope) -> &'static str { match scope { CommandTemplateScope::Global => "global", CommandTemplateScope::Connection => "connection", CommandTemplateScope::Workspace => "workspace" } }
fn read_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommandTemplate> { Ok(CommandTemplate { id: row.get(0)?, name: row.get(1)?, command: row.get(2)?, scope: match row.get::<_, String>(3)?.as_str() { "global" => CommandTemplateScope::Global, "connection" => CommandTemplateScope::Connection, _ => CommandTemplateScope::Workspace }, connection_id: row.get(4)?, workspace_uri: row.get(5)?, working_directory: row.get(6)?, tags_json: row.get(7)?, created_at: row.get(8)?, updated_at: row.get(9)? }) }
