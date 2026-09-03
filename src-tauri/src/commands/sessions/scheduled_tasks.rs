use super::dto;
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::infrastructure::scheduled_tasks::{self, ScheduledTaskLogDirectory};
use crate::platform::database::NativeDatabase;
use tauri::State;

/// 19.11: mirrors `list_evaluation_arenas.rs`'s own `DEFAULT_LIMIT`/`MAX_LIMIT` (20/50) -- the
/// same cursor-is-really-just-the-offset shape, the same clamp values, so a reader who already
/// knows one paginated list surface in this app recognizes the other. Kept local to this command
/// (not a shared constant) because `list_evaluation_arenas.rs` does not share its own either; each
/// paginated command module owns its own tiny page-size policy.
const DEFAULT_RUN_HISTORY_LIMIT: usize = 20;
const MAX_RUN_HISTORY_LIMIT: usize = 50;

#[tauri::command]
pub(crate) fn list_scheduled_tasks(
    database: State<'_, NativeDatabase>,
) -> Result<Vec<dto::ScheduledTask>, CommandError> {
    scheduled_tasks::list_scheduled_tasks(&database)
}

/// 19.11: closes the one real gap left in run-history exposure -- `scheduled_tasks::
/// list_scheduled_task_runs` used to hard-code `LIMIT 100` with no cursor/offset anywhere in this
/// command, the service contract, or the Web mock. Returns the same `{ items, nextCursor }` shape
/// `list_evaluation_arenas` already established for an identical underlying gap (18.6, this same
/// OpenSpec change) -- every other paginated list surface reachable from the frontend
/// (`MissionControlPage`, evaluation, context evidence, personalization memory, token usage) reads
/// that shape, so matching it keeps Scheduled Tasks' own run history consistent with the rest of
/// this app instead of being the one surface that hands back a raw offset/limit pair. `CommandError`
/// (not `list_evaluation_arenas.rs`'s own bare `String`) is used for the error type here because
/// every other command in this file already returns `CommandError` -- switching just this one
/// command to a different error type would be the actual inconsistency.
#[tauri::command]
pub(crate) fn list_scheduled_task_runs(
    database: State<'_, NativeDatabase>,
    task_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::ScheduledTaskRunPage, CommandError> {
    let offset = parse_run_history_cursor(cursor.as_deref())?;
    let bounded_limit = resolve_run_history_limit(limit);
    // Fetches one extra row to learn whether a next page exists without a separate COUNT query --
    // the same trick `list_evaluation_arenas.rs` uses. `MAX_RUN_HISTORY_LIMIT` (50) stays well
    // under the infrastructure layer's own hard `MAX_RUN_HISTORY_PAGE` (100, `scheduled_tasks.rs`),
    // so this `+ 1` request is never silently clamped back down before `has_more` gets to see it.
    let runs =
        scheduled_tasks::list_scheduled_task_runs(&database, &task_id, offset, bounded_limit + 1)?;
    let (page, has_more) = paginate_run_history(runs, bounded_limit);
    Ok(dto::ScheduledTaskRunPage {
        items: page,
        next_cursor: has_more.then(|| (offset + bounded_limit).to_string()),
    })
}

fn parse_run_history_cursor(cursor: Option<&str>) -> Result<usize, CommandError> {
    cursor
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| CommandError::validation("invalid scheduled task run cursor"))
        })
        .transpose()
        .map(|offset| offset.unwrap_or(0))
}

fn resolve_run_history_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_RUN_HISTORY_LIMIT)
        .clamp(1, MAX_RUN_HISTORY_LIMIT)
}

fn paginate_run_history(
    mut items: Vec<dto::ScheduledTaskRun>,
    limit: usize,
) -> (Vec<dto::ScheduledTaskRun>, bool) {
    let has_more = items.len() > limit;
    items.truncate(limit);
    (items, has_more)
}

#[tauri::command]
pub(crate) fn create_scheduled_task(
    database: State<'_, NativeDatabase>,
    input: dto::CreateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::create_scheduled_task(&database, input)
}

#[tauri::command]
pub(crate) fn set_scheduled_task_enabled(
    database: State<'_, NativeDatabase>,
    input: dto::SetScheduledTaskEnabledInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::set_scheduled_task_enabled(&database, input)
}

/// 19.8: version-checked edit of name/content/agent/frequency. See
/// `scheduled_tasks::update_scheduled_task`'s own doc comment for the conflict contract.
#[tauri::command]
pub(crate) fn update_scheduled_task(
    database: State<'_, NativeDatabase>,
    input: dto::UpdateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::update_scheduled_task(&database, input)
}

#[tauri::command]
pub(crate) fn delete_scheduled_task(
    database: State<'_, NativeDatabase>,
    log_directory: State<'_, ScheduledTaskLogDirectory>,
    task_id: String,
) -> Result<(), CommandError> {
    scheduled_tasks::delete_scheduled_task(&database, &task_id, Some(log_directory.path()))
}

/// 19.10: dispatches a task's content on demand, independent of its recurrence.
///
/// `SessionsApi` and `AgentRuntimeApi` are already registered as managed state in their own
/// right (`bootstrap/runtime.rs` `.manage()`s both, the same instances `start_scheduled_task_jobs`
/// hands to the due-task sweep), so this takes them directly as `State<'_, T>` alongside the
/// database rather than introducing a new composed API type for a single call site.
#[tauri::command]
pub(crate) fn run_scheduled_task_now(
    database: State<'_, NativeDatabase>,
    sessions: State<'_, SessionsApi>,
    agents: State<'_, AgentRuntimeApi>,
    task_id: String,
) -> Result<dto::RunScheduledTaskNowResult, CommandError> {
    scheduled_tasks::run_scheduled_task_now(&database, &sessions, &agents, &task_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str) -> dto::ScheduledTaskRun {
        dto::ScheduledTaskRun {
            id: id.to_string(),
            task_id: "task-1".to_string(),
            session_id: None,
            status: "succeeded".to_string(),
            error: None,
            started_at: "2026-08-31T09:00:00Z".to_string(),
            completed_at: Some("2026-08-31T09:05:00Z".to_string()),
        }
    }

    #[test]
    fn parse_run_history_cursor_defaults_to_zero_when_no_cursor_is_given() {
        assert_eq!(parse_run_history_cursor(None), Ok(0));
    }

    #[test]
    fn parse_run_history_cursor_reads_a_valid_cursor() {
        assert_eq!(parse_run_history_cursor(Some("40")), Ok(40));
    }

    #[test]
    fn parse_run_history_cursor_rejects_a_non_numeric_cursor_with_a_safe_message() {
        let error = parse_run_history_cursor(Some("not-a-number")).expect_err("must reject");
        assert!(error
            .message()
            .contains("invalid scheduled task run cursor"));
    }

    #[test]
    fn resolve_run_history_limit_defaults_and_clamps_into_bounds() {
        assert_eq!(resolve_run_history_limit(None), DEFAULT_RUN_HISTORY_LIMIT);
        assert_eq!(resolve_run_history_limit(Some(0)), 1);
        assert_eq!(
            resolve_run_history_limit(Some(1_000)),
            MAX_RUN_HISTORY_LIMIT
        );
        assert_eq!(resolve_run_history_limit(Some(35)), 35);
    }

    #[test]
    fn paginate_run_history_reports_has_more_only_when_the_probe_row_was_returned() {
        let (page, has_more) = paginate_run_history(vec![run("a"), run("b"), run("c")], 2);
        assert_eq!(
            page.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(has_more);

        let (page, has_more) = paginate_run_history(vec![run("a"), run("b")], 2);
        assert_eq!(
            page.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(!has_more);
    }
}
