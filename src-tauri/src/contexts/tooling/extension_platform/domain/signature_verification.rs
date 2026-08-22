// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Whether these exact bytes were authorized by a key this installation trusts.
//!
//! That is the whole question. It is not whether the extension may run, what it may reach, or
//! whether it should be enabled — a valid signature grants no authority whatsoever, and the type
//! returned here deliberately carries nothing an authority decision could be built from.
//!
//! Verification is a pure function of bytes, which is why it lives in the domain alongside the
//! manifest digest rather than behind a port. Finding the key is not: that is a lookup against
//! stored trust, and it stays on the far side of `PublisherKeyDirectory`.
//!
//! ## Order
//!
//! Cheap, local facts first; the signature check last. Not for speed — for diagnostics. An
//! operator whose package was truncated in transit should be told the hash does not match, not
//! that the signature is invalid, which is true but useless.
//!
//! ## `verify_strict`
//!
//! Plain `verify` accepts small-order public keys and non-canonically encoded signatures, so the
//! same input can verify under one library and fail under another. For a supply-chain check that
//! is a defect: "authorized here, unauthorized there" is not an answer.

use super::{
    ManifestDigest, PackageHash, PublisherKeyFingerprint, PublisherKeyRecord, PublisherTrustState,
    SignatureEnvelope,
};
use crate::contexts::tooling::extension_platform::domain::signed_payload::signed_payload;
use ed25519_dalek::{Signature, VerifyingKey};

/// The facts about the package bytes themselves, measured by whoever read them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageFacts {
    pub(crate) hash: PackageHash,
    pub(crate) byte_length: u64,
}

/// Why a signature did not establish provenance.
///
/// Every variant is a different thing for an operator to do about it, which is the test for
/// whether a reason deserves to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureRejection {
    /// The envelope names a key this installation has never been told to trust.
    UnknownPublisherKey,
    /// The key is known and has been revoked.
    RevokedPublisherKey,
    /// The stored key does not hash to the fingerprint it is filed under. Storage corruption or
    /// tampering, not a package problem.
    KeyFingerprintMismatch,
    /// The envelope is signed by a key belonging to a different publisher than it claims.
    PublisherMismatch,
    /// The bytes on disk are not the bytes the envelope describes.
    PackageHashMismatch,
    PackageLengthMismatch,
    /// The key is structurally invalid as an Ed25519 public key.
    MalformedPublisherKey,
    /// Everything matched and the signature still does not verify.
    SignatureInvalid,
}

impl SignatureRejection {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::UnknownPublisherKey => "unknown_publisher_key",
            Self::RevokedPublisherKey => "revoked_publisher_key",
            Self::KeyFingerprintMismatch => "key_fingerprint_mismatch",
            Self::PublisherMismatch => "signature_publisher_mismatch",
            Self::PackageHashMismatch => "package_hash_mismatch",
            Self::PackageLengthMismatch => "package_length_mismatch",
            Self::MalformedPublisherKey => "malformed_publisher_key",
            Self::SignatureInvalid => "signature_invalid",
        }
    }
}

pub(crate) const ALL_SIGNATURE_REJECTIONS: [SignatureRejection; 8] = [
    SignatureRejection::UnknownPublisherKey,
    SignatureRejection::RevokedPublisherKey,
    SignatureRejection::KeyFingerprintMismatch,
    SignatureRejection::PublisherMismatch,
    SignatureRejection::PackageHashMismatch,
    SignatureRejection::PackageLengthMismatch,
    SignatureRejection::MalformedPublisherKey,
    SignatureRejection::SignatureInvalid,
];

/// A signature that verified against a trusted key.
///
/// It carries the publisher's *claim* about the manifest, not a fact about it: at this point
/// nothing has opened the archive. `confirm_manifest` is the only way to get from here to
/// something a witness may record, so the comparison cannot be forgotten — it is not a rule
/// written in a comment, it is the only route through the types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedSignature {
    key_fingerprint: PublisherKeyFingerprint,
    claimed_manifest_digest: ManifestDigest,
}

impl VerifiedSignature {
    pub(crate) fn key_fingerprint(&self) -> &PublisherKeyFingerprint {
        &self.key_fingerprint
    }

    pub(crate) fn claimed_manifest_digest(&self) -> &ManifestDigest {
        &self.claimed_manifest_digest
    }

    /// Confirms the extracted manifest is the one the publisher signed for.
    ///
    /// Fails when it is not, which is the substitution this whole design is aimed at: valid
    /// signature, real publisher, different manifest.
    pub(crate) fn confirm_manifest(
        self,
        extracted: &ManifestDigest,
    ) -> Result<ConfirmedSignature, SignatureRejection> {
        if &self.claimed_manifest_digest != extracted {
            return Err(SignatureRejection::SignatureInvalid);
        }
        Ok(ConfirmedSignature {
            key_fingerprint: self.key_fingerprint,
            manifest_digest: self.claimed_manifest_digest,
        })
    }
}

/// A verified signature whose manifest claim has been checked against the extracted manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfirmedSignature {
    key_fingerprint: PublisherKeyFingerprint,
    manifest_digest: ManifestDigest,
}

impl ConfirmedSignature {
    pub(crate) fn key_fingerprint(&self) -> &PublisherKeyFingerprint {
        &self.key_fingerprint
    }

    pub(crate) fn manifest_digest(&self) -> &ManifestDigest {
        &self.manifest_digest
    }
}

/// Everything that can be known about a package's provenance, as one closed set.
///
/// Four states, kept apart on purpose. "No signature was offered" and "a signature was offered and
/// is wrong" call for completely different handling — the first is what Developer Mode exists to
/// contain, the second is an attack or a corrupt download — and an unreadable envelope is a third
/// thing again, because it never got as far as naming a key. Collapsing any two of them into a
/// boolean is how a UI ends up telling a user their package is unsigned when it is forged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SignatureState {
    /// No envelope accompanied the package.
    Unsigned,
    /// An envelope was present and could not be read as one.
    Unreadable(super::ManifestDecodeError),
    /// The envelope was read and did not establish provenance.
    Rejected(SignatureRejection),
    /// Provenance established against a key this installation trusts.
    Verified(VerifiedSignature),
}

impl SignatureState {
    /// The stable code a caller branches on and a log records.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Unsigned => "unsigned",
            Self::Unreadable(error) => error.code(),
            Self::Rejected(reason) => reason.code(),
            Self::Verified(_) => "verified",
        }
    }

    pub(crate) const fn verified(&self) -> Option<&VerifiedSignature> {
        match self {
            Self::Verified(signature) => Some(signature),
            _ => None,
        }
    }
}

/// Checks one envelope against one key and the bytes that were actually read.
pub(crate) fn verify_package_signature(
    envelope: &SignatureEnvelope,
    key: &PublisherKeyRecord,
    package: &PackageFacts,
) -> Result<VerifiedSignature, SignatureRejection> {
    if key.fingerprint() != envelope.key_fingerprint {
        return Err(SignatureRejection::KeyFingerprintMismatch);
    }
    if key.trust_state == PublisherTrustState::Revoked {
        return Err(SignatureRejection::RevokedPublisherKey);
    }
    if key.publisher != envelope.publisher {
        return Err(SignatureRejection::PublisherMismatch);
    }
    if package.hash != envelope.package_hash {
        return Err(SignatureRejection::PackageHashMismatch);
    }
    if package.byte_length != envelope.package_bytes {
        return Err(SignatureRejection::PackageLengthMismatch);
    }

    let verifying_key = VerifyingKey::from_bytes(key.key.as_bytes())
        .map_err(|_| SignatureRejection::MalformedPublisherKey)?;
    let signature = Signature::from_bytes(envelope.signature.as_bytes());
    verifying_key
        .verify_strict(&signed_payload(envelope), &signature)
        .map_err(|_| SignatureRejection::SignatureInvalid)?;

    Ok(VerifiedSignature {
        key_fingerprint: envelope.key_fingerprint.clone(),
        claimed_manifest_digest: envelope.claimed_manifest_digest.clone(),
    })
}
