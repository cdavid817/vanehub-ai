//! What recording a definition revision means, and what a second recording of the same pair means.

use super::{
    decide_definition, DefinitionContentConflict, DefinitionDigest, DefinitionOutcome,
    HookDefinitionRevision, HookEvent, HookGlobalId, HookOrigin, SnapshotRef, ALL_HOOK_EVENTS,
};

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn revision(snapshot: &str, digest: &str, event: HookEvent, at: &str) -> HookDefinitionRevision {
    HookDefinitionRevision {
        hook: HookGlobalId::parse("ext::acme.git-guardian::pre-commit").expect("hook"),
        snapshot: SnapshotRef::parse(snapshot).expect("snapshot"),
        event,
        digest: DefinitionDigest::parse(digest).expect("digest"),
        recorded_at: at.to_string(),
    }
}

#[test]
fn an_unrecorded_pair_is_bound_by_the_first_revision_to_claim_it() {
    let outcome = decide_definition(
        &revision(
            "snap-a",
            FIRST,
            HookEvent::PreToolUse,
            "2026-08-01T00:00:00Z",
        ),
        None,
    );

    assert_eq!(outcome, DefinitionOutcome::Recorded);
    assert!(outcome.admits_dispatch());
}

#[test]
fn reinstalling_a_snapshot_re_records_its_definitions_idempotently() {
    // The install path re-records on every reinstall. If that were a conflict, reinstalling would
    // break the Hook it reinstalled.
    let recorded = revision(
        "snap-a",
        FIRST,
        HookEvent::PreToolUse,
        "2026-08-01T00:00:00Z",
    );
    let again = revision(
        "snap-a",
        FIRST,
        HookEvent::PreToolUse,
        "2026-08-20T00:00:00Z",
    );

    let outcome = decide_definition(&again, Some(&recorded));

    assert_eq!(outcome, DefinitionOutcome::AlreadyRecorded);
    assert!(outcome.admits_dispatch());
}

#[test]
fn the_same_pair_with_a_different_definition_is_refused_and_both_digests_are_reported() {
    // Taking the later one would let a rebuild change what an already-installed snapshot means,
    // which is the thing an immutable revision exists to prevent. Which digest is bound and which
    // was offered is the entire content of the finding, so neither is discarded.
    let recorded = revision(
        "snap-a",
        FIRST,
        HookEvent::PreToolUse,
        "2026-08-01T00:00:00Z",
    );
    let offered = revision(
        "snap-a",
        SECOND,
        HookEvent::PreToolUse,
        "2026-08-20T00:00:00Z",
    );

    let outcome = decide_definition(&offered, Some(&recorded));

    assert_eq!(
        outcome,
        DefinitionOutcome::Conflict(DefinitionContentConflict {
            recorded_digest: DefinitionDigest::parse(FIRST).expect("digest"),
            offered_digest: DefinitionDigest::parse(SECOND).expect("digest"),
            recorded_event: HookEvent::PreToolUse,
            recorded_at: "2026-08-01T00:00:00Z".to_string(),
        })
    );
    assert!(
        !outcome.admits_dispatch(),
        "a pair with two answers must not be dispatched from; running either would be a guess"
    );
    assert_eq!(outcome.code(), "hook_definition_content_conflict");
    let DefinitionOutcome::Conflict(conflict) = &outcome else {
        panic!("expected a conflict");
    };
    assert_eq!(
        conflict.code(),
        outcome.code(),
        "the conflict and the outcome name the same finding"
    );
}

#[test]
fn a_revision_that_changed_only_its_event_is_still_a_conflict() {
    // `event` is part of what the digest covers, so re-pointing a Hook at a different trigger
    // under the same identity cannot slip through as a re-registration. If this ever passes as
    // `AlreadyRecorded`, the digest stopped covering the event.
    let recorded = revision(
        "snap-a",
        FIRST,
        HookEvent::PreToolUse,
        "2026-08-01T00:00:00Z",
    );
    let repointed = revision(
        "snap-a",
        SECOND,
        HookEvent::PostToolUse,
        "2026-08-20T00:00:00Z",
    );

    let outcome = decide_definition(&repointed, Some(&recorded));

    assert!(!outcome.admits_dispatch());
    let DefinitionOutcome::Conflict(conflict) = outcome else {
        panic!("expected a conflict");
    };
    assert_eq!(
        conflict.recorded_event,
        HookEvent::PreToolUse,
        "the conflict reports the trigger that is bound, not the one that was offered"
    );
}

#[test]
fn two_snapshots_may_each_record_the_same_subject() {
    // The whole point of versioning by snapshot: an upgrade records a new revision beside the old
    // one rather than overwriting it, so a rollback still has something to dispatch from.
    let old = revision(
        "snap-a",
        FIRST,
        HookEvent::PreToolUse,
        "2026-08-01T00:00:00Z",
    );
    let new = revision(
        "snap-b",
        SECOND,
        HookEvent::PreToolUse,
        "2026-08-20T00:00:00Z",
    );

    // Different pairs, so the second is never compared against the first.
    assert_ne!(old.snapshot, new.snapshot);
    assert_eq!(decide_definition(&new, None), DefinitionOutcome::Recorded);
}

#[test]
fn every_outcome_has_a_distinct_stable_code() {
    let outcomes = [
        DefinitionOutcome::Recorded,
        DefinitionOutcome::AlreadyRecorded,
        DefinitionOutcome::Conflict(DefinitionContentConflict {
            recorded_digest: DefinitionDigest::parse(FIRST).expect("digest"),
            offered_digest: DefinitionDigest::parse(SECOND).expect("digest"),
            recorded_event: HookEvent::PreToolUse,
            recorded_at: String::new(),
        }),
    ];
    let mut codes: Vec<&str> = outcomes.iter().map(DefinitionOutcome::code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}

#[test]
fn every_event_round_trips_through_the_spelling_that_reaches_storage() {
    for event in ALL_HOOK_EVENTS.iter().copied() {
        assert_eq!(HookEvent::parse(event.as_str()), Some(event));
    }
    assert_eq!(
        HookEvent::parse("on_everything"),
        None,
        "an event this build cannot dispatch must be refused, not stored as a Hook that never fires"
    );
}

#[test]
fn every_event_spelling_is_distinct() {
    let mut spellings: Vec<&str> = ALL_HOOK_EVENTS.iter().map(|event| event.as_str()).collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
}

#[test]
fn an_origin_round_trips_and_an_unknown_one_is_refused() {
    for origin in [HookOrigin::Builtin, HookOrigin::Extension] {
        assert_eq!(HookOrigin::parse(origin.as_str()), Some(origin));
    }
    assert_eq!(HookOrigin::parse("plugin"), None);
}
