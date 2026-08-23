//! The credential seam: handles in, handles out, secrets neither stored here nor returned.

use super::{OsConnectorCredentials, CONNECTOR_CREDENTIAL_SERVICE};
use crate::contexts::tooling::connectors::application::ConnectorCredentialPort;
use crate::contexts::tooling::connectors::domain::{CredentialHandle, InstanceId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn instance() -> InstanceId {
    InstanceId::parse("instance-1").expect("instance")
}

fn counting_credentials() -> OsConnectorCredentials {
    let counter = AtomicUsize::new(0);
    OsConnectorCredentials::new(Box::new(move || {
        format!("s{}", counter.fetch_add(1, Ordering::SeqCst))
    }))
}

#[test]
fn allocating_twice_yields_two_different_handles() {
    // The property replacement rests on. Reusing one handle for a new secret would overwrite the
    // old one in place, so a failure between the store write and the database commit would leave
    // the instance pointing at a secret that is already gone. A fresh handle keeps the old one
    // readable until something deliberately deletes it.
    let credentials = counting_credentials();

    let first = credentials.allocate(&instance()).expect("allocate");
    let second = credentials.allocate(&instance()).expect("allocate");

    assert_ne!(first, second);
    assert_eq!(first.expose_for_storage(), "instance-1-s0");
    assert_eq!(second.expose_for_storage(), "instance-1-s1");
}

#[test]
fn an_allocated_handle_is_a_valid_handle() {
    // A suffix that produced something unparseable would fail at the constructor rather than
    // producing a handle nothing can read back.
    let credentials = counting_credentials();

    assert!(credentials.allocate(&instance()).is_ok());

    let hostile = OsConnectorCredentials::new(Box::new(|| "has space".to_string()));
    assert_eq!(
        hostile
            .allocate(&instance())
            .expect_err("an unusable handle is refused"),
        "invalid_connector_credential_handle"
    );
}

#[test]
fn connector_secrets_live_under_their_own_service_name() {
    // Distinct from every other subsystem's, so a connector secret and an IM secret cannot collide
    // on an account name and revoking one family does not touch another.
    assert_eq!(CONNECTOR_CREDENTIAL_SERVICE, "ai.vanehub.app.connectors");
}

#[test]
fn a_handle_does_not_appear_in_a_failure_message() {
    // A failure that named the entry would put the map of stored credentials into whatever log
    // caught the error.
    let credentials = counting_credentials();
    let handle = credentials.allocate(&instance()).expect("allocate");

    // The store may be unavailable on a headless machine; either answer is fine, and neither may
    // name the handle. Reported rather than swallowed is the other half of the assertion.
    for message in [
        credentials.store(&handle, "sk-live-secret").err(),
        credentials.exists(&handle).err(),
        credentials.delete(&handle).err(),
    ]
    .into_iter()
    .flatten()
    {
        assert!(
            !message.contains(handle.expose_for_storage()),
            "a credential-store failure named the entry: {message}"
        );
        assert!(
            !message.contains("sk-live-secret"),
            "a credential-store failure carried the secret: {message}"
        );
    }
}

/// An in-memory stand-in, so the port's contract is exercised without an OS keyring.
#[derive(Default)]
struct FakeCredentials {
    stored: Mutex<Vec<(String, String)>>,
}

impl ConnectorCredentialPort for FakeCredentials {
    fn allocate(&self, instance: &InstanceId) -> Result<CredentialHandle, String> {
        CredentialHandle::parse(&format!("{}-allocated", instance.as_str()))
            .map_err(|error| error.code().to_string())
    }

    fn store(&self, handle: &CredentialHandle, secret: &str) -> Result<(), String> {
        self.stored
            .lock()
            .expect("lock")
            .push((handle.expose_for_storage().to_string(), secret.to_string()));
        Ok(())
    }

    fn exists(&self, handle: &CredentialHandle) -> Result<bool, String> {
        Ok(self
            .stored
            .lock()
            .expect("lock")
            .iter()
            .any(|(stored, _)| stored == handle.expose_for_storage()))
    }

    fn delete(&self, handle: &CredentialHandle) -> Result<(), String> {
        self.stored
            .lock()
            .expect("lock")
            .retain(|(stored, _)| stored != handle.expose_for_storage());
        Ok(())
    }
}

#[test]
fn the_port_answers_whether_a_secret_is_stored_and_never_what_it_is() {
    // There is deliberately no `read`. A method that returned the secret would be the one someone
    // reached for while building a DTO, and the difference between "is it configured" and "what is
    // it" is the whole boundary.
    let credentials: Arc<dyn ConnectorCredentialPort> = Arc::new(FakeCredentials::default());
    let handle = credentials.allocate(&instance()).expect("allocate");

    assert!(!credentials.exists(&handle).expect("exists"));
    credentials.store(&handle, "sk-live-secret").expect("store");
    assert!(credentials.exists(&handle).expect("exists"));
    credentials.delete(&handle).expect("delete");
    assert!(!credentials.exists(&handle).expect("exists"));
}

#[test]
fn a_replacement_leaves_the_old_secret_readable_until_it_is_deleted() {
    // What the durable transition record will eventually make crash-safe. Recorded here as the
    // property the handle scheme already provides: the old secret is still there after the new one
    // is written, so a failure in between is recoverable *in principle*. Making it recoverable in
    // *fact* -- across a process kill -- needs `connector_credential_transitions`, which belongs to
    // the Connector Lifecycle task group and does not exist yet.
    let credentials = FakeCredentials::default();
    let old = CredentialHandle::parse("instance-1-old").expect("handle");
    let new = CredentialHandle::parse("instance-1-new").expect("handle");
    credentials.store(&old, "old-secret").expect("store");

    credentials.store(&new, "new-secret").expect("store");

    assert!(credentials.exists(&old).expect("exists"));
    assert!(credentials.exists(&new).expect("exists"));
}
