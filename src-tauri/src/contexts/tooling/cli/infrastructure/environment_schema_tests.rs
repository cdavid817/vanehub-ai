// Included through `#[path]` from environment_schema.rs.
//
// Every test opens its own in-memory database. Nothing here touches the shared
// `%APPDATA%\ai.vanehub.app\vanehub.sqlite` -- all worktrees share that file, and a test that
// wrote to it would corrupt whatever another session was doing.
use super::*;

fn schema_database() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database");
    apply_environment_snapshot_schema(&connection).expect("snapshot schema");
    apply_version_catalog_schema(&connection).expect("catalog schema");
    apply_action_plan_schema(&connection).expect("plan schema");
    connection
}

fn table_exists(connection: &Connection, name: &str) -> bool {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .expect("query")
        > 0
}

#[test]
fn the_three_tables_and_their_indexes_are_created() {
    let connection = schema_database();

    for table in [
        "cli_environment_snapshots",
        "cli_version_catalogs",
        "cli_action_plans",
    ] {
        assert!(table_exists(&connection, table), "{table}");
    }
    let indexes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_cli_%'",
            [],
            |row| row.get(0),
        )
        .expect("index count");
    assert_eq!(indexes, 3);
}

#[test]
fn applying_the_schema_twice_is_a_no_op() {
    // A migration can be re-entered after an interrupted startup; `IF NOT EXISTS` has to hold.
    let connection = schema_database();
    apply_environment_snapshot_schema(&connection).expect("second snapshot apply");
    apply_version_catalog_schema(&connection).expect("second catalog apply");
    apply_action_plan_schema(&connection).expect("second plan apply");
    assert!(table_exists(&connection, "cli_action_plans"));
}

#[test]
fn one_tool_can_hold_a_catalog_per_source_and_channel() {
    // The storage key enforces what the domain requires: an npm catalog can never overwrite a
    // WinGet one for the same tool.
    let connection = schema_database();
    let insert = "INSERT INTO cli_version_catalogs
        (agent_id, scope_id, source_id, channel, catalog_json, fetched_at, expires_at)
        VALUES (?1, 'local-desktop', ?2, ?3, '{}', 'now', 'later')";

    connection
        .execute(insert, rusqlite::params!["claude-code", "npm", "stable"])
        .expect("npm catalog");
    connection
        .execute(insert, rusqlite::params!["claude-code", "winget", "stable"])
        .expect("winget catalog");
    connection
        .execute(insert, rusqlite::params!["claude-code", "npm", "next"])
        .expect("npm next channel");

    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cli_version_catalogs WHERE agent_id = 'claude-code'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 3);

    // The same key twice is a conflict, not a silent duplicate.
    let duplicate = connection.execute(insert, rusqlite::params!["claude-code", "npm", "stable"]);
    assert!(duplicate.is_err());
}

#[test]
fn a_snapshot_is_unique_per_tool_and_scope() {
    let connection = schema_database();
    let insert = "INSERT INTO cli_environment_snapshots
        (agent_id, scope_id, schema_version, environment_fingerprint, snapshot_json)
        VALUES ('claude-code', 'local-desktop', 1, 'fp', '{}')";

    connection.execute(insert, []).expect("first");
    assert!(connection.execute(insert, []).is_err(), "primary key holds");
}

#[test]
fn an_unknown_plan_state_or_kind_is_rejected_by_the_schema() {
    let connection = schema_database();
    let insert = "INSERT INTO cli_action_plans
        (plan_id, plan_kind, agent_id, scope_id, revision, state,
         environment_fingerprint, plan_json, created_at, expires_at)
        VALUES (?1, ?2, 'claude-code', 'local-desktop', 1, ?3, 'fp', '{}', 'now', 'later')";

    connection
        .execute(insert, rusqlite::params!["p1", "action", "draft"])
        .expect("a valid row");

    // A typo in a state string would otherwise persist and then fail to decode much later.
    assert!(connection
        .execute(insert, rusqlite::params!["p2", "action", "in-progress"])
        .is_err());
    assert!(connection
        .execute(insert, rusqlite::params!["p3", "singleton", "draft"])
        .is_err());
}

#[test]
fn the_legacy_cli_tool_status_table_is_untouched_by_this_schema() {
    // These migrations are additive. A first read after upgrading maps a legacy row into a stale
    // snapshot, which is impossible if the table was dropped.
    let connection = schema_database();
    assert!(!table_exists(&connection, "cli_tool_status"));
    // Nothing in the three statements references it, so applying them to a database that has it
    // leaves it alone.
    connection
        .execute_batch("CREATE TABLE cli_tool_status (agent_id TEXT PRIMARY KEY);")
        .expect("legacy table");
    apply_environment_snapshot_schema(&connection).expect("re-apply");
    assert!(table_exists(&connection, "cli_tool_status"));
}
