//! What an identity admits, and the two it exists to refuse.

use super::{
    registered_hook_failures, DefinitionDigest, HookExecutionId, HookGlobalId, HookIdentifierKind,
    HookIdentityError, HookOutcomeCode, HookScope, HookScopeKind, SnapshotRef,
    ALL_HOOK_IDENTIFIER_KINDS, ALL_HOOK_SCOPE_KINDS, GLOBAL_SCOPE_KEY,
};

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

#[test]
fn a_namespaced_contribution_id_is_a_hook_global_id() {
    // This subdomain holds ids composed elsewhere, so the grammar has to admit the separators
    // `extension_platform` composes with without this file knowing how it composes them.
    for accepted in [
        "ext::acme.git-guardian::pre-commit",
        "vanehub.session-start",
        "a",
        "hook_with_underscores",
    ] {
        assert!(
            HookGlobalId::parse(accepted).is_ok(),
            "{accepted} should parse"
        );
    }
}

#[test]
fn a_hook_global_id_refuses_what_would_break_a_log_line_or_a_key() {
    for rejected in [
        "",
        "Ext::Acme",       // upper case: two spellings of one subject
        ":leading",        // a leading separator makes an empty first segment
        "trailing.",       // and a trailing one an empty last
        "has space",       //
        "has\0nul",        //
        "hook/with/slash", // path-shaped ids invite being used as paths
    ] {
        let error = HookGlobalId::parse(rejected).expect_err(rejected);
        assert_eq!(error.code(), "invalid_hook_global_id", "{rejected:?}");
    }
}

#[test]
fn a_rejection_carries_the_offending_value_but_cannot_be_made_unbounded_by_it() {
    let hostile = "A".repeat(10_000);

    let error = HookGlobalId::parse(&hostile).expect_err("should reject");

    assert!(
        !error.value.is_empty(),
        "an operator needs to see what failed"
    );
    assert!(
        error.value.len() <= 160,
        "a hostile value must not make the diagnostic unbounded"
    );
}

#[test]
fn an_outcome_code_cannot_carry_an_error_message() {
    // The redaction floor. Every string below is what a caller reaches for when there is a
    // free-text field to reach for, and each one carries something that must not reach a durable
    // row: a path, a prompt fragment, a host name, a stderr dump.
    for message in [
        "Failed to run C:\\Users\\alice\\project\\hook.ps1",
        "connection refused to https://internal.example.com/api",
        "user asked: summarise the quarterly numbers",
        "Error: ENOENT",
        "timed out after 30s",
        "failed (exit 1)",
    ] {
        let error = HookOutcomeCode::parse(message).expect_err(message);
        assert_eq!(error.code(), "invalid_hook_outcome_code", "{message:?}");
    }

    // What it is for.
    for code in [
        "timed_out",
        "denied_by_policy",
        "exit_nonzero",
        "gate_closed",
    ] {
        assert!(HookOutcomeCode::parse(code).is_ok(), "{code} should parse");
    }
}

#[test]
fn an_outcome_code_is_bounded_so_a_row_cannot_grow_without_limit() {
    let long = "a".repeat(65);

    assert!(HookOutcomeCode::parse(&long).is_err());
    assert!(HookOutcomeCode::parse(&"a".repeat(64)).is_ok());
}

#[test]
fn the_global_scope_has_exactly_one_spelling() {
    // SQLite treats NULL as distinct from every other NULL in a unique index, so a nullable scope
    // column would admit unlimited global bindings for one Hook. The empty key is what makes the
    // index total; these assertions are what keep a second spelling from creeping in.
    let global = HookScope::global();

    assert_eq!(global.kind(), HookScopeKind::Global);
    assert_eq!(global.key(), GLOBAL_SCOPE_KEY);
    assert_eq!(
        HookScope::parse("global", GLOBAL_SCOPE_KEY).expect("global"),
        global
    );
    // Global with a key is not a narrower global; it is a row the constructors cannot produce.
    assert_eq!(
        HookScope::parse("global", "some-project")
            .expect_err("global carries no key")
            .code(),
        "invalid_hook_scope_key"
    );
    assert_eq!(
        HookScope::scoped(HookScopeKind::Global, "")
            .expect_err("global is not constructible as scoped")
            .code(),
        "invalid_hook_scope_key"
    );
}

#[test]
fn a_scoped_binding_must_say_what_it_is_scoped_to() {
    // An empty key on a project scope would be a second spelling of global, and the two would
    // disagree about whether the Hook runs everywhere.
    assert_eq!(
        HookScope::scoped(HookScopeKind::Project, "")
            .expect_err("empty key")
            .code(),
        "invalid_hook_scope_key"
    );
    assert!(HookScope::scoped(HookScopeKind::Project, "d:/work/repo").is_ok());
    assert!(HookScope::scoped(HookScopeKind::Agent, "claude-code").is_ok());
    assert!(
        HookScope::scoped(HookScopeKind::Project, "has\0nul").is_err(),
        "a NUL truncates every C-string consumer downstream of storage"
    );
}

#[test]
fn a_scope_kind_this_build_does_not_know_is_refused_rather_than_defaulted() {
    let error = HookScope::parse("session", "abc").expect_err("unknown kind");

    assert_eq!(error.code(), "invalid_hook_scope_kind");
}

#[test]
fn every_scope_round_trips_through_the_two_columns_it_is_stored_as() {
    let scopes = [
        HookScope::global(),
        HookScope::scoped(HookScopeKind::Project, "d:/work/repo").expect("project"),
        HookScope::scoped(HookScopeKind::Agent, "claude-code").expect("agent"),
    ];

    for scope in scopes {
        assert_eq!(
            HookScope::parse(scope.kind().as_str(), scope.key()).expect("round trip"),
            scope
        );
    }
}

#[test]
fn a_digest_is_lower_case_hex_of_the_right_length() {
    // A digest with letters in it, because the case rule is invisible against an all-digit one:
    // uppercasing `1111...` produces `1111...` and the assertion would pass while proving nothing.
    let mixed = "abcdef0123456789".repeat(4);
    assert!(DefinitionDigest::parse(DIGEST).is_ok());
    assert!(DefinitionDigest::parse(&mixed).is_ok());

    for rejected in [
        "",
        &DIGEST[..63],
        &format!("{DIGEST}0"),
        // Two spellings of one digest would let the same definition read as a conflict with
        // itself, so the upper-case form is refused rather than folded.
        &mixed.to_uppercase(),
        &"g".repeat(64),
    ] {
        assert_eq!(
            DefinitionDigest::parse(rejected)
                .expect_err(rejected)
                .code(),
            "invalid_hook_definition_digest"
        );
    }
}

#[test]
fn every_scope_kind_round_trips_and_has_a_distinct_spelling() {
    let mut spellings: Vec<&str> = ALL_HOOK_SCOPE_KINDS
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);

    for kind in ALL_HOOK_SCOPE_KINDS.iter().copied() {
        assert_eq!(HookScopeKind::parse(kind.as_str()), Some(kind));
    }
}

#[test]
fn a_rejection_says_which_identity_failed_as_well_as_why() {
    // The kind is what lets a caller tell "the hook id was wrong" from "the scope key was wrong"
    // when both arrive as one failed write.
    let error: HookIdentityError = HookScope::parse("global", "unexpected").expect_err("key");

    assert_eq!(error.kind, HookIdentifierKind::ScopeKey);
    assert_eq!(error.value, "unexpected");
}

#[test]
fn a_snapshot_reference_is_validated_as_text_and_never_resolved_here() {
    // Opaque on purpose: `extension_platform` owns snapshots. What this subdomain can check is
    // that the reference is a shape a log line and a SQL parameter can carry.
    assert!(SnapshotRef::parse("snap-01HXYZ").is_ok());
    assert!(HookExecutionId::parse("exec-01HXYZ").is_ok());
    for rejected in ["", "has space", "has/slash", "has:colon"] {
        assert!(SnapshotRef::parse(rejected).is_err(), "{rejected:?}");
        assert!(HookExecutionId::parse(rejected).is_err(), "{rejected:?}");
    }
}

#[test]
fn no_two_failures_in_this_subdomain_share_a_code() {
    let codes = registered_hook_failures();
    let total = codes.len();

    let mut unique = codes;
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(
        unique.len(),
        total,
        "two failures present the same code; a caller branching on it cannot tell them apart"
    );
}

#[test]
fn every_failure_code_is_lower_snake_case_and_bounded() {
    // Codes travel into logs, telemetry, and a frontend discriminated union. One with a space or
    // an uppercase letter is a wire break the first time something matches on it.
    for code in registered_hook_failures() {
        assert!(!code.is_empty());
        assert!(code.len() <= 64, "{code} is too long to be a stable code");
        assert!(
            code.chars().all(|character| character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'),
            "{code} is not lower_snake_case"
        );
    }
}

#[test]
fn every_identifier_kind_is_reachable_from_a_real_rejection() {
    // Guards the list against drift: a kind nothing can produce is a code that will never be seen,
    // and a kind missing from the list is a collision the check above cannot find.
    let produced = [
        HookGlobalId::parse("").expect_err("global").kind,
        SnapshotRef::parse("").expect_err("snapshot").kind,
        HookExecutionId::parse("").expect_err("execution").kind,
        DefinitionDigest::parse("").expect_err("digest").kind,
        HookOutcomeCode::parse("").expect_err("outcome").kind,
        HookScope::parse("session", "x").expect_err("kind").kind,
        HookScope::parse("global", "x").expect_err("key").kind,
    ];

    let mut seen: Vec<HookIdentifierKind> = produced.to_vec();
    seen.sort_unstable();
    seen.dedup();
    let mut declared: Vec<HookIdentifierKind> = ALL_HOOK_IDENTIFIER_KINDS.to_vec();
    declared.sort_unstable();

    assert_eq!(seen, declared);
}
