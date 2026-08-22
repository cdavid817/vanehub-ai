// The settings surface that calls this lands with task 12; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Listing, previewing, adding, revoking, and inspecting trusted publisher keys.
//!
//! Every operation reads the store by *fingerprint derived from the supplied bytes*, never by a
//! fingerprint the caller supplies. That is the whole safety property of this file: the question
//! "what is already filed under this key?" is asked about the key that was actually pasted, so a
//! caller cannot preview one key and commit another.
//!
//! Nothing here is a secret. A publisher key is public by construction — it verifies signatures and
//! cannot make them — so it is stored in SQLite alongside its provenance rather than in the
//! credential store. The rule that raw secrets never reach SQLite is untouched, because no secret
//! is involved at any point.

use super::ports::{PublisherKeyDirectory, TrustedPublisherKeyRepository};
use crate::contexts::tooling::extension_platform::domain::{
    decide_admission, parse_publisher_key_material, PublisherId, PublisherKeyAdmission,
    PublisherKeyError, PublisherKeyFingerprint, PublisherKeyLabel, PublisherKeyRecord,
    PublisherKeySource, PublisherTrustState, TrustedPublisherKey,
};
use std::sync::Arc;

/// What an operator supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublisherKeyRequest {
    pub(crate) publisher: String,
    pub(crate) key_material: String,
    pub(crate) label: String,
    pub(crate) source: PublisherKeySource,
}

/// What adding it would do, shown before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublisherKeyPreview {
    pub(crate) fingerprint: PublisherKeyFingerprint,
    pub(crate) publisher: PublisherId,
    pub(crate) admission: PublisherKeyAdmission,
}

pub(crate) trait PublisherKeyClock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

pub(crate) struct TrustedPublisherKeyService {
    keys: Arc<dyn TrustedPublisherKeyRepository>,
    clock: Arc<dyn PublisherKeyClock>,
}

impl TrustedPublisherKeyService {
    pub(crate) fn new(
        keys: Arc<dyn TrustedPublisherKeyRepository>,
        clock: Arc<dyn PublisherKeyClock>,
    ) -> Self {
        Self { keys, clock }
    }

    pub(crate) fn list(&self) -> Result<Vec<TrustedPublisherKey>, PublisherKeyError> {
        self.keys.list().map_err(PublisherKeyError::Storage)
    }

    pub(crate) fn inspect(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<TrustedPublisherKey, PublisherKeyError> {
        self.keys
            .find(fingerprint)
            .map_err(PublisherKeyError::Storage)?
            .ok_or(PublisherKeyError::UnknownKey)
    }

    /// Says what adding this key would mean. Writes nothing.
    pub(crate) fn preview_add(
        &self,
        request: &PublisherKeyRequest,
    ) -> Result<PublisherKeyPreview, PublisherKeyError> {
        let (publisher, key, _) = self.read_request(request)?;
        let fingerprint = key.fingerprint();
        let existing = self
            .keys
            .find(&fingerprint)
            .map_err(PublisherKeyError::Storage)?;
        Ok(PublisherKeyPreview {
            admission: decide_admission(&publisher, existing.as_ref()),
            fingerprint,
            publisher,
        })
    }

    /// Commits a key the operator previewed.
    ///
    /// `approved` is what they were shown. The admission is recomputed here rather than trusted,
    /// so a key revoked between the preview and the confirmation is refused; the approved preview
    /// only has to still describe the same key and publisher.
    pub(crate) fn add(
        &self,
        request: &PublisherKeyRequest,
        approved: &PublisherKeyPreview,
    ) -> Result<TrustedPublisherKey, PublisherKeyError> {
        let (publisher, key, label) = self.read_request(request)?;
        let fingerprint = key.fingerprint();
        if fingerprint != approved.fingerprint || publisher != approved.publisher {
            return Err(PublisherKeyError::PreviewSuperseded);
        }

        let existing = self
            .keys
            .find(&fingerprint)
            .map_err(PublisherKeyError::Storage)?;
        let admission = decide_admission(&publisher, existing.as_ref());
        if !admission.admits_write() {
            return Err(PublisherKeyError::NotAdmissible(admission));
        }

        let now = self.clock.now_rfc3339();
        // `first_seen_at` belongs to the first time this key was trusted, not to the most recent
        // paste of it. Provenance that resets on every re-add answers no question worth asking.
        let record = TrustedPublisherKey {
            publisher,
            key,
            label,
            source: request.source,
            trust_state: PublisherTrustState::Trusted,
            first_seen_at: existing
                .as_ref()
                .map_or_else(|| now.clone(), |existing| existing.first_seen_at.clone()),
            last_seen_at: now,
            revoked_at: None,
            revocation_reason: None,
        };
        self.keys
            .upsert(&record)
            .map_err(PublisherKeyError::Storage)?;
        Ok(record)
    }

    /// Withdraws trust from a key that is already here.
    ///
    /// By fingerprint, because revoking is about a specific key rather than about a publisher, and
    /// a publisher may hold more than one.
    pub(crate) fn revoke(
        &self,
        fingerprint: &PublisherKeyFingerprint,
        reason: Option<&str>,
    ) -> Result<TrustedPublisherKey, PublisherKeyError> {
        if self
            .keys
            .find(fingerprint)
            .map_err(PublisherKeyError::Storage)?
            .is_none()
        {
            return Err(PublisherKeyError::UnknownKey);
        }
        self.keys
            .revoke(fingerprint, &self.clock.now_rfc3339(), reason)
            .map_err(PublisherKeyError::Storage)?;
        self.inspect(fingerprint)
    }

    fn read_request(
        &self,
        request: &PublisherKeyRequest,
    ) -> Result<
        (
            PublisherId,
            crate::contexts::tooling::extension_platform::domain::PublisherPublicKey,
            PublisherKeyLabel,
        ),
        PublisherKeyError,
    > {
        let publisher = PublisherId::parse(&request.publisher)
            .map_err(|error| PublisherKeyError::InvalidPublisher(error.value().to_string()))?;
        let key = parse_publisher_key_material(&request.key_material)
            .map_err(PublisherKeyError::InvalidKey)?;
        let label =
            PublisherKeyLabel::parse(&request.label).map_err(PublisherKeyError::InvalidKey)?;
        Ok((publisher, key, label))
    }
}

/// The verification-side view of the same store.
///
/// A thin adapter rather than a second query path: one place decides what "trusted" means, and the
/// verifier sees only the narrow record.
pub(crate) struct RepositoryPublisherKeyDirectory {
    keys: Arc<dyn TrustedPublisherKeyRepository>,
}

impl RepositoryPublisherKeyDirectory {
    pub(crate) fn new(keys: Arc<dyn TrustedPublisherKeyRepository>) -> Self {
        Self { keys }
    }
}

impl PublisherKeyDirectory for RepositoryPublisherKeyDirectory {
    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<PublisherKeyRecord>, String> {
        Ok(self
            .keys
            .find(fingerprint)?
            .as_ref()
            .map(TrustedPublisherKey::for_verification))
    }
}
