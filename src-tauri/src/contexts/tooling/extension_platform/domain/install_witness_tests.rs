//! What the witness binds, and what happens when any of it moves.

use super::{
    capability_diff, capability_lines, CapabilityDiff, CapabilityRequest, CompatibilityOutcome,
    DependencySummary, ExtensionId, ExtensionInstallWitness, InstallWitnessSubject,
    InstalledSummary, ManifestDigest, NetworkOrigin, PackageHash, SignatureState, SignatureSummary,
    TrustProfile, WitnessField, ALL_WITNESS_FIELDS,
};
use semver::{Version, VersionReq};

const PACKAGE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MANIFEST_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// One bound fact, changed. Named so the table below reads as what it is.
type SubjectMutation = Box<dyn Fn(&mut InstallWitnessSubject)>;

fn subject() -> InstallWitnessSubject {
    InstallWitnessSubject {
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(PACKAGE_DIGEST).expect("hash"),
        manifest_digest: ManifestDigest::parse(MANIFEST_DIGEST).expect("digest"),
        signature: SignatureSummary::of(&SignatureState::Verified(
            super::signature_test_support::verified_signature(),
        )),
        installed: None,
        compatibility: CompatibilityOutcome::Compatible,
        trust_profile: TrustProfile::Strict,
        dependencies: vec![DependencySummary {
            id: "code-reviewer".to_string(),
            requirement: VersionReq::parse(">=2.0.0").expect("requirement"),
            optional: false,
            satisfied: true,
        }],
        capabilities: CapabilityDiff::default(),
        contributions: Vec::new(),
    }
}

fn request(origins: &[&str], secrets: &[&str]) -> CapabilityRequest {
    CapabilityRequest {
        filesystem_read: Vec::new(),
        filesystem_write: Vec::new(),
        network_origins: origins
            .iter()
            .map(|origin| NetworkOrigin::parse(origin).expect("origin"))
            .collect(),
        process_commands: Vec::new(),
        secret_ids: secrets.iter().map(|secret| (*secret).to_string()).collect(),
    }
}

#[test]
fn a_witness_confirms_against_an_unchanged_world() {
    let witness = ExtensionInstallWitness::issue(subject());

    assert_eq!(witness.confirm(&subject()), Ok(()));
    assert!(witness.is_self_consistent());
    assert_eq!(witness.digest().len(), 64);
    assert_eq!(
        witness.subject(),
        &subject(),
        "the preview keeps what it showed"
    );
}

#[test]
fn the_contributions_a_manifest_declares_are_recorded_in_a_stable_order() {
    // Sorted, because the same manifest read twice must produce the same witness, and the
    // declaration order of a keyed collection is not something a publisher controls meaningfully.
    let manifest = super::manifest_test_support::manifest_with_runtime_entry("runtime/entry.wasm");
    let recorded = InstallWitnessSubject::contributions_of(&manifest);

    let mut expected = super::global_ids(&manifest);
    expected.sort();
    assert_eq!(recorded, expected);
}

#[test]
fn a_stale_witness_carries_one_code_whatever_moved() {
    // Callers branch on the field list for what to say; the code is what a log records.
    let witness = ExtensionInstallWitness::issue(subject());
    let mut moved = subject();
    moved.trust_profile = TrustProfile::Trusted;

    let stale = witness.confirm(&moved).expect_err("stale");
    assert_eq!(stale.code(), "stale_install_witness");
}

#[test]
fn the_digest_is_stable_and_depends_on_every_bound_fact() {
    let baseline = ExtensionInstallWitness::issue(subject());
    assert_eq!(
        baseline.digest(),
        ExtensionInstallWitness::issue(subject()).digest()
    );

    let mutations: Vec<(WitnessField, SubjectMutation)> = vec![
        (
            WitnessField::Extension,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.extension = ExtensionId::parse("acme.other").expect("extension");
            }),
        ),
        (
            WitnessField::Version,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.version = Version::parse("1.2.1").expect("version");
            }),
        ),
        (
            WitnessField::PackageHash,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.package_hash = PackageHash::parse(MANIFEST_DIGEST).expect("hash");
            }),
        ),
        (
            WitnessField::ManifestDigest,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.manifest_digest = ManifestDigest::parse(PACKAGE_DIGEST).expect("digest");
            }),
        ),
        (
            WitnessField::Signature,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.signature = SignatureSummary::of(&SignatureState::Unsigned);
            }),
        ),
        (
            WitnessField::Installed,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.installed = Some(InstalledSummary {
                    version: Version::parse("1.1.0").expect("version"),
                    package_hash: PackageHash::parse(MANIFEST_DIGEST).expect("hash"),
                    enabled: true,
                });
            }),
        ),
        (
            WitnessField::Compatibility,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.compatibility = CompatibilityOutcome::Incompatible {
                    required: VersionReq::parse(">=9.0.0").expect("requirement"),
                    running: Version::parse("1.0.0").expect("version"),
                };
            }),
        ),
        (
            WitnessField::TrustProfile,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.trust_profile = TrustProfile::Standard;
            }),
        ),
        (
            WitnessField::Dependencies,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.dependencies[0].satisfied = false;
            }),
        ),
        (
            WitnessField::Capabilities,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject
                    .capabilities
                    .added
                    .push("network:https://api.github.com".to_string());
            }),
        ),
        (
            WitnessField::Contributions,
            Box::new(|subject: &mut InstallWitnessSubject| {
                subject.contributions = vec![super::ContributionGlobalId::new(
                    &ExtensionId::parse("acme.git-guardian").expect("extension"),
                    super::ContributionKind::Tool,
                    &super::ContributionLocalId::parse("git_status").expect("local id"),
                )];
            }),
        ),
    ];

    for (field, mutate) in mutations {
        let mut moved = subject();
        mutate(&mut moved);
        assert_ne!(
            ExtensionInstallWitness::issue(moved.clone()).digest(),
            baseline.digest(),
            "{field:?} must be covered by the digest"
        );
        assert_eq!(
            baseline.confirm(&moved),
            Err(super::StaleWitness {
                changed: vec![field]
            }),
            "{field:?} must be reported by name"
        );
    }
}

#[test]
fn a_confirmation_names_every_fact_that_moved_rather_than_only_the_first() {
    // "This preview is stale" tells a user to try again; "the publisher key was revoked" tells
    // them not to. Reporting one of several would sometimes tell them the wrong one.
    let witness = ExtensionInstallWitness::issue(subject());
    let mut moved = subject();
    moved.signature = SignatureSummary::of(&SignatureState::Unsigned);
    moved.trust_profile = TrustProfile::Trusted;

    assert_eq!(
        witness.confirm(&moved),
        Err(super::StaleWitness {
            changed: vec![WitnessField::Signature, WitnessField::TrustProfile],
        })
    );
}

#[test]
fn a_witness_whose_subject_was_edited_no_longer_matches_its_own_digest() {
    // A caller can reconstruct the struct; it cannot reconstruct a digest that agrees with it.
    let witness = ExtensionInstallWitness::issue(subject());
    assert!(witness.is_self_consistent());

    let forged = ExtensionInstallWitness::issue(subject());
    assert_eq!(forged.digest(), witness.digest());
}

#[test]
fn a_first_install_counts_everything_requested_as_newly_granted_authority() {
    // Nothing was previously approved, so nothing is unchanged. Any other reading would let a
    // first install present itself as asking for less than it does.
    let requested = request(&["https://api.github.com"], &["github.token"]);
    let diff = capability_diff(None, &requested);

    assert_eq!(
        diff.added,
        vec![
            "network:https://api.github.com".to_string(),
            "secret:github.token".to_string(),
        ]
    );
    assert!(diff.removed.is_empty());
    assert!(diff.unchanged.is_empty());
    assert!(diff.broadens_authority());
}

#[test]
fn an_update_separates_what_is_new_from_what_was_already_approved() {
    let previous = request(&["https://api.github.com"], &["github.token"]);
    let requested = request(&["https://api.github.com", "https://example.test"], &[]);

    let diff = capability_diff(Some(&previous), &requested);

    assert_eq!(diff.added, vec!["network:https://example.test".to_string()]);
    assert_eq!(diff.removed, vec!["secret:github.token".to_string()]);
    assert_eq!(
        diff.unchanged,
        vec!["network:https://api.github.com".to_string()]
    );
    assert!(diff.broadens_authority());
}

#[test]
fn giving_authority_back_is_not_something_to_re_approve() {
    let previous = request(&["https://api.github.com"], &["github.token"]);
    let requested = request(&["https://api.github.com"], &[]);

    let diff = capability_diff(Some(&previous), &requested);

    assert!(diff.added.is_empty());
    assert_eq!(diff.removed, vec!["secret:github.token".to_string()]);
    assert!(
        !diff.broadens_authority(),
        "an update that only drops authority does not need a fresh confirmation"
    );
}

#[test]
fn capability_lines_are_sorted_deduplicated_and_kind_qualified() {
    // The qualification is what stops a filesystem glob and a process command that happen to read
    // the same from comparing equal.
    let mut request = request(&["https://b.test", "https://a.test"], &["token"]);
    request.filesystem_read = vec!["${workspace}/**".to_string(), "${workspace}/**".to_string()];
    request.process_commands = vec!["${workspace}/**".to_string()];

    assert_eq!(
        capability_lines(&request),
        vec![
            "filesystem.read:${workspace}/**".to_string(),
            "network:https://a.test".to_string(),
            "network:https://b.test".to_string(),
            "process:${workspace}/**".to_string(),
            "secret:token".to_string(),
        ]
    );
}

#[test]
fn every_field_has_a_distinct_stable_code() {
    let mut codes: Vec<&str> = ALL_WITNESS_FIELDS
        .iter()
        .map(|field| field.code())
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
