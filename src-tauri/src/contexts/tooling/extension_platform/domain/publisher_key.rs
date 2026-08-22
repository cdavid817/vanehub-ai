// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! A publisher key as this application knows it.
//!
//! Trust here means one thing only: a human decided that bytes signed by this key came from a
//! publisher they are willing to receive packages from. It says nothing about what those packages
//! may then do — the runtime authority a package receives is decided separately and never by its
//! signature.
//!
//! Storage and lookup are somebody else's problem. What lives here is the key itself, the two
//! states it can be in, and the rule that a key's fingerprint is derived from its bytes rather
//! than stored beside them and hoped to match.

use super::canonical::hex;
use super::{ExtensionDomainError, IdentifierKind, PublisherId};
use sha2::{Digest, Sha256};

/// An Ed25519 public key is 32 bytes. There is no other length.
pub(crate) const PUBLISHER_KEY_BYTES: usize = 32;

/// A key's identity: SHA-256 over its raw bytes, lower-case hex.
///
/// Derived, never supplied. A fingerprint that arrived from outside is a claim about a key; this
/// type is the answer to what the key actually is, so the two can be compared.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PublisherKeyFingerprint(String);

impl PublisherKeyFingerprint {
    /// Reads a fingerprint that arrived as text — from an envelope, or back out of storage.
    ///
    /// Parsing one does not make it true. Only `PublisherPublicKey::fingerprint` says what a key's
    /// fingerprint is.
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionDomainError> {
        let valid = value.len() == 64
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
        if valid {
            Ok(Self(value.to_string()))
        } else {
            Err(ExtensionDomainError::new(
                IdentifierKind::PublisherKeyFingerprint,
                value.chars().take(120).collect::<String>(),
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The raw bytes of a publisher's Ed25519 public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublisherPublicKey([u8; PUBLISHER_KEY_BYTES]);

impl PublisherPublicKey {
    pub(crate) const fn from_bytes(bytes: [u8; PUBLISHER_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; PUBLISHER_KEY_BYTES] {
        &self.0
    }

    pub(crate) fn fingerprint(&self) -> PublisherKeyFingerprint {
        PublisherKeyFingerprint(hex(&Sha256::digest(self.0)))
    }
}

/// Whether a key may still authorize new work.
///
/// Revocation is not deletion. A revoked key stops authorizing new activation, and everything it
/// already signed keeps its evidence: which key signed an installed package is a fact about the
/// past, and losing it would make the revocation impossible to reason about afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublisherTrustState {
    Trusted,
    Revoked,
}

impl PublisherTrustState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Revoked => "revoked",
        }
    }
}

/// A trusted publisher key together with who it belongs to and whether it still counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublisherKeyRecord {
    pub(crate) publisher: PublisherId,
    pub(crate) key: PublisherPublicKey,
    pub(crate) trust_state: PublisherTrustState,
}

impl PublisherKeyRecord {
    pub(crate) fn fingerprint(&self) -> PublisherKeyFingerprint {
        self.key.fingerprint()
    }
}
