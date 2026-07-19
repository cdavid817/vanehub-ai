use crate::platform::database::{migrate, table_has_column};
use rusqlite::Connection;

const EMPTY_FIXTURE: &str = include_str!("../tests/fixtures/database/empty.sql");
const LEGACY_V1_FIXTURE: &str = include_str!("../tests/fixtures/database/legacy-v1.sql");
const CURRENT_V20_DATA_FIXTURE: &str =
    include_str!("../tests/fixtures/database/current-v20-data.sql");

fn applied_versions(conn: &Connection) -> Vec<i64> {
    conn.prepare("SELECT version FROM schema_migrations ORDER BY version")
        .expect("prepare versions")
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("query versions")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect versions")
}

#[test]
fn empty_fixture_migrates_to_latest_schema() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(EMPTY_FIXTURE)
        .expect("load empty fixture");

    migrate(&conn).expect("migrate empty fixture");

    assert_eq!(applied_versions(&conn), (1..=22).collect::<Vec<_>>());
    assert!(table_has_column(&conn, "sessions", "remote_workspace_uri")
        .expect("remote workspace column"));
    assert!(table_has_column(&conn, "messages", "rich_blocks").expect("rich block column"));
    assert!(table_has_column(&conn, "usage_records", "message_id").expect("usage record table"));
}

#[test]
fn legacy_v1_fixture_upgrades_without_losing_records() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(LEGACY_V1_FIXTURE)
        .expect("load legacy fixture");

    migrate(&conn).expect("migrate legacy fixture");

    assert_eq!(applied_versions(&conn), (1..=22).collect::<Vec<_>>());
    assert!(
        table_has_column(&conn, "agents", "managed_sdk_dependency_id").expect("managed SDK column")
    );
    assert!(
        table_has_column(&conn, "workflow_state", "active_session_id")
            .expect("active session column")
    );
    assert_eq!(
        conn.query_row(
            "SELECT display_name FROM agents WHERE id = 'legacy-agent'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("legacy agent"),
        "Legacy Agent"
    );
    assert_eq!(
        conn.query_row(
            "SELECT command FROM mcp_servers WHERE name = 'legacy-mcp'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("legacy MCP server"),
        "legacy-mcp"
    );
}

#[test]
fn current_v20_fixture_is_idempotent_and_readable() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    migrate(&conn).expect("initial current migration");
    crate::contexts::agent_runtime::infrastructure::seed_registry(&conn).expect("seed agents");
    conn.execute_batch(CURRENT_V20_DATA_FIXTURE)
        .expect("load current data fixture");

    migrate(&conn).expect("repeat current migration");

    assert_eq!(applied_versions(&conn), (1..=22).collect::<Vec<_>>());
    assert!(
        table_has_column(&conn, "sdk_operation_logs", "operation_id")
            .expect("SDK operation log column")
    );
    assert_eq!(
        conn.query_row(
            "SELECT content FROM messages WHERE id = 'fixture-message'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("fixture message"),
        "Persisted fixture"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'application_language'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("fixture setting"),
        "en"
    );
}
