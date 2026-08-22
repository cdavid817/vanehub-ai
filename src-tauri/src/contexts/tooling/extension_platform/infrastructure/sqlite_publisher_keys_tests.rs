//! What the publisher-key table does with rows that are fine, and with rows that are not.

use super::SqlitePublisherKeyRepository;
use crate::contexts::tooling::extension_platform::application::TrustedPublisherKeyRepository;
use crate::contexts::tooling::extension_platform::domain::{
    PublisherId, PublisherKeyFingerprint, PublisherKeyLabel, PublisherKeySource,
    PublisherPublicKey, PublisherTrustState, TrustedPublisherKey, PUBLISHER_KEY_BYTES,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::params;
use std::sync::Arc;

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    Fixture {
        _directory: directory,
        database,
    }
}

fn key(seed: u8) -> PublisherPublicKey {
    PublisherPublicKey::from_bytes([seed; PUBLISHER_KEY_BYTES])
}

fn trusted(seed: u8, publisher: &str, at: &str) -> TrustedPublisherKey {
    TrustedPublisherKey {
        publisher: PublisherId::parse(publisher).expect("publisher"),
        key: key(seed),
        label: PublisherKeyLabel::parse("Release signing").expect("label"),
        source: PublisherKeySource::ManualEntry,
        trust_state: PublisherTrustState::Trusted,
        first_seen_at: at.to_string(),
        last_seen_at: at.to_string(),
        revoked_at: None,
        revocation_reason: None,
    }
}

#[test]
fn a_key_round_trips_through_storage() {
    let fixture = fixture("publisher-keys-round-trip");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let record = trusted(3, "acme", "2026-08-22T00:00:00Z");

    repository.upsert(&record).expect("insert");

    assert_eq!(
        repository.find(&record.fingerprint()).expect("find"),
        Some(record.clone())
    );
    assert_eq!(repository.list().expect("list"), vec![record]);
}

#[test]
fn a_second_add_refreshes_provenance_without_moving_the_first_sighting() {
    let fixture = fixture("publisher-keys-reseen");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let first = trusted(4, "acme", "2026-08-01T00:00:00Z");
    repository.upsert(&first).expect("insert");

    let mut again = first.clone();
    again.label = PublisherKeyLabel::parse("Renamed").expect("label");
    again.source = PublisherKeySource::ImportedFile;
    again.last_seen_at = "2026-08-22T00:00:00Z".to_string();
    // A caller that got `first_seen_at` wrong must not be able to rewrite it through this path.
    again.first_seen_at = "2026-08-22T00:00:00Z".to_string();
    repository.upsert(&again).expect("update");

    let stored = repository
        .find(&first.fingerprint())
        .expect("find")
        .expect("stored");
    assert_eq!(stored.label.as_str(), "Renamed");
    assert_eq!(stored.source, PublisherKeySource::ImportedFile);
    assert_eq!(stored.last_seen_at, "2026-08-22T00:00:00Z");
    assert_eq!(stored.first_seen_at, "2026-08-01T00:00:00Z");
}

#[test]
fn revocation_is_recorded_once_and_keeps_the_moment_trust_was_withdrawn() {
    let fixture = fixture("publisher-keys-revoke");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let record = trusted(5, "acme", "2026-08-01T00:00:00Z");
    repository.upsert(&record).expect("insert");

    repository
        .revoke(
            &record.fingerprint(),
            "2026-08-10T00:00:00Z",
            Some("key rotated"),
        )
        .expect("revoke");
    repository
        .revoke(&record.fingerprint(), "2026-08-20T00:00:00Z", Some("again"))
        .expect("second revoke is a no-op");

    let stored = repository
        .find(&record.fingerprint())
        .expect("find")
        .expect("stored");
    assert_eq!(stored.trust_state, PublisherTrustState::Revoked);
    assert_eq!(stored.revoked_at.as_deref(), Some("2026-08-10T00:00:00Z"));
    assert_eq!(stored.revocation_reason.as_deref(), Some("key rotated"));
}

#[test]
fn a_revoked_key_is_not_resurrected_by_upserting_it_again() {
    // The service refuses this case before it gets here. The repository must not undo a revocation
    // either, because a second guard is what keeps the rule from depending on one caller.
    let fixture = fixture("publisher-keys-no-resurrection");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let record = trusted(6, "acme", "2026-08-01T00:00:00Z");
    repository.upsert(&record).expect("insert");
    repository
        .revoke(&record.fingerprint(), "2026-08-10T00:00:00Z", None)
        .expect("revoke");

    repository.upsert(&record).expect("re-add");

    let stored = repository
        .find(&record.fingerprint())
        .expect("find")
        .expect("stored");
    assert_eq!(stored.trust_state, PublisherTrustState::Revoked);
}

#[test]
fn a_row_that_no_longer_describes_a_key_is_dropped_rather_than_failing_every_read() {
    let fixture = fixture("publisher-keys-corrupt-row");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let good = trusted(7, "acme", "2026-08-01T00:00:00Z");
    repository.upsert(&good).expect("insert");

    let connection = fixture.database.connection().expect("connection");
    for (fingerprint, material, source, trust_state) in [
        // Key material that is not 32 bytes.
        (
            "a".repeat(64),
            STANDARD.encode([1_u8; 8]),
            "manual_entry",
            "trusted",
        ),
        // A source this build does not know.
        (
            "b".repeat(64),
            STANDARD.encode([2_u8; 32]),
            "registry",
            "trusted",
        ),
        // A trust state this build does not know.
        (
            "c".repeat(64),
            STANDARD.encode([3_u8; 32]),
            "manual_entry",
            "provisional",
        ),
    ] {
        connection
            .execute(
                "INSERT INTO extension_platform_publisher_keys \
                     (fingerprint, publisher, key_material, label, source, trust_state, \
                      first_seen_at, last_seen_at) \
                 VALUES (?1, 'acme', ?2, 'label', ?3, ?4, '2026-08-01T00:00:00Z', \
                         '2026-08-01T00:00:00Z')",
                params![fingerprint, material, source, trust_state],
            )
            .expect("insert unreadable row");
    }

    assert_eq!(repository.list().expect("list"), vec![good]);
}

#[test]
fn a_fingerprint_that_does_not_match_its_own_key_bytes_is_refused() {
    // Whoever edits the database file must not get to choose which key a fingerprint resolves to:
    // that is the whole substitution the fingerprint exists to prevent.
    let fixture = fixture("publisher-keys-forged-fingerprint");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());
    let honest = trusted(8, "acme", "2026-08-01T00:00:00Z");
    repository.upsert(&honest).expect("insert");

    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE extension_platform_publisher_keys SET key_material = ?2 WHERE fingerprint = ?1",
            params![
                honest.fingerprint().as_str(),
                STANDARD.encode(key(9).as_bytes())
            ],
        )
        .expect("swap the key under a fingerprint");

    assert_eq!(repository.find(&honest.fingerprint()).expect("find"), None);
    assert!(repository.list().expect("list").is_empty());
}

#[test]
fn an_absent_key_is_absent_rather_than_an_error() {
    let fixture = fixture("publisher-keys-absent");
    let repository = SqlitePublisherKeyRepository::new(fixture.database.clone());

    assert_eq!(
        repository
            .find(&PublisherKeyFingerprint::parse(&"f".repeat(64)).expect("fingerprint"))
            .expect("find"),
        None
    );
    assert!(repository.list().expect("list").is_empty());
}
