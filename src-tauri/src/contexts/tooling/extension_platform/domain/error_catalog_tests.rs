//! The global failure-code invariant.
//!
//! Per-type distinctness is tested where each type lives. This is the check those cannot make:
//! that no two failures across the whole context present the same identity to a caller who
//! branches on it.

use super::{
    all_decode_reasons, all_developer_mode_errors, all_gate_errors, all_integrity_reasons,
    all_publisher_key_errors, registered_failures, ErrorArea, RegisteredFailure,
    ALL_ADMISSION_REFUSALS, ALL_IDENTIFIER_KINDS, ALL_ORIGIN_REJECTIONS, ALL_PATH_REJECTIONS,
    ALL_PUBLISHER_KEY_REJECTIONS, ALL_SIGNATURE_REJECTIONS,
};

#[test]
fn no_two_failures_share_an_identity() {
    let failures: Vec<RegisteredFailure> = registered_failures();
    let total = failures.len();

    let mut identities: Vec<&str> = failures
        .iter()
        .map(|failure| failure.identity.as_str())
        .collect();
    identities.sort_unstable();
    identities.dedup();

    assert_eq!(
        identities.len(),
        total,
        "two failures present the same identity; a caller branching on it cannot tell them apart"
    );
}

#[test]
fn path_and_origin_reasons_are_qualified_because_they_genuinely_overlap() {
    // `empty` and `too_long` are reasons for both, and both are right — only the outer code says
    // which value was empty. Qualifying keeps that instead of forcing one subsystem into an
    // uglier word for the same idea.
    let shared: Vec<&str> = ALL_PATH_REJECTIONS
        .iter()
        .map(|reason| reason.as_str())
        .filter(|path_reason| {
            ALL_ORIGIN_REJECTIONS
                .iter()
                .any(|origin| origin.as_str() == *path_reason)
        })
        .collect();
    assert!(
        !shared.is_empty(),
        "if these no longer overlap, the qualification below is no longer load-bearing"
    );

    let failures = registered_failures();
    for reason in shared {
        assert!(failures
            .iter()
            .any(|failure| failure.identity == format!("invalid_package_path:{reason}")));
        assert!(failures
            .iter()
            .any(|failure| failure.identity == format!("invalid_network_origin:{reason}")));
    }
}

#[test]
fn every_area_that_has_landed_contributes_at_least_one_failure() {
    let failures = registered_failures();

    for area in [
        ErrorArea::CapabilityGate,
        ErrorArea::Identity,
        ErrorArea::PackagePath,
        ErrorArea::NetworkOrigin,
        ErrorArea::ManifestDecode,
        ErrorArea::ManifestIntegrity,
        ErrorArea::PackageSignature,
        ErrorArea::PublisherKeyManagement,
        ErrorArea::PackageAdmission,
    ] {
        assert!(
            failures.iter().any(|failure| failure.area == area),
            "{} registers nothing",
            area.as_str()
        );
    }
}

#[test]
fn the_catalog_covers_every_variant_of_every_enum_it_registers() {
    // Guards the catalog against the drift it exists to prevent: a variant added to an error enum
    // and not to the list here would be invisible to the collision check.
    let failures = registered_failures();

    let counted = |area: ErrorArea| {
        failures
            .iter()
            .filter(|failure| failure.area == area)
            .count()
    };

    assert_eq!(counted(ErrorArea::CapabilityGate), all_gate_errors().len());
    assert_eq!(counted(ErrorArea::Identity), ALL_IDENTIFIER_KINDS.len());
    assert_eq!(counted(ErrorArea::PackagePath), ALL_PATH_REJECTIONS.len());
    assert_eq!(
        counted(ErrorArea::NetworkOrigin),
        ALL_ORIGIN_REJECTIONS.len()
    );
    assert_eq!(
        counted(ErrorArea::ManifestDecode),
        all_decode_reasons().len()
    );
    assert_eq!(
        counted(ErrorArea::ManifestIntegrity),
        all_integrity_reasons().len()
    );
    assert_eq!(
        counted(ErrorArea::PackageSignature),
        ALL_SIGNATURE_REJECTIONS.len()
    );
    assert_eq!(
        counted(ErrorArea::PublisherKeyManagement),
        ALL_PUBLISHER_KEY_REJECTIONS.len() + all_publisher_key_errors().len()
    );
    assert_eq!(
        counted(ErrorArea::PackageAdmission),
        ALL_ADMISSION_REFUSALS.len() + all_developer_mode_errors().len()
    );
}

#[test]
fn every_identity_is_lower_snake_case_and_bounded() {
    // Codes travel into logs, telemetry, and a frontend discriminated union. A code with a space
    // or an uppercase letter would be a wire break the first time something matched on it.
    for failure in registered_failures() {
        let identity = &failure.identity;
        assert!(!identity.is_empty(), "an empty identity is not an identity");
        assert!(
            identity.len() <= 64,
            "{identity} is too long to be a stable code"
        );
        assert!(
            identity
                .chars()
                .all(|character| character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '_'
                    || character == ':'),
            "{identity} is not lower_snake_case"
        );
        assert!(
            !identity.starts_with('_') && !identity.ends_with('_'),
            "{identity} has a stray underscore"
        );
    }
}

#[test]
fn the_catalog_is_stable_across_calls() {
    // Nothing here may depend on iteration order of a map or on a clock; a code list that
    // reshuffles would make a stored diagnostic unmatchable.
    assert_eq!(registered_failures(), registered_failures());
}
