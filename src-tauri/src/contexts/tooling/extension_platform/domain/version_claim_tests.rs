//! What a version claim binds, and what a second claim on the same version means.

use super::{
    decide_claim, ClaimOutcome, ClaimProvenance, ExtensionId, PackageHash, PublisherId,
    VersionClaim, VersionContentConflict,
};
use semver::Version;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn claim(hash: &str, provenance: ClaimProvenance, at: &str) -> VersionClaim {
    VersionClaim {
        publisher: PublisherId::parse("acme").expect("publisher"),
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
