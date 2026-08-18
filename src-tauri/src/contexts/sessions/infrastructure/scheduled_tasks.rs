// Predates the production panic-shortcut gate; removing this attribute is the
// definition of done for this file, and it may be removed without ceremony.
// TODO(retire-production-panic-shortcuts): 1 pre-existing site.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::commands::error::CommandError;
use crate::commands::sessions::dto;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::database::NativeDatabase;
use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) use crate::commands::sessions::dto::ScheduledTask;

pub(crate) struct ScheduledTaskLogDirectory(PathBuf);

impl ScheduledTaskLogDirectory {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

pub(crate) fn list_scheduled_tasks(
    database: &NativeDatabase,
) -> Result<Vec<dto::ScheduledTask>, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name, content, agent_id, frequency, enabled, next_run_at,
                   latest_status, latest_run_at, latest_run_session_id, latest_error,
                   created_at, updated_at
            FROM scheduled_tasks
            ORDER BY next_run_at ASC
            "#,
        )
        .map_err(command_error)?;
    let tasks = statement
        .query_map([], read_task)
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(tasks)
}

pub(crate) fn list_scheduled_task_runs(
    database: &NativeDatabase,
    task_id: &str,
) -> Result<Vec<dto::ScheduledTaskRun>, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let mut statement = connection.prepare(
        "SELECT id, task_id, session_id, status, error, started_at, completed_at FROM scheduled_task_runs WHERE task_id = ?1 ORDER BY started_at DESC, id DESC LIMIT 100",
    ).map_err(command_error)?;
    let runs = statement
        .query_map([task_id], |row| {
            Ok(dto::ScheduledTaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                status: row.get(3)?,
                error: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
            })
        })
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(runs)
}

pub(crate) fn create_scheduled_task(
    database: &NativeDatabase,
    input: dto::CreateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    let name = input.name.trim();
    let content = input.content.trim();
    if name.is_empty() || content.is_empty() {
        return Err(CommandError::validation(
            "Scheduled task name and content are required.",
        ));
    }
    let next_run_at = compute_next_run(&input.frequency, Local::now())?;
    let frequency = serde_json::to_string(&input.frequency).map_err(command_error)?;
    let connection = database.connection().map_err(command_error)?;
    let launch_kind: Option<String> = connection
        .query_row(
            "SELECT launch_kind FROM agents WHERE id = ?1",
            [&input.agent_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(command_error)?;
    // Scheduled execution has an explicit API route only for OnePiece. Other API agents have no
    // runner contract, so reject them before persisting a task that could never run.
    match (input.agent_id.as_str(), launch_kind.as_deref()) {
        ("onepiece", Some("api")) | (_, Some("cli")) => {}
        (_, Some(_)) => {
            return Err(CommandError::validation(
                "Scheduled tasks support CLI Agents and OnePiece.",
            ));
        }
        (_, None) => {
            return Err(CommandError::validation(
                "Scheduled task references an unsupported Agent.",
            ));
        }
    }
    let id = format!("scheduled-task-{}", Uuid::new_v4());
    let timestamp = Utc::now().to_rfc3339();
    connection
        .execute(
            r#"
            INSERT INTO scheduled_tasks (
                id, name, content, agent_id, frequency, enabled, next_run_at,
                latest_status, latest_run_at, latest_run_session_id, latest_error,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 'never-run', NULL, NULL, NULL, ?7, ?7)
            "#,
            params![
                id,
                name,
                content,
                input.agent_id,
                frequency,
                next_run_at,
                timestamp
            ],
        )
        .map_err(command_error)?;
    load_task(&connection, &id)
}

pub(crate) fn set_scheduled_task_enabled(
    database: &NativeDatabase,
    input: dto::SetScheduledTaskEnabledInput,
) -> Result<dto::ScheduledTask, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let current = load_task(&connection, &input.task_id)?;
    let next_run_at = if input.enabled {
        compute_next_run(&current.frequency, Local::now())?
    } else {
        current.next_run_at
    };
    let timestamp = Utc::now().to_rfc3339();
    let changed = connection
        .execute(
            "UPDATE scheduled_tasks SET enabled = ?1, next_run_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![i64::from(input.enabled), next_run_at, timestamp, input.task_id],
        )
        .map_err(command_error)?;
    if changed == 0 {
        return Err(CommandError::validation("Scheduled task was not found."));
    }
    load_task(&connection, &input.task_id)
}

pub(crate) fn delete_scheduled_task(
    database: &NativeDatabase,
    task_id: &str,
    log_directory: Option<&Path>,
) -> Result<(), CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let changed = connection
        .execute("DELETE FROM scheduled_tasks WHERE id = ?1", [task_id])
        .map_err(command_error)?;
    if changed == 0 {
        return Err(CommandError::validation("Scheduled task was not found."));
    }
    if let Some(log_directory) = log_directory {
        log_scheduled_task(
            log_directory,
            LogSeverity::Info,
            "scheduled-tasks.delete",
            task_id,
            Some(task_id),
        );
    }
    Ok(())
}

fn load_task(
    connection: &rusqlite::Connection,
    task_id: &str,
) -> Result<dto::ScheduledTask, CommandError> {
    connection
        .query_row(
            r#"
            SELECT id, name, content, agent_id, frequency, enabled, next_run_at,
                   latest_status, latest_run_at, latest_run_session_id, latest_error,
                   created_at, updated_at
            FROM scheduled_tasks
            WHERE id = ?1
            "#,
            [task_id],
            read_task,
        )
        .optional()
        .map_err(command_error)?
        .ok_or_else(|| CommandError::validation("Scheduled task was not found."))
}

fn read_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<dto::ScheduledTask> {
    let raw_frequency: String = row.get(4)?;
    let frequency = serde_json::from_str(&raw_frequency).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(dto::ScheduledTask {
        id: row.get(0)?,
        name: row.get(1)?,
        content: row.get(2)?,
        agent_id: row.get(3)?,
        frequency,
        enabled: row.get::<_, i64>(5)? != 0,
        next_run_at: row.get(6)?,
        latest_status: row.get(7)?,
        latest_run_at: row.get(8)?,
        latest_run_session_id: row.get(9)?,
        latest_error: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(crate) fn compute_next_run(
    frequency: &dto::ScheduledTaskFrequency,
    from: DateTime<Local>,
) -> Result<String, CommandError> {
    let next = match frequency {
        dto::ScheduledTaskFrequency::Minutes { interval } => {
            if *interval <= 0 {
                return invalid_frequency();
            }
            from + Duration::minutes(*interval)
        }
        dto::ScheduledTaskFrequency::Hours { interval } => {
            if *interval <= 0 {
                return invalid_frequency();
            }
            from + Duration::hours(*interval)
        }
        dto::ScheduledTaskFrequency::Daily { time_of_day } => {
            next_daily(from, parse_time(time_of_day)?)
        }
        dto::ScheduledTaskFrequency::Weekly {
            weekday,
            time_of_day,
        } => {
            if !(0..=6).contains(weekday) {
                return invalid_frequency();
            }
            next_weekly(from, *weekday as u32, parse_time(time_of_day)?)
        }
        dto::ScheduledTaskFrequency::Monthly {
            day_of_month,
            time_of_day,
        } => {
            if !(1..=31).contains(day_of_month) {
                return invalid_frequency();
            }
            next_monthly(from, *day_of_month as u32, parse_time(time_of_day)?)
        }
    };
    Ok(next.with_timezone(&Utc).to_rfc3339())
}

pub(crate) fn due_tasks(
    database: &NativeDatabase,
    now: DateTime<Utc>,
) -> Result<Vec<ScheduledTask>, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name, content, agent_id, frequency, enabled, next_run_at,
                   latest_status, latest_run_at, latest_run_session_id, latest_error,
                   created_at, updated_at
            FROM scheduled_tasks
            WHERE enabled = 1 AND next_run_at <= ?1
            ORDER BY next_run_at ASC
            "#,
        )
        .map_err(command_error)?;
    let tasks = statement
        .query_map([now.to_rfc3339()], read_task)
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(tasks)
}

pub(crate) fn mark_task_running_with_trigger(
    database: &NativeDatabase,
    task_id: &str,
    backfill: bool,
) -> Result<(), CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let timestamp = Utc::now().to_rfc3339();
    let run_status = if backfill {
        "backfill_running"
    } else {
        "running"
    };
    connection.execute("INSERT INTO scheduled_task_runs (id,task_id,session_id,status,error,started_at,completed_at) VALUES (?1,?2,NULL,?3,NULL,?4,NULL)", params![format!("scheduled-run-{}", Uuid::new_v4()), task_id, run_status, timestamp]).map_err(command_error)?;
    drop(connection);
    update_task_run_metadata(database, task_id, "running", None, None)
}

pub(crate) fn record_task_skipped(
    database: &NativeDatabase,
    task_id: &str,
    reason: &str,
) -> Result<(), CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let timestamp = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO scheduled_task_runs (id,task_id,session_id,status,error,started_at,completed_at) VALUES (?1,?2,NULL,'skipped',?3,?4,?4)",
        params![format!("scheduled-run-{}", Uuid::new_v4()), task_id, reason, timestamp],
    ).map_err(command_error)?;
    Ok(())
}

pub(crate) fn mark_task_succeeded(
    database: &NativeDatabase,
    task: &ScheduledTask,
    session_id: &str,
) -> Result<(), CommandError> {
    let next_run_at = compute_next_run(&task.frequency, Local::now())?;
    update_task_run_metadata(
        database,
        &task.id,
        "succeeded",
        Some(session_id),
        Some(next_run_at),
    )
}

pub(crate) fn mark_task_failed(
    database: &NativeDatabase,
    task: &ScheduledTask,
    error: &str,
) -> Result<(), CommandError> {
    let next_run_at = compute_next_run(&task.frequency, Local::now())?;
    let connection = database.connection().map_err(command_error)?;
    let timestamp = Utc::now().to_rfc3339();
    connection
        .execute(
            r#"
            UPDATE scheduled_tasks
            SET latest_status = 'failed', latest_run_at = ?1, latest_error = ?2,
                next_run_at = ?3, updated_at = ?1
            WHERE id = ?4
            "#,
            params![timestamp, error, next_run_at, task.id],
        )
        .map_err(command_error)?;
    connection.execute("UPDATE scheduled_task_runs SET status='failed', error=?1, completed_at=?2 WHERE id=(SELECT id FROM scheduled_task_runs WHERE task_id=?3 AND completed_at IS NULL ORDER BY started_at DESC LIMIT 1)", params![error, timestamp, task.id]).map_err(command_error)?;
    Ok(())
}

fn update_task_run_metadata(
    database: &NativeDatabase,
    task_id: &str,
    status: &str,
    session_id: Option<&str>,
    next_run_at: Option<String>,
) -> Result<(), CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let timestamp = Utc::now().to_rfc3339();
    connection
        .execute(
            r#"
            UPDATE scheduled_tasks
            SET latest_status = ?1, latest_run_at = ?2, latest_run_session_id = COALESCE(?3, latest_run_session_id),
                latest_error = NULL, next_run_at = COALESCE(?4, next_run_at), updated_at = ?2
            WHERE id = ?5
            "#,
            params![status, timestamp, session_id, next_run_at, task_id],
        )
        .map_err(command_error)?;
    if status == "succeeded" {
        connection.execute("UPDATE scheduled_task_runs SET status=CASE status WHEN 'backfill_running' THEN 'backfilled' ELSE 'succeeded' END, session_id=?1, completed_at=?2 WHERE id=(SELECT id FROM scheduled_task_runs WHERE task_id=?3 AND completed_at IS NULL ORDER BY started_at DESC LIMIT 1)", params![session_id, timestamp, task_id]).map_err(command_error)?;
        if let Some(session_id) = session_id {
            connection
                .execute(
                    "UPDATE sessions SET origin_kind='scheduled_task', origin_id=?1 WHERE id=?2",
                    params![task_id, session_id],
                )
                .map_err(command_error)?;
        }
    }
    Ok(())
}

fn parse_time(value: &str) -> Result<NaiveTime, CommandError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| CommandError::validation("Invalid scheduled time."))
}

fn next_daily(from: DateTime<Local>, time: NaiveTime) -> DateTime<Local> {
    let today = from.date_naive().and_time(time);
    let candidate = Local.from_local_datetime(&today).single().unwrap_or(from);
    if candidate > from {
        candidate
    } else {
        candidate + Duration::days(1)
    }
}

fn next_weekly(from: DateTime<Local>, weekday: u32, time: NaiveTime) -> DateTime<Local> {
    let mut candidate = next_daily(from, time);
    while candidate.weekday().num_days_from_sunday() != weekday {
        candidate += Duration::days(1);
    }
    candidate
}

fn next_monthly(from: DateTime<Local>, day: u32, time: NaiveTime) -> DateTime<Local> {
    let mut candidate = next_daily(from, time);
    while candidate.day() != day.min(days_in_month(candidate.year(), candidate.month())) {
        candidate += Duration::days(1);
    }
    candidate
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("valid month");
    (next_month - Duration::days(1)).day()
}

fn invalid_frequency<T>() -> Result<T, CommandError> {
    Err(CommandError::validation(
        "Invalid scheduled task frequency.",
    ))
}

fn command_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::storage(error.to_string())
}

fn log_scheduled_task(
    log_directory: &Path,
    severity: LogSeverity,
    category: &str,
    message: &str,
    task_id: Option<&str>,
) {
    #[cfg(test)]
    let adapter = UnifiedLoggingAdapter::new(log_directory.to_path_buf());
    #[cfg(not(test))]
    let adapter = UnifiedLoggingAdapter::active(log_directory.to_path_buf());
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "scheduled-task".to_string());
    if let Some(task_id) = task_id {
        context.insert("taskId".to_string(), task_id.to_string());
    }
    let _ = adapter.write_diagnostic(DiagnosticLog {
        severity,
        category: category.to_string(),
        message: message.to_string(),
        context,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use chrono::TimeZone;

    fn database() -> (TempDirectory, NativeDatabase) {
        let directory = TempDirectory::new("scheduled-tasks");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        (directory, database)
    }

    fn insert_task(database: &NativeDatabase, id: &str, enabled: bool, next_run_at: &str) {
        let connection = database.connection().expect("connection");
        let frequency =
            serde_json::to_string(&dto::ScheduledTaskFrequency::Minutes { interval: 5 })
                .expect("frequency");
        connection
            .execute(
                r#"
                INSERT INTO scheduled_tasks (
                    id, name, content, agent_id, frequency, enabled, next_run_at,
                    latest_status, created_at, updated_at
                ) VALUES (?1, 'Task', 'Run it', 'codex-cli', ?2, ?3, ?4,
                          'never-run', '2026-07-19T00:00:00Z', '2026-07-19T00:00:00Z')
                "#,
                rusqlite::params![id, frequency, i64::from(enabled), next_run_at],
            )
            .expect("insert task");
    }

    #[test]
    fn computes_interval_next_run_times() {
        let from = Local
            .with_ymd_and_hms(2026, 7, 19, 9, 0, 0)
            .single()
            .expect("local date");

        let next = compute_next_run(&dto::ScheduledTaskFrequency::Minutes { interval: 15 }, from)
            .expect("next run");

        assert!(next > from.with_timezone(&Utc).to_rfc3339());
    }

    #[test]
    fn rejects_invalid_frequency_values() {
        assert!(compute_next_run(
            &dto::ScheduledTaskFrequency::Minutes { interval: 0 },
            Local::now(),
        )
        .is_err());
        assert!(compute_next_run(
            &dto::ScheduledTaskFrequency::Weekly {
                weekday: 9,
                time_of_day: "09:00".to_string(),
            },
            Local::now(),
        )
        .is_err());
    }

    #[test]
    fn due_scan_skips_disabled_tasks() {
        let (_directory, database) = database();
        insert_task(&database, "task-1", false, "2026-07-19T00:00:00Z");

        let tasks = due_tasks(
            &database,
            DateTime::parse_from_rfc3339("2026-07-19T01:00:00Z")
                .expect("time")
                .with_timezone(&Utc),
        )
        .expect("due tasks");

        assert!(tasks.is_empty());
    }

    #[test]
    fn due_scan_returns_one_backfill_candidate_for_missed_task() {
        let (_directory, database) = database();
        insert_task(&database, "task-1", true, "2026-07-19T00:00:00Z");

        let tasks = due_tasks(
            &database,
            DateTime::parse_from_rfc3339("2026-07-19T03:00:00Z")
                .expect("time")
                .with_timezone(&Utc),
        )
        .expect("due tasks");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[0].next_run_at, "2026-07-19T00:00:00Z");
    }

    #[test]
    fn create_scheduled_task_accepts_a_cli_agent() {
        let (_directory, database) = database();

        let task = create_scheduled_task(
            &database,
            dto::CreateScheduledTaskInput {
                name: "Task".to_string(),
                content: "Run it".to_string(),
                agent_id: "codex-cli".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 5 },
            },
        )
        .expect("create");

        assert_eq!(task.agent_id, "codex-cli");
    }

    #[test]
    fn create_scheduled_task_persists_onepiece() {
        let (_directory, database) = database();

        let task = create_scheduled_task(
            &database,
            dto::CreateScheduledTaskInput {
                name: "OnePiece task".to_string(),
                content: "Run it".to_string(),
                agent_id: "onepiece".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 5 },
            },
        )
        .expect("create");

        assert_eq!(task.agent_id, "onepiece");
        assert_eq!(
            list_scheduled_tasks(&database).expect("list")[0].agent_id,
            "onepiece"
        );
    }

    #[test]
    fn create_scheduled_task_rejects_an_agent_that_does_not_support_cli() {
        // Registered API agents do not inherit OnePiece's dedicated scheduled-task route.
        let (_directory, database) = database();
        let connection = database.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind) \
                 VALUES ('my-api-agent', 'My API Agent', 'Test', 'api')",
                [],
            )
            .expect("seed api agent");
        drop(connection);

        let result = create_scheduled_task(
            &database,
            dto::CreateScheduledTaskInput {
                name: "Task".to_string(),
                content: "Run it".to_string(),
                agent_id: "my-api-agent".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 5 },
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn latest_status_metadata_updates_after_success_and_failure() {
        let (_directory, database) = database();
        insert_task(&database, "task-1", true, "2026-07-19T00:00:00Z");
        let task = due_tasks(
            &database,
            DateTime::parse_from_rfc3339("2026-07-19T03:00:00Z")
                .expect("time")
                .with_timezone(&Utc),
        )
        .expect("due tasks")
        .remove(0);

        mark_task_running_with_trigger(&database, &task.id, false).expect("running");
        let running = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(running.latest_status, "running");
        assert!(running.latest_run_at.is_some());

        database.connection().expect("connection").execute(
            "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state, pinned, archived, created_at, updated_at) VALUES ('session-1', 'Scheduled run', 'codex-cli', 'cli', 'idle', 0, 0, '2026-07-19T03:00:00Z', '2026-07-19T03:00:00Z')",
            [],
        ).expect("session");
        mark_task_succeeded(&database, &task, "session-1").expect("succeeded");
        let succeeded = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(succeeded.latest_status, "succeeded");
        assert_eq!(
            succeeded.latest_run_session_id.as_deref(),
            Some("session-1")
        );
        assert!(succeeded.latest_error.is_none());
        assert!(succeeded.next_run_at > task.next_run_at);
        let lineage: (String, Option<String>) = database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT origin_kind, origin_id FROM sessions WHERE id = 'session-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("lineage");
        assert_eq!(
            lineage,
            ("scheduled_task".to_string(), Some("task-1".to_string()))
        );

        mark_task_failed(&database, &succeeded, "agent unavailable").expect("failed");
        let failed = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(failed.latest_status, "failed");
        assert_eq!(failed.latest_error.as_deref(), Some("agent unavailable"));
        assert!(failed.next_run_at >= succeeded.next_run_at);
    }

    #[test]
    fn run_history_records_backfill_and_skip_outcomes() {
        let (_directory, database) = database();
        insert_task(&database, "task-history", true, "2026-07-19T00:00:00Z");
        let task = list_scheduled_tasks(&database).expect("tasks").remove(0);
        database.connection().expect("connection").execute(
            "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state, pinned, archived, created_at, updated_at) VALUES ('session-backfill', 'Backfill', 'codex-cli', 'cli', 'idle', 0, 0, '2026-07-19T03:00:00Z', '2026-07-19T03:00:00Z')",
            [],
        ).expect("session");

        mark_task_running_with_trigger(&database, &task.id, true).expect("start backfill");
        mark_task_succeeded(&database, &task, "session-backfill").expect("finish backfill");
        record_task_skipped(&database, &task.id, "already claimed").expect("record skip");

        let history = list_scheduled_task_runs(&database, &task.id).expect("history");
        assert_eq!(history[0].status, "skipped");
        assert_eq!(history[0].error.as_deref(), Some("already claimed"));
        assert_eq!(history[1].status, "backfilled");
        assert_eq!(history[1].session_id.as_deref(), Some("session-backfill"));
    }

    #[test]
    fn delete_task_writes_unified_log_when_directory_is_available() {
        let (directory, database) = database();
        insert_task(&database, "task-1", true, "2026-07-19T00:00:00Z");
        let log_directory = directory.path().join("logs");

        delete_scheduled_task(&database, "task-1", Some(&log_directory)).expect("delete");

        assert!(list_scheduled_tasks(&database).expect("tasks").is_empty());
        let log_content = std::fs::read_dir(&log_directory)
            .expect("log directory")
            .filter_map(Result::ok)
            .find_map(|entry| std::fs::read_to_string(entry.path()).ok())
            .expect("log content");
        assert!(log_content.contains("scheduled-tasks.delete"));
        assert!(log_content.contains("task-1"));
    }
}
