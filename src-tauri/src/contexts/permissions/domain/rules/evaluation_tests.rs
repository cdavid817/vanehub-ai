//! How a set of rules resolves, and the one outcome that reaches the existing template.

use super::{
    evaluate_rules, AllowedScopes, AuthorizationRule, DecisionTraceStep, DecisiveRule, GrantScope,
    Matcher, OperationName, RuleDecision, RuleEffect, RuleId, RuleOutcome, RuleProvenance,
    RuleRequest, RuleScope, RuleScopeKind, RuleSource, SourceId, ALL_RULE_OUTCOMES, DECISION_ORDER,
};

const NOW: &str = "2026-08-23T00:00:00Z";

fn operation() -> OperationName {
    OperationName::parse("shell.exec").expect("operation")
}

fn rule(id: &str, effect: RuleEffect, matcher: Matcher, scope: RuleScope) -> AuthorizationRule {
    AuthorizationRule {
        source: RuleSource::User,
        source_id: SourceId::parse("user-settings").expect("source"),
        rule_id: RuleId::parse(id).expect("rule"),
        scope,
        operation: operation(),
        matcher,
        effect,
        allowed_scopes: if effect == RuleEffect::Ask {
            AllowedScopes::of(&[GrantScope::Once, GrantScope::Session])
        } else {
            AllowedScopes::none()
        },
        priority: 0,
        expires_at: None,
        provenance: RuleProvenance::UserSettings,
    }
}

fn request<'a>(resource: &'a str, scopes: &'a [RuleScope]) -> RuleRequest<'a> {
    RuleRequest {
        operation: Box::leak(Box::new(operation())),
        resource,
        scopes,
        now: NOW,
    }
}

fn global() -> Vec<RuleScope> {
    vec![RuleScope::global()]
}

#[test]
fn nothing_matching_is_no_match_and_not_ask() {
    // Collapsing these would silently replace the existing PDP: today's templates allow
    // `file.read`, so a `NoMatch` arriving as `Ask` would start prompting for reads the moment the
    // first rule set was published, for an operation the rule set never mentioned.
    let scopes = global();
    let decision = evaluate_rules(&[], &request("git push", &scopes));

    assert_eq!(decision.outcome, RuleOutcome::NoMatch);
    assert!(decision.outcome.falls_through_to_template());
    assert_eq!(decision.decisive, None);
}

#[test]
fn only_no_match_reaches_the_template() {
    for outcome in ALL_RULE_OUTCOMES.iter().copied() {
        assert_eq!(
            outcome.falls_through_to_template(),
            outcome == RuleOutcome::NoMatch,
            "{outcome:?}"
        );
    }
}

#[test]
fn deny_beats_ask_beats_allow_whatever_else_the_rules_say() {
    let scopes = global();
    let allow = rule(
        "allow",
        RuleEffect::Allow,
        Matcher::Any,
        RuleScope::global(),
    );
    let ask = rule("ask", RuleEffect::Ask, Matcher::Any, RuleScope::global());
    let deny = rule("deny", RuleEffect::Deny, Matcher::Any, RuleScope::global());

    assert_eq!(
        evaluate_rules(std::slice::from_ref(&allow), &request("git push", &scopes)).outcome,
        RuleOutcome::Allow
    );
    assert_eq!(
        evaluate_rules(&[allow.clone(), ask.clone()], &request("git push", &scopes)).outcome,
        RuleOutcome::Ask
    );
    assert_eq!(
        evaluate_rules(&[allow, ask, deny], &request("git push", &scopes)).outcome,
        RuleOutcome::Deny
    );
}

#[test]
fn no_priority_or_specificity_can_promote_an_allow_past_a_deny() {
    // A priority that could would be a mechanism for switching the safety answer off, reachable by
    // anything that can write a big enough number.
    let scopes = global();
    let shouting_allow = AuthorizationRule {
        priority: i64::MAX,
        matcher: Matcher::Exact("git push".to_string()),
        scope: RuleScope::scoped(RuleScopeKind::Session, "session-1").expect("session"),
        ..rule(
            "allow",
            RuleEffect::Allow,
            Matcher::Any,
            RuleScope::global(),
        )
    };
    let quiet_deny = AuthorizationRule {
        priority: i64::MIN,
        ..rule("deny", RuleEffect::Deny, Matcher::Any, RuleScope::global())
    };
    let mut scoped = scopes.clone();
    scoped.push(RuleScope::scoped(RuleScopeKind::Session, "session-1").expect("session"));

    let decision = evaluate_rules(&[shouting_allow, quiet_deny], &request("git push", &scoped));

    assert_eq!(decision.outcome, RuleOutcome::Deny);
}

#[test]
fn the_decisive_rule_is_the_most_specific_of_the_winning_class() {
    let mut scopes = global();
    scopes.push(RuleScope::scoped(RuleScopeKind::Project, "repo").expect("project"));
    let broad = rule("broad", RuleEffect::Deny, Matcher::Any, RuleScope::global());
    let narrow = rule(
        "narrow",
        RuleEffect::Deny,
        Matcher::Exact("git push".to_string()),
        RuleScope::scoped(RuleScopeKind::Project, "repo").expect("project"),
    );

    let decision = evaluate_rules(&[broad, narrow], &request("git push", &scopes));

    assert_eq!(decision.outcome, RuleOutcome::Deny);
    assert_eq!(
        decision
            .decisive
            .map(|rule| rule.rule_id.as_str().to_string()),
        Some("narrow".to_string())
    );
}

#[test]
fn two_rules_alike_in_every_ordering_field_still_name_one_decisive_rule() {
    // A trace that shifted between runs would be unusable as evidence, so the last tiebreak is the
    // rule id.
    let scopes = global();
    let first = rule("aaa", RuleEffect::Deny, Matcher::Any, RuleScope::global());
    let second = rule("bbb", RuleEffect::Deny, Matcher::Any, RuleScope::global());

    let forward = evaluate_rules(&[first.clone(), second.clone()], &request("x", &scopes));
    let reversed = evaluate_rules(&[second, first], &request("x", &scopes));

    assert_eq!(forward, reversed);
    assert_eq!(
        forward
            .decisive
            .map(|rule| rule.rule_id.as_str().to_string()),
        Some("aaa".to_string())
    );
}

#[test]
fn a_rule_scoped_somewhere_the_request_is_not_does_not_match() {
    let scopes = global();
    let elsewhere = rule(
        "elsewhere",
        RuleEffect::Deny,
        Matcher::Any,
        RuleScope::scoped(RuleScopeKind::Project, "other-repo").expect("project"),
    );

    assert_eq!(
        evaluate_rules(&[elsewhere], &request("git push", &scopes)).outcome,
        RuleOutcome::NoMatch
    );
}

#[test]
fn an_expired_rule_does_not_match() {
    let scopes = global();
    let expired = AuthorizationRule {
        expires_at: Some("2026-08-22T00:00:00Z".to_string()),
        ..rule(
            "expired",
            RuleEffect::Deny,
            Matcher::Any,
            RuleScope::global(),
        )
    };

    assert_eq!(
        evaluate_rules(&[expired], &request("git push", &scopes)).outcome,
        RuleOutcome::NoMatch,
        "an expired Deny falls through rather than lingering"
    );
}

#[test]
fn a_rule_about_another_operation_does_not_contribute_an_implicit_ask() {
    let scopes = global();
    let other = AuthorizationRule {
        operation: OperationName::parse("file.write").expect("operation"),
        ..rule("other", RuleEffect::Deny, Matcher::Any, RuleScope::global())
    };

    assert_eq!(
        evaluate_rules(&[other], &request("git push", &scopes)).outcome,
        RuleOutcome::NoMatch
    );
}

#[test]
fn allowed_scopes_are_intersected_across_every_asking_rule() {
    // Taking the union would let adding a permissive rule quietly extend the reach of a stricter
    // one that still applies.
    let scopes = global();
    let permissive = AuthorizationRule {
        allowed_scopes: AllowedScopes::of(&[
            GrantScope::Once,
            GrantScope::Session,
            GrantScope::Global,
        ]),
        ..rule(
            "permissive",
            RuleEffect::Ask,
            Matcher::Any,
            RuleScope::global(),
        )
    };
    let strict = AuthorizationRule {
        allowed_scopes: AllowedScopes::of(&[GrantScope::Once]),
        ..rule("strict", RuleEffect::Ask, Matcher::Any, RuleScope::global())
    };

    let decision = evaluate_rules(&[permissive, strict], &request("git push", &scopes));

    assert_eq!(decision.outcome, RuleOutcome::Ask);
    assert_eq!(decision.allowed_scopes, vec![GrantScope::Once]);
}

#[test]
fn a_decision_that_is_not_ask_names_no_grant_scopes() {
    let scopes = global();
    let deny = rule("deny", RuleEffect::Deny, Matcher::Any, RuleScope::global());

    assert!(evaluate_rules(&[deny], &request("git push", &scopes))
        .allowed_scopes
        .is_empty());
}

#[test]
fn the_decision_order_is_fixed_and_template_fallback_sits_after_the_rule_set() {
    // The order is the security property. Every step may only make the answer stricter than the
    // one before it, except the template, which is reached only when the rule set said NoMatch.
    let spelled: Vec<&str> = DECISION_ORDER.iter().map(|step| step.as_str()).collect();

    assert_eq!(
        spelled,
        vec![
            "normalize",
            "safety_floor",
            "parent_chain_deny",
            "compiled_rule_set",
            "template_fallback",
            "hook_strengthening",
            "remembered_grant",
            "user_approval",
            "redacted_trace",
        ]
    );

    let index = |step: DecisionTraceStep| {
        DECISION_ORDER
            .iter()
            .position(|candidate| *candidate == step)
            .expect("step is in the order")
    };
    assert!(index(DecisionTraceStep::SafetyFloor) < index(DecisionTraceStep::CompiledRuleSet));
    assert!(index(DecisionTraceStep::CompiledRuleSet) < index(DecisionTraceStep::TemplateFallback));
    assert!(
        index(DecisionTraceStep::TemplateFallback) < index(DecisionTraceStep::HookStrengthening)
    );
    assert!(
        index(DecisionTraceStep::HookStrengthening) < index(DecisionTraceStep::RememberedGrant)
    );
    assert!(index(DecisionTraceStep::RememberedGrant) < index(DecisionTraceStep::UserApproval));
}

#[test]
fn every_outcome_has_a_distinct_spelling_for_the_trace() {
    let mut spellings: Vec<&str> = ALL_RULE_OUTCOMES
        .iter()
        .map(|outcome| outcome.as_str())
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
    assert_eq!(RuleOutcome::NoMatch.as_str(), "no_match");
}

#[test]
fn a_decision_names_the_rule_that_produced_it() {
    // The decisive rule is what an audit line is about; a decision that only said "deny" would
    // leave an operator unable to find which rule to change.
    let scopes = global();
    let deny = rule("deny", RuleEffect::Deny, Matcher::Any, RuleScope::global());

    let decision: RuleDecision = evaluate_rules(&[deny], &request("git push", &scopes));

    let decisive: DecisiveRule = decision
        .decisive
        .expect("a matched decision names its rule");
    assert_eq!(decisive.source, RuleSource::User);
    assert_eq!(decisive.source_id.as_str(), "user-settings");
    assert_eq!(decisive.rule_id.as_str(), "deny");
}

#[test]
fn every_step_spelling_is_distinct() {
    let mut spellings: Vec<&str> = DECISION_ORDER.iter().map(|step| step.as_str()).collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);
}
