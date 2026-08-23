// Assembled in bootstrap with the connect path in the Connector Lifecycle task group.
#![cfg_attr(not(test), allow(dead_code))]

//! The OS credential store, behind this subdomain's own port.
//!
//! There is no `credentials` bounded context here; the established seam is a port owned by the
//! consuming subdomain over `platform::credentials::OsCredentialStore`, and this follows it rather
//! than depending on `communications`' port or its `im_credential_refs` table.
//!
//! **Nothing in this file may be called inside a SQLite transaction.** A keychain read can block
//! on a user prompt, a locked keyring, or a DBus round trip; a write transaction holding the
//! database lock while that happens stalls every other writer for as long as the dialog goes
//! unanswered. The repository methods take handles precisely so the two never interleave.
//!
//! **No secret is returned.** `exists` answers whether one is stored; there is deliberately no
//! `read`. A method that returned the secret would be the one someone reached for while building a
//! DTO, and the difference between "is it configured" and "what is it" is the whole boundary.

use crate::contexts::tooling::connectors::application::ConnectorCredentialPort;
use crate::contexts::tooling::connectors::domain::{CredentialHandle, InstanceId};
use crate::platform::credentials::OsCredentialStore;

/// The credential-store service name connector secrets live under.
///
/// Distinct from every other subsystem's, so a connector secret and an IM secret cannot collide on
/// an account name and so revoking one family does not touch another.
pub(crate) const CONNECTOR_CREDENTIAL_SERVICE: &str = "ai.vanehub.app.connectors";

pub(crate) struct OsConnectorCredentials {
    store: OsCredentialStore,
    /// Supplies the random part of a handle. Injected so a test can make allocation deterministic
    /// without the port having to expose a seed.
    next_suffix: Box<dyn Fn() -> String + Send + Sync>,
}

impl OsConnectorCredentials {
    pub(crate) fn new(next_suffix: Box<dyn Fn() -> String + Send + Sync>) -> Self {
        Self {
            store: OsCredentialStore::new(CONNECTOR_CREDENTIAL_SERVICE),
            next_suffix,
        }
    }
}

impl ConnectorCredentialPort for OsConnectorCredentials {
    /// A fresh handle, derived from the instance and a random suffix.
    ///
    /// The suffix is what makes replacement possible at all: reusing one handle for a new secret
    /// would overwrite the old one in place, so a failure between the store write and the database
    /// commit would leave the instance pointing at a secret that is already gone. A new handle
    /// each time keeps the old one readable until something deliberately deletes it.
    fn allocate(&self, instance: &InstanceId) -> Result<CredentialHandle, String> {
        let candidate = format!("{}-{}", instance.as_str(), (self.next_suffix)());
        CredentialHandle::parse(&candidate).map_err(|error| error.code().to_string())
    }

    fn store(&self, handle: &CredentialHandle, secret: &str) -> Result<(), String> {
        self.store
            .set(handle.expose_for_storage(), secret)
            // The store's own error text is kept, and the secret is not in it. `handle` is not
            // interpolated either: a failure message that named the entry would put the map of
            // stored credentials into whatever log caught the error.
            .map_err(|error| error.to_string())
    }

    fn exists(&self, handle: &CredentialHandle) -> Result<bool, String> {
        self.store
            .get(handle.expose_for_storage())
            .map(|secret| secret.is_some())
            .map_err(|error| error.to_string())
    }

    fn delete(&self, handle: &CredentialHandle) -> Result<(), String> {
        self.store
            .delete(handle.expose_for_storage())
            .map_err(|error| error.to_string())
    }
}
