//! What gives a rule set its identity, and why that identity cannot depend on row order.

use super::{
    all_active_rule_set_errors, canonical_rule_set_bytes, registered_rule_failures,
    ActiveRuleSetError, AllowedScopes, AuthorizationRule, GrantScope, Matcher, OperationName,
    RuleEffect, RuleId, RuleProvenance, RuleScope, RuleScopeKind, RuleSetContentConflict,
    RuleSetDigest, RuleSetId, RuleSetOutcome, RuleSource, SourceId,
};

fn rule(id: &str, operation: &str, effect: RuleEffect) -> AuthorizationRule {
    AuthorizationRule {
        source: RuleSource::User,
        source_id: SourceId::parse("user-settings").expect("source"),
        rule_id: RuleId::parse(id).expect("rule"),
        scope: RuleScope::global(),
        operation: OperationName::parse(operation).expect("operation"),
        matcher: Matcher::Any,
        effect,
        allowed_scopes: if effect == RuleEffect::Ask {
            AllowedScopes::of(&[GrantScope::Session])
        } else {
            AllowedScopes::none()
        },
        priority: 0,
        expires_at: None,
        provenance: RuleProvenance::UserSettings,
    }
}

#[test]
fn the_canonical_bytes_do_not_depend_on_the_order_the_rules_arrive_in() {
    // SQLite promises nothing about row order without an ORDER BY, and even with one an added
    // index can change the plan. If the digest depended on that, the same rules would hash
    // differently on two machines and "is this the set I reviewed?" would be unanswerable.
    let forward = [
        rule("a", "shell.exec", RuleEffect::Deny),
        rule("b", "file.write", RuleEffect::Ask),
        rule("c", "file.read", RuleEffect::Allow),
    ];
    let reversed = [forward[2].clone(), forward[1].clone(), forward[0].clone()];
    let shuffled = [forward[1].clone(), forward[2].clone(), forward[0].clone()];

    assert_eq!(
        canonical_rule_set_bytes(&forward),
        canonical_rule_set_bytes(&reversed)
    );
    assert_eq!(
        canonical_rule_set_bytes(&forward),
        canonical_rule_set_bytes(&shuffled)
    );
}

#[test]
fn two_different_sets_do_not_share_an_encoding() {
    let base = [rule("a", "shell.exec", RuleEffect::Deny)];
    let differing_effect = [rule("a", "shell.exec", RuleEffect::Allow)];
    let differing_operation = [rule("a", "file.write", RuleEffect::Deny)];
    let differing_id = [rule("b", "shell.exec", RuleEffect::Deny)];

    let encoded = canonical_rule_set_bytes(&base);
    for other in [differing_effect, differing_operation, differing_id] {
        assert_ne!(encoded, canonical_rule_set_bytes(&other));
    }
}

#[test]
fn a_set_is_not_a_prefix_of_a_larger_one() {
    // The count is inside the encoding, so appending a rule cannot produce bytes that begin with
    // the smaller set's bytes and be mistaken for it by anything comparing prefixes.
    let one = [rule("a", "shell.exec", RuleEffect::Deny)];
    let two = [
        rule("a", "shell.exec", RuleEffect::Deny),
        rule("b", "file.write", RuleEffect::Ask),
    ];

    let smaller = canonical_rule_set_bytes(&one);
    let larger = canonical_rule_set_bytes(&two);

    assert!(!larger.starts_with(&smaller));
}

#[test]
fn every_field_is_length_prefixed_so_two_rules_cannot_share_an_encoding() {
    // Without a length, ("ab", "c") and ("a", "bc") concatenate identically. These two rules
    // differ only in where the boundary between two adjacent fields falls.
    let left = AuthorizationRule {
        source_id: SourceId::parse("ab").expect("source"),
        rule_id: RuleId::parse("c").expect("rule"),
        ..rule("a", "shell.exec", RuleEffect::Deny)
    };
    let right = AuthorizationRule {
        source_id: SourceId::parse("a").expect("source"),
        rule_id: RuleId::parse("bc").expect("rule"),
        ..rule("a", "shell.exec", RuleEffect::Deny)
    };

    assert_ne!(
        canonical_rule_set_bytes(&[left]),
        canonical_rule_set_bytes(&[right])
    );
}

#[test]
fn an_absent_expiry_encodes_differently_from_a_present_one() {
    let never = rule("a", "shell.exec", RuleEffect::Deny);
    let expiring = AuthorizationRule {
        expires_at: Some(String::new()),
        ..never.clone()
    };

    assert_ne!(
        canonical_rule_set_bytes(&[never]),
        canonical_rule_set_bytes(&[expiring]),
        "absent and present-but-empty are different states"
    );
}

#[test]
fn an_empty_set_still_has_an_encoding() {
    // Distinct from "no set at all", which the active pointer represents as NULL rather than as a
    // fabricated empty set.
    assert!(!canonical_rule_set_bytes(&[]).is_empty());
    assert_ne!(
        canonical_rule_set_bytes(&[]),
        canonical_rule_set_bytes(&[rule("a", "shell.exec", RuleEffect::Deny)])
    );
}

#[test]
fn a_recorded_or_deduplicated_set_names_the_id_to_activate() {
    let offered = RuleSetId::parse("set-new").expect("id");
    let existing = RuleSetId::parse("set-stored").expect("id");

    assert_eq!(
        RuleSetOutcome::Recorded.activatable(&offered),
        Some(offered.clone())
    );
    assert_eq!(
        RuleSetOutcome::AlreadyRecorded {
            existing: existing.clone()
        }
        .activatable(&offered),
        Some(existing),
        "a caller recompiling the same rules gets a fresh id; what it needs back is the stored one"
    );
    assert!(RuleSetOutcome::Recorded.admits_activation());
    assert!(RuleSetOutcome::AlreadyRecorded {
        existing: RuleSetId::parse("set-stored").expect("id")
    }
    .admits_activation());
}

#[test]
fn a_conflict_and_its_outcome_name_the_same_finding() {
    let conflict = RuleSetContentConflict {
        stored_digest: RuleSetDigest::parse(&"a".repeat(64)).expect("digest"),
        offered_digest: RuleSetDigest::parse(&"b".repeat(64)).expect("digest"),
        stored_at: String::new(),
    };
    let outcome = RuleSetOutcome::Conflict(conflict.clone());

    assert_eq!(conflict.code(), outcome.code());
    assert!(!outcome.admits_activation());
    assert_eq!(
        outcome.activatable(&RuleSetId::parse("set-a").expect("id")),
        None,
        "a conflicted set gives a caller nothing to activate"
    );
}

#[test]
fn a_scope_has_exactly_one_global_spelling() {
    assert_eq!(
        RuleScope::parse("global", "").expect("global"),
        RuleScope::global()
    );
    assert_eq!(
        RuleScope::parse("global", "somewhere")
            .expect_err("global carries no key")
            .code(),
        "invalid_rule_scope_key"
    );
    assert_eq!(
        RuleScope::scoped(RuleScopeKind::Project, "")
            .expect_err("a scoped rule must say what it is scoped to")
            .code(),
        "invalid_rule_scope_key"
    );
    assert_eq!(
        RuleScope::parse("everything", "x")
            .expect_err("unknown kind")
            .code(),
        "invalid_rule_scope_kind"
    );
}

#[test]
fn a_stale_activation_reports_both_revisions() {
    let error = ActiveRuleSetError::StaleRevision {
        expected: 3,
        actual: 5,
    };

    assert_eq!(error.code(), "active_rule_set_stale_revision");
    let ActiveRuleSetError::StaleRevision { expected, actual } = error else {
        panic!("expected a stale revision");
    };
    assert_eq!((expected, actual), (3, 5));
}

#[test]
fn no_two_failures_in_this_subdomain_share_a_code() {
    let codes = registered_rule_failures();
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
    for code in registered_rule_failures() {
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
fn every_active_pointer_failure_has_a_distinct_code() {
    let errors = all_active_rule_set_errors();
    let total = errors.len();

    let mut codes: Vec<&str> = errors.iter().map(ActiveRuleSetError::code).collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), total);
}
