use crate::contexts::workspaces::domain::{CommandRun, CommandRunStatus};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub(crate) struct SqliteCommandRunRepository {
    database: NativeDatabase,
}

impl SqliteCommandRunRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
    pub(crate) fn save(&self, run: &CommandRun) -> Result<(), String> {
        run.validate().map_err(|error| error.to_string())?;
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection.execute("INSERT OR REPLACE INTO terminal_command_runs (id,template_id,session_id,connection_id,command_snapshot,working_directory,status,exit_code,started_at,finished_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![run.id, run.template_id, run.session_id, run.connection_id, run.command_snapshot, run.working_directory, status_name(run.status), run.exit_code, run.started_at, run.finished_at]).map_err(|error| error.to_string())?;
        Ok(())
    }
    pub(crate) fn list(
        &self,
        session_id: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<CommandRun>, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        let mut statement = connection.prepare("SELECT id,template_id,session_id,connection_id,command_snapshot,working_directory,status,exit_code,started_at,finished_at FROM terminal_command_runs WHERE session_id=?1 ORDER BY started_at DESC, id DESC LIMIT ?2 OFFSET ?3").map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![session_id, limit.clamp(1, 100), offset], read_run)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }
    pub(crate) fn find(&self, id: &str) -> Result<Option<CommandRun>, String> {
        let connection = self
            .database
            .connection()
            .map_err(|error| error.to_string())?;
        connection.query_row("SELECT id,template_id,session_id,connection_id,command_snapshot,working_directory,status,exit_code,started_at,finished_at FROM terminal_command_runs WHERE id=?1", params![id], read_run).optional().map_err(|error| error.to_string())
    }
}

fn status_name(status: CommandRunStatus) -> &'static str {
    match status {
        CommandRunStatus::Queued => "queued",
        CommandRunStatus::Running => "running",
        CommandRunStatus::Succeeded => "succeeded",
        CommandRunStatus::Failed => "failed",
        CommandRunStatus::Cancelled => "cancelled",
    }
}
fn read_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommandRun> {
    Ok(CommandRun {
        id: row.get(0)?,
        template_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_id: row.get(3)?,
        command_snapshot: row.get(4)?,
        working_directory: row.get(5)?,
        status: match row.get::<_, String>(6)?.as_str() {
            "queued" => CommandRunStatus::Queued,
            "running" => CommandRunStatus::Running,
            "succeeded" => CommandRunStatus::Succeeded,
            "cancelled" => CommandRunStatus::Cancelled,
            _ => CommandRunStatus::Failed,
        },
        exit_code: row.get(7)?,
        started_at: row.get(8)?,
        finished_at: row.get(9)?,
    })
}
