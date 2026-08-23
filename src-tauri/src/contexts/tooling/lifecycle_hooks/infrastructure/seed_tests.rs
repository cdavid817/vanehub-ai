//! What seeding does on the first launch, on the second, on an upgrade, and when it must not.
//!
//! Against a real database, because the guarantees that matter -- idempotence, conflict refusal,
//! and leaving user state alone -- are about what is in the rows afterwards.

use super::{
    SqliteHookBindingRepository, SqliteHookDefinitionRepository, SqliteHookSubjectRepository,
};
use crate::contexts::tooling::lifecycle_hooks::application::{
    seed_builtin_hooks, HookBindingRepository, HookSeedReport,
};
use crate::contexts::tooling::lifecycle_hooks::application::{
    HookDefinitionRepository, HookSubjectRepository,
};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    builtin_hook_catalog, BuiltinHookDescriptor, DefinitionDigest, HookEvent, HookGlobalId,
    HookOrigin, HookScope, HookSubject, SeedOutcome, BUILTIN_HOOK_SNAPSHOT,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::Arc;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const AT: &str = "2026-08-23T00:00:00Z";

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

fn hook(name: &str) -> HookGlobalId {
    HookGlobalId::parse(name).expect("hook")
}

fn descriptor(name: &str, digest: &str) -> BuiltinHookDescriptor {
    BuiltinHookDescriptor {
        hook: hook(name),
        event: HookEvent::SessionStart,
        digest: DefinitionDigest::parse(digest).expect("digest"),
    }
}

fn seed(fixture: &Fixture, catalog: &[BuiltinHookDescriptor]) -> HookSeedReport {
    seed_builtin_hooks(
        &SqliteHookSubjectRepository::new(fixture.database.clone()),
        &SqliteHookDefinitionRepository::new(fixture.database.clone()),
        catalog,
        AT,
    )
    .expect("seed")
}

#[test]
fn the_shipped_catalog_seeds_without_error() {
    // Empty in this build -- the host's dispatch points arrive with Task Group 7 -- so this
    // asserts the wiring rather than any content, and will start asserting content for free.
    let fixture = fixture("hook-seed-shipped");

    let report = seed(&fixture, &builtin_hook_catalog());

    assert_eq!(report, HookSeedReport::default());
    assert!(!report.changed_anything());
}

#[test]
fn a_first_launch_creates_the_subject_and_its_definition() {
    let fixture = fixture("hook-seed-first");
    let catalog = [descriptor("vanehub.session-start", FIRST)];

    let report = seed(&fixture, &catalog);

    assert_eq!(report.seeded, 1);
    assert!(report.changed_anything());
    let subjects = SqliteHookSubjectRepository::new(fixture.database.clone());
    let stored = subjects
        .get(&hook("vanehub.session-start"))
        .expect("get")
        .expect("present");
    assert_eq!(stored.origin, HookOrigin::Builtin);
}

#[test]
fn a_repeated_launch_changes_nothing() {
    // Every launch runs this. A seed that was not idempotent would produce a different database on
    // the second start than the first, and the second start is the one nobody tests by hand.
    let fixture = fixture("hook-seed-repeat");
    let catalog = [descriptor("vanehub.session-start", FIRST)];
    seed(&fixture, &catalog);

    let report = seed(&fixture, &catalog);

    assert_eq!(report.already_seeded, 1);
    assert_eq!(report.seeded, 0);
    assert!(!report.changed_anything());
    assert_eq!(
        SqliteHookSubjectRepository::new(fixture.database.clone())
            .get(&hook("vanehub.session-start"))
            .expect("get")
            .expect("present")
            .first_seen_at,
        AT,
        "the first sighting does not move on a re-seed"
    );
}

#[test]
fn an_upgrade_adds_a_revision_beside_the_old_one() {
    // A changed built-in definition is a new immutable revision under a new built-in snapshot. The
    // old one stays, which is what lets an operator see it changed and makes a downgrade
    // describable.
    let fixture = fixture("hook-seed-upgrade");
    seed(&fixture, &[descriptor("vanehub.session-start", FIRST)]);

    // What the host does when a built-in changes: a new snapshot, recorded beside the old.
    let definitions = SqliteHookDefinitionRepository::new(fixture.database.clone());
    definitions
        .record(
            &crate::contexts::tooling::lifecycle_hooks::domain::HookDefinitionRevision {
                hook: hook("vanehub.session-start"),
                snapshot: crate::contexts::tooling::lifecycle_hooks::domain::SnapshotRef::parse(
                    "builtin-2",
                )
                .expect("snapshot"),
                event: HookEvent::SessionStart,
                digest: DefinitionDigest::parse(SECOND).expect("digest"),
                recorded_at: AT.to_string(),
            },
        )
        .expect("record the upgraded revision");

    let revisions = definitions
        .revisions(&hook("vanehub.session-start"))
        .expect("revisions");
    assert_eq!(revisions.len(), 2, "the previous revision is still there");
    assert!(revisions
        .iter()
        .any(|revision| revision.snapshot.as_str() == BUILTIN_HOOK_SNAPSHOT));
}

#[test]
fn the_same_built_in_snapshot_with_a_different_definition_is_refused() {
    // Not an upgrade -- an upgrade is a new snapshot. This is one snapshot claiming two
    // definitions, which means one of two builds is wrong about what it shipped.
    let fixture = fixture("hook-seed-definition-conflict");
    seed(&fixture, &[descriptor("vanehub.session-start", FIRST)]);

    let error = seed_builtin_hooks(
        &SqliteHookSubjectRepository::new(fixture.database.clone()),
        &SqliteHookDefinitionRepository::new(fixture.database.clone()),
        &[descriptor("vanehub.session-start", SECOND)],
        AT,
    )
    .expect_err("definition conflict");

    assert_eq!(error.code(), "builtin_hook_definition_conflict");
}

#[test]
fn a_seed_never_takes_over_an_extensions_subject() {
    // Quietly reassigning it would let a launch change which contribution an operator installed.
    let fixture = fixture("hook-seed-owner-conflict");
    SqliteHookSubjectRepository::new(fixture.database.clone())
        .ensure(&HookSubject {
            hook: hook("vanehub.session-start"),
            origin: HookOrigin::Extension,
            first_seen_at: AT.to_string(),
        })
        .expect("an extension claimed it first");

    let error = seed_builtin_hooks(
        &SqliteHookSubjectRepository::new(fixture.database.clone()),
        &SqliteHookDefinitionRepository::new(fixture.database.clone()),
        &[descriptor("vanehub.session-start", FIRST)],
        AT,
    )
    .expect_err("owner conflict");

    assert_eq!(error.code(), "builtin_hook_owner_conflict");
    assert_eq!(
        SqliteHookSubjectRepository::new(fixture.database.clone())
            .get(&hook("vanehub.session-start"))
            .expect("get")
            .expect("present")
            .origin,
        HookOrigin::Extension,
        "and the stored owner is untouched -- there is no INSERT OR REPLACE in this path"
    );
}

#[test]
fn seeding_creates_no_binding_and_overwrites_no_user_enablement() {
    // The rule the whole mechanism exists for: a process that runs on every launch must not
    // produce answers a person gave. `seed_builtin_hooks` has no binding repository in reach, so
    // this asserts the consequence -- a user's disabled hook stays disabled across a re-seed.
    let fixture = fixture("hook-seed-user-state");
    let catalog = [descriptor("vanehub.session-start", FIRST)];
    seed(&fixture, &catalog);

    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());
    assert!(
        bindings
            .bindings(&hook("vanehub.session-start"))
            .expect("bindings")
            .is_empty(),
        "seeding creates no binding at all"
    );

    bindings
        .seed_default(
            &hook("vanehub.session-start"),
            &HookScope::global(),
            true,
            AT,
        )
        .expect("the user's first answer");
    bindings
        .set(
            &hook("vanehub.session-start"),
            &HookScope::global(),
            false,
            1,
            AT,
        )
        .expect("the user turns it off");

    seed(&fixture, &catalog);

    let held = bindings
        .binding(&hook("vanehub.session-start"), &HookScope::global())
        .expect("binding")
        .expect("present");
    assert!(!held.enabled, "the user's choice survives every launch");
    assert_eq!(
        held.revision, 2,
        "and the seed did not write, so it did not move"
    );
    assert_eq!(
        crate::contexts::tooling::lifecycle_hooks::domain::decide_seed(Some(&held)),
        SeedOutcome::Preserved
    );
}

#[test]
fn a_rejection_partway_through_leaves_the_earlier_descriptors_applied() {
    // Each descriptor is its own subject and its own immutable revision, so a partial pass leaves
    // a consistent database and the next launch resumes. Rolling the earlier ones back would mean
    // deleting subjects other evidence may already reference, which is what RESTRICT refuses.
    let fixture = fixture("hook-seed-partial");
    SqliteHookSubjectRepository::new(fixture.database.clone())
        .ensure(&HookSubject {
            hook: hook("vanehub.second"),
            origin: HookOrigin::Extension,
            first_seen_at: AT.to_string(),
        })
        .expect("an extension owns the second id");

    let error = seed_builtin_hooks(
        &SqliteHookSubjectRepository::new(fixture.database.clone()),
        &SqliteHookDefinitionRepository::new(fixture.database.clone()),
        &[
            descriptor("vanehub.first", FIRST),
            descriptor("vanehub.second", SECOND),
        ],
        AT,
    )
    .expect_err("the second descriptor conflicts");

    assert_eq!(error.code(), "builtin_hook_owner_conflict");
    let subjects = SqliteHookSubjectRepository::new(fixture.database.clone());
    assert!(
        subjects.get(&hook("vanehub.first")).expect("get").is_some(),
        "the descriptor that succeeded stays applied"
    );
    assert_eq!(
        subjects
            .get(&hook("vanehub.second"))
            .expect("get")
            .expect("present")
            .origin,
        HookOrigin::Extension,
        "and the one that conflicted is untouched"
    );
}

#[test]
fn two_concurrent_seeds_leave_exactly_one_subject_and_one_definition() {
    // Two independent connections. Both launches racing on a cold database is the realistic case:
    // the seed runs at startup, and nothing serialises two processes.
    let fixture = fixture("hook-seed-concurrent");
    let catalog = [descriptor("vanehub.session-start", FIRST)];

    let left_database = fixture.database.clone();
    let right_database = fixture.database.clone();
    let left_catalog = catalog.clone();
    let right_catalog = catalog.clone();
    let left = std::thread::spawn(move || {
        seed_builtin_hooks(
            &SqliteHookSubjectRepository::new(left_database.clone()),
            &SqliteHookDefinitionRepository::new(left_database),
            &left_catalog,
            AT,
        )
    });
    let right = std::thread::spawn(move || {
        seed_builtin_hooks(
            &SqliteHookSubjectRepository::new(right_database.clone()),
            &SqliteHookDefinitionRepository::new(right_database),
            &right_catalog,
            AT,
        )
    });

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    for outcome in &outcomes {
        assert!(outcome.is_ok(), "neither seed may fail: {outcomes:?}");
    }
    assert_eq!(
        SqliteHookSubjectRepository::new(fixture.database.clone())
            .all()
            .expect("all")
            .len(),
        1
    );
    assert_eq!(
        SqliteHookDefinitionRepository::new(fixture.database.clone())
            .revisions(&hook("vanehub.session-start"))
            .expect("revisions")
            .len(),
        1
    );
}
