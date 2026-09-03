use crate::commands::error::{CommandError, CommandErrorCategory};
use crate::commands::sessions::dto;
use crate::contexts::agent_runtime::api::{
    AgentChatConfiguration, AgentRuntimeApi, InteractionMode, SendMessageRequest,
};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
use crate::platform::database::NativeDatabase;
use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) use crate::commands::sessions::dto::ScheduledTask;

/// 19.11: the hard ceiling `list_scheduled_task_runs` clamps `limit` to, mirroring
/// `evaluation_repository.rs`'s own identical `MAX_PAGE` -- a defense-in-depth bound at the SQL
/// layer itself, independent of whatever smaller `MAX_RUN_HISTORY_LIMIT` the Tauri command layer
/// (`commands/sessions/scheduled_tasks.rs`) already clamps a caller's requested page size to.
const MAX_RUN_HISTORY_PAGE: usize = 100;

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
                   created_at, updated_at, version
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

/// 19.11: real `OFFSET`/`LIMIT` pagination -- this query previously hard-coded `LIMIT 100` with no
/// parameter anywhere that could reach a second page. Mirrors `evaluation_repository.rs`'s own
/// `list(offset, limit)` shape (this same OpenSpec change's 18.6 precedent for an identical gap):
/// plain `usize` offset/limit here, `limit` clamped to `MAX_RUN_HISTORY_PAGE` so a caller can never
/// force an unbounded scan regardless of what it asks for. The cursor-shaped `{ items, nextCursor
/// }` contract the frontend actually sees is assembled one layer up, in the Tauri command
/// (`commands/sessions/scheduled_tasks.rs`) -- this function only knows SQL paging, the same
/// division of labor `list_evaluation_arenas.rs` keeps from its own repository.
pub(crate) fn list_scheduled_task_runs(
    database: &NativeDatabase,
    task_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<dto::ScheduledTaskRun>, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let mut statement = connection.prepare(
        "SELECT id, task_id, session_id, status, error, started_at, completed_at FROM scheduled_task_runs WHERE task_id = ?1 ORDER BY started_at DESC, id DESC LIMIT ?2 OFFSET ?3",
    ).map_err(command_error)?;
    let bounded_limit =
        i64::try_from(limit.clamp(1, MAX_RUN_HISTORY_PAGE)).map_err(command_error)?;
    let bounded_offset = i64::try_from(offset).map_err(command_error)?;
    let runs = statement
        .query_map(params![task_id, bounded_limit, bounded_offset], |row| {
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
    validate_scheduled_task_agent(&connection, &input.agent_id)?;
    let id = format!("scheduled-task-{}", Uuid::new_v4());
    let timestamp = Utc::now().to_rfc3339();
    connection
        .execute(
            r#"
            INSERT INTO scheduled_tasks (
                id, name, content, agent_id, frequency, enabled, next_run_at,
                latest_status, latest_run_at, latest_run_session_id, latest_error,
                created_at, updated_at, version
            ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 'never-run', NULL, NULL, NULL, ?7, ?7, 1)
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

/// Shared by `create_scheduled_task` and `update_scheduled_task`: scheduled execution has an
/// explicit API route only for OnePiece, so every other API Agent is rejected before persisting a
/// task that could never run.
fn validate_scheduled_task_agent(
    connection: &rusqlite::Connection,
    agent_id: &str,
) -> Result<(), CommandError> {
    let launch_kind: Option<String> = connection
        .query_row(
            "SELECT launch_kind FROM agents WHERE id = ?1",
            [agent_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(command_error)?;
    match (agent_id, launch_kind.as_deref()) {
        ("onepiece", Some("api")) | (_, Some("cli")) => Ok(()),
        (_, Some(_)) => Err(CommandError::validation(
            "Scheduled tasks support CLI Agents and OnePiece.",
        )),
        (_, None) => Err(CommandError::validation(
            "Scheduled task references an unsupported Agent.",
        )),
    }
}

/// 19.8: overwrites name/content/agent/frequency together, guarded by `expected_version` against a
/// concurrent editor. Two checks exist for two different reasons: the pre-check below gives a
/// caller the current stored version to report a useful conflict immediately, cheaply, before any
/// validation runs; the `WHERE version = ?` on the `UPDATE` itself is the actual race-free guard
/// (mirroring `loop_repository.rs`'s `update_definition`) for the window between that read and this
/// write, where a concurrent update could otherwise land silently between the two.
///
/// Deliberately leaves `enabled` alone -- toggling it is `set_scheduled_task_enabled`'s own
/// concern, not one of the fields this call edits.
pub(crate) fn update_scheduled_task(
    database: &NativeDatabase,
    input: dto::UpdateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    let name = input.name.trim();
    let content = input.content.trim();
    if name.is_empty() || content.is_empty() {
        return Err(CommandError::validation(
            "Scheduled task name and content are required.",
        ));
    }
    let connection = database.connection().map_err(command_error)?;
    let current = load_task(&connection, &input.task_id)?;
    if current.version != input.expected_version {
        return Err(version_conflict(input.expected_version, current.version));
    }
    validate_scheduled_task_agent(&connection, &input.agent_id)?;

    // Only a real frequency change earns a fresh `next_run_at` -- mirrors
    // `set_scheduled_task_enabled`'s own conditional recompute just below. Recomputing
    // unconditionally would silently push back a task's next fire time on every edit, including
    // one that only touches `name`/`content`/`agentId`, which has nothing to do with when it runs.
    let next_run_at = if input.frequency == current.frequency {
        current.next_run_at
    } else {
        compute_next_run(&input.frequency, Local::now())?
    };
    let frequency = serde_json::to_string(&input.frequency).map_err(command_error)?;
    let next_version = input.expected_version.saturating_add(1);
    let timestamp = Utc::now().to_rfc3339();
    let changed = connection
        .execute(
            r#"
            UPDATE scheduled_tasks
            SET name = ?1, content = ?2, agent_id = ?3, frequency = ?4, next_run_at = ?5,
                version = ?6, updated_at = ?7
            WHERE id = ?8 AND version = ?9
            "#,
            params![
                name,
                content,
                input.agent_id,
                frequency,
                next_run_at,
                next_version,
                timestamp,
                input.task_id,
                input.expected_version,
            ],
        )
        .map_err(command_error)?;
    if changed == 0 {
        // The pre-check above passed, so this row raced a concurrent write between that read and
        // this write. Re-read rather than assume: the row may since have been deleted entirely.
        let latest = load_task(&connection, &input.task_id)?;
        return Err(version_conflict(input.expected_version, latest.version));
    }
    load_task(&connection, &input.task_id)
}

/// A stable code, not prose, so the frontend can match it independent of locale -- the same
/// approach `personalization-revision-conflict` established (`commands/personalization/error.rs`),
/// preferred here over Loop Center's own `AgentRuntimeApplicationError::Validation` prose
/// (`loop_service.rs`'s `update_definition`) because `CommandError`'s `Serialize` impl
/// (`commands/error.rs`) sends only this message string across the Tauri boundary -- `category` is
/// never seen by the frontend, so the message itself has to carry a machine-matchable signal.
fn version_conflict(expected: i64, stored: i64) -> CommandError {
    CommandError::typed(
        CommandErrorCategory::Conflict,
        format!("scheduled-task-version-conflict: expected {expected}, stored {stored}"),
    )
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

/// Looks up the task, dispatches its content exactly as the due-task sweep's own `run_one_task`
/// would (below), then records the outcome as its own `scheduled_task_runs` row -- but leaves
/// `next_run_at`, `latest_status`, `latest_run_at`, and `latest_run_session_id` on the
/// `scheduled_tasks` row alone. Those four columns are the sweep's own bookkeeping
/// (`mark_task_running_with_trigger` / `mark_task_succeeded` / `mark_task_failed`, all below); an
/// on-demand run does not own the task's recurrence and must not advance it, which is the entire
/// point of "Run now" over waiting for the next tick.
///
/// A dispatch failure is returned as a command error rather than a persisted row: nothing durable
/// happened, so there is nothing honest to record.
pub(crate) fn run_scheduled_task_now(
    database: &NativeDatabase,
    sessions: &SessionsApi,
    agents: &AgentRuntimeApi,
    task_id: &str,
) -> Result<dto::RunScheduledTaskNowResult, CommandError> {
    let connection = database.connection().map_err(command_error)?;
    let task = load_task(&connection, task_id)?;
    drop(connection);

    let session_id = run_one_task(sessions, agents, &task)?;
    record_manual_run(database, task_id, &session_id)
}

/// The database half of a successful on-demand run: one new, already-complete
/// `scheduled_task_runs` row. Inserted complete (both timestamps set up front) rather than left
/// open for a later step to close -- there is nothing to close, because `run_one_task`'s own
/// "succeeded" already means "the message reached the Agent," not "the Agent finished," which is
/// exactly what the sweep treats it as too (`run_due_tasks` marks a task succeeded immediately
/// after dispatch, without waiting for a reply). Inserting it complete also means this row can
/// never be the "most recent incomplete run" the sweep's own completion queries
/// (`update_task_run_metadata`) pick up, so a manual run can never race a concurrent sweep tick
/// for the same row -- the two code paths never contend for the same mutable state.
fn record_manual_run(
    database: &NativeDatabase,
    task_id: &str,
    session_id: &str,
) -> Result<dto::RunScheduledTaskNowResult, CommandError> {
    let run_id = format!("scheduled-run-{}", Uuid::new_v4());
    let timestamp = Utc::now().to_rfc3339();
    let connection = database.connection().map_err(command_error)?;
    connection
        .execute(
            "INSERT INTO scheduled_task_runs (id,task_id,session_id,status,error,started_at,completed_at) VALUES (?1,?2,?3,'succeeded',NULL,?4,?5)",
            params![run_id, task_id, session_id, timestamp, timestamp],
        )
        .map_err(command_error)?;
    Ok(dto::RunScheduledTaskNowResult {
        run: dto::ScheduledTaskRun {
            id: run_id,
            task_id: task_id.to_string(),
            session_id: Some(session_id.to_string()),
            status: "succeeded".to_string(),
            error: None,
            started_at: timestamp.clone(),
            completed_at: Some(timestamp),
        },
        operation_id: None,
    })
}

/// The actual "run one scheduled task" behavior: create a session on the task's own Agent, and
/// send its content as the first message. Shared by the due-task sweep (`run_due_tasks`, in
/// `bootstrap::scheduled_tasks`) and `run_scheduled_task_now` above -- the sweep decides *when*
/// this runs and what to do with `next_run_at`/`latest_status` afterward; this function only
/// knows how to run it once. Moved here (from `bootstrap::scheduled_tasks`, its only caller until
/// now) rather than duplicated, so the on-demand path and the sweep can never quietly drift into
/// dispatching a task two different ways.
pub(crate) fn run_one_task(
    sessions: &SessionsApi,
    agents: &AgentRuntimeApi,
    task: &ScheduledTask,
) -> Result<String, CommandError> {
    let interaction_mode = scheduled_task_interaction_mode(&task.agent_id);
    // A scheduled run has no user in front of it to choose a mode, and inventing one would make
    // the setting mean something different depending on who started the turn.
    let prepared = sessions.prepare_creation(NewSessionRequest {
        personalization_mode: None,
        agent_id: task.agent_id.clone(),
        seats: Vec::new(),
        interaction_mode: interaction_mode.as_str().to_string(),
        title: Some(task.name.clone()),
        workspace: NewSessionWorkspace::default(),
        owner: SessionOwner::desktop(),
        activation: SessionActivation::PreserveActive,
    })?;
    let session = sessions.execute_creation(prepared)?;
    agents.send_message(SendMessageRequest {
        source: crate::contexts::agent_runtime::application::AgentMessageSource::Scheduled {
            task_id: task.id.clone(),
        },
        session_id: session.id().to_string(),
        content: task.content.clone(),
        configuration: AgentChatConfiguration {
            agent_id: task.agent_id.clone(),
            interaction_mode,
            execution_mode: "inherit".to_string(),
            provider_id: None,
            model_id: None,
            reasoning_depth: None,
            streaming: true,
            thinking: false,
            long_context: false,
        },
        file_references: Vec::new(),
    })?;
    Ok(session.id().to_string())
}

fn scheduled_task_interaction_mode(agent_id: &str) -> InteractionMode {
    if agent_id == "onepiece" {
        InteractionMode::Api
    } else {
        InteractionMode::Cli
    }
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
                   created_at, updated_at, version
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
        version: row.get(13)?,
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
                   created_at, updated_at, version
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
    // `month` always comes from a `DateTime`, so it is in 1..=12 and only chrono's maximum
    // representable year could reject the first-of-next-month date. 28 is the fallback rather
    // than a larger guess because it is a valid day in every month: `next_monthly` loops until
    // `candidate.day()` reaches this value, so a month length that some month cannot reach
    // would spin forever.
    let Some(next_month) = (if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year.saturating_add(1), 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }) else {
        debug_assert!(
            false,
            "month {month} of year {year} has no first-of-next-month"
        );
        return 28;
    };
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

    /// The frequency shapes cross the IPC boundary as JSON built by the frontend, so what matters
    /// is the wire spelling, not that a Rust-constructed value round-trips through itself. Every
    /// caller sends camelCase (`ScheduledTaskFrequency` in types/agent.ts, the dialog's
    /// `initialFrequency`, and the web mock client); this pins each variant that carries a
    /// multi-word field to that spelling. Deserializing from a literal, rather than from
    /// `to_string` of a Rust value, is the part that earns its keep -- a symmetric round-trip
    /// passes just as happily with the wrong name on both sides, which is how the mismatch this
    /// covers survived.
    #[test]
    fn frequency_variants_deserialize_from_the_camel_case_the_frontend_sends() {
        let daily: dto::ScheduledTaskFrequency =
            serde_json::from_str(r#"{"kind":"daily","timeOfDay":"09:00"}"#).expect("daily");
        assert_eq!(
            daily,
            dto::ScheduledTaskFrequency::Daily {
                time_of_day: "09:00".to_string(),
            }
        );

        let weekly: dto::ScheduledTaskFrequency =
            serde_json::from_str(r#"{"kind":"weekly","weekday":1,"timeOfDay":"09:00"}"#)
                .expect("weekly");
        assert_eq!(
            weekly,
            dto::ScheduledTaskFrequency::Weekly {
                weekday: 1,
                time_of_day: "09:00".to_string(),
            }
        );

        let monthly: dto::ScheduledTaskFrequency =
            serde_json::from_str(r#"{"kind":"monthly","dayOfMonth":1,"timeOfDay":"09:00"}"#)
                .expect("monthly");
        assert_eq!(
            monthly,
            dto::ScheduledTaskFrequency::Monthly {
                day_of_month: 1,
                time_of_day: "09:00".to_string(),
            }
        );

        // Serialization has to agree, because the value is stored as JSON and read back by the
        // same enum: a rename applied to only one direction would make every stored daily task
        // unreadable on the next launch.
        assert_eq!(
            serde_json::to_string(&daily).expect("serialize"),
            r#"{"kind":"daily","timeOfDay":"09:00"}"#,
        );
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

        let history = list_scheduled_task_runs(&database, &task.id, 0, 100).expect("history");
        assert_eq!(history[0].status, "skipped");
        assert_eq!(history[0].error.as_deref(), Some("already claimed"));
        assert_eq!(history[1].status, "backfilled");
        assert_eq!(history[1].session_id.as_deref(), Some("session-backfill"));
    }

    /// 19.11: the real gap this task closed -- before this pass `limit`/`offset` did not exist as
    /// parameters at all, so a caller could never reach anything past the hard-coded newest 100
    /// rows. Five distinct rows (`record_task_skipped`, called five times in a tight loop) prove
    /// `LIMIT ?2 OFFSET ?3` actually slides the window. Deliberately does not assert a specific
    /// `reason-N` value per page: `ORDER BY started_at DESC, id DESC` ties on `id` (a random UUID)
    /// whenever two rows land on the same wall-clock instant, which a tight loop with no artificial
    /// delay cannot rule out -- asserting content order would make this test flaky on a fast enough
    /// machine for a reason that has nothing to do with whether paging itself is correct. Instead
    /// this checks the property pagination actually promises: concatenating consecutive pages under
    /// the query's own order reproduces the unpaginated result exactly, with the right page sizes
    /// and no duplicate or dropped row.
    #[test]
    fn list_scheduled_task_runs_pages_with_real_offset_and_limit() {
        let (_directory, database) = database();
        insert_task(&database, "task-paged", true, "2026-07-19T00:00:00Z");
        for index in 0..5 {
            record_task_skipped(&database, "task-paged", &format!("reason-{index}"))
                .expect("record skip");
        }

        let full = list_scheduled_task_runs(&database, "task-paged", 0, 100).expect("full history");
        assert_eq!(full.len(), 5);
        let full_ids: Vec<String> = full.iter().map(|run| run.id.clone()).collect();

        let first_page =
            list_scheduled_task_runs(&database, "task-paged", 0, 2).expect("first page");
        let second_page =
            list_scheduled_task_runs(&database, "task-paged", 2, 2).expect("second page");
        let last_page = list_scheduled_task_runs(&database, "task-paged", 4, 2).expect("last page");
        assert_eq!(first_page.len(), 2);
        assert_eq!(second_page.len(), 2);
        assert_eq!(last_page.len(), 1);

        let paged_ids: Vec<String> = first_page
            .iter()
            .chain(second_page.iter())
            .chain(last_page.iter())
            .map(|run| run.id.clone())
            .collect();
        assert_eq!(
            paged_ids, full_ids,
            "paging must reproduce the unpaginated order exactly"
        );

        let past_the_end =
            list_scheduled_task_runs(&database, "task-paged", 5, 2).expect("past the end");
        assert!(past_the_end.is_empty());
    }

    /// The other half of 19.11's own contract: `limit` is a ceiling a caller cannot exceed, not a
    /// suggestion -- mirrors `evaluation_repository.rs`'s own `MAX_PAGE` clamp test for the
    /// identical shape.
    #[test]
    fn list_scheduled_task_runs_clamps_limit_to_the_max_run_history_page() {
        let (_directory, database) = database();
        insert_task(&database, "task-many-runs", true, "2026-07-19T00:00:00Z");
        for index in 0..3 {
            record_task_skipped(&database, "task-many-runs", &format!("reason-{index}"))
                .expect("record skip");
        }

        let runs =
            list_scheduled_task_runs(&database, "task-many-runs", 0, 10_000).expect("clamped page");

        assert_eq!(runs.len(), 3);
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

    /// Moved from `bootstrap::scheduled_tasks` along with `scheduled_task_interaction_mode`
    /// itself (19.10): OnePiece is the one Agent scheduled tasks dispatch through the API
    /// runner, every other supported Agent goes through CLI.
    #[test]
    fn onepiece_scheduled_tasks_use_api_and_cli_agents_keep_cli_mode() {
        assert_eq!(
            scheduled_task_interaction_mode("onepiece"),
            InteractionMode::Api
        );
        assert_eq!(
            scheduled_task_interaction_mode("codex-cli"),
            InteractionMode::Cli
        );
    }

    /// 19.10's own stated requirement: an on-demand run must not change recurrence. This exercises
    /// the real database code `run_scheduled_task_now` calls after a successful dispatch --
    /// `run_one_task` itself needs a live `SessionsApi`/`AgentRuntimeApi` this file has no fixture
    /// for, so this pins the half that actually touches `next_run_at`, the same way `run_one_task`
    /// and `run_due_tasks` are exercised elsewhere rather than unit-tested directly here.
    #[test]
    fn record_manual_run_leaves_recurrence_and_latest_status_untouched() {
        let (_directory, database) = database();
        insert_task(&database, "task-1", true, "2026-07-19T00:00:00Z");
        let before = list_scheduled_tasks(&database).expect("tasks").remove(0);

        let receipt =
            record_manual_run(&database, "task-1", "session-manual").expect("record manual run");

        assert_eq!(receipt.run.task_id, "task-1");
        assert_eq!(receipt.run.session_id.as_deref(), Some("session-manual"));
        assert_eq!(receipt.run.status, "succeeded");
        assert!(receipt.run.error.is_none());
        assert!(receipt.run.completed_at.is_some());
        assert!(receipt.operation_id.is_none());

        let after = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(after.next_run_at, before.next_run_at);
        assert_eq!(after.latest_status, before.latest_status);
        assert_eq!(after.latest_run_at, before.latest_run_at);
        assert_eq!(after.latest_run_session_id, before.latest_run_session_id);
        assert_eq!(after.updated_at, before.updated_at);

        let history = list_scheduled_task_runs(&database, "task-1", 0, 100).expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, receipt.run.id);
        assert_eq!(history[0].session_id.as_deref(), Some("session-manual"));
        assert_eq!(history[0].status, "succeeded");
    }

    /// The concurrency claim in `record_manual_run`'s own doc comment, made concrete: a manual run
    /// recorded while the sweep has its own run "open" (`mark_task_running_with_trigger`, not yet
    /// completed) must not be the row `mark_task_succeeded` later completes, and the sweep's own
    /// open row must not be disturbed by the manual insert either.
    #[test]
    fn record_manual_run_does_not_disturb_a_concurrently_open_sweep_run() {
        let (_directory, database) = database();
        insert_task(&database, "task-1", true, "2026-07-19T00:00:00Z");
        let task = list_scheduled_tasks(&database).expect("tasks").remove(0);

        mark_task_running_with_trigger(&database, &task.id, false).expect("sweep run opens");
        record_manual_run(&database, &task.id, "session-manual").expect("manual run recorded");

        let mid_flight = list_scheduled_task_runs(&database, &task.id, 0, 100).expect("history");
        assert_eq!(mid_flight.len(), 2);
        let sweep_row = mid_flight
            .iter()
            .find(|run| run.session_id.is_none())
            .expect("the sweep's own row is still open with no session yet");
        assert_eq!(sweep_row.status, "running");
        assert!(sweep_row.completed_at.is_none());
        let manual_row = mid_flight
            .iter()
            .find(|run| run.session_id.as_deref() == Some("session-manual"))
            .expect("the manual row recorded its own session");
        assert_eq!(manual_row.status, "succeeded");
        assert!(manual_row.completed_at.is_some());

        database.connection().expect("connection").execute(
            "INSERT INTO sessions (id, title, agent_id, interaction_mode, lifecycle_state, pinned, archived, created_at, updated_at) VALUES ('session-sweep', 'Sweep run', 'codex-cli', 'cli', 'idle', 0, 0, '2026-07-19T03:00:00Z', '2026-07-19T03:00:00Z')",
            [],
        ).expect("session");
        mark_task_succeeded(&database, &task, "session-sweep").expect("sweep completes");

        let settled = list_scheduled_task_runs(&database, &task.id, 0, 100).expect("history");
        let manual_row = settled
            .iter()
            .find(|run| run.session_id.as_deref() == Some("session-manual"))
            .expect("the manual row is untouched by the sweep's own completion");
        assert_eq!(manual_row.status, "succeeded");
        let sweep_row = settled
            .iter()
            .find(|run| run.session_id.as_deref() == Some("session-sweep"))
            .expect("the sweep's own open row -- not the manual one -- was the row completed");
        assert_eq!(sweep_row.status, "succeeded");

        let after = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert!(after.next_run_at > task.next_run_at);
        assert_eq!(
            after.latest_run_session_id.as_deref(),
            Some("session-sweep")
        );
    }

    fn create_task_for_update(database: &NativeDatabase) -> dto::ScheduledTask {
        create_scheduled_task(
            database,
            dto::CreateScheduledTaskInput {
                name: "Task".to_string(),
                content: "Run it".to_string(),
                agent_id: "codex-cli".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 5 },
            },
        )
        .expect("create")
    }

    /// 19.8: a version-matched edit persists every editable field and advances the counter by
    /// exactly one -- mirroring `loop_repository.rs`'s own "version must advance by exactly one"
    /// invariant for the same reason: a caller that reads the returned `version` back has to be
    /// able to trust it as the next `expectedVersion` to send.
    #[test]
    fn update_scheduled_task_persists_a_version_matched_edit_and_advances_the_version() {
        let (_directory, database) = database();
        let created = create_task_for_update(&database);
        assert_eq!(created.version, 1);

        let updated = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                expected_version: created.version,
                name: " Renamed task ".to_string(),
                content: " Do something else ".to_string(),
                agent_id: "onepiece".to_string(),
                frequency: dto::ScheduledTaskFrequency::Hours { interval: 3 },
            },
        )
        .expect("update");

        assert_eq!(updated.name, "Renamed task");
        assert_eq!(updated.content, "Do something else");
        assert_eq!(updated.agent_id, "onepiece");
        assert_eq!(
            updated.frequency,
            dto::ScheduledTaskFrequency::Hours { interval: 3 }
        );
        assert_eq!(updated.version, 2);

        let persisted = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(persisted.version, 2);
        assert_eq!(persisted.agent_id, "onepiece");
    }

    /// A real bug caught during review, not a hypothetical: recomputing `next_run_at`
    /// unconditionally on every edit would silently push back a task's schedule from an edit that
    /// never touched `frequency` at all -- mirrors `set_scheduled_task_enabled`'s own conditional
    /// recompute (only when the toggle direction actually changes what "next" means).
    #[test]
    fn update_scheduled_task_preserves_next_run_at_when_frequency_is_unchanged() {
        let (_directory, database) = database();
        let created = create_task_for_update(&database);
        let original_next_run_at = created.next_run_at.clone();

        let renamed_only = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                expected_version: created.version,
                name: "Renamed, same schedule".to_string(),
                content: created.content.clone(),
                agent_id: created.agent_id.clone(),
                frequency: created.frequency.clone(),
            },
        )
        .expect("update name only");

        assert_eq!(renamed_only.next_run_at, original_next_run_at);
    }

    /// The other half of the same fix: a genuine frequency change still recomputes `next_run_at`
    /// from now, exactly as it always has -- this test would already have passed before the fix
    /// above, but pins the behavior so a future change cannot "fix" the false-preserve case by
    /// preserving unconditionally instead.
    #[test]
    fn update_scheduled_task_recomputes_next_run_at_when_frequency_changes() {
        let (_directory, database) = database();
        let created = create_task_for_update(&database);
        let original_next_run_at = created.next_run_at.clone();

        let rescheduled = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                expected_version: created.version,
                name: created.name.clone(),
                content: created.content.clone(),
                agent_id: created.agent_id.clone(),
                frequency: dto::ScheduledTaskFrequency::Hours { interval: 3 },
            },
        )
        .expect("update frequency");

        assert_ne!(rescheduled.next_run_at, original_next_run_at);
    }

    /// The race-free half of the guard: a second update sent with the version the first update
    /// already consumed must be rejected outright, and -- the part a service-layer-only check could
    /// get wrong -- must not mutate the row at all, not even partially.
    #[test]
    fn update_scheduled_task_rejects_a_stale_version_without_mutating_the_row() {
        let (_directory, database) = database();
        let created = create_task_for_update(&database);
        let first_update = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                expected_version: created.version,
                name: "Renamed task".to_string(),
                content: "Do something else".to_string(),
                agent_id: "codex-cli".to_string(),
                frequency: dto::ScheduledTaskFrequency::Hours { interval: 3 },
            },
        )
        .expect("first update");
        assert_eq!(first_update.version, 2);

        let stale = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                // Stale: the row is already at version 2.
                expected_version: created.version,
                name: "Conflicting edit".to_string(),
                content: "Should never be persisted".to_string(),
                agent_id: "codex-cli".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 1 },
            },
        );

        let error = stale.expect_err("stale version must be rejected");
        assert_eq!(error.category(), CommandErrorCategory::Conflict);
        assert!(error.message().contains("scheduled-task-version-conflict"));
        assert!(error.message().contains("expected 1"));
        assert!(error.message().contains("stored 2"));

        let unchanged = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(unchanged.name, "Renamed task");
        assert_eq!(unchanged.content, "Do something else");
        assert_eq!(unchanged.version, 2);
    }

    /// Reuses `create_scheduled_task`'s own Agent validation (`validate_scheduled_task_agent`)
    /// rather than re-deriving it -- this pins that the shared helper is actually wired in, not
    /// just present.
    #[test]
    fn update_scheduled_task_rejects_an_agent_that_does_not_support_cli() {
        let (_directory, database) = database();
        let created = create_task_for_update(&database);
        let connection = database.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind) \
                 VALUES ('my-api-agent', 'My API Agent', 'Test', 'api')",
                [],
            )
            .expect("seed api agent");
        drop(connection);

        let result = update_scheduled_task(
            &database,
            dto::UpdateScheduledTaskInput {
                task_id: created.id.clone(),
                expected_version: created.version,
                name: "Task".to_string(),
                content: "Run it".to_string(),
                agent_id: "my-api-agent".to_string(),
                frequency: dto::ScheduledTaskFrequency::Minutes { interval: 5 },
            },
        );

        assert!(result.is_err());
        let unchanged = list_scheduled_tasks(&database).expect("tasks").remove(0);
        assert_eq!(unchanged.agent_id, "codex-cli");
        assert_eq!(unchanged.version, 1);
    }
}
