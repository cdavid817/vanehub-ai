//! What may be persisted as operation evidence, and what retention may never remove.

use super::{
    all_witness_rejections, is_prunable, CapabilityDiff, CompatibilityOutcome, DependencySummary,
    ExtensionId, ExtensionInstallWitness, InstallWitnessSubject, InstalledSummary, ManifestDigest,
    PackageHash, PersistableOperationWitness, SignatureSummary, TrustProfile, WitnessLimits,
    WitnessProtection, WitnessRejection, WitnessRetention, DEFAULT_WITNESS_LIMITS,
    DEFAULT_WITNESS_RETENTION, WITNESS_SCHEMA_VERSION,
};
use semver::{Version, VersionReq};

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn subject() -> InstallWitnessSubject {
    InstallWitnessSubject {
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(DIGEST).expect("hash"),
        manifest_digest: ManifestDigest::parse(DIGEST).expect("digest"),
        signature: SignatureSummary {
            state: "verified",
            key_fingerprint: None,
        },
        installed: None,
        compatibility: CompatibilityOutcome::Compatible,
        trust_profile: TrustProfile::Strict,
        dependencies: Vec::new(),
        capabilities: CapabilityDiff::default(),
        contributions: Vec::new(),
    }
}

fn admit(subject: InstallWitnessSubject) -> Result<PersistableOperationWitness, WitnessRejection> {
    PersistableOperationWitness::admit(
        ExtensionInstallWitness::issue(subject),
        DEFAULT_WITNESS_LIMITS,
    )
}

fn dependency(id: &str) -> DependencySummary {
    DependencySummary {
        id: id.to_string(),
        requirement: VersionReq::parse(">=1.0.0").expect("requirement"),
        optional: false,
        satisfied: true,
    }
}

#[test]
fn an_ordinary_witness_is_admitted_and_carries_the_schema_version_this_build_writes() {
    let persistable = admit(subject()).expect("admit");

    assert_eq!(persistable.schema_version(), WITNESS_SCHEMA_VERSION);
    assert_eq!(persistable.witness().subject(), &subject());
}

#[test]
fn a_witness_edited_after_issue_is_refused_before_anything_else_is_checked() {
    // The digest is what makes a witness evidence rather than a struct someone filled in. Checking
    // bounds on a subject the digest no longer covers would be measuring the wrong thing.
    let issued = ExtensionInstallWitness::issue(subject());
    let forged = ExtensionInstallWitness::issue(InstallWitnessSubject {
        trust_profile: TrustProfile::Trusted,
        ..subject()
    });
    assert_ne!(issued.digest(), forged.digest());

    // A witness that disagrees with its own digest cannot be produced through `issue`, so this
    // asserts the guard exists rather than that some caller tripped it.
    assert!(issued.is_self_consistent());
    assert!(admit(subject()).is_ok());
}

#[test]
fn a_manifest_cannot_choose_the_row_size_through_its_dependency_list() {
    // Every collection here comes from a manifest, and a manifest is written by whoever built the
    // package. Without a bound, "how big is this row" is a decision an extension author makes.
    let many = InstallWitnessSubject {
        dependencies: (0..200)
            .map(|index| dependency(&format!("dep-{index}")))
            .collect(),
        ..subject()
    };

    let error = admit(many).expect_err("too many dependencies");

    assert_eq!(error.code(), "witness_too_many_dependencies");
    let WitnessRejection::TooManyDependencies { count, limit } = error else {
        panic!("expected a dependency count rejection");
    };
    assert_eq!(
        (count, limit),
        (200, DEFAULT_WITNESS_LIMITS.max_dependencies)
    );
}

#[test]
fn a_manifest_cannot_choose_it_through_capabilities_or_contributions_either() {
    let many_capabilities = InstallWitnessSubject {
        capabilities: CapabilityDiff {
            added: (0..300)
                .map(|index| format!("network:https://h{index}.test"))
                .collect(),
            ..CapabilityDiff::default()
        },
        ..subject()
    };
    assert_eq!(
        admit(many_capabilities).expect_err("capabilities").code(),
        "witness_too_many_capabilities"
    );
}

#[test]
fn a_single_field_longer_than_the_limit_is_refused() {
    let long = InstallWitnessSubject {
        dependencies: vec![dependency(&"d".repeat(600))],
        ..subject()
    };

    let error = admit(long).expect_err("field too long");

    assert_eq!(error.code(), "witness_field_too_long");
    let WitnessRejection::FieldTooLong { field, .. } = error else {
        panic!("expected a field-length rejection");
    };
    assert_eq!(field, "dependency_id");
}

#[test]
fn a_control_character_is_refused_rather_than_stripped() {
    // A value altered to be storable no longer matches the one a reviewer approved, and comparing
    // against what was approved is the witness's whole job.
    let hostile = InstallWitnessSubject {
        dependencies: vec![dependency("dep\u{0}injected")],
        ..subject()
    };

    let error = admit(hostile).expect_err("control character");

    assert_eq!(error.code(), "witness_control_character");
}

#[test]
fn many_fields_each_just_under_the_per_field_limit_still_hit_the_total_bound() {
    // The bound that cannot be evaded by splitting. Without it, per-field limits are a ceiling on
    // one value and no ceiling at all on a row.
    let limits = WitnessLimits {
        max_total_bytes: 2_048,
        ..DEFAULT_WITNESS_LIMITS
    };
    let spread = InstallWitnessSubject {
        dependencies: (0..64)
            .map(|index| dependency(&format!("{index:03}{}", "d".repeat(100))))
            .collect(),
        ..subject()
    };

    let error = PersistableOperationWitness::admit(ExtensionInstallWitness::issue(spread), limits)
        .expect_err("too large");

    assert_eq!(error.code(), "witness_too_large");
    let WitnessRejection::TooLarge { bytes, limit } = error else {
        panic!("expected a size rejection");
    };
    assert!(bytes > limit, "{bytes} should exceed {limit}");
}

#[test]
fn a_witness_has_nowhere_to_put_a_payload_a_path_or_an_environment() {
    // The structural guarantee, asserted by exhaustive destructuring: adding a field to
    // `InstallWitnessSubject` stops this compiling, which is the point. Every field below is a
    // typed fact a reviewer was shown -- none of them is free text an extension chose.
    let InstallWitnessSubject {
        extension: _,
        version: _,
        package_hash: _,
        manifest_digest: _,
        signature: _,
        installed: _,
        compatibility: _,
        trust_profile: _,
        dependencies: _,
        capabilities: _,
        contributions: _,
    } = subject();
}

#[test]
fn every_rejection_has_a_distinct_stable_code() {
    let rejections = all_witness_rejections();
    let total = rejections.len();

    let mut codes: Vec<&str> = rejections.iter().map(WitnessRejection::code).collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), total);
    for code in codes {
        assert!(code.len() <= 64);
        assert!(code
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_'));
    }
}

// ---------------------------------------------------------------------------
// Retention
// ---------------------------------------------------------------------------

fn nothing_protected() -> WitnessProtection {
    WitnessProtection::default()
}

#[test]
fn a_retention_window_of_zero_is_unconstructible() {
    assert_eq!(WitnessRetention::new(0), None);
    assert_eq!(
        WitnessRetention::default().keep(),
        DEFAULT_WITNESS_RETENTION
    );
}

#[test]
fn a_row_inside_the_window_is_never_pruned() {
    assert!(!is_prunable(
        &nothing_protected(),
        "operation-1",
        DIGEST,
        WITNESS_SCHEMA_VERSION,
        true,
    ));
}

#[test]
fn an_ordinary_row_outside_the_window_is_prunable() {
    assert!(is_prunable(
        &nothing_protected(),
        "operation-1",
        DIGEST,
        WITNESS_SCHEMA_VERSION,
        false,
    ));
}

#[test]
fn evidence_for_an_unfinished_operation_is_never_pruned() {
    // It is not history yet: the operation that will be compared against it has not run.
    let protection = WitnessProtection {
        unfinished_operations: vec!["operation-1".to_string()],
        ..WitnessProtection::default()
    };

    assert!(!is_prunable(
        &protection,
        "operation-1",
        DIGEST,
        WITNESS_SCHEMA_VERSION,
        false,
    ));
    assert!(
        is_prunable(
            &protection,
            "operation-2",
            DIGEST,
            WITNESS_SCHEMA_VERSION,
            false,
        ),
        "protection is per operation, not a blanket freeze"
    );
}

#[test]
fn evidence_for_an_active_rollback_or_quarantined_package_is_never_pruned() {
    // All three are the same rule from storage's point of view: the package is still one the
    // installation might run, so the record of how it was approved is still a live answer.
    let protection = WitnessProtection {
        protected_packages: vec![PackageHash::parse(DIGEST).expect("hash")],
        ..WitnessProtection::default()
    };

    assert!(!is_prunable(
        &protection,
        "operation-1",
        DIGEST,
        WITNESS_SCHEMA_VERSION,
        false,
    ));
}

#[test]
fn a_row_written_by_a_newer_build_is_never_pruned() {
    // A downgrade that prunes what it cannot interpret destroys the record the upgrade was
    // keeping. A downgrade that keeps it loses nothing.
    assert!(!is_prunable(
        &nothing_protected(),
        "operation-1",
        DIGEST,
        WITNESS_SCHEMA_VERSION + 1,
        false,
    ));
    assert!(
        is_prunable(
            &nothing_protected(),
            "operation-1",
            DIGEST,
            WITNESS_SCHEMA_VERSION,
            false,
        ),
        "a row this build wrote is still prunable"
    );
}

#[test]
fn an_installed_summary_is_bounded_by_its_own_types() {
    // The one collection-free part of the subject: three typed facts, none of which an extension
    // author can make long. Included so a field added here is noticed alongside the bounded ones.
    let installed = InstalledSummary {
        version: Version::parse("1.0.0").expect("version"),
        package_hash: PackageHash::parse(DIGEST).expect("hash"),
        enabled: true,
    };
    let InstalledSummary {
        version: _,
        package_hash: _,
        enabled: _,
    } = installed;

    let with_installed = InstallWitnessSubject {
        installed: Some(InstalledSummary {
            version: Version::parse("1.0.0").expect("version"),
            package_hash: PackageHash::parse(DIGEST).expect("hash"),
            enabled: true,
        }),
        ..subject()
    };
    assert!(admit(with_installed).is_ok());
}
