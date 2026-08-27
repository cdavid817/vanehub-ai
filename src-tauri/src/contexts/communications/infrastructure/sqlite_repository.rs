use crate::contexts::communications::application::{
    CommunicationsApplicationError, CommunicationsRepository,
};
use crate::contexts::communications::domain::{
    BindingState, ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig,
    ConnectorKind, InboundEventIdentity, PairingIntent, RoutingSettings, SessionBinding,
    SessionConnectorAccess,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Row};
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

    fn connection(&self) -> Result<PooledSqlite, CommunicationsApplicationError> {
        self.database
            .connection()
            .map_err(|_| repository_unavailable())
    }

    #[cfg(test)]
    pub(crate) fn find_binding(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT session_id FROM im_session_bindings \
                 WHERE connector = ?1 AND external_chat_hash = ?2 AND state = 'active'",
                params![
                    key.connector().as_str(),
                    stable_hash(key.external_chat_id())
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_error)
    }

    #[cfg(test)]
    pub(crate) fn save_binding(
        &self,
        binding: &crate::contexts::communications::domain::ChatBinding,
        created_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO im_session_bindings
                   (connector, external_chat_hash, session_id, created_at, state,
                    completion_notifications, updated_at)
                   VALUES (?1, ?2, ?3, ?4, 'active', 0, ?4)
                   ON CONFLICT(connector, external_chat_hash) DO UPDATE SET
                     session_id = excluded.session_id,
                     state = 'active',
                     updated_at = excluded.updated_at"#,
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

    pub(crate) fn binding_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT connector, session_id, state, completion_notifications,
                          created_at, updated_at
                   FROM im_session_bindings
                   WHERE session_id = ?1
                   ORDER BY CASE state WHEN 'active' THEN 0 ELSE 1 END, created_at
                   LIMIT 1"#,
                [session_id],
                read_session_binding,
            )
            .optional()
            .map_err(sqlite_error)?
            .map(session_binding_from_row)
            .transpose()
    }

    pub(crate) fn binding_for_chat(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT connector, session_id, state, completion_notifications,
                          created_at, updated_at
                   FROM im_session_bindings
                   WHERE connector = ?1 AND external_chat_hash = ?2"#,
                params![
                    key.connector().as_str(),
                    stable_hash(key.external_chat_id())
                ],
                read_session_binding,
            )
            .optional()
            .map_err(sqlite_error)?
            .map(session_binding_from_row)
            .transpose()
    }

    pub(crate) fn session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT enabled, updated_at
                   FROM im_session_connector_access
                   WHERE session_id = ?1 AND connector = ?2"#,
                params![session_id, connector.as_str()],
                |row| {
                    Ok(SessionConnectorAccess {
                        session_id: session_id.to_string(),
                        connector,
                        enabled: row.get::<_, i64>(0)? != 0,
                        updated_at: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)
            .map(|access| {
                access.unwrap_or_else(|| SessionConnectorAccess::disabled(session_id, connector))
            })
    }

    pub(crate) fn set_session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let session_exists = connection
            .query_row("SELECT 1 FROM sessions WHERE id = ?1", [session_id], |_| {
                Ok(())
            })
            .optional()
            .map_err(sqlite_error)?
            .is_some();
        if !session_exists {
            return Err(CommunicationsApplicationError::user_visible(
                "im-session-not-found",
                "The selected session no longer exists.",
            ));
        }
        connection
            .execute(
                r#"INSERT INTO im_session_connector_access
                   (session_id, connector, enabled, updated_at)
                   VALUES (?1, ?2, ?3, ?4)
                   ON CONFLICT(session_id, connector) DO UPDATE SET
                     enabled = excluded.enabled,
                     updated_at = excluded.updated_at"#,
                params![session_id, connector.as_str(), enabled, updated_at],
            )
            .map_err(sqlite_error)?;
        Ok(SessionConnectorAccess {
            session_id: session_id.to_string(),
            connector,
            enabled,
            updated_at: updated_at.to_string(),
        })
    }

    pub(crate) fn set_binding_state(
        &self,
        session_id: &str,
        state: BindingState,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE im_session_bindings SET state = ?2, updated_at = ?3 WHERE session_id = ?1",
                params![session_id, state.as_str(), updated_at],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(CommunicationsApplicationError::user_visible(
                "im-binding-not-found",
                "This session has no IM binding.",
            ));
        }
        self.binding_for_session(session_id)?
            .ok_or_else(|| CommunicationsApplicationError::failure("im-binding-update-lost"))
    }

    pub(crate) fn set_completion_notifications(
        &self,
        session_id: &str,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let changed = self.connection()?.execute(
            "UPDATE im_session_bindings SET completion_notifications = ?2, updated_at = ?3 WHERE session_id = ?1",
            params![session_id, enabled, updated_at],
        ).map_err(sqlite_error)?;
        if changed == 0 {
            return Err(CommunicationsApplicationError::user_visible(
                "im-binding-not-found",
                "This session has no IM binding.",
            ));
        }
        self.binding_for_session(session_id)?
            .ok_or_else(|| CommunicationsApplicationError::failure("im-binding-update-lost"))
    }

    pub(crate) fn remove_session_binding(
        &self,
        session_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_session_bindings WHERE session_id = ?1",
                [session_id],
            )
            .map(|changed| changed > 0)
            .map_err(sqlite_error)
    }

    pub(crate) fn claim_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
        delivered_at: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "INSERT OR IGNORE INTO im_notification_deliveries \
                 (message_id, session_id, connector, delivered_at) VALUES (?1, ?2, ?3, ?4)",
                params![message_id, session_id, connector.as_str(), delivered_at],
            )
            .map(|changed| changed > 0)
            .map_err(sqlite_error)
    }

    pub(crate) fn release_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_notification_deliveries \
                 WHERE message_id = ?1 AND session_id = ?2 AND connector = ?3",
                params![message_id, session_id, connector.as_str()],
            )
            .map(|_| ())
            .map_err(sqlite_error)
    }

    pub(crate) fn save_pairing_intent(
        &self,
        intent: &PairingIntent,
    ) -> Result<(), CommunicationsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        transaction
            .execute(
                "DELETE FROM im_pairing_intents WHERE session_id = ?1 AND connector = ?2",
                params![intent.session_id, intent.connector.as_str()],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute(
                r#"INSERT INTO im_pairing_intents
                   (id, connector, session_id, code_hash, salt, expires_at, created_at,
                    replace_existing)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                params![
                    intent.id,
                    intent.connector.as_str(),
                    intent.session_id,
                    intent.code_hash,
                    intent.salt,
                    intent.expires_at,
                    intent.created_at,
                    intent.replace_existing,
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)
    }

    pub(crate) fn pairing_intents(
        &self,
        connector: ConnectorKind,
        now: &str,
    ) -> Result<Vec<PairingIntent>, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT id, connector, session_id, code_hash, salt, expires_at, created_at,
                          replace_existing
                   FROM im_pairing_intents
                   WHERE connector = ?1 AND expires_at > ?2
                   ORDER BY created_at DESC LIMIT 16"#,
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![connector.as_str(), now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)? != 0,
                ))
            })
            .map_err(sqlite_error)?;
        let mut intents = Vec::new();
        for row in rows {
            let (
                id,
                connector,
                session_id,
                code_hash,
                salt,
                expires_at,
                created_at,
                replace_existing,
            ) = row.map_err(sqlite_error)?;
            let connector = ConnectorKind::parse(&connector).ok_or_else(invalid_repository_data)?;
            intents.push(PairingIntent::new(
                id,
                connector,
                session_id,
                (code_hash, salt),
                (expires_at, created_at),
                replace_existing,
            )?);
        }
        Ok(intents)
    }

    pub(crate) fn consume_pairing_intent(
        &self,
        intent_id: &str,
        key: &ChatBindingKey,
        now: &str,
        replace: bool,
        delivery_credential_ref: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        let intent = transaction
            .query_row(
                "SELECT connector, session_id, expires_at FROM im_pairing_intents WHERE id = ?1",
                [intent_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)?
            .ok_or_else(|| {
                CommunicationsApplicationError::user_visible(
                    "im-pairing-invalid",
                    "The pairing code is invalid or has already been used.",
                )
            })?;
        if intent.0 != key.connector().as_str() || intent.2.as_str() <= now {
            return Err(CommunicationsApplicationError::user_visible(
                "im-pairing-invalid",
                "The pairing code is invalid or expired.",
            ));
        }
        let external_hash = stable_hash(key.external_chat_id());
        let chat_conflict: Option<String> = transaction.query_row(
            "SELECT session_id FROM im_session_bindings WHERE connector = ?1 AND external_chat_hash = ?2",
            params![key.connector().as_str(), external_hash], |row| row.get(0),
        ).optional().map_err(sqlite_error)?;
        let session_conflict: Option<String> = transaction.query_row(
            "SELECT external_chat_hash FROM im_session_bindings WHERE session_id = ?1 AND state = 'active' LIMIT 1",
            [&intent.1], |row| row.get(0),
        ).optional().map_err(sqlite_error)?;
        if !replace
            && (chat_conflict.as_deref().is_some_and(|id| id != intent.1)
                || session_conflict
                    .as_deref()
                    .is_some_and(|hash| hash != external_hash))
        {
            return Err(CommunicationsApplicationError::user_visible(
                "im-binding-replacement-required",
                "Confirm replacement before changing this IM binding.",
            ));
        }
        if replace {
            transaction.execute(
                "DELETE FROM im_session_bindings WHERE session_id = ?1 OR (connector = ?2 AND external_chat_hash = ?3)",
                params![intent.1, key.connector().as_str(), external_hash],
            ).map_err(sqlite_error)?;
        }
        transaction.execute(
            r#"INSERT INTO im_session_bindings
               (connector, external_chat_hash, session_id, created_at, state,
                completion_notifications, updated_at, delivery_credential_ref)
               VALUES (?1, ?2, ?3, ?4, 'active', 0, ?4, ?5)
               ON CONFLICT(connector, external_chat_hash) DO UPDATE SET
                 session_id = excluded.session_id, state = 'active', updated_at = excluded.updated_at,
                 delivery_credential_ref = excluded.delivery_credential_ref"#,
            params![key.connector().as_str(), external_hash, intent.1, now, delivery_credential_ref],
        ).map_err(sqlite_error)?;
        transaction
            .execute("DELETE FROM im_pairing_intents WHERE id = ?1", [intent_id])
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        self.binding_for_session(&intent.1)?
            .ok_or_else(|| CommunicationsApplicationError::failure("im-binding-create-lost"))
    }

    pub(crate) fn binding_delivery_reference(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT delivery_credential_ref FROM im_session_bindings WHERE session_id = ?1 LIMIT 1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_error)
            .map(Option::flatten)
    }

    pub(crate) fn replacement_delivery_references(
        &self,
        session_id: &str,
        key: &ChatBindingKey,
    ) -> Result<Vec<String>, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT delivery_credential_ref FROM im_session_bindings \
                 WHERE delivery_credential_ref IS NOT NULL AND \
                 (session_id = ?1 OR (connector = ?2 AND external_chat_hash = ?3))",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(
                params![
                    session_id,
                    key.connector().as_str(),
                    stable_hash(key.external_chat_id())
                ],
                |row| row.get(0),
            )
            .map_err(sqlite_error)?;
        let mut references = Vec::new();
        for row in rows {
            references.push(row.map_err(sqlite_error)?);
        }
        Ok(references)
    }

    pub(crate) fn cancel_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_pairing_intents WHERE session_id = ?1 AND connector = ?2",
                params![session_id, connector.as_str()],
            )
            .map(|changed| changed > 0)
            .map_err(sqlite_error)
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

    pub(crate) fn touch_wechat_reply_context(
        &self,
        chat_hash: &str,
        credential_account: &str,
        last_used_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO im_wechat_reply_contexts
                   (chat_hash, credential_account, last_used_at)
                   VALUES (?1, ?2, ?3)
                   ON CONFLICT(chat_hash) DO UPDATE SET
                     credential_account = excluded.credential_account,
                     last_used_at = excluded.last_used_at"#,
                params![chat_hash, credential_account, last_used_at],
            )
            .map(|_| ())
            .map_err(sqlite_error)
    }

    pub(crate) fn expired_wechat_reply_contexts(
        &self,
        cutoff: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, String)>, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT chat_hash, credential_account, last_used_at FROM im_wechat_reply_contexts \
                 WHERE last_used_at < ?1 ORDER BY last_used_at LIMIT ?2",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![cutoff, limit as i64], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(sqlite_error)?;
        let mut contexts = Vec::new();
        for row in rows {
            contexts.push(row.map_err(sqlite_error)?);
        }
        Ok(contexts)
    }

    pub(crate) fn wechat_reply_contexts(
        &self,
        limit: usize,
    ) -> Result<Vec<(String, String)>, CommunicationsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT chat_hash, credential_account FROM im_wechat_reply_contexts \
                 ORDER BY chat_hash LIMIT ?1",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(sqlite_error)?;
        let mut contexts = Vec::new();
        for row in rows {
            contexts.push(row.map_err(sqlite_error)?);
        }
        Ok(contexts)
    }

    pub(crate) fn delete_wechat_reply_context(
        &self,
        chat_hash: &str,
        credential_account: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_wechat_reply_contexts \
                 WHERE chat_hash = ?1 AND credential_account = ?2",
                params![chat_hash, credential_account],
            )
            .map(|changed| changed == 1)
            .map_err(sqlite_error)
    }

    pub(crate) fn delete_expired_wechat_reply_context(
        &self,
        chat_hash: &str,
        credential_account: &str,
        cutoff: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_wechat_reply_contexts \
                 WHERE chat_hash = ?1 AND credential_account = ?2 AND last_used_at < ?3",
                params![chat_hash, credential_account, cutoff],
            )
            .map(|changed| changed == 1)
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

    fn session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::session_access(self, session_id, connector)
    }

    fn set_session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::set_session_access(
            self, session_id, connector, enabled, updated_at,
        )
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

    fn delete_configuration(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        transaction
            .execute(
                "DELETE FROM im_credential_refs WHERE connector = ?1",
                [kind.as_str()],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "DELETE FROM im_connector_configs WHERE connector = ?1",
                [kind.as_str()],
            )
            .map_err(sqlite_error)?;
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

    fn cleanup_dedup_before(
        &self,
        cutoff: &str,
        limit: usize,
    ) -> Result<usize, CommunicationsApplicationError> {
        self.connection()?
            .execute(
                "DELETE FROM im_inbound_dedup WHERE rowid IN (\
                   SELECT rowid FROM im_inbound_dedup \
                   WHERE received_at < ?1 ORDER BY received_at, rowid LIMIT ?2\
                 )",
                params![cutoff, i64::try_from(limit).unwrap_or(i64::MAX)],
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

    fn binding_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::binding_for_session(self, session_id)
    }

    fn binding_for_chat(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::binding_for_chat(self, key)
    }

    fn save_pairing_intent(
        &self,
        intent: &PairingIntent,
    ) -> Result<(), CommunicationsApplicationError> {
        SqliteCommunicationsRepository::save_pairing_intent(self, intent)
    }

    fn pairing_intents(
        &self,
        connector: ConnectorKind,
        now: &str,
    ) -> Result<Vec<PairingIntent>, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::pairing_intents(self, connector, now)
    }

    fn consume_pairing_intent(
        &self,
        intent_id: &str,
        key: &ChatBindingKey,
        now: &str,
        replace: bool,
        delivery_credential_ref: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::consume_pairing_intent(
            self,
            intent_id,
            key,
            now,
            replace,
            delivery_credential_ref,
        )
    }

    fn binding_delivery_reference(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::binding_delivery_reference(self, session_id)
    }

    fn replacement_delivery_references(
        &self,
        session_id: &str,
        key: &ChatBindingKey,
    ) -> Result<Vec<String>, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::replacement_delivery_references(self, session_id, key)
    }

    fn cancel_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<bool, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::cancel_pairing(self, session_id, connector)
    }

    fn set_binding_state(
        &self,
        session_id: &str,
        state: BindingState,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::set_binding_state(self, session_id, state, updated_at)
    }

    fn set_completion_notifications(
        &self,
        session_id: &str,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::set_completion_notifications(
            self, session_id, enabled, updated_at,
        )
    }

    fn remove_session_binding(
        &self,
        session_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::remove_session_binding(self, session_id)
    }

    fn claim_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
        delivered_at: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        SqliteCommunicationsRepository::claim_notification_delivery(
            self,
            message_id,
            session_id,
            connector,
            delivered_at,
        )
    }

    fn release_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        SqliteCommunicationsRepository::release_notification_delivery(
            self, message_id, session_id, connector,
        )
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

type SessionBindingRow = (String, String, String, bool, String, String);

fn read_session_binding(row: &Row<'_>) -> rusqlite::Result<SessionBindingRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get::<_, i64>(3)? != 0,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn session_binding_from_row(
    row: SessionBindingRow,
) -> Result<SessionBinding, CommunicationsApplicationError> {
    Ok(SessionBinding {
        connector: ConnectorKind::parse(&row.0).ok_or_else(invalid_repository_data)?,
        session_id: row.1,
        state: BindingState::parse(&row.2).ok_or_else(invalid_repository_data)?,
        completion_notifications: row.3,
        created_at: row.4,
        updated_at: row.5,
    })
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
    use crate::contexts::communications::domain::ChatBinding;
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

    fn insert_session(fixture: &Fixture, session_id: &str) {
        fixture
            .database
            .connection()
            .expect("connection")
            .execute(
                r#"INSERT INTO sessions
               (id, title, agent_id, interaction_mode, lifecycle_state,
                pinned, archived, created_at, updated_at)
               VALUES (?1, ?1, 'codex-cli', 'interactive', 'idle', 0, 0, ?2, ?2)"#,
                params![session_id, "2026-08-12T00:00:00Z"],
            )
            .expect("session");
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
                .cleanup_dedup_before("2026-07-11T00:00:00Z", 1)
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
    fn dedup_cleanup_never_exceeds_the_requested_batch() {
        let fixture = fixture("communications-dedup-batch");
        let repository = &fixture.repository;
        for index in 0..3 {
            let event = InboundEventIdentity::new(
                ConnectorKind::Telegram,
                format!("private-event-{index}"),
            )
            .expect("event");
            assert!(repository
                .claim_event(&event, "2026-07-10T00:00:00Z")
                .expect("claim"));
        }

        assert_eq!(
            repository
                .cleanup_dedup_before("2026-07-11T00:00:00Z", 2)
                .expect("first batch"),
            2
        );
        let remaining: i64 = fixture
            .database
            .connection()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM im_inbound_dedup", [], |row| {
                row.get(0)
            })
            .expect("remaining rows");
        assert_eq!(remaining, 1);
        assert_eq!(
            repository
                .cleanup_dedup_before("2026-07-11T00:00:00Z", 2)
                .expect("second batch"),
            1
        );
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

        let default_access = fixture
            .repository
            .session_access("session-feishu", ConnectorKind::Feishu)
            .expect("default access");
        assert!(!default_access.enabled);
        let enabled_access = fixture
            .repository
            .set_session_access(
                "session-feishu",
                ConnectorKind::Feishu,
                true,
                "2026-07-18T03:00:30Z",
            )
            .expect("enable access");
        assert!(enabled_access.enabled);
        let restarted_repository = SqliteCommunicationsRepository::new(fixture.database.clone());
        assert!(
            restarted_repository
                .session_access("session-feishu", ConnectorKind::Feishu)
                .expect("access after repository restart")
                .enabled
        );
        assert!(
            !fixture
                .repository
                .session_access("session-telegram", ConnectorKind::Feishu)
                .expect("isolated access")
                .enabled
        );

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
        let access_rows: i64 = fixture
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM im_session_connector_access WHERE session_id = 'session-feishu'",
                [],
                |row| row.get(0),
            )
            .expect("access after delete");
        assert_eq!(access_rows, 0);
    }

    #[test]
    fn pairing_is_connector_scoped_single_use_and_enforces_replacement() {
        let fixture = fixture("communications-pairing-consumption");
        insert_session(&fixture, "session-1");
        insert_session(&fixture, "session-2");
        let first = PairingIntent::new(
            "pair-1",
            ConnectorKind::Telegram,
            "session-1",
            ("hash-1", "salt-1"),
            ("2026-08-12T00:10:00Z", "2026-08-12T00:00:00Z"),
            false,
        )
        .expect("intent");
        fixture
            .repository
            .save_pairing_intent(&first)
            .expect("save intent");
        assert!(fixture
            .repository
            .pairing_intents(ConnectorKind::Feishu, "2026-08-12T00:01:00Z")
            .expect("other connector")
            .is_empty());
        let key = ChatBindingKey::new(ConnectorKind::Telegram, "private-chat-1").expect("key");
        let binding = fixture
            .repository
            .consume_pairing_intent("pair-1", &key, "2026-08-12T00:01:00Z", false, "ref-1")
            .expect("consume");
        assert_eq!(binding.session_id, "session-1");
        assert!(fixture
            .repository
            .consume_pairing_intent("pair-1", &key, "2026-08-12T00:02:00Z", false, "ref-1")
            .is_err());

        let second = PairingIntent::new(
            "pair-2",
            ConnectorKind::Telegram,
            "session-2",
            ("hash-2", "salt-2"),
            ("2026-08-12T00:10:00Z", "2026-08-12T00:02:00Z"),
            true,
        )
        .expect("intent");
        fixture
            .repository
            .save_pairing_intent(&second)
            .expect("save second");
        let error = fixture
            .repository
            .consume_pairing_intent("pair-2", &key, "2026-08-12T00:03:00Z", false, "ref-2")
            .expect_err("replacement required");
        assert_eq!(error.safe_code(), "im-binding-replacement-required");
        let replaced = fixture
            .repository
            .consume_pairing_intent("pair-2", &key, "2026-08-12T00:03:00Z", true, "ref-2")
            .expect("replace");
        assert_eq!(replaced.session_id, "session-2");
    }

    #[test]
    fn expired_pairing_cannot_bind_and_binding_controls_round_trip() {
        let fixture = fixture("communications-pairing-expiry-controls");
        insert_session(&fixture, "session-1");
        let expired = PairingIntent::new(
            "pair-expired",
            ConnectorKind::Telegram,
            "session-1",
            ("hash", "salt"),
            ("2026-08-12T00:01:00Z", "2026-08-12T00:00:00Z"),
            false,
        )
        .expect("intent");
        fixture
            .repository
            .save_pairing_intent(&expired)
            .expect("save");
        assert!(fixture
            .repository
            .pairing_intents(ConnectorKind::Telegram, "2026-08-12T00:02:00Z")
            .expect("active intents")
            .is_empty());
        let key = ChatBindingKey::new(ConnectorKind::Telegram, "private-chat").expect("key");
        assert!(fixture
            .repository
            .consume_pairing_intent(
                "pair-expired",
                &key,
                "2026-08-12T00:02:00Z",
                false,
                "ref-expired",
            )
            .is_err());

        fixture
            .repository
            .save_binding(
                &ChatBinding::new(key.clone(), "session-1").expect("binding"),
                "2026-08-12T00:03:00Z",
            )
            .expect("save binding");
        let paused = fixture
            .repository
            .set_binding_state("session-1", BindingState::Paused, "2026-08-12T00:04:00Z")
            .expect("pause");
        assert!(!paused.is_active());
        assert!(fixture
            .repository
            .find_binding(&key)
            .expect("active lookup")
            .is_none());
        let notified = fixture
            .repository
            .set_completion_notifications("session-1", true, "2026-08-12T00:05:00Z")
            .expect("notifications");
        assert!(notified.completion_notifications);
        assert!(fixture
            .repository
            .remove_session_binding("session-1")
            .expect("remove"));
        assert!(fixture
            .repository
            .binding_for_session("session-1")
            .expect("lookup")
            .is_none());
    }

    #[test]
    fn binding_mutations_preserve_session_execution_and_history_continuity() {
        let fixture = fixture("communications-binding-session-continuity");
        insert_session(&fixture, "session-continuity");
        fixture
            .database
            .connection()
            .expect("connection")
            .execute_batch(
                r#"
                UPDATE sessions
                   SET project_path = 'D:/project', worktree_path = 'D:/project-feature',
                       worktree_name = 'feature', worktree_branch = 'feat/im',
                       runtime_session_id = 'provider-session-7', history_revision = 7,
                       chat_preferences = '{"permissionMode":"agent","providerId":"openai","modelId":"gpt-continuity","reasoningDepth":"high","streaming":true,"thinking":true,"longContext":false}'
                 WHERE id = 'session-continuity';
                INSERT INTO messages
                    (id, session_id, role, status, content, created_at, updated_at, session_sequence)
                VALUES
                    ('message-continuity', 'session-continuity', 'assistant', 'completed',
                     'preserved history', '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z', 1);
                "#,
            )
            .expect("seed continuity");
        let snapshot = || {
            fixture
                .database
                .connection()
                .expect("connection")
                .query_row(
                    r#"SELECT agent_id || '|' || COALESCE(project_path, '') || '|' ||
                              COALESCE(worktree_path, '') || '|' || COALESCE(runtime_session_id, '') || '|' ||
                              COALESCE(chat_preferences, '') || '|' || CAST(history_revision AS TEXT) || '|' ||
                              COALESCE((SELECT group_concat(role || ':' || content, ',')
                                          FROM messages WHERE session_id = sessions.id), '')
                         FROM sessions WHERE id = 'session-continuity'"#,
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("session snapshot")
        };
        let before = snapshot();
        let key = ChatBindingKey::new(ConnectorKind::Telegram, "private-chat").expect("key");
        fixture
            .repository
            .save_binding(
                &ChatBinding::new(key, "session-continuity").expect("binding"),
                "2026-08-12T00:01:00Z",
            )
            .expect("save binding");
        fixture
            .repository
            .set_binding_state(
                "session-continuity",
                BindingState::Paused,
                "2026-08-12T00:02:00Z",
            )
            .expect("pause");
        fixture
            .repository
            .set_completion_notifications("session-continuity", true, "2026-08-12T00:03:00Z")
            .expect("notifications");
        fixture
            .repository
            .remove_session_binding("session-continuity")
            .expect("remove");

        assert_eq!(snapshot(), before);
    }

    #[test]
    fn wechat_context_retention_is_bounded_restart_safe_and_does_not_delete_refreshed_rows() {
        let fixture = fixture("communications-wechat-context-retention");
        for index in 0..3 {
            fixture
                .repository
                .touch_wechat_reply_context(
                    &format!("chat-{index}"),
                    &format!("account-{index}"),
                    "2026-01-01T00:00:00Z",
                )
                .expect("touch context");
        }

        let restarted = SqliteCommunicationsRepository::new(fixture.database.clone());
        let expired = restarted
            .expired_wechat_reply_contexts("2026-02-01T00:00:00Z", 2)
            .expect("expired contexts");
        assert_eq!(expired.len(), 2);

        let (chat_hash, account, _) = &expired[0];
        restarted
            .touch_wechat_reply_context(chat_hash, account, "2026-03-01T00:00:00Z")
            .expect("refresh context");
        assert!(!restarted
            .delete_expired_wechat_reply_context(chat_hash, account, "2026-02-01T00:00:00Z",)
            .expect("stale retention delete"));

        let (stale_hash, stale_account, _) = &expired[1];
        assert!(restarted
            .delete_expired_wechat_reply_context(stale_hash, stale_account, "2026-02-01T00:00:00Z",)
            .expect("retention delete"));
        let remaining: i64 = fixture
            .database
            .connection()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM im_wechat_reply_contexts", [], |row| {
                row.get(0)
            })
            .expect("context count");
        assert_eq!(remaining, 2);
    }
}
