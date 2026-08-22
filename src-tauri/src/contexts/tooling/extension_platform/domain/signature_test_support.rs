//! A real verified signature, for tests that need one but are not about verification.
//!
//! Built by signing and verifying rather than by constructing `VerifiedSignature` directly. A
//! test-only constructor would let a test assert something about a state the production path
//! cannot actually produce, which is how a test suite ends up green about a shape that never
//! occurs.

use super::{
    signed_payload, verify_package_signature, ExtensionId, ManifestDigest, PackageFacts,
    PackageHash, PackageSignature, PublisherId, PublisherKeyRecord, PublisherPublicKey,
    PublisherTrustState, SignatureAlgorithm, SignatureEnvelope, VerifiedSignature,
    PUBLISHER_KEY_BYTES, SIGNATURE_BYTES,
};
use ed25519_dalek::{Signer, SigningKey};
use semver::Version;

pub(super) const PACKAGE_DIGEST: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";
pub(super) const MANIFEST_DIGEST: &str =
    "2222222222222222222222222222222222222222222222222222222222222222";

pub(super) fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; PUBLISHER_KEY_BYTES])
}

pub(super) fn key_record(trust_state: PublisherTrustState) -> PublisherKeyRecord {
    PublisherKeyRecord {
        publisher: PublisherId::parse("acme").expect("publisher"),
        key: PublisherPublicKey::from_bytes(signing_key().verifying_key().to_bytes()),
        trust_state,
    }
}

pub(super) fn envelope() -> SignatureEnvelope {
    let mut envelope = SignatureEnvelope {
        envelope_version: 1,
        algorithm: SignatureAlgorithm::Ed25519,
        publisher: PublisherId::parse("acme").expect("publisher"),
        extension: ExtensionId::parse("acme.linter").expect("extension"),
        version: Version::parse("1.4.2").expect("version"),
        package_hash: PackageHash::parse(PACKAGE_DIGEST).expect("package hash"),
        package_bytes: 1_048_576,
        claimed_manifest_digest: ManifestDigest::parse(MANIFEST_DIGEST).expect("manifest digest"),
        key_fingerprint: key_record(PublisherTrustState::Trusted).fingerprint(),
        signature: PackageSignature::from_bytes([0_u8; SIGNATURE_BYTES]),
    };
    let signature = signing_key().sign(&signed_payload(&envelope));
    envelope.signature = PackageSignature::from_bytes(signature.to_bytes());
    envelope
}

pub(super) fn facts() -> PackageFacts {
    PackageFacts {
        hash: PackageHash::parse(PACKAGE_DIGEST).expect("package hash"),
        byte_length: 1_048_576,
    }
}

pub(super) fn verified_signature() -> VerifiedSignature {
    verify_package_signature(
        &envelope(),
        &key_record(PublisherTrustState::Trusted),
        &facts(),
    )
    .expect("the support fixture must verify")
}
