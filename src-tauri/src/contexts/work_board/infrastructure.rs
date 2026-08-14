use crate::platform::database::{DatabaseError, NativeDatabase};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use super::models::{PlanSummary, WorkItem, WorkItemSourceLink};

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS work_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            stage TEXT NOT NULL CHECK(stage IN ('inbox','planned','in_progress','review','done')),
            priority TEXT NOT NULL CHECK(priority IN ('none','low','medium','high','urgent')),
            rank INTEGER NOT NULL,
            project_path TEXT,
            due_at TEXT,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS work_item_links (
            work_item_id TEXT NOT NULL,
            source_kind TEXT NOT NULL CHECK(source_kind IN ('session','plan','plan_run','scheduled_task')),
            source_id TEXT NOT NULL,
            relation TEXT NOT NULL CHECK(relation IN ('primary','execution','automation','supporting')),
            created_at TEXT NOT NULL,
            PRIMARY KEY (source_kind, source_id),
            FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_work_items_board ON work_items(archived, stage, rank);
        CREATE INDEX IF NOT EXISTS idx_work_items_project ON work_items(project_path, archived);
        CREATE INDEX IF NOT EXISTS idx_work_item_links_item ON work_item_links(work_item_id);
        CREATE TABLE IF NOT EXISTS scheduled_task_runs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            session_id TEXT,
            status TEXT NOT NULL,
            error TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (task_id) REFERENCES scheduled_tasks(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_scheduled_task_runs_task ON scheduled_task_runs(task_id, started_at DESC);
        "#,
    )?;
    add_column(
        connection,
        "sessions",
        "origin_kind",
        "TEXT NOT NULL DEFAULT 'user'",
    )?;
    add_column(connection, "sessions", "origin_id", "TEXT")?;
    connection.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_origin ON sessions(origin_kind, origin_id);
        UPDATE sessions
        SET origin_kind = 'scheduled_task',
            origin_id = (
                SELECT task.id
                FROM scheduled_tasks AS task
                WHERE task.latest_run_session_id = sessions.id
                LIMIT 1
            )
        WHERE EXISTS (
            SELECT 1 FROM scheduled_tasks AS task
            WHERE task.latest_run_session_id = sessions.id
        );
        UPDATE sessions
        SET origin_kind = 'scheduled_task',
            origin_id = (
                SELECT history.task_id
                FROM scheduled_task_runs AS history
                WHERE history.session_id = sessions.id
                ORDER BY history.started_at DESC
                LIMIT 1
            )
        WHERE EXISTS (
            SELECT 1 FROM scheduled_task_runs AS history
            WHERE history.session_id = sessions.id
        );
        UPDATE sessions
        SET origin_kind = 'plan_attempt',
            origin_id = (
                SELECT run.plan_id
                FROM plan_subtask_attempts AS attempt
                JOIN plan_subtask_runs AS task ON task.id = attempt.subtask_run_id
                JOIN plan_runs AS run ON run.id = task.plan_run_id
                WHERE attempt.session_id = sessions.id
                LIMIT 1
            )
        WHERE EXISTS (
            SELECT 1
            FROM plan_subtask_attempts AS attempt
            WHERE attempt.session_id = sessions.id
        );
        "#,
    )?;
    Ok(())
}

fn add_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), DatabaseError> {
    if crate::platform::database::table_has_column(connection, table, column)? {
        return Ok(());
    }
    connection.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}"
    ))?;
    Ok(())
}

pub(crate) fn reconcile(database: &NativeDatabase) -> Result<(), String> {
    let mut connection = database.connection().map_err(error)?;
    let transaction = connection.transaction().map_err(error)?;
    reconcile_query(&transaction, "session", "inbox", "SELECT id, title, project_path, updated_at FROM sessions WHERE archived = 0 AND COALESCE(origin_kind, 'user') = 'user'")?;
    reconcile_query(
        &transaction,
        "scheduled_task",
        "planned",
        "SELECT id, name, NULL, updated_at FROM scheduled_tasks",
    )?;
    reconcile_plans(&transaction)?;
    transaction.commit().map_err(error)
}

fn reconcile_query(
    connection: &Connection,
    kind: &str,
    stage: &str,
    query: &str,
) -> Result<(), String> {
    let mut statement = connection.prepare(query).map_err(error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (id, title, project, timestamp) = row.map_err(error)?;
        insert_source_item(
            connection,
            kind,
            &id,
            &title,
            stage,
            project.as_deref(),
            &timestamp,
        )?;
    }
    Ok(())
}

fn reconcile_plans(connection: &Connection) -> Result<(), String> {
    let mut statement = connection.prepare(
        "SELECT plan.id, COALESCE(version.goal, 'Plan ' || plan.id), version.project_path, plan.updated_at FROM plans AS plan LEFT JOIN plan_versions AS version ON version.plan_id = plan.id AND version.version = plan.current_version",
    ).map_err(error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (id, title, project, timestamp) = row.map_err(error)?;
        insert_source_item(
            connection,
            "plan",
            &id,
            &title,
            "planned",
            project.as_deref(),
            &timestamp,
        )?;
    }
    Ok(())
}

fn insert_source_item(
    connection: &Connection,
    kind: &str,
    source_id: &str,
    title: &str,
    stage: &str,
    project: Option<&str>,
    timestamp: &str,
) -> Result<(), String> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM work_item_links WHERE source_kind = ?1 AND source_id = ?2",
            params![kind, source_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(error)?
        .is_some();
    if exists {
        return Ok(());
    }
    let id = format!("work-item-{}", Uuid::new_v4());
    let rank: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(rank), 0) + 1000 FROM work_items WHERE stage = ?1",
            [stage],
            |row| row.get(0),
        )
        .map_err(error)?;
    connection.execute("INSERT INTO work_items (id,title,description,stage,priority,rank,project_path,due_at,archived,created_at,updated_at) VALUES (?1,?2,'',?3,'none',?4,?5,NULL,0,?6,?6)", params![id,title,stage,rank,project,timestamp]).map_err(error)?;
    connection.execute("INSERT INTO work_item_links (work_item_id,source_kind,source_id,relation,created_at) VALUES (?1,?2,?3,'primary',?4)", params![id,kind,source_id,timestamp]).map_err(error)?;
    Ok(())
}

pub(crate) fn list(database: &NativeDatabase, archived: bool) -> Result<Vec<WorkItem>, String> {
    let connection = database.connection().map_err(error)?;
    let mut statement = connection.prepare("SELECT id,title,description,stage,priority,rank,project_path,due_at,archived,created_at,updated_at FROM work_items WHERE archived = ?1 ORDER BY stage, rank, id").map_err(error)?;
    let mut items = statement
        .query_map([i64::from(archived)], read_item)
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    for item in &mut items {
        item.sources = load_sources(&connection, &item.id)?;
    }
    Ok(items)
}

pub(crate) fn load(database: &NativeDatabase, id: &str) -> Result<WorkItem, String> {
    let connection = database.connection().map_err(error)?;
    let mut item = connection.query_row("SELECT id,title,description,stage,priority,rank,project_path,due_at,archived,created_at,updated_at FROM work_items WHERE id = ?1", [id], read_item).optional().map_err(error)?.ok_or_else(|| "Work item not found.".to_string())?;
    item.sources = load_sources(&connection, id)?;
    Ok(item)
}

fn read_item(row: &Row<'_>) -> rusqlite::Result<WorkItem> {
    Ok(WorkItem {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        stage: row.get(3)?,
        priority: row.get(4)?,
        rank: row.get(5)?,
        project_path: row.get(6)?,
        due_at: row.get(7)?,
        archived: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        sources: Vec::new(),
    })
}

fn load_sources(connection: &Connection, item_id: &str) -> Result<Vec<WorkItemSourceLink>, String> {
    let mut statement = connection.prepare("SELECT source_kind,source_id,relation FROM work_item_links WHERE work_item_id = ?1 ORDER BY created_at, source_kind").map_err(error)?;
    let rows = statement
        .query_map([item_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    rows.into_iter()
        .map(|(kind, id, relation)| project_source(connection, &kind, &id, &relation))
        .collect()
}

fn project_source(
    connection: &Connection,
    kind: &str,
    id: &str,
    relation: &str,
) -> Result<WorkItemSourceLink, String> {
    let query = match kind { "session" => "SELECT title,lifecycle_state,project_path,updated_at FROM sessions WHERE id=?1", "scheduled_task" => "SELECT name,CASE WHEN enabled=0 THEN 'disabled' ELSE latest_status END,NULL,updated_at FROM scheduled_tasks WHERE id=?1", "plan" => "SELECT COALESCE(version.goal,'Plan '||plan.id),plan.status,version.project_path,plan.updated_at FROM plans AS plan LEFT JOIN plan_versions AS version ON version.plan_id=plan.id AND version.version=plan.current_version WHERE plan.id=?1", "plan_run" => "SELECT 'Plan run '||id,status,project_path,updated_at FROM plan_runs WHERE id=?1", _ => return Err("Unsupported work item source kind.".to_string()) };
    let projection = connection
        .query_row(query, [id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .optional()
        .map_err(error)?;
    let (title, status, project, updated, available) = projection.map_or_else(
        || (id.to_string(), "unavailable".to_string(), None, None, false),
        |(a, b, c, d)| (a, b, c, Some(d), true),
    );
    Ok(WorkItemSourceLink {
        source_kind: kind.to_string(),
        source_id: id.to_string(),
        relation: relation.to_string(),
        title,
        status,
        available,
        project_path: project,
        updated_at: updated,
    })
}

pub(crate) fn timestamp() -> String {
    Utc::now().to_rfc3339()
}
pub(crate) fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}

pub(crate) fn list_plan_summaries(database: &NativeDatabase) -> Result<Vec<PlanSummary>, String> {
    let connection = database.connection().map_err(error)?;
    let mut statement = connection.prepare(
        r#"SELECT plan.id, COALESCE(version.goal, 'Plan ' || plan.id), COALESCE(version.project_path, ''), plan.status,
                  run.id, run.status, plan.created_at, plan.updated_at
           FROM plans AS plan
           LEFT JOIN plan_versions AS version ON version.plan_id=plan.id AND version.version=plan.current_version
           LEFT JOIN plan_runs AS run ON run.id=(SELECT id FROM plan_runs WHERE plan_id=plan.id ORDER BY created_at DESC,id DESC LIMIT 1)
           ORDER BY plan.updated_at DESC, plan.id DESC"#,
    ).map_err(error)?;
    let values = statement
        .query_map([], |row| {
            Ok(PlanSummary {
                id: row.get(0)?,
                goal: row.get(1)?,
                project_path: row.get(2)?,
                status: row.get(3)?,
                latest_run_id: row.get(4)?,
                latest_run_status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(values)
}
