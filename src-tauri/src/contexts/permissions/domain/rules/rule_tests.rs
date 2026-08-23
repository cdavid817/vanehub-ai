//! What a rule may say, and the two things a source is not allowed to say.

use super::{
    AllowedScopes, AuthorizationRule, GrantScope, Matcher, OperationName, RuleAdmissionError,
    RuleEffect, RuleId, RuleProvenance, RuleScope, RuleScopeKind, RuleSource, SourceId,
    ALL_GRANT_SCOPES, ALL_RULE_EFFECTS, ALL_RULE_PROVENANCES, ALL_RULE_SOURCES,
};

fn rule(source: RuleSource, effect: RuleEffect) -> AuthorizationRule {
    AuthorizationRule {
        source,
        source_id: SourceId::parse("acme-git-guardian").expect("source"),
        rule_id: RuleId::parse("no-force-push").expect("rule"),
        scope: RuleScope::global(),
        operation: OperationName::parse("shell.exec").expect("operation"),
        matcher: Matcher::Prefix("git push --force".to_string()),
        effect,
        allowed_scopes: if effect == RuleEffect::Ask {
            AllowedScopes::of(&[GrantScope::Once])
        } else {
            AllowedScopes::none()
        },
        priority: 0,
        expires_at: None,
        provenance: RuleProvenance::ExtensionManifest,
    }
}

#[test]
fn an_extension_may_narrow_what_is_permitted() {
    for effect in [RuleEffect::Ask, RuleEffect::Deny] {
        assert!(
            AuthorizationRule::admit(rule(RuleSource::Extension, effect)).is_ok(),
            "{effect:?} should be admitted"
        );
    }
}

#[test]
fn an_extension_may_never_widen_it() {
    // A downloaded package contributing `Allow` could grant itself -- or anything else -- a
    // permission the user never approved, which inverts the point of installing it behind a
    // review. Refused in the constructor, and again by a table CHECK.
    let error = AuthorizationRule::admit(rule(RuleSource::Extension, RuleEffect::Allow))
        .expect_err("extensions may not allow");

    assert_eq!(error, RuleAdmissionError::ExtensionMayNotAllow);
    assert_eq!(error.code(), "extension_rule_may_not_allow");
}

#[test]
fn a_user_or_project_rule_may_allow() {
    for source in [RuleSource::User, RuleSource::Project] {
        assert!(
            AuthorizationRule::admit(rule(source, RuleEffect::Allow)).is_ok(),
            "{source:?} should be able to allow"
        );
        assert!(source.admits_allow());
    }
    assert!(!RuleSource::Extension.admits_allow());
}

#[test]
fn a_rule_whose_two_halves_disagree_is_refused() {
    // An `Ask` that may never be remembered would leave the evaluator with a rule that asks and
    // forbids every answer; a `Deny` naming grant scopes describes remembering a decision that is
    // never offered. Both are contradictions, not preferences.
    let asking_without_scopes = AuthorizationRule {
        allowed_scopes: AllowedScopes::none(),
        ..rule(RuleSource::User, RuleEffect::Ask)
    };
    let denying_with_scopes = AuthorizationRule {
        allowed_scopes: AllowedScopes::of(&[GrantScope::Global]),
        ..rule(RuleSource::User, RuleEffect::Deny)
    };

    for offered in [asking_without_scopes, denying_with_scopes] {
        assert_eq!(
            AuthorizationRule::admit(offered).expect_err("contradiction"),
            RuleAdmissionError::AllowedScopesContradictEffect
        );
    }
}

#[test]
fn deny_beats_ask_beats_allow_by_precedence() {
    assert!(RuleEffect::Deny.precedence() > RuleEffect::Ask.precedence());
    assert!(RuleEffect::Ask.precedence() > RuleEffect::Allow.precedence());
}

#[test]
fn allowed_scopes_round_trip_in_a_canonical_order() {
    // Two authors writing the same set in different orders must produce the same bytes, or the
    // rule-set digest would depend on how a rule happened to be typed.
    let one = AllowedScopes::of(&[GrantScope::Global, GrantScope::Once, GrantScope::Session]);
    let other = AllowedScopes::of(&[GrantScope::Session, GrantScope::Global, GrantScope::Once]);

    assert_eq!(one, other);
    assert_eq!(one.as_str(), other.as_str());
    assert_eq!(AllowedScopes::parse(&one.as_str()).expect("parse"), one);
    assert_eq!(
        AllowedScopes::parse("").expect("empty"),
        AllowedScopes::none()
    );
    assert!(AllowedScopes::parse("forever").is_err());
}

#[test]
fn a_duplicate_scope_does_not_change_the_set() {
    let once = AllowedScopes::of(&[GrantScope::Once, GrantScope::Once]);

    assert_eq!(once.as_str(), "once");
    assert!(once.admits(GrantScope::Once));
    assert!(!once.admits(GrantScope::Global));
}

#[test]
fn a_matcher_is_one_of_three_shapes_and_nothing_else() {
    // No expression language: an evaluator that cannot decide is one that has to guess, in the
    // component whose job is deciding whether something dangerous may happen.
    for spelling in ["any", "exact:git push", "prefix:git "] {
        let matcher = Matcher::parse(spelling).expect(spelling);
        assert_eq!(matcher.as_str(), spelling);
    }
    for rejected in [
        "",
        "glob:*",
        "regex:^git",
        // A second, less obvious spelling of `any`. Two spellings of one rule is how a review
        // misses one.
        "prefix:",
        "exact:",
        "any:extra",
    ] {
        assert_eq!(
            Matcher::parse(rejected).expect_err(rejected).code(),
            "invalid_rule_matcher",
            "{rejected:?}"
        );
    }
}

#[test]
fn a_matcher_matches_what_its_shape_says() {
    assert!(Matcher::Any.matches("anything at all"));
    assert!(Matcher::Exact("git push".to_string()).matches("git push"));
    assert!(!Matcher::Exact("git push".to_string()).matches("git push --force"));
    assert!(Matcher::Prefix("git ".to_string()).matches("git push --force"));
    assert!(!Matcher::Prefix("git ".to_string()).matches("npm install"));
}

#[test]
fn a_more_specific_matcher_outranks_a_less_specific_one() {
    let any = Matcher::Any.specificity();
    let short = Matcher::Prefix("git".to_string()).specificity();
    let long = Matcher::Prefix("git push --force".to_string()).specificity();
    let exact = Matcher::Exact("git push".to_string()).specificity();

    assert!(any < short && short < long && long < exact);
}

#[test]
fn a_narrower_scope_outranks_a_wider_one() {
    let session = AuthorizationRule {
        scope: RuleScope::scoped(RuleScopeKind::Session, "session-1").expect("session"),
        matcher: Matcher::Any,
        ..rule(RuleSource::User, RuleEffect::Deny)
    };
    let global_exact = AuthorizationRule {
        scope: RuleScope::global(),
        matcher: Matcher::Exact("git push".to_string()),
        ..rule(RuleSource::User, RuleEffect::Deny)
    };

    assert!(
        session.specificity() > global_exact.specificity(),
        "a rule about this session outranks one about everyone, whatever it matches"
    );
}

#[test]
fn an_expired_rule_is_expired_at_and_after_its_instant() {
    let expiring = AuthorizationRule {
        expires_at: Some("2026-08-23T00:00:00Z".to_string()),
        ..rule(RuleSource::User, RuleEffect::Deny)
    };

    assert!(!expiring.is_expired_at("2026-08-22T23:59:59Z"));
    assert!(expiring.is_expired_at("2026-08-23T00:00:00Z"));
    assert!(expiring.is_expired_at("2026-08-24T00:00:00Z"));
    assert!(!rule(RuleSource::User, RuleEffect::Deny).is_expired_at("2099-01-01T00:00:00Z"));
}

#[test]
fn every_closed_vocabulary_round_trips_and_refuses_what_it_does_not_know() {
    for source in ALL_RULE_SOURCES.iter().copied() {
        assert_eq!(RuleSource::parse(source.as_str()), Ok(source));
    }
    for effect in ALL_RULE_EFFECTS.iter().copied() {
        assert_eq!(RuleEffect::parse(effect.as_str()), Ok(effect));
    }
    for provenance in ALL_RULE_PROVENANCES.iter().copied() {
        assert_eq!(RuleProvenance::parse(provenance.as_str()), Ok(provenance));
    }
    for scope in ALL_GRANT_SCOPES.iter().copied() {
        assert_eq!(GrantScope::parse(scope.as_str()), Ok(scope));
    }

    // Fail closed: a stored value this build cannot name is refused, never read as a default.
    assert_eq!(
        RuleSource::parse("template").expect_err("template").code(),
        "invalid_rule_source",
        "templates are not a persisted rule source"
    );
    assert!(RuleEffect::parse("permit").is_err());
    assert!(RuleProvenance::parse("somewhere").is_err());
}
