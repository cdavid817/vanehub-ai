// See `persistence_schema.rs` for what lands with which task group.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapters for connector subjects and their versioned definitions.

use crate::contexts::tooling::connectors::application::{
    ConnectorDefinitionRepository, ConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::domain::{
    decide_connector_definition, ConnectorDefinitionDigest, ConnectorDefinitionOutcome,
    ConnectorDefinitionRevision, ConnectorGlobalId, ConnectorSnapshotRef, ConnectorSubject,
    OwnerExtensionId,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

use super::is_foreign_key_violation;

pub(crate) struct SqliteConnectorSubjectRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteConnectorSubjectRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

fn read_subject(
    connector: &ConnectorGlobalId,
    row: (String, String),
) -> Result<ConnectorSubject, String> {
    let (owner, first_seen_at) = row;
    Ok(ConnectorSubject {
        connector: connector.clone(),
        owner_extension: OwnerExtensionId::parse(&owner)
            .map_err(|error| error.code().to_string())?,
        first_seen_at,
    })
}

impl ConnectorSubjectRepository for SqliteConnectorSubjectRepository {
    /// `DO NOTHING` rather than an upsert.
    ///
    /// `first_seen_at` is written once, and so is the owner: re-seeding is not a new sighting, and
    /// rewriting the owner would erase which package an operator has to uninstall.
    fn ensure(&self, subject: &ConnectorSubject) -> Result<(), String> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO connector_subjects \
                     (connector_global_id, owner_extension_id, first_seen_at) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT(connector_global_id) DO NOTHING",
                params![
                    subject.connector.as_str(),
                    subject.owner_extension.as_str(),
                    subject.first_seen_at,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn get(&self, connector: &ConnectorGlobalId) -> Result<Option<ConnectorSubject>, String> {
        let connection = self.connection()?;
        let row: Option<(String, String)> = connection
            .query_row(
                "SELECT owner_extension_id, first_seen_at FROM connector_subjects \
                 WHERE connector_global_id = ?1",
                params![connector.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        row.map(|row| read_subject(connector, row)).transpose()
    }

    fn all(&self) -> Result<Vec<ConnectorSubject>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT connector_global_id, owner_extension_id, first_seen_at \
                 FROM connector_subjects ORDER BY connector_global_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut subjects = Vec::new();
        for row in rows {
            let (connector, owner, first_seen_at) = row.map_err(|error| error.to_string())?;
            let connector =
                ConnectorGlobalId::parse(&connector).map_err(|error| error.code().to_string())?;
            subjects.push(read_subject(&connector, (owner, first_seen_at))?);
        }
        Ok(subjects)
    }
}

pub(crate) struct SqliteConnectorDefinitionRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteConnectorDefinitionRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

fn read_revision(
    connector: &ConnectorGlobalId,
    snapshot: &ConnectorSnapshotRef,
    row: (String, String),
) -> Result<ConnectorDefinitionRevision, String> {
    let (digest, recorded_at) = row;
    Ok(ConnectorDefinitionRevision {
        snapshot: snapshot.clone(),
        connector: connector.clone(),
        digest: ConnectorDefinitionDigest::parse(&digest)
            .map_err(|error| error.code().to_string())?,
        recorded_at,
    })
}

impl ConnectorDefinitionRepository for SqliteConnectorDefinitionRepository {
    fn record(
        &self,
        revision: &ConnectorDefinitionRevision,
    ) -> Result<ConnectorDefinitionOutcome, String> {
        let connection = self.connection()?;
        let transaction =
            begin_write_transaction(&connection).map_err(|error| error.to_string())?;

        // Read inside the transaction, so two installs racing on the same snapshot cannot both see
        // "unrecorded" and both insert.
        let recorded: Option<(String, String)> = transaction
            .query_row(
                "SELECT definition_digest, recorded_at FROM connector_definition_revisions \
                 WHERE snapshot_id = ?1 AND connector_global_id = ?2",
                params![revision.snapshot.as_str(), revision.connector.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let held = match recorded {
            Some(row) => Some(read_revision(&revision.connector, &revision.snapshot, row)?),
            None => None,
        };

        let outcome = decide_connector_definition(revision, held.as_ref());
        if matches!(outcome, ConnectorDefinitionOutcome::Recorded) {
            transaction
                .execute(
                    "INSERT INTO connector_definition_revisions \
                         (snapshot_id, connector_global_id, definition_digest, recorded_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        revision.snapshot.as_str(),
                        revision.connector.as_str(),
                        revision.digest.as_str(),
                        revision.recorded_at,
                    ],
                )
                .map_err(|error| {
                    if is_foreign_key_violation(&error) {
                        "unknown_connector_subject".to_string()
                    } else {
                        error.to_string()
                    }
                })?;
        }
        // A conflict commits nothing and the stored row is untouched.
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(outcome)
    }

    fn recorded(
        &self,
        connector: &ConnectorGlobalId,
        snapshot: &ConnectorSnapshotRef,
    ) -> Result<Option<ConnectorDefinitionRevision>, String> {
        let connection = self.connection()?;
        let row: Option<(String, String)> = connection
            .query_row(
                "SELECT definition_digest, recorded_at FROM connector_definition_revisions \
                 WHERE snapshot_id = ?1 AND connector_global_id = ?2",
                params![snapshot.as_str(), connector.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        row.map(|row| read_revision(connector, snapshot, row))
            .transpose()
    }

    fn revisions(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<Vec<ConnectorDefinitionRevision>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT snapshot_id, definition_digest, recorded_at \
                 FROM connector_definition_revisions WHERE connector_global_id = ?1 \
                 ORDER BY recorded_at DESC, snapshot_id DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![connector.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut revisions = Vec::new();
        for row in rows {
            let (snapshot, digest, recorded_at) = row.map_err(|error| error.to_string())?;
            let snapshot =
                ConnectorSnapshotRef::parse(&snapshot).map_err(|error| error.code().to_string())?;
            revisions.push(read_revision(connector, &snapshot, (digest, recorded_at))?);
        }
        Ok(revisions)
    }
}
