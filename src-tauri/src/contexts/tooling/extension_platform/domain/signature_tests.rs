//! The envelope, what it commits to, and what verifying it does and does not establish.

use super::{
    parse_signature_envelope, signed_payload, verify_package_signature, ConfirmedSignature,
    ExtensionId, ManifestDigest, PackageFacts, PackageHash, PackageSignature, PublisherId,
    PublisherKeyFingerprint, PublisherKeyRecord, PublisherPublicKey, PublisherTrustState,
    SignatureAlgorithm, SignatureEnvelope, SignatureRejection, VerifiedSignature,
    ALL_SIGNATURE_REJECTIONS, ENVELOPE_YAML_LIMITS, PUBLISHER_KEY_BYTES, SIGNATURE_BYTES,
    SUPPORTED_ENVELOPE_VERSION,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use semver::Version;

const PACKAGE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MANIFEST_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; PUBLISHER_KEY_BYTES])
}

fn key_record(trust_state: PublisherTrustState) -> PublisherKeyRecord {
    PublisherKeyRecord {
        publisher: PublisherId::parse("acme").expect("publisher"),
        key: PublisherPublicKey::from_bytes(signing_key().verifying_key().to_bytes()),
        trust_state,
    }
}

fn envelope_fields() -> SignatureEnvelope {
    SignatureEnvelope {
        envelope_version: SUPPORTED_ENVELOPE_VERSION,
        algorithm: SignatureAlgorithm::Ed25519,
        publisher: PublisherId::parse("acme").expect("publisher"),
        extension: ExtensionId::parse("acme.linter").expect("extension"),
        version: Version::parse("1.4.2").expect("version"),
        package_hash: PackageHash::parse(PACKAGE_DIGEST).expect("package hash"),
        package_bytes: 1_048_576,
        claimed_manifest_digest: ManifestDigest::parse(MANIFEST_DIGEST).expect("manifest digest"),
        key_fingerprint: key_record(PublisherTrustState::Trusted).fingerprint(),
        signature: PackageSignature::from_bytes([0_u8; SIGNATURE_BYTES]),
    }
}

/// One covered field, changed. Named so the table below reads as what it is.
type FieldMutation = Box<dyn Fn(&mut SignatureEnvelope)>;

/// Signs whatever the envelope currently says, so a test that mutates a field afterwards is
/// testing tampering rather than a mismatched fixture.
fn signed(mut envelope: SignatureEnvelope) -> SignatureEnvelope {
    let signature = signing_key().sign(&signed_payload(&envelope));
    envelope.signature = PackageSignature::from_bytes(signature.to_bytes());
    envelope
}

fn facts() -> PackageFacts {
    PackageFacts {
        hash: PackageHash::parse(PACKAGE_DIGEST).expect("package hash"),
        byte_length: 1_048_576,
    }
}

fn envelope_text(overrides: &[(&str, &str)]) -> String {
    let signature = STANDARD.encode(signed(envelope_fields()).signature.as_bytes());
    let fingerprint = key_record(PublisherTrustState::Trusted).fingerprint();
    let mut fields: Vec<(String, String)> = vec![
        (
            "envelope_version".into(),
            SUPPORTED_ENVELOPE_VERSION.to_string(),
        ),
        ("algorithm".into(), "ed25519".into()),
        ("publisher".into(), "acme".into()),
        ("extension".into(), "acme.linter".into()),
        ("version".into(), "1.4.2".into()),
        ("package_sha256".into(), PACKAGE_DIGEST.into()),
        ("package_bytes".into(), "1048576".into()),
        ("manifest_sha256".into(), MANIFEST_DIGEST.into()),
        ("key_fingerprint".into(), fingerprint.as_str().into()),
        ("signature".into(), signature),
    ];
    for (key, value) in overrides {
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some(existing) if value.is_empty() => {
                existing.1 = String::new();
            }
            Some(existing) => existing.1 = (*value).to_string(),
            None => fields.push(((*key).to_string(), (*value).to_string())),
        }
    }
    fields
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{key}: {value}\n"))
        .collect()
}

fn parse_code(text: &str) -> (String, &'static str) {
    let error = parse_signature_envelope(text.as_bytes()).expect_err("envelope should be refused");
    (error.field().to_string(), error.code())
}

#[test]
fn a_well_formed_envelope_reads_back_every_field() {
    let parsed = parse_signature_envelope(envelope_text(&[]).as_bytes()).expect("envelope");
    let expected = signed(envelope_fields());

    assert_eq!(parsed, expected);
}

#[test]
fn every_field_is_required() {
    for field in [
        "envelope_version",
        "algorithm",
        "publisher",
        "extension",
        "version",
        "package_sha256",
        "package_bytes",
        "manifest_sha256",
        "key_fingerprint",
        "signature",
    ] {
        let (reported, code) = parse_code(&envelope_text(&[(field, "")]));
        assert_eq!(
            (reported.as_str(), code),
            (field, "missing_field"),
            "{field}"
        );
    }
}

#[test]
fn a_field_this_build_does_not_read_is_refused_rather_than_ignored() {
    // The whole point of an envelope is that it says what was signed. A key nobody reads is a
    // claim the signer made and this build silently dropped.
    assert_eq!(
        parse_code(&envelope_text(&[("timestamp", "2026-08-22T00:00:00Z")])),
        ("timestamp".to_string(), "unknown_field")
    );
}

#[test]
fn the_envelope_version_is_read_before_anything_it_governs() {
    // Both wrong: the version and the algorithm. The version has to win, because the fields below
    // it are only meaningful under a shape this build implements.
    let unsupported = (SUPPORTED_ENVELOPE_VERSION + 1).to_string();
    assert_eq!(
        parse_code(&envelope_text(&[
            ("envelope_version", unsupported.as_str()),
            ("algorithm", "rsa")
        ])),
        ("envelope_version".to_string(), "unsupported_schema_version")
    );
    assert_eq!(
        parse_code(&envelope_text(&[("algorithm", "rsa")])),
        ("algorithm".to_string(), "unknown_value")
    );
}

#[test]
fn malformed_values_name_the_field_that_holds_them() {
    let cases = [
        ("envelope_version", "one", "not_permitted"),
        ("publisher", "Acme Corp", "invalid_identifier"),
        ("extension", "linter", "invalid_identifier"),
        ("version", "1.4", "invalid_version"),
        ("package_sha256", "abc", "invalid_identifier"),
        ("package_bytes", "-1", "not_permitted"),
        ("manifest_sha256", &MANIFEST_DIGEST[..63], "not_permitted"),
        ("key_fingerprint", "not-a-fingerprint", "invalid_identifier"),
        ("signature", "not base64", "not_permitted"),
    ];
    for (field, value, expected) in cases {
        let (reported, code) = parse_code(&envelope_text(&[(field, value)]));
        assert_eq!((reported.as_str(), code), (field, expected), "{field}");
    }
}

#[test]
fn a_signature_must_decode_to_exactly_sixty_four_bytes() {
    for wrong_length in [63_usize, 65] {
        let encoded = STANDARD.encode(vec![0_u8; wrong_length]);
        assert_eq!(
            parse_code(&envelope_text(&[("signature", &encoded)])),
            ("signature".to_string(), "not_permitted"),
            "{wrong_length} bytes"
        );
    }
}

#[test]
fn bytes_that_are_not_a_readable_document_are_refused_before_any_field() {
    assert_eq!(
        parse_code("envelope_version: 1\n\talgorithm: ed25519\n"),
        (String::new(), "malformed_document")
    );

    let padding = "x".repeat(ENVELOPE_YAML_LIMITS.max_bytes);
    let oversized = format!("envelope_version: 1\n# {padding}\n");
    assert_eq!(
        parse_code(&oversized),
        (String::new(), "malformed_document")
    );

    let error = parse_signature_envelope(&[0xff, 0xfe]).expect_err("invalid UTF-8");
    assert_eq!((error.field(), error.code()), ("", "not_permitted"));
}

#[test]
fn the_signed_payload_is_stable_and_covers_every_field_except_the_signature() {
    let envelope = signed(envelope_fields());
    assert_eq!(signed_payload(&envelope), signed_payload(&envelope));

    let mut different_signature = envelope.clone();
    different_signature.signature = PackageSignature::from_bytes([9_u8; SIGNATURE_BYTES]);
    assert_eq!(
        signed_payload(&different_signature),
        signed_payload(&envelope),
        "a signature cannot cover itself"
    );

    let mutations: Vec<(&str, FieldMutation)> = vec![
        (
            "publisher",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.publisher = PublisherId::parse("other").expect("publisher");
            }),
        ),
        (
            "extension",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.extension = ExtensionId::parse("acme.other").expect("extension");
            }),
        ),
        (
            "version",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.version = Version::parse("1.4.3").expect("version");
            }),
        ),
        (
            "package_sha256",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.package_hash = PackageHash::parse(MANIFEST_DIGEST).expect("hash");
            }),
        ),
        (
            "package_bytes",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.package_bytes += 1;
            }),
        ),
        (
            "manifest_sha256",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.claimed_manifest_digest =
                    ManifestDigest::parse(PACKAGE_DIGEST).expect("digest");
            }),
        ),
        (
            "key_fingerprint",
            Box::new(|envelope: &mut SignatureEnvelope| {
                envelope.key_fingerprint =
                    PublisherKeyFingerprint::parse(PACKAGE_DIGEST).expect("fingerprint");
            }),
        ),
    ];
    for (field, mutate) in mutations {
        let mut mutated = envelope.clone();
        mutate(&mut mutated);
        assert_ne!(
            signed_payload(&mutated),
            signed_payload(&envelope),
            "{field} must be covered by the signature"
        );
    }
}

#[test]
fn the_payload_names_its_own_format_so_a_signature_cannot_be_replayed_from_elsewhere() {
    let payload = signed_payload(&signed(envelope_fields()));
    let text = String::from_utf8(payload).expect("canonical payload is text");
    assert!(
        text.starts_with("47:vanehub.extension-platform.package-signature.v1;"),
        "the context string must be the first thing signed: {text}"
    );
}

#[test]
fn the_pinned_verifier_accepts_the_rfc_8032_reference_vector() {
    // Checks the dependency, not our payload. If a future upgrade changed what `verify_strict`
    // accepts, every other test here would still pass by signing and verifying with the same new
    // behavior; this one would not.
    let public_key = decode_hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let signature = decode_hex(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e3970\
         1cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    let key: [u8; 32] = public_key.try_into().expect("32-byte key");
    let bytes: [u8; SIGNATURE_BYTES] = signature.try_into().expect("64-byte signature");

    let verifying = ed25519_dalek::VerifyingKey::from_bytes(&key).expect("valid key");
    assert!(verifying
        .verify_strict(b"", &ed25519_dalek::Signature::from_bytes(&bytes))
        .is_ok());
    assert!(
        verifying
            .verify_strict(b"x", &ed25519_dalek::Signature::from_bytes(&bytes))
            .is_err(),
        "the same signature over different bytes must not verify"
    );
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ascii"), 16).expect("hex byte")
        })
        .collect()
}

#[test]
fn a_signature_over_the_canonical_payload_verifies_against_the_trusted_key() {
    let envelope = signed(envelope_fields());
    let verified = verify_package_signature(
        &envelope,
        &key_record(PublisherTrustState::Trusted),
        &facts(),
    )
    .expect("signature verifies");

    assert_eq!(
        verified.key_fingerprint(),
        &key_record(PublisherTrustState::Trusted).fingerprint()
    );
    assert_eq!(
        verified.claimed_manifest_digest(),
        &ManifestDigest::parse(MANIFEST_DIGEST).expect("digest")
    );
}

#[test]
fn tampering_with_any_covered_field_invalidates_the_signature() {
    let signed_envelope = signed(envelope_fields());

    let mut swapped_version = signed_envelope.clone();
    swapped_version.version = Version::parse("9.9.9").expect("version");
    assert_eq!(
        verify_package_signature(
            &swapped_version,
            &key_record(PublisherTrustState::Trusted),
            &facts()
        ),
        Err(SignatureRejection::SignatureInvalid)
    );

    // The substitution this design exists to stop: a real signature by a real publisher, moved
    // onto a different set of bytes. Caught before the signature check, by the hash.
    let mut other_package = facts();
    other_package.hash = PackageHash::parse(MANIFEST_DIGEST).expect("hash");
    assert_eq!(
        verify_package_signature(
            &signed_envelope,
            &key_record(PublisherTrustState::Trusted),
            &other_package
        ),
        Err(SignatureRejection::PackageHashMismatch)
    );
}

#[test]
fn verification_reports_the_first_thing_that_is_wrong_in_a_fixed_order() {
    let envelope = signed(envelope_fields());
    let trusted = key_record(PublisherTrustState::Trusted);

    // A key filed under a fingerprint it does not hash to is a storage problem, and is reported
    // ahead of anything about the package — nothing else can be trusted until it is resolved.
    let mut wrong_key = trusted.clone();
    wrong_key.key = PublisherPublicKey::from_bytes([1_u8; PUBLISHER_KEY_BYTES]);
    assert_eq!(
        verify_package_signature(&envelope, &wrong_key, &facts()),
        Err(SignatureRejection::KeyFingerprintMismatch)
    );

    // Revocation outranks every package-level problem: the answer is the same regardless of what
    // else is wrong, and saying "hash mismatch" would invite a retry with a corrected package.
    let mut mismatched = facts();
    mismatched.byte_length += 1;
    assert_eq!(
        verify_package_signature(
            &envelope,
            &key_record(PublisherTrustState::Revoked),
            &mismatched
        ),
        Err(SignatureRejection::RevokedPublisherKey)
    );

    let mut other_publisher = trusted.clone();
    other_publisher.publisher = PublisherId::parse("other").expect("publisher");
    assert_eq!(
        verify_package_signature(&envelope, &other_publisher, &mismatched),
        Err(SignatureRejection::PublisherMismatch)
    );

    assert_eq!(
        verify_package_signature(&envelope, &trusted, &mismatched),
        Err(SignatureRejection::PackageLengthMismatch)
    );
}

/// Named return types, so a change in what verification hands back cannot pass unnoticed here.
fn verified_signature() -> VerifiedSignature {
    verify_package_signature(
        &signed(envelope_fields()),
        &key_record(PublisherTrustState::Trusted),
        &facts(),
    )
    .expect("signature verifies")
}

#[test]
fn the_two_trust_states_keep_the_spellings_that_reach_storage() {
    // These strings are written to and read back from SQLite in task 2.4. Changing one silently
    // reinterprets every stored row.
    assert_eq!(PublisherTrustState::Trusted.as_str(), "trusted");
    assert_eq!(PublisherTrustState::Revoked.as_str(), "revoked");
}

#[test]
fn a_verified_signature_becomes_evidence_only_once_the_manifest_matches() {
    let verified: VerifiedSignature = verify_package_signature(
        &signed(envelope_fields()),
        &key_record(PublisherTrustState::Trusted),
        &facts(),
    )
    .expect("signature verifies");

    let extracted = ManifestDigest::parse(MANIFEST_DIGEST).expect("digest");
    let substituted = ManifestDigest::parse(PACKAGE_DIGEST).expect("digest");

    assert_eq!(
        verified.clone().confirm_manifest(&substituted),
        Err(SignatureRejection::SignatureInvalid),
        "a real signature over a different manifest is still a substitution"
    );
    let confirmed: ConfirmedSignature = verified.confirm_manifest(&extracted).expect("confirmed");
    assert_eq!(confirmed.manifest_digest(), &extracted);
    assert_eq!(
        confirmed.key_fingerprint(),
        verified_signature().key_fingerprint()
    );
}

#[test]
fn every_rejection_has_a_distinct_stable_code() {
    let mut codes: Vec<&str> = ALL_SIGNATURE_REJECTIONS
        .iter()
        .map(|rejection| rejection.code())
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
