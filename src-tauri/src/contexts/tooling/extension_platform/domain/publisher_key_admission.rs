// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What adding a key would do, decided before anything is written.
//!
//! Trusting a publisher key is the single most consequential thing an operator does in this
//! subsystem: every package that key ever signs is admitted on the strength of it. So the answer
//! is computed and shown first, and the same computation gates the write — a preview an operator
//! approved and a commit that does something else is the failure mode a preview exists to prevent.

use super::{PublisherId, PublisherKeyRejection, TrustedPublisherKey};

/// What the store already knows about this exact key, and therefore what adding it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PublisherKeyAdmission {
    /// Nothing here has this fingerprint. Adding it establishes new trust.
    New,
    /// Already trusted, for the same publisher. Adding it again updates provenance and changes no
    /// trust decision.
    AlreadyTrusted,
    /// Known, and revoked.
    ///
    /// Refused rather than re-trusted. Revocation is a deliberate withdrawal of trust, and an
    /// "add" that silently reversed it would make revocation depend on nobody pasting the key
    /// again. Re-trusting a revoked key is not a thing V1 offers at all: the safe way back is a
    /// new key.
    Revoked,
    /// The same key bytes are already filed under a different publisher.
    ///
    /// One key, two publishers is either an operator error or an attempt to have a package
    /// verify under an identity its signer does not hold. Neither is something to resolve by
    /// picking one.
    ClaimedByAnotherPublisher { existing: PublisherId },
}

impl PublisherKeyAdmission {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::AlreadyTrusted => "already_trusted",
            Self::Revoked => "revoked",
            Self::ClaimedByAnotherPublisher { .. } => "claimed_by_another_publisher",
        }
    }

    /// Whether a commit may proceed on this answer.
    pub(crate) const fn admits_write(&self) -> bool {
        matches!(self, Self::New | Self::AlreadyTrusted)
    }
}

/// Decides what adding `publisher`'s key would mean, given whatever is already filed under its
/// fingerprint.
pub(crate) fn decide_admission(
    publisher: &PublisherId,
    existing: Option<&TrustedPublisherKey>,
) -> PublisherKeyAdmission {
    let Some(existing) = existing else {
        return PublisherKeyAdmission::New;
    };
    if &existing.publisher != publisher {
        return PublisherKeyAdmission::ClaimedByAnotherPublisher {
            existing: existing.publisher.clone(),
        };
    }
    match existing.trust_state {
        super::PublisherTrustState::Revoked => PublisherKeyAdmission::Revoked,
        super::PublisherTrustState::Trusted => PublisherKeyAdmission::AlreadyTrusted,
    }
}

/// Why a key-management operation did not happen.
///
/// Its codes are its own, even where they sound like the verifier's. "A package named a key we do
/// not trust" and "you asked to revoke a key that is not here" are different events with different
/// callers, and giving them one code would make a log impossible to read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PublisherKeyError {
    /// The publisher id was not readable. Carries the offending text, already bounded by the
    /// identifier parser that refused it.
    InvalidPublisher(String),
    InvalidKey(PublisherKeyRejection),
    /// The admission answer forbids writing - a revoked key, or a key another publisher holds.
    NotAdmissible(PublisherKeyAdmission),
    /// The preview an operator approved does not describe the key now being committed.
    PreviewSuperseded,
    UnknownKey,
    Storage(String),
}

impl PublisherKeyError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidPublisher(_) => "publisher_key_invalid_publisher",
            Self::InvalidKey(rejection) => rejection.code(),
            Self::NotAdmissible(_) => "publisher_key_not_admissible",
            Self::PreviewSuperseded => "publisher_key_preview_superseded",
            Self::UnknownKey => "publisher_key_unknown",
            Self::Storage(_) => "publisher_key_storage_failure",
        }
    }
}

/// Every management failure, for the catalog. The `InvalidKey` family is registered through
/// `ALL_PUBLISHER_KEY_REJECTIONS` and is deliberately absent here.
pub(crate) fn all_publisher_key_errors() -> Vec<PublisherKeyError> {
    vec![
        PublisherKeyError::InvalidPublisher(String::new()),
        PublisherKeyError::NotAdmissible(PublisherKeyAdmission::New),
        PublisherKeyError::PreviewSuperseded,
        PublisherKeyError::UnknownKey,
        PublisherKeyError::Storage(String::new()),
    ]
}
