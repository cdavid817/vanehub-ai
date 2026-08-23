//! What an identity admits, and the diagnostics a rejection carries.

use super::{
    Matcher, OperationName, RuleId, RuleIdentifierKind, RuleIdentityError, RuleScope,
    RuleScopeKind, RuleSetDigest, RuleSetId, SourceId, ALL_RULE_IDENTIFIER_KINDS,
    ALL_RULE_SCOPE_KINDS, GLOBAL_RULE_SCOPE_KEY,
};

#[test]
fn an_opaque_identifier_admits_what_a_host_generates_and_refuses_what_breaks_a_key() {
    for accepted in ["set-01hxyz", "user_settings", "a", "ACME-1"] {
        assert!(RuleSetId::parse(accepted).is_ok(), "{accepted}");
        assert!(RuleId::parse(accepted).is_ok(), "{accepted}");
        assert!(SourceId::parse(accepted).is_ok(), "{accepted}");
    }
    for rejected in ["", "has space", "has/slash", "has\0nul", "has:colon"] {
        assert!(RuleSetId::parse(rejected).is_err(), "{rejected:?}");
    }
}

#[test]
fn a_rejection_carries_the_offending_value_but_cannot_be_made_unbounded_by_it() {
    let hostile = "!".repeat(10_000);

    let error = RuleSetId::parse(&hostile).expect_err("should reject");

    assert!(
        !error.value.is_empty(),
        "an operator needs to see what failed"
    );
    assert!(
        error.value.len() <= 128,
        "a hostile value must not make the diagnostic unbounded"
    );
}

#[test]
fn an_operation_name_is_open_ended_but_cannot_become_a_sentence() {
    for accepted in ["shell.exec", "file.write", "mcp.tool.call", "memory_write"] {
        assert!(OperationName::parse(accepted).is_ok(), "{accepted}");
    }
    for rejected in [
        "",
        "Shell.Exec",
        ".leading",
        "trailing.",
        "shell exec",
        "shell/exec",
        &"a".repeat(97),
    ] {
        assert_eq!(
            OperationName::parse(rejected).expect_err(rejected).code(),
            "invalid_rule_operation",
            "{rejected:?}"
        );
    }
}

#[test]
fn a_digest_is_lower_case_hex_of_the_right_length() {
    let mixed = "abcdef0123456789".repeat(4);
    assert!(RuleSetDigest::parse(&mixed).is_ok());

    for rejected in [
        "",
        &mixed[..63],
        &format!("{mixed}0"),
        // Two spellings of one digest would let the same set read as a conflict with itself.
        &mixed.to_uppercase(),
        &"g".repeat(64),
    ] {
        assert_eq!(
            RuleSetDigest::parse(rejected).expect_err(rejected).code(),
            "invalid_rule_set_digest"
        );
    }
}

#[test]
fn every_scope_kind_round_trips_and_orders_from_wide_to_narrow() {
    let mut previous = i64::MIN;
    for kind in ALL_RULE_SCOPE_KINDS.iter().copied() {
        assert_eq!(RuleScopeKind::parse(kind.as_str()), Some(kind));
        assert!(
            kind.specificity() > previous,
            "{kind:?} must be narrower than the one before it"
        );
        previous = kind.specificity();
    }
    assert_eq!(RuleScopeKind::parse("everything"), None);
}

#[test]
fn every_scope_round_trips_through_the_two_columns_it_is_stored_as() {
    let scopes = [
        RuleScope::global(),
        RuleScope::scoped(RuleScopeKind::User, "alice").expect("user"),
        RuleScope::scoped(RuleScopeKind::Project, "d:/work/repo").expect("project"),
        RuleScope::scoped(RuleScopeKind::Principal, "agent-1").expect("principal"),
        RuleScope::scoped(RuleScopeKind::Session, "session-1").expect("session"),
    ];

    for scope in scopes {
        assert_eq!(
            RuleScope::parse(scope.kind().as_str(), scope.key()).expect("round trip"),
            scope
        );
    }
}

#[test]
fn the_global_scope_key_is_the_empty_string_so_the_index_is_total() {
    // A nullable scope column would let SQLite treat every NULL as distinct and admit unlimited
    // "global" rows for one rule id, each invisible to the others.
    assert_eq!(RuleScope::global().key(), GLOBAL_RULE_SCOPE_KEY);
    assert!(GLOBAL_RULE_SCOPE_KEY.is_empty());
}

#[test]
fn a_scope_key_is_bounded_and_cannot_contain_a_nul() {
    // A NUL truncates every C-string consumer downstream of storage.
    assert!(RuleScope::scoped(RuleScopeKind::Project, "has\0nul").is_err());
    assert!(RuleScope::scoped(RuleScopeKind::Project, &"a".repeat(257)).is_err());
    assert!(RuleScope::scoped(RuleScopeKind::Project, &"a".repeat(256)).is_ok());
}

#[test]
fn a_matcher_value_is_bounded_and_cannot_contain_a_nul() {
    assert!(Matcher::parse("exact:has\0nul").is_err());
    assert!(Matcher::parse(&format!("prefix:{}", "a".repeat(513))).is_err());
    assert!(Matcher::parse(&format!("prefix:{}", "a".repeat(512))).is_ok());
}

#[test]
fn a_rejection_says_which_identity_failed_as_well_as_why() {
    let error: RuleIdentityError = RuleScope::parse("global", "unexpected").expect_err("key");

    assert_eq!(error.kind, RuleIdentifierKind::ScopeKey);
    assert_eq!(error.value, "unexpected");
}

#[test]
fn every_identifier_kind_has_a_distinct_lower_snake_case_code() {
    let mut codes: Vec<&str> = ALL_RULE_IDENTIFIER_KINDS
        .iter()
        .map(|kind| kind.code())
        .collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);

    for code in ALL_RULE_IDENTIFIER_KINDS.iter().map(|kind| kind.code()) {
        assert!(code
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_'));
    }
}
