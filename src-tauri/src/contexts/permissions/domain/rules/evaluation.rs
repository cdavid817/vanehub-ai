// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! Resolving a set of rules to one outcome, and the fixed order the whole decision runs in.
//!
//! ## Four outcomes, not three
//!
//! `NoMatch` is a distinct answer from `Ask`, and collapsing them would silently replace the
//! existing PDP. Today's templates decide `Allow` for `file.read`; if "no rule matched" arrived as
//! `Ask`, every read would start prompting the moment the first rule set was published, and the
//! rule set would have changed behaviour for operations it never mentioned. Only `NoMatch` falls
//! through to the template.
//!
//! ## Effect precedence is absolute
//!
//! `Deny > Ask > Allow`, over every matching rule regardless of source, scope, priority, or
//! specificity. Those last two order rules *within* the winning class so the trace can name one
//! decisive rule; they cannot promote an `Allow` past a `Deny`. A priority that could would be a
//! mechanism for switching the safety answer off, reachable by anything that can write a number.

use super::{
    AuthorizationRule, GrantScope, OperationName, RuleEffect, RuleId, RuleScope, RuleSource,
    SourceId,
};

/// What a request is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleRequest<'a> {
    pub(crate) operation: &'a OperationName,
    pub(crate) resource: &'a str,
    /// Every scope in force for this request — global, plus the project, principal, and session it
    /// is happening in. A rule matches only if its own scope is one of these.
    pub(crate) scopes: &'a [RuleScope],
    /// Compared against `expires_at` as a string, so both must be the fixed-width UTC form.
    pub(crate) now: &'a str,
}

/// What the compiled rule set decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleOutcome {
    Deny,
    Ask,
    Allow,
    /// No rule in the set covered this request. The only outcome that falls through to the
    /// existing template/PDP.
    NoMatch,
}

impl RuleOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Ask => "ask",
            Self::Allow => "allow",
            Self::NoMatch => "no_match",
        }
    }

    pub(crate) const fn of(effect: RuleEffect) -> Self {
        match effect {
            RuleEffect::Deny => Self::Deny,
            RuleEffect::Ask => Self::Ask,
            RuleEffect::Allow => Self::Allow,
        }
    }

    /// Whether the existing template/PDP is consulted next.
    pub(crate) const fn falls_through_to_template(self) -> bool {
        matches!(self, Self::NoMatch)
    }
}

pub(crate) const ALL_RULE_OUTCOMES: &[RuleOutcome] = &[
    RuleOutcome::Deny,
    RuleOutcome::Ask,
    RuleOutcome::Allow,
    RuleOutcome::NoMatch,
];

/// Which rule decided, named for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecisiveRule {
    pub(crate) source: RuleSource,
    pub(crate) source_id: SourceId,
    pub(crate) rule_id: RuleId,
}

/// The outcome, and what produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleDecision {
    pub(crate) outcome: RuleOutcome,
    /// Absent exactly when the outcome is `NoMatch`.
    pub(crate) decisive: Option<DecisiveRule>,
    /// The scopes an `Ask` may be remembered at, intersected across every rule that produced it.
    ///
    /// Intersected rather than unioned: if one rule says this may be remembered for the session
    /// and another says only once, the answer is once. Taking the union would let adding a
    /// permissive rule quietly extend the reach of a stricter one that still applies.
    pub(crate) allowed_scopes: Vec<GrantScope>,
}

impl RuleDecision {
    fn no_match() -> Self {
        Self {
            outcome: RuleOutcome::NoMatch,
            decisive: None,
            allowed_scopes: Vec::new(),
        }
    }
}

fn matches(rule: &AuthorizationRule, request: &RuleRequest<'_>) -> bool {
    &rule.operation == request.operation
        && rule.matcher.matches(request.resource)
        && request.scopes.contains(&rule.scope)
        && !rule.is_expired_at(request.now)
}

/// Resolves every matching rule to one decision.
///
/// Deterministic: among the rules in the winning effect class, the decisive one is the most
/// specific, then the highest priority, then the lowest rule id. The last tiebreak exists so that
/// two rules identical in every ordering field still name the same decisive rule on every machine
/// — a trace that shifted between runs would be unusable as evidence.
pub(crate) fn evaluate_rules(
    rules: &[AuthorizationRule],
    request: &RuleRequest<'_>,
) -> RuleDecision {
    let matching: Vec<&AuthorizationRule> =
        rules.iter().filter(|rule| matches(rule, request)).collect();
    if matching.is_empty() {
        return RuleDecision::no_match();
    }

    let winning = matching
        .iter()
        .map(|rule| rule.effect)
        .max_by_key(|effect| effect.precedence())
        .unwrap_or(RuleEffect::Ask);

    let mut decisive_candidates: Vec<&&AuthorizationRule> = matching
        .iter()
        .filter(|rule| rule.effect == winning)
        .collect();
    decisive_candidates.sort_by(|left, right| {
        right
            .specificity()
            .cmp(&left.specificity())
            .then(right.priority.cmp(&left.priority))
            .then(left.rule_id.as_str().cmp(right.rule_id.as_str()))
    });

    let allowed_scopes = if winning == RuleEffect::Ask {
        intersect_allowed_scopes(&decisive_candidates)
    } else {
        Vec::new()
    };

    RuleDecision {
        outcome: RuleOutcome::of(winning),
        decisive: decisive_candidates.first().map(|rule| DecisiveRule {
            source: rule.source,
            source_id: rule.source_id.clone(),
            rule_id: rule.rule_id.clone(),
        }),
        allowed_scopes,
    }
}

/// The scopes every asking rule agrees may be remembered.
fn intersect_allowed_scopes(asking: &[&&AuthorizationRule]) -> Vec<GrantScope> {
    super::ALL_GRANT_SCOPES
        .iter()
        .copied()
        .filter(|scope| rule_set_admits(asking, *scope))
        .collect()
}

fn rule_set_admits(asking: &[&&AuthorizationRule], scope: GrantScope) -> bool {
    !asking.is_empty() && asking.iter().all(|rule| rule.allowed_scopes.admits(scope))
}

/// One step of the authorization decision, in the order it runs.
///
/// Written down as a type because the order is the security property. Each step may only make the
/// answer stricter than the one before it, except `TemplateFallback`, which is reached only when
/// the rule set said `NoMatch` and therefore has nothing to be stricter than.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DecisionTraceStep {
    /// The request is normalised and its risk assessed.
    Normalize,
    /// The immutable floor. Not reachable by any rule, template, hook, or user setting.
    SafetyFloor,
    /// An explicit `Deny` anywhere up the delegating principal's chain.
    ParentChainDeny,
    /// The active compiled rule set.
    CompiledRuleSet,
    /// The existing template/PDP, consulted **only** when the rule set said `NoMatch`.
    TemplateFallback,
    /// Hooks, which may strengthen the answer and never weaken it.
    HookStrengthening,
    /// A remembered grant, considered only for an answer that is still `Ask`.
    RememberedGrant,
    /// Asking the person.
    UserApproval,
    /// What is written down afterwards, redacted.
    RedactedTrace,
}

impl DecisionTraceStep {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Normalize => "normalize",
            Self::SafetyFloor => "safety_floor",
            Self::ParentChainDeny => "parent_chain_deny",
            Self::CompiledRuleSet => "compiled_rule_set",
            Self::TemplateFallback => "template_fallback",
            Self::HookStrengthening => "hook_strengthening",
            Self::RememberedGrant => "remembered_grant",
            Self::UserApproval => "user_approval",
            Self::RedactedTrace => "redacted_trace",
        }
    }
}

/// The fixed order. Changing it is a change to the security model, not a refactor.
pub(crate) const DECISION_ORDER: &[DecisionTraceStep] = &[
    DecisionTraceStep::Normalize,
    DecisionTraceStep::SafetyFloor,
    DecisionTraceStep::ParentChainDeny,
    DecisionTraceStep::CompiledRuleSet,
    DecisionTraceStep::TemplateFallback,
    DecisionTraceStep::HookStrengthening,
    DecisionTraceStep::RememberedGrant,
    DecisionTraceStep::UserApproval,
    DecisionTraceStep::RedactedTrace,
];
