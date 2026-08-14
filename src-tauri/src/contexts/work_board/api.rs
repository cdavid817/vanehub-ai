use rusqlite::params;
use uuid::Uuid;

use super::infrastructure::{error, list, load, reconcile, timestamp};
use super::models::{
    CreateWorkItemInput, LinkWorkItemSourceInput, MoveWorkItemInput, UpdateWorkItemInput, WorkItem,
    WorkItemFilters,
};
use crate::platform::database::NativeDatabase;

const STAGES: [&str; 5] = ["inbox", "planned", "in_progress", "review", "done"];
const PRIORITIES: [&str; 5] = ["none", "low", "medium", "high", "urgent"];
const SOURCES: [&str; 4] = ["session", "plan", "plan_run", "scheduled_task"];
const RELATIONS: [&str; 4] = ["primary", "execution", "automation", "supporting"];

pub(crate) fn list_items(
    database: &NativeDatabase,
    filters: WorkItemFilters,
) -> Result<Vec<WorkItem>, String> {
    reconcile(database)?;
    list(database, filters.archived)
}

pub(crate) fn create(
    database: &NativeDatabase,
    input: CreateWorkItemInput,
) -> Result<WorkItem, String> {
    let title = valid_title(&input.title)?;
    let stage = input.stage.as_deref().unwrap_or("inbox");
    valid(stage, &STAGES, "stage")?;
    let priority = input.priority.as_deref().unwrap_or("none");
    valid(priority, &PRIORITIES, "priority")?;
    let connection = database.connection().map_err(error)?;
    let rank: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(rank),0)+1000 FROM work_items WHERE stage=?1",
            [stage],
            |row| row.get(0),
        )
        .map_err(error)?;
    let id = format!("work-item-{}", Uuid::new_v4());
    let now = timestamp();
    connection.execute("INSERT INTO work_items (id,title,description,stage,priority,rank,project_path,due_at,archived,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,?9,?9)", params![id,title,input.description.trim(),stage,priority,rank,clean(input.project_path),input.due_at,now]).map_err(error)?;
    load(database, &id)
}

pub(crate) fn update(
    database: &NativeDatabase,
    input: UpdateWorkItemInput,
) -> Result<WorkItem, String> {
    let current = load(database, &input.work_item_id)?;
    let title = input
        .title
        .as_deref()
        .map(valid_title)
        .transpose()?
        .unwrap_or(current.title);
    let priority = input.priority.unwrap_or(current.priority);
    valid(&priority, &PRIORITIES, "priority")?;
    let description = input.description.unwrap_or(current.description);
    let project = input
        .project_path
        .map(clean)
        .unwrap_or(current.project_path);
    let due = input.due_at.unwrap_or(current.due_at);
    let now = timestamp();
    database.connection().map_err(error)?.execute("UPDATE work_items SET title=?1,description=?2,priority=?3,project_path=?4,due_at=?5,updated_at=?6 WHERE id=?7",params![title,description.trim(),priority,project,due,now,input.work_item_id]).map_err(error)?;
    load(database, &input.work_item_id)
}

pub(crate) fn move_item(
    database: &NativeDatabase,
    input: MoveWorkItemInput,
) -> Result<WorkItem, String> {
    valid(&input.stage, &STAGES, "stage")?;
    load(database, &input.work_item_id)?;
    let mut connection = database.connection().map_err(error)?;
    let transaction = connection.transaction().map_err(error)?;
    let rank: i64 = if let Some(before) = input.before_work_item_id.as_deref() {
        normalize_stage(&transaction, &input.stage, &input.work_item_id)?;
        let target: i64 = transaction
            .query_row(
                "SELECT rank FROM work_items WHERE id=?1 AND stage=?2",
                params![before, input.stage],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?
            .ok_or_else(|| "Target work item was not found in the stage.".to_string())?;
        let previous = transaction
            .query_row(
                "SELECT rank FROM work_items WHERE stage=?1 AND id<>?2 AND rank<?3 ORDER BY rank DESC, id DESC LIMIT 1",
                params![input.stage, input.work_item_id, target],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(error)?;
        previous.map_or(target - 1_000, |rank| rank + ((target - rank) / 2))
    } else {
        transaction
            .query_row(
                "SELECT COALESCE(MAX(rank),0)+1000 FROM work_items WHERE stage=?1 AND id<>?2",
                params![input.stage, input.work_item_id],
                |row| row.get(0),
            )
            .map_err(error)?
    };
    transaction
        .execute(
            "UPDATE work_items SET stage=?1,rank=?2,updated_at=?3 WHERE id=?4",
            params![input.stage, rank, timestamp(), input.work_item_id],
        )
        .map_err(error)?;
    transaction.commit().map_err(error)?;
    load(database, &input.work_item_id)
}

fn normalize_stage(
    connection: &rusqlite::Connection,
    stage: &str,
    moving_id: &str,
) -> Result<(), String> {
    let ids = {
        let mut statement = connection
            .prepare("SELECT id FROM work_items WHERE stage=?1 AND id<>?2 ORDER BY rank, id")
            .map_err(error)?;
        let rows = statement
            .query_map(params![stage, moving_id], |row| row.get::<_, String>(0))
            .map_err(error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?;
        rows
    };
    for (index, id) in ids.iter().enumerate() {
        connection
            .execute(
                "UPDATE work_items SET rank=?1 WHERE id=?2",
                params![(index as i64 + 1) * 1_000, id],
            )
            .map_err(error)?;
    }
    Ok(())
}

pub(crate) fn link(
    database: &NativeDatabase,
    input: LinkWorkItemSourceInput,
) -> Result<WorkItem, String> {
    load(database, &input.work_item_id)?;
    valid(&input.source_kind, &SOURCES, "source kind")?;
    valid(&input.relation, &RELATIONS, "relation")?;
    database.connection().map_err(error)?.execute("INSERT INTO work_item_links (work_item_id,source_kind,source_id,relation,created_at) VALUES (?1,?2,?3,?4,?5)",params![input.work_item_id,input.source_kind,input.source_id,input.relation,timestamp()]).map_err(|_|"Source is already linked to a work item.".to_string())?;
    load(database, &input.work_item_id)
}
pub(crate) fn set_archived(
    database: &NativeDatabase,
    id: &str,
    archived: bool,
) -> Result<WorkItem, String> {
    let changed = database
        .connection()
        .map_err(error)?
        .execute(
            "UPDATE work_items SET archived=?1,updated_at=?2 WHERE id=?3",
            params![i64::from(archived), timestamp(), id],
        )
        .map_err(error)?;
    if changed == 0 {
        return Err("Work item not found.".to_string());
    }
    load(database, id)
}
pub(crate) fn delete(database: &NativeDatabase, id: &str) -> Result<(), String> {
    let changed = database
        .connection()
        .map_err(error)?
        .execute("DELETE FROM work_items WHERE id=?1 AND archived=1", [id])
        .map_err(error)?;
    if changed == 0 {
        Err("Archive a work item before permanently deleting it.".to_string())
    } else {
        Ok(())
    }
}
fn valid<'a>(value: &'a str, allowed: &[&str], name: &str) -> Result<&'a str, String> {
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(format!("Unknown work item {name}."))
    }
}
fn valid_title(value: &str) -> Result<String, String> {
    let title = value.trim();
    if title.is_empty() || title.chars().count() > 200 {
        Err("Work item title must contain 1-200 characters.".to_string())
    } else {
        Ok(title.to_string())
    }
}
fn clean(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let result = value.trim().to_string();
        (!result.is_empty()).then_some(result)
    })
}
use rusqlite::OptionalExtension;
