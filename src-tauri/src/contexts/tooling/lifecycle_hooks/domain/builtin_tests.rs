//! Who may claim a subject id, and what a built-in descriptor is.

use super::{
    all_hook_seed_rejections, builtin_hook_catalog, decide_owner, BuiltinHookDescriptor,
    DefinitionDigest, HookEvent, HookGlobalId, HookOrigin, HookSeedOutcome, HookSeedRejection,
    SnapshotRef, BUILTIN_HOOK_SNAPSHOT,
};

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn hook() -> HookGlobalId {
    HookGlobalId::parse("vanehub.session-start").expect("hook")
}

#[test]
fn an_unclaimed_id_may_be_seeded() {
    assert!(decide_owner(&hook(), None, HookOrigin::Builtin).is_ok());
}

#[test]
fn the_same_owner_seeding_again_is_the_ordinary_repeated_launch() {
    assert!(decide_owner(&hook(), Some(HookOrigin::Builtin), HookOrigin::Builtin).is_ok());
    assert!(decide_owner(&hook(), Some(HookOrigin::Extension), HookOrigin::Extension).is_ok());
}

#[test]
fn ownership_is_refused_in_both_directions() {
    // A seed taking over an extension's id would reassign a contribution an operator installed; an
    // extension taking a built-in id would let a package impersonate the host. Both are the same
    // rule, and neither is a silent overwrite.
    let seed_over_extension =
        decide_owner(&hook(), Some(HookOrigin::Extension), HookOrigin::Builtin)
            .expect_err("seed must not take over");
    let extension_over_builtin =
        decide_owner(&hook(), Some(HookOrigin::Builtin), HookOrigin::Extension)
            .expect_err("extension must not take over");

    assert_eq!(seed_over_extension.code(), "builtin_hook_owner_conflict");
    assert_eq!(extension_over_builtin.code(), "builtin_hook_owner_conflict");
    let HookSeedRejection::OwnerConflict {
        stored, offered, ..
    } = seed_over_extension
    else {
        panic!("expected an owner conflict");
    };
    assert_eq!(
        (stored, offered),
        (HookOrigin::Extension, HookOrigin::Builtin),
        "the conflict reports which way round it was, or an operator cannot tell what happened"
    );
}

#[test]
fn a_built_in_definition_is_recorded_under_a_reserved_snapshot() {
    // Built-ins do not come from an extension package. A reserved snapshot keeps the
    // `(subject, snapshot)` key total; a nullable one would make the primary key partial and
    // reintroduce SQLite's NULL-uniqueness trap.
    let descriptor = BuiltinHookDescriptor {
        hook: hook(),
        event: HookEvent::SessionStart,
        digest: DefinitionDigest::parse(DIGEST).expect("digest"),
    };

    assert_eq!(
        descriptor.snapshot(),
        SnapshotRef::parse(BUILTIN_HOOK_SNAPSHOT).expect("snapshot")
    );
    assert!(
        BUILTIN_HOOK_SNAPSHOT.ends_with("-1"),
        "the generation suffix is what makes an upgrade a new revision rather than an edit"
    );
}

#[test]
fn the_shipped_catalog_is_empty_in_this_build() {
    // Stated rather than assumed. The host's dispatch points arrive with Task Group 7; seeding
    // descriptors for hooks nothing dispatches would create rows no code reads.
    assert!(builtin_hook_catalog().is_empty());
}

#[test]
fn every_outcome_and_rejection_has_a_distinct_stable_code() {
    let mut outcome_codes: Vec<&str> = [
        HookSeedOutcome::Seeded,
        HookSeedOutcome::AlreadySeeded,
        HookSeedOutcome::RevisionAdded,
    ]
    .iter()
    .map(|outcome| outcome.code())
    .collect();
    let outcomes = outcome_codes.len();
    outcome_codes.sort_unstable();
    outcome_codes.dedup();
    assert_eq!(outcome_codes.len(), outcomes);

    let rejections = all_hook_seed_rejections();
    let total = rejections.len();
    let mut codes: Vec<&str> = rejections.iter().map(HookSeedRejection::code).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}
