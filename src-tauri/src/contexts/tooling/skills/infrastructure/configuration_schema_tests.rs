use super::apply_skill_configuration_schema;
use rusqlite::Connection;

fn schema_applied() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database");
    apply_skill_configuration_schema(&connection).expect("apply schema");
    connection
}

fn insert(
    connection: &Connection,
    skill_id: &str,
    scope: &str,
    workspace: &str,
) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO skill_configuration_records \
         (skill_id, scope, workspace_identity, schema_hash, base_revision, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 'hash', 'rev', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![skill_id, scope, workspace],
    )
}

#[test]
fn applying_the_schema_twice_is_a_no_op() {
    let connection = schema_applied();
    apply_skill_configuration_schema(&connection).expect("re-apply schema");

    insert(&connection, "configured-skill", "user", "").expect("insert survives re-apply");
}

#[test]
fn a_new_record_starts_at_revision_zero_with_no_values_and_no_secrets() {
    let connection = schema_applied();
    insert(&connection, "configured-skill", "user", "").expect("insert");

    let (revision, values, secrets, validation, cleanup, orphaned): (
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT stored_revision, values_json, secret_keys_json, validation_state, \
             cleanup_state, orphaned_at FROM skill_configuration_records",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("read defaults");

    assert_eq!(revision, 0);
    assert_eq!(values, "{}");
    assert_eq!(secrets, "[]");
    assert_eq!(validation, "compatible");
    assert_eq!(cleanup, "none");
    assert_eq!(orphaned, None);
}

#[test]
fn one_skill_cannot_hold_two_records_for_the_same_scope_and_workspace() {
    let connection = schema_applied();
    insert(&connection, "configured-skill", "user", "").expect("first user record");

    let duplicate = insert(&connection, "configured-skill", "user", "");

    assert!(
        duplicate.is_err(),
        "a second User record was accepted: {duplicate:?}"
    );
}

#[test]
fn scopes_workspaces_and_skills_are_isolated_from_each_other() {
    let connection = schema_applied();

    insert(&connection, "configured-skill", "user", "").expect("user record");
    insert(&connection, "configured-skill", "project", "/workspace/one").expect("project one");
    insert(&connection, "configured-skill", "project", "/workspace/two").expect("project two");
    // A different Skill whose id merely shares a prefix must not collide.
    insert(&connection, "configured-skill-extra", "user", "").expect("similarly named skill");

    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM skill_configuration_records",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 4);
}

#[test]
fn scope_validation_and_cleanup_columns_reject_values_outside_their_vocabulary() {
    let connection = schema_applied();

    assert!(
        insert(&connection, "configured-skill", "system", "").is_err(),
        "an unsupported scope was accepted"
    );

    insert(&connection, "configured-skill", "user", "").expect("valid record");
    assert!(
        connection
            .execute(
                "UPDATE skill_configuration_records SET validation_state = 'unknown'",
                [],
            )
            .is_err(),
        "an unsupported validation state was accepted"
    );
    assert!(
        connection
            .execute(
                "UPDATE skill_configuration_records SET cleanup_state = 'maybe'",
                [],
            )
            .is_err(),
        "an unsupported cleanup state was accepted"
    );
}

#[test]
fn an_existing_database_without_the_tables_gains_them_without_touching_other_data() {
    let connection = Connection::open_in_memory().expect("in-memory database");
    connection
        .execute_batch(
            "CREATE TABLE skills (id TEXT PRIMARY KEY, name TEXT NOT NULL);\
             INSERT INTO skills (id, name) VALUES ('existing-skill', 'Existing');",
        )
        .expect("seed a pre-migration database");

    apply_skill_configuration_schema(&connection).expect("apply schema");

    let name: String = connection
        .query_row(
            "SELECT name FROM skills WHERE id = 'existing-skill'",
            [],
            |row| row.get(0),
        )
        .expect("existing row survives");
    assert_eq!(name, "Existing");

    let configured: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM skill_configuration_records",
            [],
            |row| row.get(0),
        )
        .expect("new table is present");
    assert_eq!(configured, 0);
}
