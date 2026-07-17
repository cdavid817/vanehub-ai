use super::credentials::{credential_account, delete_connector_credential, get_connector_credential, CredentialStore};
use super::models::{ConnectorConfig, ConnectorKind, RoutingSettings};
use crate::AppError;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

pub fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
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
    migrate_legacy_wechat_id(conn)?;
    migrate_credential_refs(conn)?;
    Ok(())
}

pub fn apply_session_source_schema(conn: &Connection) -> Result<(), AppError> {
    if !table_has_column(conn, "sessions", "source_kind")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'desktop'",
            [],
        )?;
    }
    if !table_has_column(conn, "sessions", "source_connector")? {
        conn.execute("ALTER TABLE sessions ADD COLUMN source_connector TEXT", [])?;
    }
    conn.execute(
        "UPDATE sessions SET source_connector = 'weixin' WHERE source_connector = 'wechat'",
        [],
    )?;
    Ok(())
}

fn migrate_legacy_wechat_id(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
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

fn migrate_credential_refs(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
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

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for candidate in columns {
        if candidate? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub struct ImRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ImRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn save_connector(&self, config: &ConnectorConfig) -> Result<(), AppError> {
        let public_config = serde_json::to_string(&config.public_config)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        self.conn.execute(
            r#"INSERT INTO im_connector_configs
               (connector, enabled, display_name, public_config, credential_ref, updated_at)
               VALUES (?1, ?2, ?3, ?4, NULL, ?5)
               ON CONFLICT(connector) DO UPDATE SET
                 enabled = excluded.enabled,
                 display_name = excluded.display_name,
                 public_config = excluded.public_config,
                 updated_at = excluded.updated_at"#,
            params![
                config.kind.as_str(),
                config.enabled,
                config.display_name,
                public_config,
                Utc::now().to_rfc3339(),
            ],
        )?;
        match &config.credential_ref {
            Some(credential_ref) => {
                self.conn.execute(
                    r#"INSERT INTO im_credential_refs (connector, credential_ref, updated_at)
                       VALUES (?1, ?2, ?3)
                       ON CONFLICT(connector) DO UPDATE SET
                         credential_ref = excluded.credential_ref,
                         updated_at = excluded.updated_at"#,
                    params![
                        config.kind.as_str(),
                        credential_ref,
                        Utc::now().to_rfc3339()
                    ],
                )?;
            }
            None => {
                self.conn.execute(
                    "DELETE FROM im_credential_refs WHERE connector = ?1",
                    [config.kind.as_str()],
                )?;
            }
        }
        Ok(())
    }

    pub fn connector(&self, kind: ConnectorKind) -> Result<Option<ConnectorConfig>, AppError> {
        self.conn
            .query_row(
                r#"SELECT configs.enabled, configs.display_name, configs.public_config,
                          refs.credential_ref
                   FROM im_connector_configs AS configs
                   LEFT JOIN im_credential_refs AS refs ON refs.connector = configs.connector
                   WHERE configs.connector = ?1"#,
                [kind.as_str()],
                |row| {
                    let public_config: String = row.get(2)?;
                    Ok(ConnectorConfig {
                        kind,
                        enabled: row.get(0)?,
                        display_name: row.get(1)?,
                        public_config: serde_json::from_str(&public_config).unwrap_or_default(),
                        credential_ref: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn save_routing(&self, routing: &RoutingSettings) -> Result<(), AppError> {
        self.conn.execute(
            r#"INSERT INTO im_routing_settings (id, agent_id, project_path, updated_at)
               VALUES (1, ?1, ?2, ?3)
               ON CONFLICT(id) DO UPDATE SET agent_id = excluded.agent_id,
                 project_path = excluded.project_path, updated_at = excluded.updated_at"#,
            params![
                routing.agent_id,
                routing.project_path,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn routing(&self) -> Result<Option<RoutingSettings>, AppError> {
        self.conn
            .query_row(
                "SELECT agent_id, project_path FROM im_routing_settings WHERE id = 1",
                [],
                |row| {
                    Ok(RoutingSettings {
                        agent_id: row.get(0)?,
                        project_path: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn bind_chat(
        &self,
        kind: ConnectorKind,
        chat_id: &str,
        session_id: &str,
    ) -> Result<(), AppError> {
        self.conn.execute(
            r#"INSERT INTO im_session_bindings (connector, external_chat_hash, session_id, created_at)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(connector, external_chat_hash) DO UPDATE SET session_id = excluded.session_id"#,
            params![kind.as_str(), stable_hash(chat_id), session_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn bound_session(
        &self,
        kind: ConnectorKind,
        chat_id: &str,
    ) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT session_id FROM im_session_bindings WHERE connector = ?1 AND external_chat_hash = ?2",
                params![kind.as_str(), stable_hash(chat_id)],
                |row| row.get(0),
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn claim_event(&self, kind: ConnectorKind, event_id: &str) -> Result<bool, AppError> {
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO im_inbound_dedup (connector, event_hash, received_at) VALUES (?1, ?2, ?3)",
            params![kind.as_str(), stable_hash(event_id), Utc::now().to_rfc3339()],
        )?;
        Ok(changed == 1)
    }

    pub fn cleanup_dedup_before(&self, cutoff: &str) -> Result<usize, AppError> {
        self.conn
            .execute(
                "DELETE FROM im_inbound_dedup WHERE received_at < ?1",
                [cutoff],
            )
            .map_err(AppError::from)
    }

    pub fn save_checkpoint(
        &self,
        kind: ConnectorKind,
        key: &str,
        value: &str,
    ) -> Result<(), AppError> {
        self.conn.execute(
            r#"INSERT INTO im_connector_checkpoints (connector, checkpoint_key, value, updated_at)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(connector, checkpoint_key) DO UPDATE SET value = excluded.value,
                 updated_at = excluded.updated_at"#,
            params![kind.as_str(), key, value, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn checkpoint(&self, kind: ConnectorKind, key: &str) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT value FROM im_connector_checkpoints WHERE connector = ?1 AND checkpoint_key = ?2",
                params![kind.as_str(), key],
                |row| row.get(0),
            )
            .optional()
            .map_err(AppError::from)
    }
}

fn stable_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn save_connector_with_secret(
    conn: &Connection,
    credentials: &dyn CredentialStore,
    mut config: ConnectorConfig,
    replacement_secret: Option<&str>,
) -> Result<(), AppError> {
    reject_sensitive_public_config(&config.public_config)?;
    let account = credential_account(config.kind, "default");
    let previous = if replacement_secret.is_some() {
        credentials.get(&account)?
    } else {
        None
    };
    if let Some(secret) = replacement_secret {
        credentials.set(&account, secret)?;
        config.credential_ref = Some(account.clone());
    }

    let save_result = (|| {
        let transaction = conn.unchecked_transaction()?;
        ImRepository::new(&transaction).save_connector(&config)?;
        transaction.commit()?;
        Ok(())
    })();
    if let Err(error) = save_result {
        if replacement_secret.is_some() {
            compensate_credential(credentials, &account, previous.as_deref())?;
        }
        return Err(error);
    }
    Ok(())
}

pub fn clear_connector_credentials(
    conn: &Connection,
    credentials: &dyn CredentialStore,
    kind: ConnectorKind,
) -> Result<(), AppError> {
    let account = credential_account(kind, "default");
    let previous = get_connector_credential(credentials, kind, "default")?;
    delete_connector_credential(credentials, kind, "default")?;
    let Some(mut config) = ImRepository::new(conn).connector(kind)? else {
        return Ok(());
    };
    config.credential_ref = None;
    if let Err(error) = ImRepository::new(conn).save_connector(&config) {
        compensate_credential(credentials, &account, previous.as_deref())?;
        return Err(error);
    }
    Ok(())
}

fn compensate_credential(
    credentials: &dyn CredentialStore,
    account: &str,
    previous: Option<&String>,
) -> Result<(), AppError> {
    match previous {
        Some(secret) => credentials.set(account, secret),
        None => credentials.delete(account),
    }
}

fn reject_sensitive_public_config(value: &serde_json::Value) -> Result<(), AppError> {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let normalized = key.to_ascii_lowercase();
                if normalized.contains("secret")
                    || normalized.contains("token")
                    || normalized.contains("password")
                    || normalized.contains("authorization")
                {
                    return Err(AppError::Validation(format!(
                        "sensitive connector field is not allowed in public config: {key}"
                    )));
                }
                reject_sensitive_public_config(value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                reject_sensitive_public_config(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::im::credentials::{credential_account, MemoryCredentialStore};
    use serde_json::json;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE agents (id TEXT PRIMARY KEY); CREATE TABLE sessions (id TEXT PRIMARY KEY);").unwrap();
        conn.execute("INSERT INTO agents (id) VALUES ('codex-cli')", [])
            .unwrap();
        conn.execute("INSERT INTO sessions (id) VALUES ('session-1')", [])
            .unwrap();
        apply_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn stores_public_config_without_external_identity() {
        let conn = database();
        let repository = ImRepository::new(&conn);
        repository
            .save_connector(&ConnectorConfig {
                kind: ConnectorKind::Telegram,
                enabled: true,
                display_name: Some("Support bot".into()),
                public_config: json!({"apiBase": "https://api.telegram.org"}),
                credential_ref: Some("im/telegram/default".into()),
            })
            .unwrap();
        repository
            .bind_chat(ConnectorKind::Telegram, "private-chat-42", "session-1")
            .unwrap();

        assert_eq!(
            repository
                .connector(ConnectorKind::Telegram)
                .unwrap()
                .unwrap()
                .enabled,
            true
        );
        assert_eq!(
            repository
                .bound_session(ConnectorKind::Telegram, "private-chat-42")
                .unwrap(),
            Some("session-1".into())
        );
        let dump: String = conn
            .query_row(
                "SELECT external_chat_hash FROM im_session_bindings",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!dump.contains("private-chat-42"));
    }

    #[test]
    fn deduplicates_events_and_restores_checkpoints() {
        let conn = database();
        let repository = ImRepository::new(&conn);
        assert!(repository
            .claim_event(ConnectorKind::Feishu, "event-1")
            .unwrap());
        assert!(!repository
            .claim_event(ConnectorKind::Feishu, "event-1")
            .unwrap());
        repository
            .save_checkpoint(ConnectorKind::Telegram, "offset", "42")
            .unwrap();
        assert_eq!(
            repository
                .checkpoint(ConnectorKind::Telegram, "offset")
                .unwrap(),
            Some("42".into())
        );
    }

    #[test]
    fn session_delete_cascades_connector_binding() {
        let conn = database();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        let repository = ImRepository::new(&conn);
        repository
            .bind_chat(ConnectorKind::WeCom, "external-chat", "session-1")
            .unwrap();
        conn.execute("DELETE FROM sessions WHERE id = 'session-1'", [])
            .unwrap();

        assert!(repository
            .bound_session(ConnectorKind::WeCom, "external-chat")
            .unwrap()
            .is_none());
    }

    #[test]
    fn atomically_saves_secret_outside_sqlite_and_clears_it() {
        let conn = database();
        let credentials = MemoryCredentialStore::default();
        let config = ConnectorConfig {
            kind: ConnectorKind::Telegram,
            enabled: true,
            display_name: None,
            public_config: json!({"apiBase": "https://api.telegram.org"}),
            credential_ref: None,
        };
        save_connector_with_secret(&conn, &credentials, config, Some("fixture-private-value"))
            .unwrap();

        let serialized: String = conn
            .query_row(
                "SELECT public_config || coalesce(credential_ref, '') FROM im_connector_configs",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!serialized.contains("fixture-private-value"));
        let legacy_ref: Option<String> = conn
            .query_row(
                "SELECT credential_ref FROM im_connector_configs",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(legacy_ref.is_none());
        let stored_ref: String = conn
            .query_row(
                "SELECT credential_ref FROM im_credential_refs",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let account = credential_account(ConnectorKind::Telegram, "default");
        assert_eq!(stored_ref, account);
        assert_eq!(
            credentials.get(&account).unwrap().unwrap().as_str(),
            "fixture-private-value"
        );

        clear_connector_credentials(&conn, &credentials, ConnectorKind::Telegram).unwrap();
        assert!(credentials.get(&account).unwrap().is_none());
        assert!(ImRepository::new(&conn)
            .connector(ConnectorKind::Telegram)
            .unwrap()
            .unwrap()
            .credential_ref
            .is_none());
    }

    #[test]
    fn restores_previous_secret_when_database_save_fails() {
        let conn = Connection::open_in_memory().unwrap();
        let credentials = MemoryCredentialStore::default();
        let account = credential_account(ConnectorKind::DingTalk, "default");
        credentials.set(&account, "previous-value").unwrap();
        let result = save_connector_with_secret(
            &conn,
            &credentials,
            ConnectorConfig {
                kind: ConnectorKind::DingTalk,
                enabled: true,
                display_name: None,
                public_config: json!({}),
                credential_ref: Some(account.clone()),
            },
            Some("replacement-value"),
        );

        assert!(result.is_err());
        assert_eq!(
            credentials.get(&account).unwrap().unwrap().as_str(),
            "previous-value"
        );
    }

    #[test]
    fn rejects_secret_fields_in_public_config() {
        let conn = database();
        let credentials = MemoryCredentialStore::default();
        let result = save_connector_with_secret(
            &conn,
            &credentials,
            ConnectorConfig {
                kind: ConnectorKind::Feishu,
                enabled: false,
                display_name: None,
                public_config: json!({"appSecret": "must-not-persist"}),
                credential_ref: None,
            },
            None,
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn migrates_legacy_wechat_rows_to_weixin() {
        let conn = database();
        conn.execute(
            "INSERT INTO im_connector_configs VALUES ('wechat', 0, NULL, '{}', 'wechat/default', 'now')",
            [],
        )
        .unwrap();
        apply_schema(&conn).unwrap();
        let (connector, credential_ref): (String, Option<String>) = conn
            .query_row(
                "SELECT connector, credential_ref FROM im_connector_configs",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(connector, "weixin");
        assert!(credential_ref.is_none());
        let migrated_ref: String = conn
            .query_row(
                "SELECT credential_ref FROM im_credential_refs WHERE connector = 'weixin'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated_ref, "weixin/default");
    }
}
