//! Reading key material and labels, and deciding what adding a key would mean.

use super::{
    decide_admission, parse_publisher_key_material, PublisherId, PublisherKeyAdmission,
    PublisherKeyLabel, PublisherKeyRejection, PublisherKeySource, PublisherPublicKey,
    PublisherTrustState, TrustedPublisherKey, ALL_PUBLISHER_KEY_REJECTIONS,
    MAX_PUBLISHER_KEY_LABEL_CHARACTERS, PUBLISHER_KEY_BYTES,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

fn stored(publisher: &str, trust_state: PublisherTrustState) -> TrustedPublisherKey {
    TrustedPublisherKey {
        publisher: PublisherId::parse(publisher).expect("publisher"),
        key: PublisherPublicKey::from_bytes([2_u8; PUBLISHER_KEY_BYTES]),
        label: PublisherKeyLabel::parse("Release signing").expect("label"),
        source: PublisherKeySource::ManualEntry,
        trust_state,
        first_seen_at: "2026-08-01T00:00:00Z".to_string(),
        last_seen_at: "2026-08-01T00:00:00Z".to_string(),
        revoked_at: None,
        revocation_reason: None,
    }
}

#[test]
fn a_fingerprint_is_derived_from_the_bytes_and_two_keys_never_share_one() {
    let first = PublisherPublicKey::from_bytes([1_u8; PUBLISHER_KEY_BYTES]);
    let second = PublisherPublicKey::from_bytes([2_u8; PUBLISHER_KEY_BYTES]);

    assert_eq!(first.fingerprint(), first.fingerprint());
    assert_ne!(first.fingerprint(), second.fingerprint());
    assert_eq!(first.fingerprint().as_str().len(), 64);
}

#[test]
fn key_material_is_standard_base64_of_exactly_thirty_two_bytes() {
    let bytes = [9_u8; PUBLISHER_KEY_BYTES];
    assert_eq!(
        parse_publisher_key_material(&STANDARD.encode(bytes)),
        Ok(PublisherPublicKey::from_bytes(bytes))
    );
    assert_eq!(
        parse_publisher_key_material(&format!("  {}  \n", STANDARD.encode(bytes))),
        Ok(PublisherPublicKey::from_bytes(bytes)),
        "surrounding whitespace survives a copy and paste and means nothing"
    );

    for wrong_length in [31_usize, 33] {
        assert_eq!(
            parse_publisher_key_material(&STANDARD.encode(vec![0_u8; wrong_length])),
            Err(PublisherKeyRejection::KeyMaterialWrongLength),
            "{wrong_length} bytes"
        );
    }

    // Hexadecimal is the other encoding an operator might reasonably paste. It is refused with a
    // specific diagnostic rather than accepted as some other 32 bytes.
    let hexadecimal = "00".repeat(PUBLISHER_KEY_BYTES);
    assert_eq!(
        parse_publisher_key_material(&hexadecimal),
        Err(PublisherKeyRejection::KeyMaterialWrongLength)
    );
    assert_eq!(
        parse_publisher_key_material("not base64!"),
        Err(PublisherKeyRejection::KeyMaterialNotBase64)
    );
    assert_eq!(
        parse_publisher_key_material(""),
        Err(PublisherKeyRejection::KeyMaterialWrongLength)
    );
}

#[test]
fn a_label_is_bounded_trimmed_and_free_of_control_characters() {
    assert_eq!(
        PublisherKeyLabel::parse("  Release signing  ")
            .expect("label")
            .as_str(),
        "Release signing"
    );
    assert_eq!(
        PublisherKeyLabel::parse(&"x".repeat(MAX_PUBLISHER_KEY_LABEL_CHARACTERS))
            .expect("label at the limit")
            .as_str()
            .chars()
            .count(),
        MAX_PUBLISHER_KEY_LABEL_CHARACTERS
    );
    assert_eq!(
        PublisherKeyLabel::parse(&"x".repeat(MAX_PUBLISHER_KEY_LABEL_CHARACTERS + 1)),
        Err(PublisherKeyRejection::LabelTooLong)
    );
    assert_eq!(
        PublisherKeyLabel::parse("   "),
        Err(PublisherKeyRejection::EmptyLabel)
    );
    // A label is rendered next to a trust decision; one carrying a newline or an escape sequence
    // is a way to make one key look like another.
    for hostile in ["two\nlines", "escape\u{1b}[31m", "null\u{0}byte"] {
        assert_eq!(
            PublisherKeyLabel::parse(hostile),
            Err(PublisherKeyRejection::LabelControlCharacter),
            "{hostile:?}"
        );
    }
    // The limit is characters, not bytes: a label of emoji is as long as it looks.
    assert!(PublisherKeyLabel::parse(&"🔑".repeat(MAX_PUBLISHER_KEY_LABEL_CHARACTERS)).is_ok());
}

#[test]
fn adding_an_unknown_key_establishes_new_trust() {
    let publisher = PublisherId::parse("acme").expect("publisher");
    assert_eq!(
        decide_admission(&publisher, None),
        PublisherKeyAdmission::New
    );
    assert!(PublisherKeyAdmission::New.admits_write());
}

#[test]
fn adding_a_key_that_is_already_trusted_changes_no_trust_decision() {
    let publisher = PublisherId::parse("acme").expect("publisher");
    let existing = stored("acme", PublisherTrustState::Trusted);

    let admission = decide_admission(&publisher, Some(&existing));
    assert_eq!(admission, PublisherKeyAdmission::AlreadyTrusted);
    assert!(admission.admits_write());
}

#[test]
fn a_revoked_key_is_not_re_trusted_by_adding_it_again() {
    // Revocation is a deliberate withdrawal. An "add" that reversed it would make revocation
    // depend on nobody pasting the key a second time.
    let publisher = PublisherId::parse("acme").expect("publisher");
    let existing = stored("acme", PublisherTrustState::Revoked);

    let admission = decide_admission(&publisher, Some(&existing));
    assert_eq!(admission, PublisherKeyAdmission::Revoked);
    assert!(!admission.admits_write());
}

#[test]
fn one_key_claimed_by_two_publishers_is_refused_rather_than_resolved() {
    let publisher = PublisherId::parse("other").expect("publisher");
    let existing = stored("acme", PublisherTrustState::Trusted);

    let admission = decide_admission(&publisher, Some(&existing));
    assert_eq!(
        admission,
        PublisherKeyAdmission::ClaimedByAnotherPublisher {
            existing: PublisherId::parse("acme").expect("publisher"),
        }
    );
    assert!(!admission.admits_write());
    assert_eq!(
        decide_admission(
            &publisher,
            Some(&stored("acme", PublisherTrustState::Revoked))
        ),
        PublisherKeyAdmission::ClaimedByAnotherPublisher {
            existing: PublisherId::parse("acme").expect("publisher"),
        },
        "who holds the key is settled before whether it is revoked"
    );
}

#[test]
fn the_verification_view_carries_nothing_but_what_a_signature_check_may_read() {
    let key = stored("acme", PublisherTrustState::Trusted);
    let record = key.for_verification();

    assert_eq!(record.publisher, key.publisher);
    assert_eq!(record.trust_state, key.trust_state);
    assert_eq!(record.fingerprint(), key.fingerprint());
}

#[test]
fn every_rejection_and_admission_has_a_distinct_stable_code() {
    let mut codes: Vec<&str> = ALL_PUBLISHER_KEY_REJECTIONS
        .iter()
        .map(|rejection| rejection.code())
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);

    let admissions = [
        PublisherKeyAdmission::New,
        PublisherKeyAdmission::AlreadyTrusted,
        PublisherKeyAdmission::Revoked,
        PublisherKeyAdmission::ClaimedByAnotherPublisher {
            existing: PublisherId::parse("acme").expect("publisher"),
        },
    ];
    let mut admission_codes: Vec<&str> =
        admissions.iter().map(PublisherKeyAdmission::code).collect();
    let admission_total = admission_codes.len();
    admission_codes.sort_unstable();
    admission_codes.dedup();
    assert_eq!(admission_codes.len(), admission_total);
}

#[test]
fn the_stored_source_spellings_are_the_ones_that_reach_the_database() {
    for source in [
        PublisherKeySource::ManualEntry,
        PublisherKeySource::ImportedFile,
    ] {
        assert_eq!(PublisherKeySource::parse(source.as_str()), Some(source));
    }
    assert_eq!(PublisherKeySource::parse("registry"), None);
}
