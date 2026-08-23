// See `persistence_schema.rs` for what lands with which task group.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapters for configured instances and their bindings.
//!
//! `label_key` is computed here from the display label and never taken from a caller. It is
//! derived, so accepting it would let a writer store a key that disagrees with the one the domain
//! computes — and the disagreement would surface as a duplicate label the uniqueness index
//! cheerfully accepted.
//!
//! `save` does not take a credential handle. Attaching one is `attach_credential`, so an ordinary
//! settings edit cannot clear or overwrite a credential by omitting it — the failure mode of a
//! single "save everything" method is a form that round-trips a `None` and silently detaches a
//! secret the user still needs.

use crate::contexts::tooling::connectors::application::{
    ConnectorBindingRepository, ConnectorInstanceRepository,
};
use crate::contexts::tooling::connectors::domain::{
    BindingId, ConnectorBinding, ConnectorBindingError, ConnectorGlobalId, ConnectorInstance,
    ConnectorInstanceError, ConnectorTarget, CredentialHandle, DisplayLabel, InstanceEdit,
    InstanceId, PublicConfiguration, ABSENT_REVISION,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use std::sync::Arc;

use super::is_foreign_key_violation;

pub(crate) struct SqliteConnectorInstanceRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteConnectorInstanceRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, ConnectorInstanceError> {
        self.database
            .connection()
            .map_err(|error| ConnectorInstanceError::Storage(error.to_string()))
    }
}

fn instance_error(error: rusqlite::Error) -> ConnectorInstanceError {
    if is_foreign_key_violation(&error) {
        ConnectorInstanceError::UnknownSubject
    } else {
        ConnectorInstanceError::Storage(error.to_string())
    }
}

const INSTANCE_COLUMNS: &str = "instance_id, connector_global_id, display_label, \
                                desired_enabled, public_configuration, credential_handle, \
                                revision, updated_at";

fn read_instance(row: &Row<'_>) -> Result<ConnectorInstance, rusqlite::Error> {
    let convert = |error: crate::contexts::tooling::connectors::domain::ConnectorIdentityError| {
        rusqlite::Error::InvalidColumnName(error.code().to_string())
    };
    let handle: Option<String> = row.get(5)?;
    Ok(ConnectorInstance {
        instance: InstanceId::parse(&row.get::<_, String>(0)?).map_err(convert)?,
        connector: ConnectorGlobalId::parse(&row.get::<_, String>(1)?).map_err(convert)?,
        display_label: DisplayLabel::parse(&row.get::<_, String>(2)?).map_err(convert)?,
        desired_enabled: row.get::<_, i64>(3)? != 0,
        configuration: PublicConfiguration::parse(&row.get::<_, String>(4)?).map_err(convert)?,
        credential: handle
            .map(|value| CredentialHandle::parse(&value))
            .transpose()
            .map_err(convert)?,
        revision: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

/// Reads the current revision and credential handle inside the caller's transaction.
fn current_state(
    transaction: &Transaction<'_>,
    instance: &InstanceId,
) -> Result<Option<(i64, Option<String>)>, ConnectorInstanceError> {
    transaction
        .query_row(
            "SELECT revision, credential_handle FROM connector_instances WHERE instance_id = ?1",
            params![instance.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(instance_error)
}

impl ConnectorInstanceRepository for SqliteConnectorInstanceRepository {
    fn get(
        &self,
        instance: &InstanceId,
    ) -> Result<Option<ConnectorInstance>, ConnectorInstanceError> {
        let connection = self.connection()?;
        connection
            .query_row(
                &format!(
                    "SELECT {INSTANCE_COLUMNS} FROM connector_instances WHERE instance_id = ?1"
                ),
                params![instance.as_str()],
                read_instance,
            )
            .optional()
            .map_err(instance_error)
    }

    fn for_connector(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<Vec<ConnectorInstance>, ConnectorInstanceError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {INSTANCE_COLUMNS} FROM connector_instances \
                 WHERE connector_global_id = ?1 ORDER BY label_key"
            ))
            .map_err(instance_error)?;
        let rows = statement
            .query_map(params![connector.as_str()], read_instance)
            .map_err(instance_error)?;

        let mut instances = Vec::new();
        for row in rows {
            instances.push(row.map_err(instance_error)?);
        }
        Ok(instances)
    }

    fn save(&self, edit: &InstanceEdit<'_>) -> Result<ConnectorInstance, ConnectorInstanceError> {
        let InstanceEdit {
            instance,
            connector,
            label,
            desired_enabled,
            configuration,
            expected_revision,
            at,
        } = *edit;
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| ConnectorInstanceError::Storage(error.to_string()))?;

        // Read inside the transaction, so two editors racing cannot both see the same revision.
        let current = current_state(&transaction, instance)?;
        let (current_revision, credential) = match current {
            Some((revision, handle)) => (revision, handle),
            None => (ABSENT_REVISION, None),
        };
        if current_revision != expected_revision {
            return Err(ConnectorInstanceError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        // Derived here, never accepted from a caller.
        let label_key = label.key();
        let colliding: Option<String> = transaction
            .query_row(
                "SELECT instance_id FROM connector_instances \
                 WHERE connector_global_id = ?1 AND label_key = ?2 AND instance_id <> ?3",
                params![connector.as_str(), label_key.as_str(), instance.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(instance_error)?;
        if let Some(existing) = colliding {
            return Err(ConnectorInstanceError::DuplicateLabel {
                existing: InstanceId::parse(&existing)
                    .map_err(|error| ConnectorInstanceError::Storage(error.code().to_string()))?,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO connector_instances \
                     (instance_id, connector_global_id, display_label, label_key, \
                      desired_enabled, public_configuration, credential_handle, revision, \
                      updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                 ON CONFLICT(instance_id) DO UPDATE SET \
                     display_label = excluded.display_label, \
                     label_key = excluded.label_key, \
                     desired_enabled = excluded.desired_enabled, \
                     public_configuration = excluded.public_configuration, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at",
                params![
                    instance.as_str(),
                    connector.as_str(),
                    label.as_str(),
                    label_key.as_str(),
                    i64::from(desired_enabled),
                    configuration.as_str(),
                    // Carried through unchanged. The upsert above does not list it, so an edit
                    // cannot detach a credential by omitting it.
                    credential,
                    revision,
                    at,
                ],
            )
            .map_err(instance_error)?;
        transaction
            .commit()
            .map_err(|error| ConnectorInstanceError::Storage(error.to_string()))?;

        Ok(ConnectorInstance {
            instance: instance.clone(),
            connector: connector.clone(),
            display_label: label.clone(),
            desired_enabled,
            configuration: configuration.clone(),
            credential: credential
                .map(|value| CredentialHandle::parse(&value))
                .transpose()
                .map_err(|error| ConnectorInstanceError::Storage(error.code().to_string()))?,
            revision,
            updated_at: at.to_string(),
        })
    }

    fn attach_credential(
        &self,
        instance: &InstanceId,
        credential: Option<&CredentialHandle>,
        expected_revision: i64,
        at: &str,
    ) -> Result<ConnectorInstance, ConnectorInstanceError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| ConnectorInstanceError::Storage(error.to_string()))?;

        let Some((current_revision, _)) = current_state(&transaction, instance)? else {
            // Attaching a credential to an instance that does not exist would leave a secret in
            // the store that nothing points at.
            return Err(ConnectorInstanceError::UnknownSubject);
        };
        if current_revision != expected_revision {
            return Err(ConnectorInstanceError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "UPDATE connector_instances \
                 SET credential_handle = ?1, revision = ?2, updated_at = ?3 \
                 WHERE instance_id = ?4",
                params![
                    credential.map(CredentialHandle::expose_for_storage),
                    revision,
                    at,
                    instance.as_str(),
                ],
            )
            .map_err(instance_error)?;
        transaction
            .commit()
            .map_err(|error| ConnectorInstanceError::Storage(error.to_string()))?;

        self.get(instance)?
            .ok_or_else(|| ConnectorInstanceError::Storage("instance vanished".to_string()))
    }
}

pub(crate) struct SqliteConnectorBindingRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteConnectorBindingRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, ConnectorBindingError> {
        self.database
            .connection()
            .map_err(|error| ConnectorBindingError::Storage(error.to_string()))
    }
}

fn binding_error(error: rusqlite::Error) -> ConnectorBindingError {
    if is_foreign_key_violation(&error) {
        ConnectorBindingError::UnknownInstance
    } else {
        ConnectorBindingError::Storage(error.to_string())
    }
}

fn read_binding(
    instance: &InstanceId,
    row: (String, String, String, i64, i64, String),
) -> Result<ConnectorBinding, ConnectorBindingError> {
    let (binding, kind, key, enabled, revision, updated_at) = row;
    let storage = |code: &str| ConnectorBindingError::Storage(code.to_string());
    Ok(ConnectorBinding {
        binding: BindingId::parse(&binding).map_err(|error| storage(error.code()))?,
        instance: instance.clone(),
        target: ConnectorTarget::parse(&kind, &key).map_err(|error| storage(error.code()))?,
        enabled: enabled != 0,
        revision,
        updated_at,
    })
}

impl ConnectorBindingRepository for SqliteConnectorBindingRepository {
    fn binding(
        &self,
        instance: &InstanceId,
        target: &ConnectorTarget,
    ) -> Result<Option<ConnectorBinding>, ConnectorBindingError> {
        let connection = self.connection()?;
        let row: Option<(String, String, String, i64, i64, String)> = connection
            .query_row(
                "SELECT binding_id, target_kind, target_key, enabled, revision, updated_at \
                 FROM connector_bindings \
                 WHERE instance_id = ?1 AND target_kind = ?2 AND target_key = ?3",
                params![instance.as_str(), target.kind().as_str(), target.key()],
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
            .optional()
            .map_err(binding_error)?;

        row.map(|row| read_binding(instance, row)).transpose()
    }

    fn bindings(
        &self,
        instance: &InstanceId,
    ) -> Result<Vec<ConnectorBinding>, ConnectorBindingError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT binding_id, target_kind, target_key, enabled, revision, updated_at \
                 FROM connector_bindings WHERE instance_id = ?1 \
                 ORDER BY target_kind, target_key",
            )
            .map_err(binding_error)?;
        let rows = statement
            .query_map(params![instance.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(binding_error)?;

        let mut bindings = Vec::new();
        for row in rows {
            bindings.push(read_binding(instance, row.map_err(binding_error)?)?);
        }
        Ok(bindings)
    }

    fn set(
        &self,
        binding: &BindingId,
        instance: &InstanceId,
        target: &ConnectorTarget,
        enabled: bool,
        expected_revision: i64,
        at: &str,
    ) -> Result<ConnectorBinding, ConnectorBindingError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| ConnectorBindingError::Storage(error.to_string()))?;

        // Keyed on the target, not the binding id: the identity of "this instance at this target"
        // is the pair, and a caller creating one has no id to read a revision from yet.
        let current: Option<(String, i64)> = transaction
            .query_row(
                "SELECT binding_id, revision FROM connector_bindings \
                 WHERE instance_id = ?1 AND target_kind = ?2 AND target_key = ?3",
                params![instance.as_str(), target.kind().as_str(), target.key()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(binding_error)?;
        let (binding_id, current_revision) = match &current {
            Some((existing, revision)) => (existing.clone(), *revision),
            None => (binding.as_str().to_string(), ABSENT_REVISION),
        };
        if current_revision != expected_revision {
            return Err(ConnectorBindingError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO connector_bindings \
                     (binding_id, instance_id, target_kind, target_key, enabled, revision, \
                      updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(instance_id, target_kind, target_key) DO UPDATE SET \
                     enabled = excluded.enabled, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at",
                params![
                    binding_id,
                    instance.as_str(),
                    target.kind().as_str(),
                    target.key(),
                    i64::from(enabled),
                    revision,
                    at,
                ],
            )
            .map_err(binding_error)?;
        transaction
            .commit()
            .map_err(|error| ConnectorBindingError::Storage(error.to_string()))?;

        Ok(ConnectorBinding {
            binding: BindingId::parse(&binding_id)
                .map_err(|error| ConnectorBindingError::Storage(error.code().to_string()))?,
            instance: instance.clone(),
            target: target.clone(),
            enabled,
            revision,
            updated_at: at.to_string(),
        })
    }
}
