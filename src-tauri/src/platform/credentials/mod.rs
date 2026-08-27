//! OS credential-store adapter with zeroizing secret reads.
//!
//! In a desktop test build the OS store is replaced by a file inside the run's isolated data
//! directory. That is not a convenience: the desktop suite saves and clears connector tokens and
//! provider configuration, and against the real store those writes land in the developer's own
//! Windows Credential Manager or login keychain and outlive the run. It also made the suite
//! unrunnable where no store exists at all -- a hosted macOS or Linux runner reports "A default
//! keychain could not be found" or "No default store has been set", which fails the specs for the
//! absence of a store rather than for anything about the code.

use crate::platform::error::InfrastructureError;
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub(crate) struct OsCredentialStore {
    service_name: String,
}

impl OsCredentialStore {
    pub(crate) fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub(crate) fn set(&self, account: &str, secret: &str) -> Result<(), InfrastructureError> {
        backend::set(&self.service_name, account, secret)
    }

    pub(crate) fn get(
        &self,
        account: &str,
    ) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        backend::get(&self.service_name, account)
    }

    pub(crate) fn delete(&self, account: &str) -> Result<(), InfrastructureError> {
        backend::delete(&self.service_name, account)
    }
}

/// The real OS store. One of the two `backend` modules is compiled, never both.
#[cfg(not(feature = "desktop-e2e"))]
mod backend {
    use super::{InfrastructureError, Zeroizing};

    pub(super) fn set(
        service: &str,
        account: &str,
        secret: &str,
    ) -> Result<(), InfrastructureError> {
        keyring::Entry::new(service, account)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|error| credential_error("write", error))
    }

    pub(super) fn get(
        service: &str,
        account: &str,
    ) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|error| credential_error("open", error))?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(credential_error("read", error)),
        }
    }

    pub(super) fn delete(service: &str, account: &str) -> Result<(), InfrastructureError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|error| credential_error("open", error))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(credential_error("delete", error)),
        }
    }

    fn credential_error(action: &str, error: keyring::Error) -> InfrastructureError {
        InfrastructureError::Credential(format!("credential store {action} failed: {error}"))
    }
}

/// File-backed stand-in for the OS store, present only in a desktop test build.
///
/// It lives under `VANEHUB_APP_DATA_DIR`, which the orchestrator creates per run, validates against
/// the real application data directory, and deletes afterwards. So it survives the relaunches the
/// persistence layers depend on, and nothing it holds outlives the run.
#[cfg(feature = "desktop-e2e")]
mod backend {
    use super::InfrastructureError;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use zeroize::Zeroizing;

    fn store_path() -> PathBuf {
        let root = std::env::var_os("VANEHUB_APP_DATA_DIR")
            .map(PathBuf::from)
            // A test build with no isolated directory still must not reach the OS store, so this
            // falls back to another temporary location rather than to the real keychain.
            .unwrap_or_else(std::env::temp_dir);
        root.join("desktop-e2e-credentials.json")
    }

    fn key(service: &str, account: &str) -> String {
        // The separator cannot appear in either half, so no pair of names can collide with another.
        format!("{service}\u{0}{account}")
    }

    fn load() -> Result<BTreeMap<String, String>, InfrastructureError> {
        match std::fs::read_to_string(store_path()) {
            Ok(raw) => serde_json::from_str(&raw).map_err(|error| {
                InfrastructureError::Credential(format!("credential store read failed: {error}"))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(error) => Err(InfrastructureError::Credential(format!(
                "credential store read failed: {error}"
            ))),
        }
    }

    pub(super) fn get(
        service: &str,
        account: &str,
    ) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        Ok(load()?
            .get(&key(service, account))
            .cloned()
            .map(Zeroizing::new))
    }

    pub(super) fn set(
        service: &str,
        account: &str,
        secret: &str,
    ) -> Result<(), InfrastructureError> {
        store(service, account, Some(secret))
    }

    pub(super) fn delete(service: &str, account: &str) -> Result<(), InfrastructureError> {
        store(service, account, None)
    }

    fn store(
        service: &str,
        account: &str,
        secret: Option<&str>,
    ) -> Result<(), InfrastructureError> {
        let mut entries = load()?;
        match secret {
            Some(secret) => entries.insert(key(service, account), secret.to_string()),
            None => entries.remove(&key(service, account)),
        };
        let encoded = serde_json::to_string(&entries).map_err(|error| {
            InfrastructureError::Credential(format!("credential store write failed: {error}"))
        })?;
        std::fs::write(store_path(), encoded).map_err(|error| {
            InfrastructureError::Credential(format!("credential store write failed: {error}"))
        })
    }
}
