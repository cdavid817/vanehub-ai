//! What a version claim binds, and what a second claim on the same version means.

use super::{
    decide_claim, ClaimAuthority, ClaimOutcome, ClaimProvenance, ExtensionId, PackageHash,
    PublisherId, PublisherKeyRecord, PublisherPublicKey, PublisherTrustState, VersionClaim,
    VersionContentConflict, LOCAL_DEVELOPER_NAMESPACE, PUBLISHER_KEY_BYTES,
};
use semver::Version;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// An authority established the only way one can be: from a stored key record.
fn verified(publisher: &str) -> ClaimAuthority {
    ClaimAuthority::of_verified_key(&PublisherKeyRecord {
        publisher: PublisherId::parse(publisher).expect("publisher"),
        key: PublisherPublicKey::from_bytes([1_u8; PUBLISHER_KEY_BYTES]),
        trust_state: PublisherTrustState::Trusted,
    })
}

fn claim(hash: &str, provenance: ClaimProvenance, at: &str) -> VersionClaim {
    VersionClaim {
        authority: verified("acme"),
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(hash).expect("hash"),
        provenance,
        first_claimed_at: at.to_string(),
    }
}

#[test]
fn an_unclaimed_version_is_bound_by_the_first_package_to_claim_it() {
    let outcome = decide_claim(
        &claim(FIRST, ClaimProvenance::Signed, "2026-08-01T00:00:00Z"),
        None,
    );

    assert_eq!(outcome, ClaimOutcome::Bound);
    assert!(outcome.admits_snapshot());
}

#[test]
fn reinstalling_the_identical_package_is_idempotent() {
    let held = claim(FIRST, ClaimProvenance::Signed, "2026-08-01T00:00:00Z");
    let again = claim(FIRST, ClaimProvenance::Signed, "2026-08-20T00:00:00Z");

    let outcome = decide_claim(&again, Some(&held));

    assert_eq!(outcome, ClaimOutcome::AlreadyBound);
    assert!(outcome.admits_snapshot());
}

#[test]
fn the_same_version_with_different_bytes_is_refused_and_both_hashes_are_reported() {
    // The whole content of the finding is which bytes hold the version and which were offered for
    // it; a conflict that said only "conflict" would leave an operator with nothing to compare.
    let held = claim(FIRST, ClaimProvenance::Signed, "2026-08-01T00:00:00Z");
    let offered = claim(SECOND, ClaimProvenance::Signed, "2026-08-20T00:00:00Z");

    let outcome = decide_claim(&offered, Some(&held));

    assert_eq!(
        outcome,
        ClaimOutcome::Conflict(VersionContentConflict {
            bound_hash: PackageHash::parse(FIRST).expect("hash"),
            offered_hash: PackageHash::parse(SECOND).expect("hash"),
            bound_provenance: ClaimProvenance::Signed,
            bound_at: "2026-08-01T00:00:00Z".to_string(),
        })
    );
    assert!(
        !outcome.admits_snapshot(),
        "no activatable snapshot may be created for a version held by other bytes"
    );
    assert_eq!(outcome.code(), "version_content_conflict");
    let ClaimOutcome::Conflict(conflict) = &outcome else {
        panic!("expected a conflict");
    };
    assert_eq!(
        conflict.code(),
        outcome.code(),
        "the conflict and the outcome name the same finding"
    );
}

#[test]
fn developer_mode_does_not_get_to_overwrite_a_version_in_place() {
    // A build loop that reuses a version number is how an unreviewed change reaches an installed
    // extension, so the rule is the same for unsigned content. Provenance is recorded, not
    // consulted.
    let cases = [
        (ClaimProvenance::Unsigned, ClaimProvenance::Unsigned),
        (ClaimProvenance::Unsigned, ClaimProvenance::Signed),
        (ClaimProvenance::Signed, ClaimProvenance::Unsigned),
    ];

    for (held, offered) in cases {
        let outcome = decide_claim(
            &claim(SECOND, offered, "2026-08-20T00:00:00Z"),
            Some(&claim(FIRST, held, "2026-08-01T00:00:00Z")),
        );
        assert!(
            !outcome.admits_snapshot(),
            "{held:?} then {offered:?} must not be admitted"
        );
    }
}

#[test]
fn provenance_round_trips_through_the_spellings_that_reach_storage() {
    for provenance in [ClaimProvenance::Signed, ClaimProvenance::Unsigned] {
        assert_eq!(
            ClaimProvenance::parse(provenance.as_str()),
            Some(provenance)
        );
    }
    assert_eq!(ClaimProvenance::parse("developer"), None);
}

#[test]
fn every_outcome_has_a_distinct_stable_code() {
    let outcomes = [
        ClaimOutcome::Bound,
        ClaimOutcome::AlreadyBound,
        ClaimOutcome::Conflict(VersionContentConflict {
            bound_hash: PackageHash::parse(FIRST).expect("hash"),
            offered_hash: PackageHash::parse(SECOND).expect("hash"),
            bound_provenance: ClaimProvenance::Signed,
            bound_at: String::new(),
        }),
    ];
    let mut codes: Vec<&str> = outcomes.iter().map(ClaimOutcome::code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}

#[test]
fn a_claim_is_filed_under_a_verified_key_never_under_the_manifests_publisher_field() {
    // The manifest's `publisher` is a string inside a file the claimant wrote. Filing under it
    // would let a local build take the version binding for a real publisher and make every later
    // genuine release a conflict -- a cheap denial of service, purchased with a text editor.
    //
    // There is no constructor that takes it. The only two ways to get an authority are a stored
    // key record and the host's own namespace, and this asserts the first still says what the
    // *record* says rather than what any package asked for.
    let record = PublisherKeyRecord {
        publisher: PublisherId::parse("real-publisher").expect("publisher"),
        key: PublisherPublicKey::from_bytes([2_u8; PUBLISHER_KEY_BYTES]),
        trust_state: PublisherTrustState::Trusted,
    };

    let authority = ClaimAuthority::of_verified_key(&record);

    assert_eq!(authority.as_str(), "real-publisher");
    assert!(authority.is_verified());
}

#[test]
fn unsigned_content_is_filed_under_a_host_namespace_that_no_publisher_can_occupy() {
    let local = ClaimAuthority::LocalDeveloper;

    assert_eq!(local.as_str(), LOCAL_DEVELOPER_NAMESPACE);
    assert!(!local.is_verified());
    // The namespace contains a colon, and `PublisherId` admits only lowercase letters, digits, and
    // hyphens. So it is not a name a verified publisher could ever hold, and not a name a manifest
    // could ask to be filed under.
    assert!(
        PublisherId::parse(LOCAL_DEVELOPER_NAMESPACE).is_err(),
        "the reserved namespace must be unrepresentable as a publisher"
    );
}

#[test]
fn an_authority_read_back_from_storage_is_a_publisher_or_the_host_namespace_and_nothing_else() {
    assert_eq!(
        ClaimAuthority::parse(LOCAL_DEVELOPER_NAMESPACE),
        Some(ClaimAuthority::LocalDeveloper)
    );
    assert_eq!(
        ClaimAuthority::parse("acme"),
        Some(ClaimAuthority::VerifiedPublisher(
            PublisherId::parse("acme").expect("publisher")
        ))
    );
    // A hand-edited row naming something that is neither is refused rather than read back as a
    // publisher nobody vetted.
    for forged in ["Acme Corp", "local:other", "", "acme."] {
        assert_eq!(ClaimAuthority::parse(forged), None, "{forged:?}");
    }
}

#[test]
fn a_local_build_cannot_take_a_verified_publishers_version_binding() {
    // The two file under different keys, so a local build claiming 1.2.0 does not touch the
    // binding a verified publisher holds for the same extension and version.
    let signed = VersionClaim {
        authority: verified("acme"),
        ..claim(FIRST, ClaimProvenance::Signed, "2026-08-01T00:00:00Z")
    };
    let local = VersionClaim {
        authority: ClaimAuthority::LocalDeveloper,
        ..claim(SECOND, ClaimProvenance::Unsigned, "2026-08-20T00:00:00Z")
    };

    assert_ne!(
        signed.authority.as_str(),
        local.authority.as_str(),
        "different authorities are different rows, so neither can squat the other's version"
    );
}
