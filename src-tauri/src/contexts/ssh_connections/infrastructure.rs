use super::application::{
    SshConnectionClock, SshConnectionCredentialPort, SshConnectionError, SshConnectionRepository,
    SshConnectionTester,
};
use super::domain::{SshAuthMode, SshConnectionProfile, SshConnectionTestStatus};
use crate::platform::credentials::OsCredentialStore;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Row};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "io.vanehub.ai.ssh";

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
                "SELECT id, name, host, port, user, default_path, auth_mode, key_path, credential_ref,
                        test_status, last_connected_at, last_error, created_at, updated_at
                 FROM ssh_connections
                 ORDER BY updated_at DESC, name ASC",
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
            "SELECT id, name, host, port, user, default_path, auth_mode, key_path, credential_ref,
                    test_status, last_connected_at, last_error, created_at, updated_at
             FROM ssh_connections WHERE id = ?1",
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
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                r#"
                INSERT INTO ssh_connections
                    (id, name, host, port, user, default_path, auth_mode, key_path, credential_ref,
                     test_status, last_connected_at, last_error, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
                    profile.test_status.as_str(),
                    profile.last_connected_at,
                    profile.last_error,
                    profile.created_at,
                    profile.updated_at,
                ],
            )
            .map_err(sql_error)?;
        Ok(())
    }

    fn update(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        self.database
            .connection()
            .map_err(repository_error)?
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
                    test_status = ?10,
                    last_connected_at = ?11,
                    last_error = ?12,
                    created_at = ?13,
                    updated_at = ?14
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
                    profile.test_status.as_str(),
                    profile.last_connected_at,
                    profile.last_error,
                    profile.created_at,
                    profile.updated_at,
                ],
            )
            .map_err(sql_error)?;
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

fn read_profile(row: &Row<'_>) -> Result<SshConnectionProfile, rusqlite::Error> {
    let auth_mode = SshAuthMode::parse(row.get::<_, String>(6)?.as_str()).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let test_status =
        SshConnectionTestStatus::parse(row.get::<_, String>(9)?.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
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
        test_status,
        last_connected_at: row.get(10)?,
        last_error: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

#[derive(Clone)]
pub(crate) struct SshConnectionCredentialAdapter {
    store: OsCredentialStore,
}

impl SshConnectionCredentialAdapter {
    pub(crate) fn new() -> Self {
        Self {
            store: OsCredentialStore::new(SERVICE_NAME),
        }
    }
}

impl SshConnectionCredentialPort for SshConnectionCredentialAdapter {
    fn load(&self, id: &str) -> Result<Option<Zeroizing<String>>, SshConnectionError> {
        self.store.get(&credential_account(id)).map_err(|error| {
            SshConnectionError::Credential(error.command_safe_message().to_string())
        })
    }

    fn store(&self, id: &str, password: &str) -> Result<String, SshConnectionError> {
        let account = credential_account(id);
        self.store.set(&account, password).map_err(|error| {
            SshConnectionError::Credential(error.command_safe_message().to_string())
        })?;
        Ok(account)
    }

    fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        self.store.delete(&credential_account(id)).map_err(|error| {
            SshConnectionError::Credential(error.command_safe_message().to_string())
        })
    }
}

fn credential_account(id: &str) -> String {
    format!("ssh-connection/{id}")
}

pub(crate) struct SystemSshConnectionClock;

impl SshConnectionClock for SystemSshConnectionClock {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}

pub(crate) struct TcpSshConnectionTester;

impl SshConnectionTester for TcpSshConnectionTester {
    fn test(
        &self,
        profile: &SshConnectionProfile,
        _password: Option<&str>,
    ) -> Result<String, SshConnectionError> {
        let address = format!("{}:{}", profile.host, profile.port);
        let mut addrs = address.to_socket_addrs().map_err(|error| {
            SshConnectionError::Test(format!("SSH host resolution failed: {error}"))
        })?;
        let socket = addrs.next().ok_or_else(|| {
            SshConnectionError::Test("SSH host did not resolve to an address.".to_string())
        })?;
        TcpStream::connect_timeout(&socket, Duration::from_secs(5))
            .map_err(|error| SshConnectionError::Test(format!("SSH TCP probe failed: {error}")))?;
        Ok("SSH TCP probe succeeded.".to_string())
    }
}

fn repository_error(error: DatabaseError) -> SshConnectionError {
    SshConnectionError::Repository(error.to_string())
}

fn sql_error(error: rusqlite::Error) -> SshConnectionError {
    SshConnectionError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn repository_round_trips_profiles_without_password_plaintext() {
        let directory = TempDirectory::new("ssh-connections-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteSshConnectionRepository::new(database.clone());
        let profile = SshConnectionProfile {
            id: "ssh-fixture".to_string(),
            name: "Fixture".to_string(),
            host: "dev.example.com".to_string(),
            port: 2222,
            user: "cdavid".to_string(),
            default_path: "/work/app".to_string(),
            auth_mode: SshAuthMode::Password,
            key_path: None,
            credential_ref: Some("ssh-connection/ssh-fixture".to_string()),
            test_status: SshConnectionTestStatus::NotTested,
            last_connected_at: None,
            last_error: None,
            created_at: "2026-07-21T00:00:00.000Z".to_string(),
            updated_at: "2026-07-21T00:00:00.000Z".to_string(),
        };

        repository.insert(&profile).expect("insert");
        let loaded = repository
            .find("ssh-fixture")
            .expect("find")
            .expect("profile");

        assert_eq!(loaded, profile);
        let raw = database.connection().expect("connection").query_row(
            "SELECT group_concat(COALESCE(name, '') || COALESCE(host, '') || COALESCE(credential_ref, ''), '|') FROM ssh_connections",
            [],
            |row| row.get::<_, String>(0),
        ).expect("raw");
        assert!(!raw.contains("private-password"));
    }
}
