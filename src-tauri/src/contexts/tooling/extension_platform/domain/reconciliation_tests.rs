//! What a restart may collect, and everything it must leave alone.

use super::{
    judge_entry, ExtensionRootScope, ReconciliationReason, ReconciliationSummary,
    ReconciliationVerdict, ALL_EXTENSION_ROOT_SCOPES, ALL_RECONCILIATION_REASONS,
};
use std::collections::BTreeSet;

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const OTHER: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn referenced(hashes: &[&str]) -> BTreeSet<String> {
    hashes.iter().map(|hash| (*hash).to_string()).collect()
}

#[test]
fn quarantine_scratch_and_sidecar_entries_are_collected_because_none_survives_a_restart() {
    let none = referenced(&[]);

    assert_eq!(
        judge_entry(ExtensionRootScope::Quarantine, &["operation-1"], &none),
        ReconciliationVerdict::Collect(ReconciliationReason::AbandonedQuarantine)
    );
    assert_eq!(
        judge_entry(
            ExtensionRootScope::Scratch,
            &["install-1", "generation-1"],
            &none
        ),
        ReconciliationVerdict::Collect(ReconciliationReason::StaleScratch)
    );
    assert_eq!(
        judge_entry(
            ExtensionRootScope::Sidecars,
            &["install-1", "generation-1"],
            &none
        ),
        ReconciliationVerdict::Collect(ReconciliationReason::OrphanSidecar)
    );
}

#[test]
fn package_content_is_kept_when_any_snapshot_row_names_it() {
    // Rows rather than pointers. A snapshot that is no longer active is still the rollback target,
    // and collecting its bytes because nothing points at it right now would delete what a rollback
    // needs.
    assert_eq!(
        judge_entry(
            ExtensionRootScope::Packages,
            &["sha256", DIGEST],
            &referenced(&[DIGEST])
        ),
        ReconciliationVerdict::RetainReferencedPackage
    );
    assert_eq!(
        judge_entry(
            ExtensionRootScope::Packages,
            &["sha256", DIGEST],
            &referenced(&[OTHER])
        ),
        ReconciliationVerdict::Collect(ReconciliationReason::UnreferencedPackage)
    );
}

#[test]
fn anything_whose_shape_does_not_match_is_left_alone() {
    // The rule that makes this safe to run unattended. A cleanup that deletes what it does not
    // understand is one nobody can run at startup.
    let none = referenced(&[]);
    let cases: [(ExtensionRootScope, &[&str]); 8] = [
        // Too shallow: a file where a directory belongs.
        (ExtensionRootScope::Packages, &["sha256"]),
        (ExtensionRootScope::Scratch, &["install-1"]),
        // Too deep.
        (
            ExtensionRootScope::Quarantine,
            &["operation-1", "unexpected"],
        ),
        // The wrong algorithm directory.
        (ExtensionRootScope::Packages, &["sha512", DIGEST]),
        // Not a digest.
        (ExtensionRootScope::Packages, &["sha256", "not-a-digest"]),
        // Not an application-generated identifier.
        (ExtensionRootScope::Quarantine, &["my notes.txt"]),
        (ExtensionRootScope::Sidecars, &["install-1", "generation 1"]),
        // Nothing at all.
        (ExtensionRootScope::Scratch, &[]),
    ];

    for (scope, segments) in cases {
        assert_eq!(
            judge_entry(scope, segments, &none),
            ReconciliationVerdict::Unrecognised,
            "{scope:?} {segments:?}"
        );
    }
}

#[test]
fn an_identifier_shaped_name_is_required_before_anything_is_collected() {
    // Reconciliation deletes what the identifier rule admits, so a name that arrived from anywhere
    // other than this application must not match it.
    let none = referenced(&[]);
    for hostile in ["../escape", "a/b", "a.b", "", &"x".repeat(129)] {
        assert_eq!(
            judge_entry(ExtensionRootScope::Quarantine, &[hostile], &none),
            ReconciliationVerdict::Unrecognised,
            "{hostile:?}"
        );
    }
    assert_eq!(
        judge_entry(ExtensionRootScope::Quarantine, &[&"x".repeat(128)], &none),
        ReconciliationVerdict::Collect(ReconciliationReason::AbandonedQuarantine),
        "the limit itself is admissible"
    );
}

#[test]
fn every_verdict_and_reason_has_a_distinct_stable_code() {
    let mut codes: Vec<&str> = ALL_RECONCILIATION_REASONS
        .iter()
        .map(|reason| reason.code())
        .collect();
    codes.push(ReconciliationVerdict::RetainReferencedPackage.code());
    codes.push(ReconciliationVerdict::Unrecognised.code());
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);

    let mut scopes: Vec<&str> = ALL_EXTENSION_ROOT_SCOPES
        .iter()
        .map(|scope| scope.code())
        .collect();
    let scope_total = scopes.len();
    scopes.sort_unstable();
    scopes.dedup();
    assert_eq!(scopes.len(), scope_total);

    assert!(ReconciliationVerdict::Collect(ReconciliationReason::StaleScratch).collects());
    assert!(!ReconciliationVerdict::Unrecognised.collects());
    assert!(!ReconciliationVerdict::RetainReferencedPackage.collects());
}

#[test]
fn a_summary_is_clean_only_when_nothing_was_left_unexplained() {
    let mut summary = ReconciliationSummary {
        collected: vec!["quarantine/operation-1".to_string()],
        ..ReconciliationSummary::default()
    };
    assert!(summary.is_clean());

    summary
        .unrecognised
        .push("quarantine/notes.txt".to_string());
    assert!(!summary.is_clean());

    summary.unrecognised.clear();
    summary.uncollectable.push("scratch/install-1".to_string());
    assert!(!summary.is_clean());
}
