use crate::platform::database::table_has_column;
use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS im_connector_configs (
            connector TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 0,
            display_name TEXT,
            public_config TEXT NOT NULL DEFAULT '{}',
            credential_ref TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS im_credential_refs (
            connector TEXT PRIMARY KEY,
            credential_ref TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (connector) REFERENCES im_connector_configs(connector) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS im_routing_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            agent_id TEXT NOT NULL,
            project_path TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );
        CREATE TABLE IF NOT EXISTS im_session_bindings (
            connector TEXT NOT NULL,
            external_chat_hash TEXT NOT NULL,
            session_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (connector, external_chat_hash),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_im_bindings_session
            ON im_session_bindings(session_id);
        CREATE TABLE IF NOT EXISTS im_inbound_dedup (
            connector TEXT NOT NULL,
            event_hash TEXT NOT NULL,
            received_at TEXT NOT NULL,
            PRIMARY KEY (connector, event_hash)
        );
        CREATE INDEX IF NOT EXISTS idx_im_dedup_received
            ON im_inbound_dedup(received_at);
        CREATE TABLE IF NOT EXISTS im_connector_checkpoints (
            connector TEXT NOT NULL,
            checkpoint_key TEXT NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (connector, checkpoint_key)
        );
        "#,
    )?;
    ensure_connector_config_columns(connection)?;
    ensure_credential_ref_columns(connection)?;
    migrate_legacy_wechat_id(connection)?;
    migrate_credential_refs(connection)?;
    Ok(())
}

pub(crate) fn apply_session_source_schema(connection: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(connection, "sessions", "source_kind")? {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'desktop'",
            [],
        )?;
    }
    if !table_has_column(connection, "sessions", "source_connector")? {
        connection.execute("ALTER TABLE sessions ADD COLUMN source_connector TEXT", [])?;
    }
    connection.execute(
        "UPDATE sessions SET source_connector = 'weixin' WHERE source_connector = 'wechat'",
        [],
    )?;
    Ok(())
}

fn migrate_legacy_wechat_id(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        INSERT OR IGNORE INTO im_connector_configs
            (connector, enabled, display_name, public_config, credential_ref, updated_at)
        SELECT 'weixin', enabled, display_name, public_config,
            replace(credential_ref, 'wechat/', 'weixin/'), updated_at
        FROM im_connector_configs WHERE connector = 'wechat';

        INSERT OR IGNORE INTO im_credential_refs (connector, credential_ref, updated_at)
        SELECT 'weixin', replace(credential_ref, 'wechat/', 'weixin/'), updated_at
        FROM im_credential_refs WHERE connector = 'wechat';
        DELETE FROM im_credential_refs WHERE connector = 'wechat';
        DELETE FROM im_connector_configs WHERE connector = 'wechat';

        INSERT OR IGNORE INTO im_session_bindings
            (connector, external_chat_hash, session_id, created_at)
        SELECT 'weixin', external_chat_hash, session_id, created_at
        FROM im_session_bindings WHERE connector = 'wechat';
        DELETE FROM im_session_bindings WHERE connector = 'wechat';

        INSERT OR IGNORE INTO im_inbound_dedup (connector, event_hash, received_at)
        SELECT 'weixin', event_hash, received_at
        FROM im_inbound_dedup WHERE connector = 'wechat';
        DELETE FROM im_inbound_dedup WHERE connector = 'wechat';

        INSERT OR IGNORE INTO im_connector_checkpoints
            (connector, checkpoint_key, value, updated_at)
        SELECT 'weixin', checkpoint_key, value, updated_at
        FROM im_connector_checkpoints WHERE connector = 'wechat';
        DELETE FROM im_connector_checkpoints WHERE connector = 'wechat';
        "#,
    )?;
    Ok(())
}

fn ensure_connector_config_columns(connection: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(connection, "im_connector_configs", "display_name")? {
        connection.execute(
            "ALTER TABLE im_connector_configs ADD COLUMN display_name TEXT",
            [],
        )?;
    }
    if !table_has_column(connection, "im_connector_configs", "public_config")? {
        connection.execute(
            "ALTER TABLE im_connector_configs ADD COLUMN public_config TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }
    if !table_has_column(connection, "im_connector_configs", "credential_ref")? {
        connection.execute(
            "ALTER TABLE im_connector_configs ADD COLUMN credential_ref TEXT",
            [],
        )?;
    }
    if !table_has_column(connection, "im_connector_configs", "updated_at")? {
        connection.execute(
            "ALTER TABLE im_connector_configs ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z'",
            [],
        )?;
    }
    Ok(())
}

fn ensure_credential_ref_columns(connection: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(connection, "im_credential_refs", "updated_at")? {
        connection.execute(
            "ALTER TABLE im_credential_refs ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z'",
            [],
        )?;
    }
    Ok(())
}

fn migrate_credential_refs(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        INSERT INTO im_credential_refs (connector, credential_ref, updated_at)
        SELECT connector, credential_ref, updated_at
        FROM im_connector_configs
        WHERE credential_ref IS NOT NULL
        ON CONFLICT(connector) DO UPDATE SET
            credential_ref = excluded.credential_ref,
            updated_at = excluded.updated_at;
        UPDATE im_connector_configs SET credential_ref = NULL
        WHERE credential_ref IS NOT NULL;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_additive_and_moves_legacy_wechat_data() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agents (id TEXT PRIMARY KEY);
                CREATE TABLE sessions (id TEXT PRIMARY KEY);
                CREATE TABLE im_connector_configs (
                    connector TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 0,
                    display_name TEXT,
                    public_config TEXT NOT NULL DEFAULT '{}',
                    credential_ref TEXT,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO im_connector_configs
                    (connector, enabled, display_name, public_config, credential_ref, updated_at)
                VALUES ('wechat', 1, 'Legacy', '{}', 'wechat/default', '2026-01-01T00:00:00Z');
                "#,
            )
            .expect("legacy fixture");

        apply_schema(&connection).expect("migration");

        let migrated: (String, i64, Option<String>) = connection
            .query_row(
                "SELECT connector, enabled, credential_ref FROM im_connector_configs",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migrated config");
        let credential_ref: String = connection
            .query_row(
                "SELECT credential_ref FROM im_credential_refs WHERE connector = 'weixin'",
                [],
                |row| row.get(0),
            )
            .expect("migrated credential reference");

        assert_eq!(migrated, ("weixin".to_string(), 1, None));
        assert_eq!(credential_ref, "weixin/default");
        assert!(
            table_has_column(&connection, "im_connector_configs", "updated_at").expect("column")
        );
    }

    #[test]
    fn migration_repairs_older_connector_tables_missing_later_columns() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agents (id TEXT PRIMARY KEY);
                CREATE TABLE sessions (id TEXT PRIMARY KEY);
                CREATE TABLE im_connector_configs (
                    connector TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE im_credential_refs (
                    connector TEXT PRIMARY KEY,
                    credential_ref TEXT NOT NULL
                );
                INSERT INTO im_connector_configs (connector, enabled)
                VALUES ('telegram', 0);
                "#,
            )
            .expect("old fixture");

        apply_schema(&connection).expect("migration");

        for column in [
            "display_name",
            "public_config",
            "credential_ref",
            "updated_at",
        ] {
            assert!(
                table_has_column(&connection, "im_connector_configs", column).expect("column"),
                "missing im_connector_configs.{column}",
            );
        }
        assert!(table_has_column(&connection, "im_credential_refs", "updated_at").expect("column"));
        let public_config: String = connection
            .query_row(
                "SELECT public_config FROM im_connector_configs WHERE connector = 'telegram'",
                [],
                |row| row.get(0),
            )
            .expect("public config");
        assert_eq!(public_config, "{}");
    }
}
