use super::api;
use super::models::{CreateWorkItemInput, MoveWorkItemInput, WorkItemFilters};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

#[test]
fn manual_work_is_durable_movable_and_independently_archived() {
    let directory = TempDirectory::new("work-board-manual");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let created = api::create(
        &database,
        CreateWorkItemInput {
            title: " Review release ".to_string(),
            description: "Check evidence".to_string(),
            stage: None,
            priority: Some("high".to_string()),
            project_path: Some("D:/app".to_string()),
            due_at: None,
        },
    )
    .expect("create");
    assert_eq!(created.title, "Review release");
    assert_eq!(created.stage, "inbox");
    let moved = api::move_item(
        &database,
        MoveWorkItemInput {
            work_item_id: created.id.clone(),
            stage: "review".to_string(),
            before_work_item_id: None,
        },
    )
    .expect("move");
    assert_eq!(moved.stage, "review");
    api::set_archived(&database, &created.id, true).expect("archive");
    assert!(api::list_items(&database, WorkItemFilters::default())
        .expect("active")
        .iter()
        .all(|item| item.id != created.id));
    assert!(
        api::list_items(&database, WorkItemFilters { archived: true })
            .expect("archive list")
            .iter()
            .any(|item| item.id == created.id)
    );
    api::delete(&database, &created.id).expect("delete");
}

#[test]
fn reconciliation_is_idempotent_for_scheduled_tasks() {
    let directory = TempDirectory::new("work-board-reconcile");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    connection.execute("INSERT INTO scheduled_tasks (id,name,content,agent_id,frequency,enabled,next_run_at,latest_status,created_at,updated_at) VALUES ('task-board','Nightly','Run tests','codex-cli','{\"kind\":\"daily\",\"timeOfDay\":\"09:00\"}',1,'2026-01-02','never-run','2026-01-01','2026-01-01')", []).expect("task");
    drop(connection);
    let first = api::list_items(&database, WorkItemFilters::default()).expect("first");
    let second = api::list_items(&database, WorkItemFilters::default()).expect("second");
    assert_eq!(
        first
            .iter()
            .filter(|item| item
                .sources
                .iter()
                .any(|source| source.source_id == "task-board"))
            .count(),
        1
    );
    assert_eq!(first.len(), second.len());
}

#[test]
fn move_before_allocates_a_sparse_midpoint_and_preserves_order() {
    let directory = TempDirectory::new("work-board-order");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let create = |title: &str| {
        api::create(
            &database,
            CreateWorkItemInput {
                title: title.to_string(),
                description: String::new(),
                stage: None,
                priority: None,
                project_path: None,
                due_at: None,
            },
        )
        .expect("create")
    };
    let first = create("First");
    let second = create("Second");
    let third = create("Third");

    let moved = api::move_item(
        &database,
        MoveWorkItemInput {
            work_item_id: third.id.clone(),
            stage: "inbox".to_string(),
            before_work_item_id: Some(second.id.clone()),
        },
    )
    .expect("move before");

    assert!(moved.rank > first.rank && moved.rank < second.rank);
    let ordered = api::list_items(&database, WorkItemFilters::default()).expect("ordered");
    assert_eq!(
        ordered
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["First", "Third", "Second"]
    );
}
