//! The one canonical authorization-rule model.
//!
//! There is exactly one `AuthorizationRule`, one immutable rule set, one active pointer, one
//! repository, and one evaluator. The Unified Extension Platform contributes an `Extension` rule
//! source into this model; the Permission Policy Center manages the `User` and `Project` sources
//! in the same model. Neither builds a second rule table, and there is no dual-write layer.
//!
//! Templates are **not** part of it. `agent_principals.template_name`, `PolicyTemplateName`, and
//! `policies_for_template` stay with the existing PDP, which is consulted only when the rule set
//! returns `NoMatch`. Remembered grants are not part of it either: they stay with the Approval
//! Broker and are consulted only for an answer that is still `Ask`.

mod evaluation;
#[cfg(test)]
mod evaluation_tests;
mod identity;
#[cfg(test)]
mod identity_tests;
mod rule;
mod rule_set;
#[cfg(test)]
mod rule_set_tests;
#[cfg(test)]
mod rule_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use evaluation::{
    evaluate_rules, DecisionTraceStep, DecisiveRule, RuleDecision, RuleOutcome, RuleRequest,
    ALL_RULE_OUTCOMES, DECISION_ORDER,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use identity::{
    Matcher, OperationName, RuleId, RuleIdentifierKind, RuleIdentityError, RuleScope,
    RuleScopeKind, RuleSetDigest, RuleSetId, SourceId, ALL_RULE_IDENTIFIER_KINDS,
    ALL_RULE_SCOPE_KINDS, GLOBAL_RULE_SCOPE_KEY,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use rule::{
    AllowedScopes, AuthorizationRule, GrantScope, RuleAdmissionError, RuleEffect, RuleProvenance,
    RuleSource, ALL_GRANT_SCOPES, ALL_RULE_EFFECTS, ALL_RULE_PROVENANCES, ALL_RULE_SOURCES,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use rule_set::{
    all_active_rule_set_errors, canonical_rule_set_bytes, ActiveRuleSet, ActiveRuleSetError,
    RuleSet, RuleSetContentConflict, RuleSetOutcome,
};

/// Every stable failure code this subdomain can present to a caller.
///
/// Kept per-subdomain: what matters is that no two failures *within* one present the same identity
/// to a caller branching on it.
#[cfg(test)]
pub(crate) fn registered_rule_failures() -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = ALL_RULE_IDENTIFIER_KINDS
        .iter()
        .map(|kind| kind.code())
        .collect();
    codes.extend(
        all_active_rule_set_errors()
            .iter()
            .map(ActiveRuleSetError::code)
            .collect::<Vec<_>>(),
    );
    codes.push(RuleAdmissionError::ExtensionMayNotAllow.code());
    codes.push(RuleAdmissionError::AllowedScopesContradictEffect.code());
    codes.push(RuleSetOutcome::Recorded.code());
    codes.push("rule_set_already_recorded");
    codes.push("rule_set_content_conflict");
    codes
}
