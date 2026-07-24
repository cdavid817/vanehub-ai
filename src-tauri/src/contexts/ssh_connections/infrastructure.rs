mod runtime;
mod sqlite_repository;
#[cfg(test)]
mod sqlite_repository_tests;

use super::application::{
    SshConnectionClock, SshConnectionCredentialPort, SshConnectionError, SshConnectionIdentity,
    SshConnectionTester,
};
use super::domain::SshConnectionProfile;
use crate::platform::credentials::OsCredentialStore;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use zeroize::Zeroizing;

pub(crate) use runtime::{RusshSshConnector, SystemRemoteSshPoolClock};
pub(crate) use sqlite_repository::{apply_schema, SqliteSshConnectionRepository};

const SERVICE_NAME: &str = "io.vanehub.ai.ssh";

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

pub(crate) struct UuidSshConnectionIdentity;

impl SshConnectionIdentity for UuidSshConnectionIdentity {
    fn next_id(&self) -> String {
        format!("ssh-{}", uuid::Uuid::new_v4().simple())
    }
}
