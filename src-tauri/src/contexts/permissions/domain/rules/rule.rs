// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! One authorization rule: who wrote it, what it covers, and what it decides.
//!
//! ## What an extension may contribute
//!
//! An `Extension` rule may carry `Ask` or `Deny` and nothing else. A downloaded package that could
//! contribute `Allow` would be able to grant itself — or anything else — a permission the user
//! never approved, which inverts the entire point of installing it behind a review. The
//! restriction is enforced twice, in the constructor and by a table `CHECK`, because an `Allow`
//! written by any route at all is a privilege escalation and one of the two will be the one that
//! is still there after a refactor.
//!
//! ## What `priority` and `specificity` do not do
//!
//! Neither can promote `Allow` over `Deny`. Effect precedence is absolute — `Deny > Ask > Allow` —
//! and these two only order rules *within* the winning class, so the trace can name a decisive
//! rule deterministically. A priority that could override a Deny would be a mechanism for turning
//! the safety answer off, reachable by anything that can write a rule with a big enough number.

use super::{
    Matcher, OperationName, RuleId, RuleIdentifierKind, RuleIdentityError, RuleScope, SourceId,
};

/// Who wrote a rule.
///
/// A closed set: a rule whose author this build cannot name is a rule whose entitlement it cannot
/// check, and the safe reading of that is to refuse the rule set rather than to trust the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleSource {
    /// Written by the person using this installation.
    User,
    /// Written into a workspace, applying to anyone who opens it.
    Project,
    /// Contributed by an installed extension package.
    Extension,
}

impl RuleSource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Extension => "extension",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        ALL_RULE_SOURCES
            .iter()
            .copied()
            .find(|source| source.as_str() == value)
            .ok_or_else(|| RuleIdentityError::new(RuleIdentifierKind::Source, value))
    }

    /// Whether this source may contribute an `Allow`. See the module header.
    pub(crate) const fn admits_allow(self) -> bool {
        match self {
            Self::User | Self::Project => true,
            Self::Extension => false,
        }
    }
}

pub(crate) const ALL_RULE_SOURCES: &[RuleSource] =
    &[RuleSource::User, RuleSource::Project, RuleSource::Extension];

/// What a rule decides for what it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleEffect {
    Deny,
    Ask,
    Allow,
}

impl RuleEffect {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Ask => "ask",
            Self::Allow => "allow",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        ALL_RULE_EFFECTS
            .iter()
            .copied()
            .find(|effect| effect.as_str() == value)
            .ok_or_else(|| RuleIdentityError::new(RuleIdentifierKind::Effect, value))
    }

    /// Higher wins. `Deny > Ask > Allow`, absolutely and without exception.
    pub(crate) const fn precedence(self) -> u8 {
        match self {
            Self::Allow => 0,
            Self::Ask => 1,
            Self::Deny => 2,
        }
    }
}

pub(crate) const ALL_RULE_EFFECTS: &[RuleEffect] =
    &[RuleEffect::Deny, RuleEffect::Ask, RuleEffect::Allow];

/// How long an `Ask` this rule produces may be remembered for.
///
/// A rule that decides `Ask` also decides how durable the answer may be. Without this, a rule
/// author could only choose between "always ask" and "ask once, then whatever scope the approval
/// dialog happened to offer" — and the second is how a one-off approval becomes permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GrantScope {
    Once,
    Session,
    Project,
    Global,
}

impl GrantScope {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Session => "session",
            Self::Project => "project",
            Self::Global => "global",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        ALL_GRANT_SCOPES
            .iter()
            .copied()
            .find(|scope| scope.as_str() == value)
            .ok_or_else(|| RuleIdentityError::new(RuleIdentifierKind::GrantScope, value))
    }
}

pub(crate) const ALL_GRANT_SCOPES: &[GrantScope] = &[
    GrantScope::Once,
    GrantScope::Session,
    GrantScope::Project,
    GrantScope::Global,
];

/// The scopes a remembered grant may satisfy this rule's `Ask` at.
///
/// Canonically ordered and deduplicated, so two authors writing the same set in different orders
/// produce the same bytes — which is what keeps the rule-set digest from depending on how a rule
/// happened to be typed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct AllowedScopes(Vec<GrantScope>);

impl AllowedScopes {
    /// Empty means the answer may never be remembered: every occurrence asks again.
    pub(crate) fn none() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn of(scopes: &[GrantScope]) -> Self {
        let mut ordered: Vec<GrantScope> = scopes.to_vec();
        ordered.sort_unstable();
        ordered.dedup();
        Self(ordered)
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        if value.is_empty() {
            return Ok(Self::none());
        }
        let scopes = value
            .split(',')
            .map(GrantScope::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::of(&scopes))
    }

    pub(crate) fn as_str(&self) -> String {
        self.0
            .iter()
            .map(|scope| scope.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub(crate) fn admits(&self, scope: GrantScope) -> bool {
        self.0.contains(&scope)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Where a rule's text came from.
///
/// A closed vocabulary rather than free text, for the reason `HookOutcomeCode` is: an audited row
/// with a free field is where an absolute path or a prompt fragment ends up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleProvenance {
    UserSettings,
    ProjectSettings,
    ExtensionManifest,
    HostDefault,
}

impl RuleProvenance {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::UserSettings => "user_settings",
            Self::ProjectSettings => "project_settings",
            Self::ExtensionManifest => "extension_manifest",
            Self::HostDefault => "host_default",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        ALL_RULE_PROVENANCES
            .iter()
            .copied()
            .find(|provenance| provenance.as_str() == value)
            .ok_or_else(|| RuleIdentityError::new(RuleIdentifierKind::Provenance, value))
    }
}

pub(crate) const ALL_RULE_PROVENANCES: &[RuleProvenance] = &[
    RuleProvenance::UserSettings,
    RuleProvenance::ProjectSettings,
    RuleProvenance::ExtensionManifest,
    RuleProvenance::HostDefault,
];

/// One rule, as it is stored and evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizationRule {
    pub(crate) source: RuleSource,
    pub(crate) source_id: SourceId,
    pub(crate) rule_id: RuleId,
    pub(crate) scope: RuleScope,
    pub(crate) operation: OperationName,
    pub(crate) matcher: Matcher,
    pub(crate) effect: RuleEffect,
    pub(crate) allowed_scopes: AllowedScopes,
    pub(crate) priority: i64,
    /// Absent means it never expires. An RFC 3339 instant, compared as a string, which is why the
    /// format has to be the fixed-width UTC one everything else in this repository writes.
    pub(crate) expires_at: Option<String>,
    pub(crate) provenance: RuleProvenance,
}

/// Why a rule could not be admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuleAdmissionError {
    /// An extension contributed an `Allow`.
    ExtensionMayNotAllow,
    /// A rule that decides `Ask` but permits no remembering, or one that decides `Deny`/`Allow`
    /// yet names grant scopes. Both are contradictions rather than preferences, and storing one
    /// would leave the evaluator holding a rule whose two halves disagree.
    AllowedScopesContradictEffect,
}

impl RuleAdmissionError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::ExtensionMayNotAllow => "extension_rule_may_not_allow",
            Self::AllowedScopesContradictEffect => "rule_allowed_scopes_contradict_effect",
        }
    }
}

impl AuthorizationRule {
    /// Admits a rule, or says why not.
    ///
    /// The only constructor. Building the struct literally is possible inside this crate, which is
    /// why the table carries the same `CHECK`: two independent refusals, because an `Allow` from a
    /// downloaded package is a privilege escalation and one guard will eventually be refactored
    /// past.
    pub(crate) fn admit(rule: Self) -> Result<Self, RuleAdmissionError> {
        if rule.effect == RuleEffect::Allow && !rule.source.admits_allow() {
            return Err(RuleAdmissionError::ExtensionMayNotAllow);
        }
        let names_scopes = !rule.allowed_scopes.is_empty();
        if (rule.effect == RuleEffect::Ask) != names_scopes {
            return Err(RuleAdmissionError::AllowedScopesContradictEffect);
        }
        Ok(rule)
    }

    /// How specific this rule is, for ordering *within* one effect class.
    ///
    /// Scope dominates matcher: a session-scoped `any` is about this session, while a global
    /// `exact:` is about everyone. Never compared across effects — see the module header.
    pub(crate) fn specificity(&self) -> i64 {
        self.scope
            .kind()
            .specificity()
            .saturating_mul(1_000_000)
            .saturating_add(self.matcher.specificity().min(1_000))
    }

    /// Whether the rule has expired at `now`.
    pub(crate) fn is_expired_at(&self, now: &str) -> bool {
        self.expires_at
            .as_deref()
            .is_some_and(|expiry| expiry <= now)
    }
}
