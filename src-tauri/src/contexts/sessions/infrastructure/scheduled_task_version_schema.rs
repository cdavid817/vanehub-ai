use crate::platform::database::{table_has_column, DatabaseError};
use rusqlite::Connection;

/// Adds `scheduled_tasks.version`, the optimistic-concurrency counter `update_scheduled_task`
/// (19.8) checks before writing an edited name/content/agent/frequency.
///
/// `DEFAULT 1` gives every pre-existing task the same starting point a freshly created one gets:
/// `create_scheduled_task` also writes `1` explicitly rather than leaning on this default, so the
/// two paths agree by inspection, not by coincidence of a schema default nobody else restates. A
/// task that predates this column was never edited through a version-checked path, so "1" is not a
/// guess about its history -- it is simply the first value this counter is able to express,
/// matching how `LoopDefinition.version` and `PersonalizationPolicy.revision` both begin counting
/// at a fixed point rather than trying to reconstruct one.
///
/// Deliberately does not bump on the due-task sweep's own status bookkeeping
/// (`mark_task_running_with_trigger` / `mark_task_succeeded` / `mark_task_failed`) or on
/// `set_scheduled_task_enabled`: none of those write the fields this column guards, and bumping it
/// there would fail an unrelated, legitimate concurrent edit for a reason the editor never caused.
pub(crate) fn apply_scheduled_task_version_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    if table_has_column(connection, "scheduled_tasks", "version")? {
        return Ok(());
    }
    connection.execute(
        "ALTER TABLE scheduled_tasks ADD COLUMN version INTEGER NOT NULL DEFAULT 1",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::database::migrate;

    fn seeded_agent(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind) \
                 VALUES ('codex-cli', 'Codex CLI', 'OpenAI', 'cli')",
                [],
            )
            .expect("seed agent");
    }

    /// A fresh `migrate()` already carries this migration, so the interesting case is a row
    /// inserted the way pre-19.8 code always did -- no `version` in the column list -- which
    /// exercises the schema `DEFAULT` rather than an application-level default.
    #[test]
    fn the_migration_defaults_existing_rows_to_version_one_and_is_idempotent() {
        let connection = Connection::open_in_memory().expect("in-memory sqlite");
        migrate(&connection).expect("migrate");
        seeded_agent(&connection);
        connection
            .execute(
                "INSERT INTO scheduled_tasks \
                 (id, name, content, agent_id, frequency, enabled, next_run_at, \
                  latest_status, created_at, updated_at) \
                 VALUES ('task-1', 'Task', 'Do it', 'codex-cli', \
                 '{\"kind\":\"minutes\",\"interval\":5}', 1, '2026-08-27T00:00:00Z', \
                 'never-run', '2026-08-27T00:00:00Z', '2026-08-27T00:00:00Z')",
                [],
            )
            .expect("insert task without specifying version");

        let version: i64 = connection
            .query_row(
                "SELECT version FROM scheduled_tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .expect("version column readable");
        assert_eq!(version, 1);

        // Re-applying must not clobber a version an edit already advanced -- the guard is on the
        // column's existence, not on "have I run before."
        connection
            .execute(
                "UPDATE scheduled_tasks SET version = 3 WHERE id = 'task-1'",
                [],
            )
            .expect("advance version");
        apply_scheduled_task_version_schema(&connection).expect("idempotent re-apply");
        let after: i64 = connection
            .query_row(
                "SELECT version FROM scheduled_tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .expect("version column still readable");
        assert_eq!(after, 3);
    }
}
