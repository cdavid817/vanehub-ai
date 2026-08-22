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
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
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
///
/// Exactly what verification needs and nothing else. The stored record carries provenance too;
/// keeping the verifier's view this narrow means a label or a timestamp can never become an input
/// to whether a signature is accepted.
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

/// How a key came to be here.
///
/// A closed set rather than free text, and no path is kept. Which file an operator picked is not
/// evidence about the key — the key's own bytes are — and a stored path is one more place for
/// something that identifies a person to end up in a database that is not treated as sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublisherKeySource {
    ManualEntry,
    ImportedFile,
}

impl PublisherKeySource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ManualEntry => "manual_entry",
            Self::ImportedFile => "imported_file",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "manual_entry" => Some(Self::ManualEntry),
            "imported_file" => Some(Self::ImportedFile),
            _ => None,
        }
    }
}

pub(crate) const MAX_PUBLISHER_KEY_LABEL_CHARACTERS: usize = 64;

/// What an operator calls this key in a list.
///
/// Display only — nothing decides anything from it. Bounded and control-free because it is
/// rendered next to a trust decision, and a label carrying newlines or terminal escapes is a way
/// to make one key look like another.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PublisherKeyLabel(String);

impl PublisherKeyLabel {
    pub(crate) fn parse(value: &str) -> Result<Self, PublisherKeyRejection> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PublisherKeyRejection::EmptyLabel);
        }
        if trimmed.chars().count() > MAX_PUBLISHER_KEY_LABEL_CHARACTERS {
            return Err(PublisherKeyRejection::LabelTooLong);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(PublisherKeyRejection::LabelControlCharacter);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why key material or a label was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublisherKeyRejection {
    KeyMaterialNotBase64,
    KeyMaterialWrongLength,
    EmptyLabel,
    LabelTooLong,
    LabelControlCharacter,
}

impl PublisherKeyRejection {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::KeyMaterialNotBase64 => "publisher_key_not_base64",
            Self::KeyMaterialWrongLength => "publisher_key_wrong_length",
            Self::EmptyLabel => "publisher_key_label_empty",
            Self::LabelTooLong => "publisher_key_label_too_long",
            Self::LabelControlCharacter => "publisher_key_label_control_character",
        }
    }
}

pub(crate) const ALL_PUBLISHER_KEY_REJECTIONS: [PublisherKeyRejection; 5] = [
    PublisherKeyRejection::KeyMaterialNotBase64,
    PublisherKeyRejection::KeyMaterialWrongLength,
    PublisherKeyRejection::EmptyLabel,
    PublisherKeyRejection::LabelTooLong,
    PublisherKeyRejection::LabelControlCharacter,
];

/// Reads key material an operator supplied.
///
/// Standard base64 of exactly 32 bytes, and nothing else. Accepting hexadecimal as well would
/// double the ways a nearly-right key can be misread, and an operator pasting the wrong encoding
/// gets a specific diagnostic rather than a key that is 32 plausible bytes of something else.
pub(crate) fn parse_publisher_key_material(
    value: &str,
) -> Result<PublisherPublicKey, PublisherKeyRejection> {
    let decoded = STANDARD
        .decode(value.trim())
        .map_err(|_| PublisherKeyRejection::KeyMaterialNotBase64)?;
    let bytes: [u8; PUBLISHER_KEY_BYTES] = decoded
        .try_into()
        .map_err(|_| PublisherKeyRejection::KeyMaterialWrongLength)?;
    Ok(PublisherPublicKey::from_bytes(bytes))
}

/// A trusted key as it is stored: the verification facts plus how it got here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrustedPublisherKey {
    pub(crate) publisher: PublisherId,
    pub(crate) key: PublisherPublicKey,
    pub(crate) label: PublisherKeyLabel,
    pub(crate) source: PublisherKeySource,
    pub(crate) trust_state: PublisherTrustState,
    pub(crate) first_seen_at: String,
    pub(crate) last_seen_at: String,
    /// When trust was withdrawn, and why. Present exactly when the state is `Revoked`.
    pub(crate) revoked_at: Option<String>,
    pub(crate) revocation_reason: Option<String>,
}

impl TrustedPublisherKey {
    pub(crate) fn fingerprint(&self) -> PublisherKeyFingerprint {
        self.key.fingerprint()
    }

    /// The narrow view verification is allowed to see.
    pub(crate) fn for_verification(&self) -> PublisherKeyRecord {
        PublisherKeyRecord {
            publisher: self.publisher.clone(),
            key: self.key.clone(),
            trust_state: self.trust_state,
        }
    }
}
