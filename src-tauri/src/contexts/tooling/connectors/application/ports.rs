// The connect and execute paths that drive these land with the Connector Lifecycle task group.
#![cfg_attr(not(test), allow(dead_code))]

//! What connector storage, the platform, and the credential store may be asked to do.
//!
//! ## The credential seam
//!
//! There is no `credentials` bounded context in this repository. The established pattern — used by
//! `communications`, `agent_runtime`, `ssh_connections`, `cli_config`, and
//! `execution_observability` — is a port owned by the consuming subdomain, implemented over
//! `platform::credentials::OsCredentialStore` and assembled in bootstrap. `connectors` follows it
//! with its own port rather than depending on another subdomain's, and there is no cross-context
//! foreign key.
//!
//! `ConnectorCredentialPort` never returns a secret to a repository and never takes one from a
//! DTO. **No method on it may be called inside a SQLite transaction**: the OS credential store can
//! block on a keychain prompt, a locked keyring, or a DBus round trip, and a write transaction
//! holding the database lock while that happens would stall every other writer for as long as the
//! user ignores a dialog.
//!
//! ## What is deliberately absent
//!
//! There is no crash-safe credential *replacement* here. Replacing a secret durably needs a
//! recorded transition — the new handle written down before the secret exists, then
//! `prepared -> new_stored -> switched -> cleanup_pending/completed`, with every non-terminal
//! transition reconciled at startup. A `write-new -> CAS -> delete-old` sequence only survives an
//! error return; a process killed between the store write and the database commit leaves an
//! orphaned secret and an instance pointing at the old one, and nothing on the next launch knows
//! to look.
//!
//! Task Group 3 has no connect or execute path, so there is no real replacement to make safe.
//! Building the state machine now would mean shipping recovery code no code path can reach and no
//! test can exercise honestly. It belongs to the Connector Lifecycle task group, together with the
//! `connector_credential_transitions` table, and is recorded there as an open task rather than
//! implied to exist.

use crate::contexts::tooling::connectors::domain::{
    ActiveConnectorSnapshot, ConnectorBinding, ConnectorBindingError, ConnectorDefinitionOutcome,
    ConnectorDefinitionRevision, ConnectorGlobalId, ConnectorInstance, ConnectorInstanceError,
    ConnectorSnapshotRef, ConnectorSubject, ConnectorTarget, CredentialHandle, InstanceEdit,
    InstanceId,
};

/// Stable connector identities.
pub(crate) trait ConnectorSubjectRepository: Send + Sync {
    /// Idempotent. A subject already present is left alone, including its `first_seen_at` and its
    /// owner: re-seeding is not a new sighting, and rewriting the owner would erase which package
    /// an operator has to uninstall.
    fn ensure(&self, subject: &ConnectorSubject) -> Result<(), String>;

    fn get(&self, connector: &ConnectorGlobalId) -> Result<Option<ConnectorSubject>, String>;

    fn all(&self) -> Result<Vec<ConnectorSubject>, String>;
}

/// Immutable `(snapshot, subject)` definitions.
pub(crate) trait ConnectorDefinitionRepository: Send + Sync {
    fn record(
        &self,
        revision: &ConnectorDefinitionRevision,
    ) -> Result<ConnectorDefinitionOutcome, String>;

    fn recorded(
        &self,
        connector: &ConnectorGlobalId,
        snapshot: &ConnectorSnapshotRef,
    ) -> Result<Option<ConnectorDefinitionRevision>, String>;

    /// Every revision recorded for a subject.
    ///
    /// **Diagnostic only.** Ordering by recording time is exactly the answer readiness must not
    /// use: a version recorded but never activated leads it, and after a rollback the abandoned
    /// newer version still leads it.
    fn revisions(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<Vec<ConnectorDefinitionRevision>, String>;
}

/// Configured instances.
pub(crate) trait ConnectorInstanceRepository: Send + Sync {
    fn get(
        &self,
        instance: &InstanceId,
    ) -> Result<Option<ConnectorInstance>, ConnectorInstanceError>;

    fn for_connector(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<Vec<ConnectorInstance>, ConnectorInstanceError>;

    /// Creates or updates one instance, refusing the write if someone else changed it since
    /// `expected_revision` was read. `expected_revision` is `ABSENT_REVISION` for a create.
    ///
    /// The credential handle is *not* a parameter: attaching one is `attach_credential`, so an
    /// ordinary settings edit cannot clear or overwrite it by omission.
    fn save(&self, edit: &InstanceEdit<'_>) -> Result<ConnectorInstance, ConnectorInstanceError>;

    /// Points an instance at a credential-store entry, or at none.
    ///
    /// Takes a handle, never a secret. The secret is written to the credential store *before* this
    /// is called and outside any transaction — see the module header for why that sequence is not
    /// yet crash-safe and where the durable version belongs.
    fn attach_credential(
        &self,
        instance: &InstanceId,
        credential: Option<&CredentialHandle>,
        expected_revision: i64,
        at: &str,
    ) -> Result<ConnectorInstance, ConnectorInstanceError>;
}

/// Where instances are bound.
pub(crate) trait ConnectorBindingRepository: Send + Sync {
    fn binding(
        &self,
        instance: &InstanceId,
        target: &ConnectorTarget,
    ) -> Result<Option<ConnectorBinding>, ConnectorBindingError>;

    fn bindings(
        &self,
        instance: &InstanceId,
    ) -> Result<Vec<ConnectorBinding>, ConnectorBindingError>;

    fn set(
        &self,
        binding: &crate::contexts::tooling::connectors::domain::BindingId,
        instance: &InstanceId,
        target: &ConnectorTarget,
        enabled: bool,
        expected_revision: i64,
        at: &str,
    ) -> Result<ConnectorBinding, ConnectorBindingError>;
}

/// What snapshot the platform is running for a connector's contribution.
///
/// Consumer-owned: this subdomain declares the interface and an adapter in its own infrastructure
/// satisfies it by calling `extension_platform`'s published API.
pub(crate) trait ActiveConnectorSnapshotPort: Send + Sync {
    fn active_snapshot(
        &self,
        connector: &ConnectorGlobalId,
    ) -> Result<ActiveConnectorSnapshot, String>;
}

/// Secrets, by handle.
///
/// Nothing here may be called inside a SQLite transaction. See the module header.
pub(crate) trait ConnectorCredentialPort: Send + Sync {
    /// The handle the next secret for this instance will be stored under.
    ///
    /// Generated before anything is written, so a durable transition record can name it in
    /// advance. Nothing uses that property yet; the record itself is the Connector Lifecycle task
    /// group's, and this exists so the handle is not invented halfway through a write.
    fn allocate(&self, instance: &InstanceId) -> Result<CredentialHandle, String>;

    /// Writes a secret under a handle. The secret is borrowed and never returned or logged.
    fn store(&self, handle: &CredentialHandle, secret: &str) -> Result<(), String>;

    /// Whether a handle currently names a stored secret.
    ///
    /// Deliberately not "read the secret": a repository or a reconciliation only ever needs to
    /// know whether one is there, and a method that returned it would be the one someone reached
    /// for while building a DTO.
    fn exists(&self, handle: &CredentialHandle) -> Result<bool, String>;

    fn delete(&self, handle: &CredentialHandle) -> Result<(), String>;
}
