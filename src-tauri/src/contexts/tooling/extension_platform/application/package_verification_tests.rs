//! What the service adds on top of the domain verifier: finding the key, and the three answers
//! that are not "the signature verified".

use super::{PackageVerificationService, PublisherKeyDirectory, PublisherLookupUnavailable};
use crate::contexts::tooling::extension_platform::domain::{
    signed_payload, ExtensionId, ManifestDigest, PackageFacts, PackageHash, PackageSignature,
    PublisherId, PublisherKeyFingerprint, PublisherKeyRecord, PublisherPublicKey,
    PublisherTrustState, SignatureAlgorithm, SignatureEnvelope, SignatureRejection, SignatureState,
    SIGNATURE_BYTES,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use semver::Version;
use std::sync::Arc;

const PACKAGE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MANIFEST_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn key_record(trust_state: PublisherTrustState) -> PublisherKeyRecord {
    PublisherKeyRecord {
        publisher: PublisherId::parse("acme").expect("publisher"),
        key: PublisherPublicKey::from_bytes(signing_key().verifying_key().to_bytes()),
        trust_state,
    }
}

fn facts() -> PackageFacts {
    PackageFacts {
        hash: PackageHash::parse(PACKAGE_DIGEST).expect("package hash"),
        byte_length: 1_048_576,
    }
}

fn envelope_bytes() -> Vec<u8> {
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

    format!(
        "envelope_version: 1\nalgorithm: ed25519\npublisher: acme\nextension: acme.linter\n\
         version: 1.4.2\npackage_sha256: {PACKAGE_DIGEST}\npackage_bytes: 1048576\n\
         manifest_sha256: {MANIFEST_DIGEST}\nkey_fingerprint: {}\nsignature: {}\n",
        envelope.key_fingerprint.as_str(),
        STANDARD.encode(envelope.signature.as_bytes())
    )
    .into_bytes()
}

struct Directory(Result<Option<PublisherKeyRecord>, String>);

impl PublisherKeyDirectory for Directory {
    fn find(
        &self,
        _fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<PublisherKeyRecord>, String> {
        self.0.clone()
    }
}

/// Records which fingerprint was asked for, so the lookup key itself can be asserted on.
struct RecordingDirectory(std::sync::Mutex<Vec<String>>);

impl PublisherKeyDirectory for RecordingDirectory {
    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<PublisherKeyRecord>, String> {
        if let Ok(mut asked) = self.0.lock() {
            asked.push(fingerprint.as_str().to_string());
        }
        Ok(Some(key_record(PublisherTrustState::Trusted)))
    }
}

fn service(directory: impl PublisherKeyDirectory + 'static) -> PackageVerificationService {
    PackageVerificationService::new(Arc::new(directory))
}

#[test]
fn a_package_with_no_envelope_is_unsigned_rather_than_rejected() {
    // Unsigned is a state Developer Mode contains, not a verification failure. Reporting it as a
    // rejection would make the two indistinguishable to whatever renders the result.
    let state = service(Directory(Ok(None)))
        .verify(None, &facts())
        .expect("no lookup happens");

    assert_eq!(state, SignatureState::Unsigned);
    assert_eq!(state.code(), "unsigned");
}

#[test]
fn a_signed_package_verifies_against_the_key_its_envelope_names() {
    let directory = RecordingDirectory(std::sync::Mutex::new(Vec::new()));
    let expected_fingerprint = key_record(PublisherTrustState::Trusted).fingerprint();
    let service = PackageVerificationService::new(Arc::new(directory));

    let state = service
        .verify(Some(&envelope_bytes()), &facts())
        .expect("lookup succeeds");

    let verified = state.verified().expect("verified");
    assert_eq!(verified.key_fingerprint(), &expected_fingerprint);
    assert_eq!(state.code(), "verified");
}

#[test]
fn the_lookup_is_by_the_fingerprint_the_envelope_names() {
    let asked = Arc::new(RecordingDirectory(std::sync::Mutex::new(Vec::new())));
    let service = PackageVerificationService::new(asked.clone());

    service
        .verify(Some(&envelope_bytes()), &facts())
        .expect("lookup succeeds");

    let fingerprints = asked.0.lock().expect("recorded lookups").clone();
    assert_eq!(
        fingerprints,
        vec![key_record(PublisherTrustState::Trusted)
            .fingerprint()
            .as_str()
            .to_string()],
        "looking a key up by publisher would let a package choose which key to be checked against"
    );
}

#[test]
fn an_envelope_that_cannot_be_read_is_its_own_state() {
    let state = service(Directory(Ok(None)))
        .verify(Some(b"envelope_version: 1\n"), &facts())
        .expect("no lookup happens");

    assert!(
        matches!(state, SignatureState::Unreadable(_)),
        "{state:?} should be unreadable"
    );
    assert_eq!(state.code(), "missing_field");
}

#[test]
fn a_fingerprint_nobody_trusts_is_rejected_without_consulting_the_signature() {
    let state = service(Directory(Ok(None)))
        .verify(Some(&envelope_bytes()), &facts())
        .expect("lookup succeeds");

    assert_eq!(
        state,
        SignatureState::Rejected(SignatureRejection::UnknownPublisherKey)
    );
}

#[test]
fn a_revoked_key_is_rejected_even_though_the_signature_is_genuine() {
    let state = service(Directory(Ok(Some(key_record(
        PublisherTrustState::Revoked,
    )))))
    .verify(Some(&envelope_bytes()), &facts())
    .expect("lookup succeeds");

    assert_eq!(
        state,
        SignatureState::Rejected(SignatureRejection::RevokedPublisherKey)
    );
}

#[test]
fn a_store_that_cannot_be_read_is_not_an_answer_about_the_package() {
    // The failure that must never be flattened into a verdict. "Untrusted" and "we could not
    // check" look identical to a caller that only sees a `SignatureState`, and one of them would
    // then be reported to a user as a definite finding.
    let outcome = service(Directory(Err("database is locked".to_string())))
        .verify(Some(&envelope_bytes()), &facts());

    assert_eq!(
        outcome,
        Err(PublisherLookupUnavailable("database is locked".to_string()))
    );
}
