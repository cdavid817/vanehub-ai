use crate::contexts::communications::application::{
    CommunicationsApplicationError, CommunicationsRepository,
};
use crate::contexts::communications::domain::{
    ChatBinding, ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig,
    ConnectorKind, InboundEventIdentity, RoutingSettings,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

const CONNECTOR_SELECT: &str = r#"
    SELECT configs.connector, configs.enabled, configs.display_name, configs.public_config,
           refs.credential_ref
    FROM im_connector_configs AS configs
    LEFT JOIN im_credential_refs AS refs ON refs.connector = configs.connector
"#;

#[derive(Clone)]
pub(crate) struct SqliteCommunicationsRepository {
    database: NativeDatabase,
}

impl SqliteCommunicationsRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<Connection, CommunicationsApplicationError> {
        self.database
            .connection()
            .map_err(|_| repository_unavailable())
    }

    pub(crate) fn find_binding(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT session_id FROM im_session_bindings \
                 WHERE connector = ?1 AND external_chat_hash = ?2",
                params![
                    key.connector().as_str(),
                    stable_hash(key.external_chat_id())
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_error)
    }

    pub(crate) fn save_binding(
        &self,
        binding: &ChatBinding,
        created_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO im_session_bindings
                   (connector, external_chat_hash, session_id, created_at)
                   VALUES (?1, ?2, ?3, ?4)
                   ON CONFLICT(connector, external_chat_hash) DO UPDATE SET
                     session_id = excluded.session_id"#,
                params![
                    binding.key().connector().as_str(),
                    stable_hash(binding.key().external_chat_id()),
                    binding.session_id(),
                    created_at,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub(crate) fn reset_bindings(
        &self,
        kind: Option<ConnectorKind>,
    ) -> Result<usize, CommunicationsApplicationError> {
        let connection = self.connection()?;
        match kind {
            Some(kind) => connection.execute(
                "DELETE FROM im_session_bindings WHERE connector = ?1",
                [kind.as_str()],
            ),
            None => connection.execute("DELETE FROM im_session_bindings", []),
        }
        .map_err(sqlite_error)
    }
}

impl CommunicationsRepository for SqliteCommunicationsRepository {
    fn list_configurations(&self) -> Result<Vec<ConnectorConfig>, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!("{CONNECTOR_SELECT} ORDER BY configs.connector"))
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], ConnectorRow::read)
            .map_err(sqlite_error)?;
        let mut configurations = Vec::new();
        for row in rows {
            configurations.push(row.map_err(sqlite_error)?.into_domain()?);
        }
        Ok(configurations)
    }

    fn find_configuration(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorConfig>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                &format!("{CONNECTOR_SELECT} WHERE configs.connector = ?1"),
                [kind.as_str()],
                ConnectorRow::read,
            )
            .optional()
            .map_err(sqlite_error)?
            .map(ConnectorRow::into_domain)
            .transpose()
    }

    fn save_configuration(
        &self,
        configuration: &ConnectorConfig,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        configuration.validate()?;
        let public_config = serde_json::to_string(&configuration.public_config)
            .map_err(|_| invalid_repository_data())?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        transaction
            .execute(
                r#"INSERT INTO im_connector_configs
                   (connector, enabled, display_name, public_config, credential_ref, updated_at)
                   VALUES (?1, ?2, ?3, ?4, NULL, ?5)
                   ON CONFLICT(connector) DO UPDATE SET
                     enabled = excluded.enabled,
                     display_name = excluded.display_name,
                     public_config = excluded.public_config,
                     credential_ref = NULL,
                     updated_at = excluded.updated_at"#,
                params![
                    configuration.kind.as_str(),
                    configuration.enabled,
                    configuration.display_name.as_deref(),
                    public_config,
                    updated_at,
                ],
            )
            .map_err(sqlite_error)?;
        match configuration.credential_ref.as_deref() {
            Some(credential_ref) => {
                transaction
                    .execute(
                        r#"INSERT INTO im_credential_refs
                           (connector, credential_ref, updated_at)
                           VALUES (?1, ?2, ?3)
                           ON CONFLICT(connector) DO UPDATE SET
                             credential_ref = excluded.credential_ref,
                             updated_at = excluded.updated_at"#,
                        params![configuration.kind.as_str(), credential_ref, updated_at],
                    )
                    .map_err(sqlite_error)?;
            }
            None => {
                transaction
                    .execute(
                        "DELETE FROM im_credential_refs WHERE connector = ?1",
                        [configuration.kind.as_str()],
                    )
                    .map_err(sqlite_error)?;
            }
        }
        transaction.commit().map_err(sqlite_error)
    }

    fn load_routing(&self) -> Result<Option<RoutingSettings>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT agent_id, project_path FROM im_routing_settings WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(sqlite_error)?
            .map(|(agent_id, project_path)| {
                RoutingSettings::new(agent_id, project_path).map_err(Into::into)
            })
            .transpose()
    }

    fn save_routing(
        &self,
        routing: &RoutingSettings,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        let routing = routing.normalized()?;
        self.connection()?
            .execute(
                r#"INSERT INTO im_routing_settings (id, agent_id, project_path, updated_at)
                   VALUES (1, ?1, ?2, ?3)
                   ON CONFLICT(id) DO UPDATE SET
                     agent_id = excluded.agent_id,
                     project_path = excluded.project_path,
                     updated_at = excluded.updated_at"#,
                params![routing.agent_id, routing.project_path, updated_at],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn claim_event(
        &self,
        event: &InboundEventIdentity,
        received_at: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "INSERT OR IGNORE INTO im_inbound_dedup \
                 (connector, event_hash, received_at) VALUES (?1, ?2, ?3)",
                params![
                    event.connector().as_str(),
                    stable_hash(event.event_id()),
                    received_at,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(changed == 1)
    }

    fn cleanup_dedup_before(&self, cutoff: &str) -> Result<usize, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_inbound_dedup WHERE received_at < ?1",
                [cutoff],
            )
            .map_err(sqlite_error)
    }

    fn load_checkpoint(
        &self,
        key: &CheckpointKey,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT value FROM im_connector_checkpoints \
                 WHERE connector = ?1 AND checkpoint_key = ?2",
                params![key.connector().as_str(), key.name()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_error)
    }

    fn save_checkpoint(
        &self,
        checkpoint: &ConnectorCheckpoint,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO im_connector_checkpoints
                   (connector, checkpoint_key, value, updated_at)
                   VALUES (?1, ?2, ?3, ?4)
                   ON CONFLICT(connector, checkpoint_key) DO UPDATE SET
                     value = excluded.value,
                     updated_at = excluded.updated_at"#,
                params![
                    checkpoint.key().connector().as_str(),
                    checkpoint.key().name(),
                    checkpoint.value(),
                    updated_at,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }
}

struct ConnectorRow {
    connector: String,
    enabled: bool,
    display_name: Option<String>,
    public_config: String,
    credential_ref: Option<String>,
}

impl ConnectorRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            connector: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            display_name: row.get(2)?,
            public_config: row.get(3)?,
            credential_ref: row.get(4)?,
        })
    }

    fn into_domain(self) -> Result<ConnectorConfig, CommunicationsApplicationError> {
        let kind = ConnectorKind::parse(&self.connector).ok_or_else(invalid_repository_data)?;
        let configuration = ConnectorConfig {
            kind,
            enabled: self.enabled,
            display_name: self.display_name,
            public_config: serde_json::from_str(&self.public_config)
                .map_err(|_| invalid_repository_data())?,
            credential_ref: self.credential_ref,
        };
        configuration.validate()?;
        Ok(configuration)
    }
}

fn stable_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sqlite_error(_error: rusqlite::Error) -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-repository-failed")
}

fn repository_unavailable() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-repository-unavailable")
}

fn invalid_repository_data() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-repository-data-invalid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use serde_json::json;

    struct Fixture {
        repository: SqliteCommunicationsRepository,
        database: NativeDatabase,
        _directory: TempDirectory,
    }

    fn fixture(name: &str) -> Fixture {
        let directory = TempDirectory::new(name);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        Fixture {
            repository: SqliteCommunicationsRepository::new(database.clone()),
            database,
            _directory: directory,
        }
    }

    fn configuration(
        kind: ConnectorKind,
        display_name: &str,
        credential_ref: Option<&str>,
    ) -> ConnectorConfig {
        ConnectorConfig {
            kind,
            enabled: true,
            display_name: Some(display_name.to_string()),
            public_config: json!({"apiBase": "https://example.test"}),
            credential_ref: credential_ref.map(str::to_string),
        }
    }

    #[test]
    fn round_trips_configuration_routing_deduplication_and_checkpoint() {
        let fixture = fixture("communications-sqlite-round-trip");
        let repository = &fixture.repository;
        let config = configuration(ConnectorKind::Telegram, "Support", Some("telegram/default"));
        repository
            .save_configuration(&config, "2026-07-18T01:00:00Z")
            .expect("save config");
        assert_eq!(
            repository
                .find_configuration(ConnectorKind::Telegram)
                .expect("find config"),
            Some(config.clone())
        );
        assert_eq!(
            repository.list_configurations().expect("list"),
            vec![config]
        );

        let routing = RoutingSettings::new("codex-cli", "D:/repo").expect("routing");
        repository
            .save_routing(&routing, "2026-07-18T01:01:00Z")
            .expect("save routing");
        assert_eq!(repository.load_routing().expect("routing"), Some(routing));

        let event =
            InboundEventIdentity::new(ConnectorKind::Telegram, "private-event-42").expect("event");
        assert!(repository
            .claim_event(&event, "2026-07-10T00:00:00Z")
            .expect("first claim"));
        assert!(!repository
            .claim_event(&event, "2026-07-18T00:00:00Z")
            .expect("duplicate claim"));
        assert_eq!(
            repository
                .cleanup_dedup_before("2026-07-11T00:00:00Z")
                .expect("cleanup"),
            1
        );
        assert!(repository
            .claim_event(&event, "2026-07-18T00:00:00Z")
            .expect("claim after cleanup"));

        let checkpoint = ConnectorCheckpoint::new(
            CheckpointKey::new(ConnectorKind::Telegram, "offset").expect("key"),
            "42",
        );
        repository
            .save_checkpoint(&checkpoint, "2026-07-18T01:02:00Z")
            .expect("save checkpoint");
        assert_eq!(
            repository
                .load_checkpoint(checkpoint.key())
                .expect("load checkpoint")
                .as_deref(),
            Some("42")
        );

        let event_hash: String = fixture
            .database
            .connection()
            .expect("connection")
            .query_row("SELECT event_hash FROM im_inbound_dedup", [], |row| {
                row.get(0)
            })
            .expect("event hash");
        assert!(!event_hash.contains("private-event-42"));
    }

    #[test]
    fn configuration_and_credential_reference_mutate_atomically_and_delete_cleanly() {
        let fixture = fixture("communications-sqlite-atomic-config");
        let repository = &fixture.repository;
        let original = configuration(ConnectorKind::DingTalk, "Original", None);
        repository
            .save_configuration(&original, "2026-07-18T02:00:00Z")
            .expect("original config");
        fixture
            .database
            .connection()
            .expect("connection")
            .execute_batch(
                r#"
                CREATE TRIGGER reject_im_credential_ref
                BEFORE INSERT ON im_credential_refs
                BEGIN
                    SELECT RAISE(ABORT, 'fixture rejection');
                END;
                "#,
            )
            .expect("trigger");

        let replacement = configuration(
            ConnectorKind::DingTalk,
            "Replacement",
            Some("dingtalk/default"),
        );
        assert!(repository
            .save_configuration(&replacement, "2026-07-18T02:01:00Z")
            .is_err());
        assert_eq!(
            repository
                .find_configuration(ConnectorKind::DingTalk)
                .expect("config after rollback"),
            Some(original)
        );

        fixture
            .database
            .connection()
            .expect("connection")
            .execute("DROP TRIGGER reject_im_credential_ref", [])
            .expect("drop trigger");
        repository
            .save_configuration(&replacement, "2026-07-18T02:02:00Z")
            .expect("replacement");
        let without_credential = ConnectorConfig {
            credential_ref: None,
            ..replacement
        };
        repository
            .save_configuration(&without_credential, "2026-07-18T02:03:00Z")
            .expect("delete credential reference");
        let count: i64 = fixture
            .database
            .connection()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM im_credential_refs", [], |row| {
                row.get(0)
            })
            .expect("credential count");
        assert_eq!(count, 0);
    }

    #[test]
    fn bindings_hash_external_ids_support_scoped_reset_and_cascade_on_session_delete() {
        let fixture = fixture("communications-sqlite-bindings");
        let connection = fixture.database.connection().expect("connection");
        for session_id in ["session-telegram", "session-feishu"] {
            connection
                .execute(
                    r#"INSERT INTO sessions
                       (id, title, agent_id, interaction_mode, lifecycle_state,
                        pinned, archived, created_at, updated_at)
                       VALUES (?1, ?1, 'codex-cli', 'interactive', 'idle', 0, 0, ?2, ?2)"#,
                    params![session_id, "2026-07-18T03:00:00Z"],
                )
                .expect("session");
        }
        drop(connection);

        let telegram_key =
            ChatBindingKey::new(ConnectorKind::Telegram, "private-chat-telegram").expect("key");
        let feishu_key =
            ChatBindingKey::new(ConnectorKind::Feishu, "private-chat-feishu").expect("key");
        fixture
            .repository
            .save_binding(
                &ChatBinding::new(telegram_key.clone(), "session-telegram").expect("binding"),
                "2026-07-18T03:01:00Z",
            )
            .expect("telegram binding");
        fixture
            .repository
            .save_binding(
                &ChatBinding::new(feishu_key.clone(), "session-feishu").expect("binding"),
                "2026-07-18T03:01:00Z",
            )
            .expect("feishu binding");

        let stored_hash: String = fixture
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT external_chat_hash FROM im_session_bindings WHERE connector = 'telegram'",
                [],
                |row| row.get(0),
            )
            .expect("stored hash");
        assert!(!stored_hash.contains("private-chat-telegram"));
        assert_eq!(
            fixture
                .repository
                .reset_bindings(Some(ConnectorKind::Telegram))
                .expect("reset"),
            1
        );
        assert!(fixture
            .repository
            .find_binding(&telegram_key)
            .expect("telegram lookup")
            .is_none());
        assert_eq!(
            fixture
                .repository
                .find_binding(&feishu_key)
                .expect("feishu lookup")
                .as_deref(),
            Some("session-feishu")
        );

        fixture
            .database
            .connection()
            .expect("connection")
            .execute("DELETE FROM sessions WHERE id = 'session-feishu'", [])
            .expect("delete session");
        assert!(fixture
            .repository
            .find_binding(&feishu_key)
            .expect("binding after delete")
            .is_none());
    }
}
