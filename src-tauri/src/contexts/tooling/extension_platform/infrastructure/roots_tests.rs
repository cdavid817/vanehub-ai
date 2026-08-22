//! The four roots: where each path lands, and what is refused before it is touched.

use super::{ExtensionRoots, RootError};
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionRootScope, ALL_EXTENSION_ROOT_SCOPES,
};
use crate::contexts::tooling::extension_platform::domain::{
    InstallationId, OperationWitness, PackageHash, RuntimeGenerationId,
};
use crate::platform::filesystem::OwnershipError;
use crate::test_support::TempDirectory;

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn roots(home: &TempDirectory) -> ExtensionRoots {
    ExtensionRoots::new(home.path().join("extensions"))
}

fn installation() -> InstallationId {
    InstallationId::parse("install-1").expect("installation id")
}

fn generation() -> RuntimeGenerationId {
    RuntimeGenerationId::parse("generation-1").expect("generation id")
}

#[test]
fn preparing_creates_all_four_roots_and_is_repeatable() {
    let home = TempDirectory::new("roots-prepare");
    let roots = roots(&home);

    roots.prepare().expect("prepare");
    for kind in ALL_EXTENSION_ROOT_SCOPES {
        assert!(roots.root(kind).is_dir(), "{kind:?}");
    }

    roots.prepare().expect("prepare again");
}

#[test]
fn each_kind_of_path_lands_where_the_design_says_it_does() {
    let home = TempDirectory::new("roots-layout");
    let roots = roots(&home);
    let base = roots.base().to_path_buf();

    assert_eq!(
        roots
            .quarantine(&OperationWitness::parse("operation-1").expect("witness"))
            .expect("quarantine"),
        base.join("quarantine").join("operation-1")
    );
    assert_eq!(
        roots
            .package(&PackageHash::parse(DIGEST).expect("hash"))
            .expect("package"),
        base.join("packages").join("sha256").join(DIGEST)
    );
    assert_eq!(
        roots
            .scratch(&installation(), &generation())
            .expect("scratch"),
        base.join("scratch").join("install-1").join("generation-1")
    );
    assert_eq!(
        roots
            .sidecar(&installation(), &generation())
            .expect("sidecar"),
        base.join("sidecars").join("install-1").join("generation-1")
    );
}

#[test]
fn the_four_roots_are_separate_directories() {
    // Their lifetimes differ, so a shared directory would make "is this safe to delete?" a
    // question nobody could answer.
    let home = TempDirectory::new("roots-separate");
    let roots = roots(&home);

    let mut paths: Vec<String> = ALL_EXTENSION_ROOT_SCOPES
        .iter()
        .map(|kind: &ExtensionRootScope| roots.root(*kind).to_string_lossy().to_string())
        .collect();
    let total = paths.len();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), total);
}

#[test]
fn a_directory_is_created_verified_and_discarded_through_the_ownership_checks() {
    let home = TempDirectory::new("roots-lifecycle");
    let roots = roots(&home);
    roots.prepare().expect("prepare");
    let scratch = roots
        .scratch(&installation(), &generation())
        .expect("scratch");

    roots.create(&scratch).expect("create");
    assert!(scratch.is_dir());
    roots.verify(&scratch).expect("verify");

    std::fs::write(scratch.join("work.bin"), b"content").expect("write");
    roots.discard(&scratch).expect("discard");
    assert!(!scratch.exists());
    roots
        .discard(&scratch)
        .expect("discarding twice is done, not an error");
}

#[test]
fn a_path_outside_the_base_is_refused() {
    let home = TempDirectory::new("roots-outside");
    let roots = roots(&home);
    roots.prepare().expect("prepare");
    let elsewhere = home.path().join("elsewhere");

    assert_eq!(
        roots.create(&elsewhere),
        Err(RootError::Ownership(OwnershipError::OutsideRoot))
    );
    assert_eq!(
        roots.discard(&elsewhere),
        Err(RootError::Ownership(OwnershipError::OutsideRoot))
    );
    assert!(!elsewhere.exists());
}

#[test]
fn an_identifier_that_cannot_be_a_path_segment_is_refused_before_a_path_is_built() {
    // The identifier rule permits ASCII alphanumerics, which includes `CON` — a device on Windows.
    // Application-generated ids do not look like that; one edited into a database by hand might.
    let home = TempDirectory::new("roots-unusable");
    let roots = roots(&home);

    assert_eq!(
        roots.quarantine(&OperationWitness::parse("CON").expect("witness")),
        Err(RootError::UnusableSegment)
    );
    assert_eq!(
        roots.scratch(
            &InstallationId::parse("CON").expect("installation"),
            &generation()
        ),
        Err(RootError::UnusableSegment)
    );
}

#[test]
fn every_error_has_a_distinct_stable_code() {
    let mut codes = vec![
        RootError::UnusableSegment.code(),
        RootError::Ownership(OwnershipError::OutsideRoot).code(),
        RootError::Ownership(OwnershipError::NotOwned).code(),
        RootError::Ownership(OwnershipError::Io).code(),
    ];
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
