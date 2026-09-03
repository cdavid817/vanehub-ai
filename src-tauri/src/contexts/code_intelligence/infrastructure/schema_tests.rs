use super::schema::{apply_language_registry_schema, apply_schema};
use rusqlite::{params, Connection};

/// Builds the pre-registry shape the language-registry migration has to upgrade: the original
/// `apply_schema` table, still carrying its `language_id` CHECK constraint and its NOT NULL
/// startup arguments.
fn legacy_connection() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory sqlite");
    apply_schema(&connection).expect("legacy LSP schema");
    connection
}

#[test]
fn language_registry_migration_preserves_revision_and_updated_at() {
    let connection = legacy_connection();
    connection
        .execute(
            "UPDATE lsp_language_configurations
             SET enabled = 1,
                 executable_override = 'C:/tools/rust-analyzer.exe',
                 initialization_options_json = '{\"check\":{\"command\":\"clippy\"}}',
                 revision = 7,
                 updated_at = '2020-01-01T00:00:00Z'
             WHERE language_id = 'rust'",
            [],
        )
        .expect("seed a configured language");

    apply_language_registry_schema(&connection).expect("language registry migration");

    let (enabled, executable_override, options, revision, updated_at) = connection
        .query_row(
            "SELECT enabled, executable_override, initialization_options_json, revision, updated_at
             FROM lsp_language_configurations WHERE language_id = 'rust'",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .expect("migrated rust row");

    // Revision feeds the server-instance configuration fingerprint. Resetting it here would make
    // every running server look stale and restart on the next launch -- a visible regression
    // produced by a migration that otherwise "worked".
    assert_eq!(revision, 7);
    assert_eq!(updated_at, "2020-01-01T00:00:00Z");
    assert_eq!(enabled, 1);
    assert_eq!(
        executable_override.as_deref(),
        Some("C:/tools/rust-analyzer.exe")
    );
    assert_eq!(options, "{\"check\":{\"command\":\"clippy\"}}");
}

#[test]
fn language_registry_migration_keeps_every_pre_existing_row() {
    let connection = legacy_connection();
    apply_language_registry_schema(&connection).expect("language registry migration");

    let ids = connection
        .prepare("SELECT language_id FROM lsp_language_configurations ORDER BY language_id")
        .expect("prepare language ids")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query language ids")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect language ids");

    assert_eq!(ids, vec!["rust", "typescript_javascript"]);
}

#[test]
fn language_registry_migration_admits_language_ids_the_old_check_rejected() {
    let connection = legacy_connection();
    // Proves the fixture is really the constrained shape, so the assertion after the migration
    // means the constraint was dropped rather than never having been there.
    connection
        .execute(
            "INSERT INTO lsp_language_configurations (language_id) VALUES ('go')",
            [],
        )
        .expect_err("legacy schema must reject an unlisted language id");

    apply_language_registry_schema(&connection).expect("language registry migration");

    connection
        .execute(
            "INSERT INTO lsp_language_configurations (language_id) VALUES ('go')",
            [],
        )
        .expect("registry schema admits any language id");
    let stored: String = connection
        .query_row(
            "SELECT language_id FROM lsp_language_configurations WHERE language_id = 'go'",
            [],
            |row| row.get(0),
        )
        .expect("stored language id");
    assert_eq!(stored, "go");
}

#[test]
fn language_registry_migration_distinguishes_unset_from_empty_startup_arguments() {
    let connection = legacy_connection();
    apply_language_registry_schema(&connection).expect("language registry migration");

    // NULL means "use the registry default"; an empty array means "the user chose no arguments".
    // Conflating them would strip `--stdio` from the TypeScript server whenever a user cleared
    // the field, which reads as a discovery bug rather than a configuration one.
    connection
        .execute(
            "UPDATE lsp_language_configurations SET startup_arguments_json = NULL
             WHERE language_id = 'rust'",
            [],
        )
        .expect("clear startup arguments to unset");
    connection
        .execute(
            "UPDATE lsp_language_configurations SET startup_arguments_json = '[]'
             WHERE language_id = 'typescript_javascript'",
            [],
        )
        .expect("set startup arguments to an explicit empty list");

    let unset: Option<String> = connection
        .query_row(
            "SELECT startup_arguments_json FROM lsp_language_configurations
             WHERE language_id = 'rust'",
            [],
            |row| row.get(0),
        )
        .expect("unset startup arguments");
    let empty: Option<String> = connection
        .query_row(
            "SELECT startup_arguments_json FROM lsp_language_configurations
             WHERE language_id = 'typescript_javascript'",
            [],
            |row| row.get(0),
        )
        .expect("empty startup arguments");

    assert_eq!(unset, None);
    assert_eq!(empty.as_deref(), Some("[]"));
}

#[test]
fn a_row_for_an_unregistered_language_survives_and_is_excluded_from_the_effective_configuration() {
    // Reachable by downgrading: a build that registers fewer languages than the one that wrote the
    // database. Loading must neither fail nor delete the row, because re-upgrading has to restore
    // the user's settings for that language exactly as they left them.
    let connection = legacy_connection();
    apply_language_registry_schema(&connection).expect("language registry migration");
    connection
        .execute(
            "INSERT INTO lsp_language_configurations (
                language_id, enabled, executable_override, initialization_options_json, revision
             ) VALUES ('go', 1, 'C:/tools/gopls.exe', '{\"build\":{}}', 9)",
            [],
        )
        .expect("insert configuration for an unregistered language");

    let (enabled, executable_override, options, revision) = connection
        .query_row(
            "SELECT enabled, executable_override, initialization_options_json, revision
             FROM lsp_language_configurations WHERE language_id = 'go'",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .expect("unregistered row is still present");

    assert_eq!(enabled, 1);
    assert_eq!(executable_override.as_deref(), Some("C:/tools/gopls.exe"));
    assert_eq!(options, "{\"build\":{}}");
    assert_eq!(revision, 9);
}

#[test]
fn language_registry_migration_is_idempotent() {
    let connection = legacy_connection();
    connection
        .execute(
            "UPDATE lsp_language_configurations SET revision = 4 WHERE language_id = 'rust'",
            params![],
        )
        .expect("seed revision");

    apply_language_registry_schema(&connection).expect("first application");
    apply_language_registry_schema(&connection).expect("second application");

    let revision: i64 = connection
        .query_row(
            "SELECT revision FROM lsp_language_configurations WHERE language_id = 'rust'",
            [],
            |row| row.get(0),
        )
        .expect("revision after repeated application");
    assert_eq!(revision, 4);
}
