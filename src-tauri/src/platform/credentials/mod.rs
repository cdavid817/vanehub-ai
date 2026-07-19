//! OS credential-store adapter with zeroizing secret reads.

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
        keyring::Entry::new(&self.service_name, account)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|error| credential_error("write", error))
    }

    pub(crate) fn get(
        &self,
        account: &str,
    ) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        let entry = keyring::Entry::new(&self.service_name, account)
            .map_err(|error| credential_error("open", error))?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(credential_error("read", error)),
        }
    }

    pub(crate) fn delete(&self, account: &str) -> Result<(), InfrastructureError> {
        let entry = keyring::Entry::new(&self.service_name, account)
            .map_err(|error| credential_error("open", error))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(credential_error("delete", error)),
        }
    }
}

fn credential_error(action: &str, error: keyring::Error) -> InfrastructureError {
    InfrastructureError::Credential(format!("credential store {action} failed: {error}"))
}
