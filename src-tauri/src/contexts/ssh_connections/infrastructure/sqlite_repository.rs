use super::super::application::{SshConnectionError, SshConnectionRepository};
use super::super::domain::{
    SshAuthMode, SshConnectionProfile, SshConnectionTestStatus, SshHostTrustMetadata,
};
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Connection, Row};

pub(crate) fn apply_schema(connection: &rusqlite::Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS ssh_connections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            user TEXT NOT NULL,
            default_path TEXT NOT NULL,
            auth_mode TEXT NOT NULL,
            key_path TEXT,
            credential_ref TEXT,
            revision INTEGER NOT NULL DEFAULT 1,
            test_status TEXT NOT NULL DEFAULT 'not-tested',
            last_connected_at TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_ssh_connections_updated
            ON ssh_connections(updated_at DESC);
        "#,
    )?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct SqliteSshConnectionRepository {
    database: NativeDatabase,
}

impl SqliteSshConnectionRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl SshConnectionRepository for SqliteSshConnectionRepository {
    fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let mut statement = connection
            .prepare(
                "SELECT connections.id, connections.name, connections.host, connections.port,
                        connections.user, connections.default_path, connections.auth_mode,
                        connections.key_path, connections.credential_ref, connections.revision,
                        connections.test_status, connections.last_connected_at,
                        connections.last_error, connections.created_at, connections.updated_at,
                        trust.host, trust.port, trust.algorithm, trust.fingerprint, trust.confirmed_at
                 FROM ssh_connections AS connections
                 LEFT JOIN ssh_host_trust AS trust ON trust.connection_id = connections.id
                 ORDER BY connections.updated_at DESC, connections.name ASC",
            )
            .map_err(sql_error)?;
        let profiles = statement
            .query_map([], read_profile)
            .map_err(sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_error)?;
        Ok(profiles)
    }

    fn find(&self, id: &str) -> Result<Option<SshConnectionProfile>, SshConnectionError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let result = connection.query_row(
            "SELECT connections.id, connections.name, connections.host, connections.port,
                    connections.user, connections.default_path, connections.auth_mode,
                    connections.key_path, connections.credential_ref, connections.revision,
                    connections.test_status, connections.last_connected_at,
                    connections.last_error, connections.created_at, connections.updated_at,
                    trust.host, trust.port, trust.algorithm, trust.fingerprint, trust.confirmed_at
             FROM ssh_connections AS connections
             LEFT JOIN ssh_host_trust AS trust ON trust.connection_id = connections.id
             WHERE connections.id = ?1",
            params![id],
            read_profile,
        );
        match result {
            Ok(profile) => Ok(Some(profile)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(sql_error(error)),
        }
    }

    fn insert(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
        insert_profile(&transaction, profile).map_err(sql_error)?;
        sync_host_trust(&transaction, profile).map_err(sql_error)?;
        transaction.commit().map_err(sql_error)?;
        Ok(())
    }

    fn update(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
        transaction
            .execute(
                r#"
                UPDATE ssh_connections SET
                    name = ?2,
                    host = ?3,
                    port = ?4,
                    user = ?5,
                    default_path = ?6,
                    auth_mode = ?7,
                    key_path = ?8,
                    credential_ref = ?9,
                    revision = ?10,
                    test_status = ?11,
                    last_connected_at = ?12,
                    last_error = ?13,
                    created_at = ?14,
                    updated_at = ?15
                WHERE id = ?1
                "#,
                params![
                    profile.id,
                    profile.name,
                    profile.host,
                    profile.port,
                    profile.user,
                    profile.default_path,
                    profile.auth_mode.as_str(),
                    profile.key_path,
                    profile.credential_ref,
                    profile.revision,
                    profile.test_status.as_str(),
                    profile.last_connected_at,
                    profile.last_error,
                    profile.created_at,
                    profile.updated_at,
                ],
            )
            .map_err(sql_error)?;
        sync_host_trust(&transaction, profile).map_err(sql_error)?;
        transaction.commit().map_err(sql_error)?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .execute("DELETE FROM ssh_connections WHERE id = ?1", params![id])
            .map_err(sql_error)?;
        Ok(())
    }
}

fn insert_profile(
    connection: &Connection,
    profile: &SshConnectionProfile,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        r#"
        INSERT INTO ssh_connections
            (id, name, host, port, user, default_path, auth_mode, key_path, credential_ref,
             revision, test_status, last_connected_at, last_error, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        params![
            profile.id,
            profile.name,
            profile.host,
            profile.port,
            profile.user,
            profile.default_path,
            profile.auth_mode.as_str(),
            profile.key_path,
            profile.credential_ref,
            profile.revision,
            profile.test_status.as_str(),
            profile.last_connected_at,
            profile.last_error,
            profile.created_at,
            profile.updated_at,
        ],
    )?;
    Ok(())
}

fn sync_host_trust(
    connection: &Connection,
    profile: &SshConnectionProfile,
) -> Result<(), rusqlite::Error> {
    match &profile.host_trust {
        Some(trust) => {
            connection.execute(
                r#"
                INSERT INTO ssh_host_trust
                    (connection_id, host, port, algorithm, fingerprint, confirmed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(connection_id) DO UPDATE SET
                    host = excluded.host,
                    port = excluded.port,
                    algorithm = excluded.algorithm,
                    fingerprint = excluded.fingerprint,
                    confirmed_at = excluded.confirmed_at
                "#,
                params![
                    profile.id,
                    trust.host,
                    trust.port,
                    trust.algorithm,
                    trust.fingerprint,
                    trust.confirmed_at
                ],
            )?;
        }
        None => {
            connection.execute(
                "DELETE FROM ssh_host_trust WHERE connection_id = ?1",
                params![profile.id],
            )?;
        }
    }
    Ok(())
}

fn read_profile(row: &Row<'_>) -> Result<SshConnectionProfile, rusqlite::Error> {
    let auth_mode = SshAuthMode::parse(row.get::<_, String>(6)?.as_str()).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let test_status =
        SshConnectionTestStatus::parse(row.get::<_, String>(10)?.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                10,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let host_trust = row
        .get::<_, Option<String>>(15)?
        .map(|host| -> Result<SshHostTrustMetadata, rusqlite::Error> {
            Ok(SshHostTrustMetadata {
                host,
                port: row.get(16)?,
                algorithm: row.get(17)?,
                fingerprint: row.get(18)?,
                confirmed_at: row.get(19)?,
            })
        })
        .transpose()?;
    Ok(SshConnectionProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        host: row.get(2)?,
        port: row.get(3)?,
        user: row.get(4)?,
        default_path: row.get(5)?,
        auth_mode,
        key_path: row.get(7)?,
        credential_ref: row.get(8)?,
        revision: row.get(9)?,
        host_trust,
        test_status,
        last_connected_at: row.get(11)?,
        last_error: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn repository_error(error: DatabaseError) -> SshConnectionError {
    SshConnectionError::Repository(error.to_string())
}

fn sql_error(error: rusqlite::Error) -> SshConnectionError {
    SshConnectionError::Repository(error.to_string())
}
