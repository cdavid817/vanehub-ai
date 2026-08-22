//! Preview, add, revoke, and inspect, against an in-memory store.

use super::{
    PublisherKeyClock, PublisherKeyPreview, PublisherKeyRequest, RepositoryPublisherKeyDirectory,
    TrustedPublisherKeyRepository, TrustedPublisherKeyService,
};
use crate::contexts::tooling::extension_platform::application::PublisherKeyDirectory;
use crate::contexts::tooling::extension_platform::domain::{
    PublisherId, PublisherKeyAdmission, PublisherKeyError, PublisherKeyFingerprint,
    PublisherKeyRejection, PublisherKeySource, PublisherPublicKey, PublisherTrustState,
    TrustedPublisherKey, PUBLISHER_KEY_BYTES,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// A store that behaves like the SQLite one for the rules the service depends on.
#[derive(Default)]
struct MemoryKeys {
    rows: Mutex<BTreeMap<String, TrustedPublisherKey>>,
    failure: Option<String>,
}

impl MemoryKeys {
    fn failing(message: &str) -> Self {
        Self {
            rows: Mutex::new(BTreeMap::new()),
            failure: Some(message.to_string()),
        }
    }

    fn guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, TrustedPublisherKey>>, String> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        self.rows.lock().map_err(|_| "poisoned".to_string())
    }
}

impl TrustedPublisherKeyRepository for MemoryKeys {
    fn list(&self) -> Result<Vec<TrustedPublisherKey>, String> {
        Ok(self.guard()?.values().cloned().collect())
    }

    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<TrustedPublisherKey>, String> {
        Ok(self.guard()?.get(fingerprint.as_str()).cloned())
    }

    fn upsert(&self, key: &TrustedPublisherKey) -> Result<(), String> {
        let mut rows = self.guard()?;
        match rows.get_mut(key.fingerprint().as_str()) {
            Some(existing) => {
                existing.label = key.label.clone();
                existing.source = key.source;
                existing.last_seen_at = key.last_seen_at.clone();
            }
            None => {
                rows.insert(key.fingerprint().as_str().to_string(), key.clone());
            }
        }
        Ok(())
    }

    fn revoke(
        &self,
        fingerprint: &PublisherKeyFingerprint,
        revoked_at: &str,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let mut rows = self.guard()?;
        if let Some(existing) = rows.get_mut(fingerprint.as_str()) {
            if existing.trust_state == PublisherTrustState::Trusted {
                existing.trust_state = PublisherTrustState::Revoked;
                existing.revoked_at = Some(revoked_at.to_string());
                existing.revocation_reason = reason.map(str::to_string);
            }
        }
        Ok(())
    }
}

/// Hands out a different time on each call, so a test can tell first sighting from most recent.
struct SteppingClock(Mutex<usize>);

impl PublisherKeyClock for SteppingClock {
    fn now_rfc3339(&self) -> String {
        let mut step = self.0.lock().map_or(0, |guard| *guard);
        step += 1;
        if let Ok(mut guard) = self.0.lock() {
            *guard = step;
        }
        format!("2026-08-{step:02}T00:00:00Z")
    }
}

fn service(keys: Arc<MemoryKeys>) -> TrustedPublisherKeyService {
    TrustedPublisherKeyService::new(keys, Arc::new(SteppingClock(Mutex::new(0))))
}

fn request(publisher: &str, seed: u8, label: &str) -> PublisherKeyRequest {
    PublisherKeyRequest {
        publisher: publisher.to_string(),
        key_material: STANDARD.encode([seed; PUBLISHER_KEY_BYTES]),
        label: label.to_string(),
        source: PublisherKeySource::ManualEntry,
    }
}

#[test]
fn a_preview_reports_the_derived_fingerprint_and_writes_nothing() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 1, "Release signing");

    let preview: PublisherKeyPreview = service.preview_add(&request).expect("preview");

    assert_eq!(
        preview.fingerprint,
        PublisherPublicKey::from_bytes([1_u8; PUBLISHER_KEY_BYTES]).fingerprint(),
        "the fingerprint comes from the pasted bytes, never from the caller"
    );
    assert_eq!(preview.admission, PublisherKeyAdmission::New);
    assert!(service.list().expect("list").is_empty());
}

#[test]
fn adding_a_key_records_it_as_trusted_with_its_provenance() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 2, "Release signing");
    let preview = service.preview_add(&request).expect("preview");

    let added = service.add(&request, &preview).expect("add");

    assert_eq!(added.trust_state, PublisherTrustState::Trusted);
    assert_eq!(added.publisher, PublisherId::parse("acme").expect("id"));
    assert_eq!(added.label.as_str(), "Release signing");
    assert_eq!(added.source, PublisherKeySource::ManualEntry);
    assert_eq!(added.revoked_at, None);
    assert_eq!(service.list().expect("list"), vec![added.clone()]);
    assert_eq!(
        service.inspect(&added.fingerprint()).expect("inspect"),
        added
    );
}

#[test]
fn re_adding_a_trusted_key_moves_the_most_recent_sighting_and_not_the_first() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let first = request("acme", 3, "Release signing");
    let preview = service.preview_add(&first).expect("preview");
    let added = service.add(&first, &preview).expect("add");

    let renamed = request("acme", 3, "Renamed");
    let second_preview = service.preview_add(&renamed).expect("preview");
    assert_eq!(
        second_preview.admission,
        PublisherKeyAdmission::AlreadyTrusted
    );
    let updated = service.add(&renamed, &second_preview).expect("re-add");

    assert_eq!(updated.first_seen_at, added.first_seen_at);
    assert_ne!(updated.last_seen_at, added.last_seen_at);
    assert_eq!(
        service
            .inspect(&added.fingerprint())
            .expect("inspect")
            .label
            .as_str(),
        "Renamed"
    );
}

#[test]
fn revoking_withdraws_trust_and_the_key_stays_as_evidence() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 4, "Release signing");
    let preview = service.preview_add(&request).expect("preview");
    let added = service.add(&request, &preview).expect("add");

    let revoked = service
        .revoke(&added.fingerprint(), Some("key rotated"))
        .expect("revoke");

    assert_eq!(revoked.trust_state, PublisherTrustState::Revoked);
    assert!(revoked.revoked_at.is_some());
    assert_eq!(revoked.revocation_reason.as_deref(), Some("key rotated"));
    assert_eq!(
        service.list().expect("list").len(),
        1,
        "evidence is retained"
    );
}

#[test]
fn a_revoked_key_cannot_be_re_trusted_by_adding_it_again() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 5, "Release signing");
    let preview = service.preview_add(&request).expect("preview");
    let added = service.add(&request, &preview).expect("add");
    service.revoke(&added.fingerprint(), None).expect("revoke");

    let preview = service.preview_add(&request).expect("preview");
    assert_eq!(preview.admission, PublisherKeyAdmission::Revoked);
    assert_eq!(
        service.add(&request, &preview),
        Err(PublisherKeyError::NotAdmissible(
            PublisherKeyAdmission::Revoked
        ))
    );
    assert_eq!(
        service
            .inspect(&added.fingerprint())
            .expect("inspect")
            .trust_state,
        PublisherTrustState::Revoked
    );
}

#[test]
fn a_key_revoked_between_preview_and_commit_is_refused() {
    // The preview is what the operator saw; the admission is recomputed anyway, because the answer
    // can change while a dialog is open and the stale answer is the dangerous one.
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 6, "Release signing");
    let stale_preview = service.preview_add(&request).expect("preview");
    let added = service.add(&request, &stale_preview).expect("add");
    service.revoke(&added.fingerprint(), None).expect("revoke");

    assert_eq!(
        service.add(&request, &stale_preview),
        Err(PublisherKeyError::NotAdmissible(
            PublisherKeyAdmission::Revoked
        ))
    );
}

#[test]
fn committing_a_different_key_than_was_previewed_is_refused() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let previewed = service
        .preview_add(&request("acme", 7, "Release signing"))
        .expect("preview");

    assert_eq!(
        service.add(&request("acme", 8, "Release signing"), &previewed),
        Err(PublisherKeyError::PreviewSuperseded)
    );
    assert_eq!(
        service.add(&request("other", 7, "Release signing"), &previewed),
        Err(PublisherKeyError::PreviewSuperseded)
    );
    assert!(service.list().expect("list").is_empty());
}

#[test]
fn one_key_claimed_by_a_second_publisher_is_refused() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let first = request("acme", 9, "Release signing");
    let preview = service.preview_add(&first).expect("preview");
    service.add(&first, &preview).expect("add");

    let second = request("other", 9, "Also mine");
    let preview = service.preview_add(&second).expect("preview");
    assert_eq!(
        preview.admission,
        PublisherKeyAdmission::ClaimedByAnotherPublisher {
            existing: PublisherId::parse("acme").expect("id"),
        }
    );
    assert!(matches!(
        service.add(&second, &preview),
        Err(PublisherKeyError::NotAdmissible(_))
    ));
}

#[test]
fn a_malformed_request_is_refused_before_the_store_is_consulted() {
    let keys = Arc::new(MemoryKeys::failing("the store must not be reached"));
    let service = service(keys);

    assert!(matches!(
        service.preview_add(&request("Acme Corp", 1, "label")),
        Err(PublisherKeyError::InvalidPublisher(_))
    ));
    let mut short = request("acme", 1, "label");
    short.key_material = STANDARD.encode([0_u8; 8]);
    assert_eq!(
        service.preview_add(&short),
        Err(PublisherKeyError::InvalidKey(
            PublisherKeyRejection::KeyMaterialWrongLength
        ))
    );
    assert_eq!(
        service.preview_add(&request("acme", 1, "   ")),
        Err(PublisherKeyError::InvalidKey(
            PublisherKeyRejection::EmptyLabel
        ))
    );
}

#[test]
fn revoking_or_inspecting_something_that_is_not_here_says_so() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys);
    let absent = PublisherPublicKey::from_bytes([200_u8; PUBLISHER_KEY_BYTES]).fingerprint();

    assert_eq!(service.inspect(&absent), Err(PublisherKeyError::UnknownKey));
    assert_eq!(
        service.revoke(&absent, None),
        Err(PublisherKeyError::UnknownKey)
    );
}

#[test]
fn a_store_failure_stays_a_store_failure() {
    let service = service(Arc::new(MemoryKeys::failing("database is locked")));

    assert_eq!(
        service.list(),
        Err(PublisherKeyError::Storage("database is locked".to_string()))
    );
    assert_eq!(
        service.list().expect_err("storage").code(),
        "publisher_key_storage_failure"
    );
}

#[test]
fn the_verification_directory_reads_the_same_store_through_the_narrow_port() {
    let keys = Arc::new(MemoryKeys::default());
    let service = service(keys.clone());
    let request = request("acme", 11, "Release signing");
    let preview = service.preview_add(&request).expect("preview");
    let added = service.add(&request, &preview).expect("add");

    let directory = RepositoryPublisherKeyDirectory::new(keys);
    assert_eq!(
        directory.find(&added.fingerprint()).expect("find"),
        Some(added.for_verification())
    );

    service.revoke(&added.fingerprint(), None).expect("revoke");
    assert_eq!(
        directory
            .find(&added.fingerprint())
            .expect("find")
            .map(|record| record.trust_state),
        Some(PublisherTrustState::Revoked),
        "a revoked key is still found, and found as revoked"
    );
}
