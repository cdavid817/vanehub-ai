// The PDP integration that consumes these lands with its own task group; Task Group 3 lands the
// model and the storage. Dead-code analysis walks from live roots.
#![cfg_attr(not(test), allow(dead_code))]

//! Validated identities for authorization rules, the sets they belong to, and what they match.
//!
//! Three of these carry weight beyond tidiness.
//!
//! `Matcher` is a **closed set of shapes**, not a pattern language. A rule that could carry an
//! arbitrary expression would need an evaluator, and an evaluator that cannot decide is one that
//! has to guess — in a component whose job is to decide whether something dangerous may happen.
//!
//! `RuleProvenance` is a **closed vocabulary**, for the same reason `HookOutcomeCode` is: a free
//! text field on an audited row is where an absolute path or a prompt fragment ends up, written by
//! whoever was in a hurry.
//!
//! `RuleScope` is a kind plus a key rather than one nullable column. SQLite treats `NULL` as
//! distinct from every other `NULL` in a unique index, so a nullable scope would admit unlimited
//! "global" rows for one rule id, each invisible to the others.

const MAX_ID_CHARACTERS: usize = 128;
const MAX_MATCHER_VALUE_CHARACTERS: usize = 512;
const MAX_SCOPE_KEY_CHARACTERS: usize = 256;
const MAX_OPERATION_CHARACTERS: usize = 96;
/// SHA-256, rendered lower-case hex.
const DIGEST_CHARACTERS: usize = 64;

/// Which identity failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleIdentifierKind {
    RuleSet,
    Rule,
    Source,
    Digest,
    Operation,
    Matcher,
    ScopeKind,
    ScopeKey,
    Effect,
    Provenance,
    GrantScope,
}

impl RuleIdentifierKind {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::RuleSet => "invalid_rule_set_id",
            Self::Rule => "invalid_rule_id",
            Self::Source => "invalid_rule_source",
            Self::Digest => "invalid_rule_set_digest",
            Self::Operation => "invalid_rule_operation",
            Self::Matcher => "invalid_rule_matcher",
            Self::ScopeKind => "invalid_rule_scope_kind",
            Self::ScopeKey => "invalid_rule_scope_key",
            Self::Effect => "invalid_rule_effect",
            Self::Provenance => "invalid_rule_provenance",
            Self::GrantScope => "invalid_rule_allowed_scope",
        }
    }
}

pub(crate) const ALL_RULE_IDENTIFIER_KINDS: &[RuleIdentifierKind] = &[
    RuleIdentifierKind::RuleSet,
    RuleIdentifierKind::Rule,
    RuleIdentifierKind::Source,
    RuleIdentifierKind::Digest,
    RuleIdentifierKind::Operation,
    RuleIdentifierKind::Matcher,
    RuleIdentifierKind::ScopeKind,
    RuleIdentifierKind::ScopeKey,
    RuleIdentifierKind::Effect,
    RuleIdentifierKind::Provenance,
    RuleIdentifierKind::GrantScope,
];

/// Why a value could not be read as an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleIdentityError {
    pub(crate) kind: RuleIdentifierKind,
    pub(crate) value: String,
}

impl RuleIdentityError {
    pub(super) fn new(kind: RuleIdentifierKind, value: &str) -> Self {
        Self {
            kind,
            // Bounded, so a hostile value cannot make the diagnostic itself unbounded.
            value: value.chars().take(MAX_ID_CHARACTERS).collect(),
        }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.kind.code()
    }
}

/// Host-generated opaque identifier: lower-case alphanumerics, hyphen, underscore.
fn is_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_CHARACTERS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

macro_rules! opaque_identifier {
    ($name:ident, $kind:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
                if is_opaque_id(value) {
                    Ok(Self(value.to_string()))
                } else {
                    Err(RuleIdentityError::new($kind, value))
                }
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_identifier!(RuleSetId, RuleIdentifierKind::RuleSet);
opaque_identifier!(RuleId, RuleIdentifierKind::Rule);
opaque_identifier!(SourceId, RuleIdentifierKind::Source);

/// The digest of a rule set's canonical form.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RuleSetDigest(String);

impl RuleSetDigest {
    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        if value.len() == DIGEST_CHARACTERS
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            Ok(Self(value.to_string()))
        } else {
            Err(RuleIdentityError::new(RuleIdentifierKind::Digest, value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a rule is about: `shell.exec`, `file.write`, `mcp.tool.call`.
///
/// Open-ended, like the PDP's existing `Action`, so the operation vocabulary can grow without a
/// breaking change. Bounded and lexically constrained so it cannot become a sentence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OperationName(String);

impl OperationName {
    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        let acceptable = |character: char| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_')
        };
        if !value.is_empty()
            && value.len() <= MAX_OPERATION_CHARACTERS
            && !value.starts_with(['.', '_'])
            && !value.ends_with(['.', '_'])
            && value.chars().all(acceptable)
        {
            Ok(Self(value.to_string()))
        } else {
            Err(RuleIdentityError::new(RuleIdentifierKind::Operation, value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// What resources a rule covers.
///
/// Three shapes and no expression language. See the module header.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Matcher {
    /// Every resource for the operation.
    Any,
    /// Exactly this resource.
    Exact(String),
    /// Every resource starting with this. Empty prefixes are refused: `prefix:` would be a second,
    /// less obvious spelling of `any`, and two spellings of one rule is how a review misses one.
    Prefix(String),
}

impl Matcher {
    pub(crate) fn as_str(&self) -> String {
        match self {
            Self::Any => "any".to_string(),
            Self::Exact(value) => format!("exact:{value}"),
            Self::Prefix(value) => format!("prefix:{value}"),
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, RuleIdentityError> {
        let refuse = || RuleIdentityError::new(RuleIdentifierKind::Matcher, value);
        let bounded = |body: &str| {
            (!body.is_empty() && body.len() <= MAX_MATCHER_VALUE_CHARACTERS && !body.contains('\0'))
                .then(|| body.to_string())
        };
        match value {
            "any" => Ok(Self::Any),
            _ => {
                if let Some(body) = value.strip_prefix("exact:") {
                    return bounded(body).map(Self::Exact).ok_or_else(refuse);
                }
                if let Some(body) = value.strip_prefix("prefix:") {
                    return bounded(body).map(Self::Prefix).ok_or_else(refuse);
                }
                Err(refuse())
            }
        }
    }

    pub(crate) fn matches(&self, resource: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => expected == resource,
            Self::Prefix(prefix) => resource.starts_with(prefix.as_str()),
        }
    }

    /// How specific this matcher is. Only ever compared, never interpreted as a quantity.
    pub(crate) fn specificity(&self) -> i64 {
        match self {
            Self::Any => 0,
            // A longer prefix is more specific than a shorter one, and an exact match is more
            // specific than any prefix of it.
            Self::Prefix(value) => 1 + i64::try_from(value.len()).unwrap_or(i64::MAX - 2),
            Self::Exact(_) => i64::MAX - 1,
        }
    }
}

/// Where a rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleScopeKind {
    Global,
    User,
    Project,
    Principal,
    Session,
}

impl RuleScopeKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::User => "user",
            Self::Project => "project",
            Self::Principal => "principal",
            Self::Session => "session",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_RULE_SCOPE_KINDS
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
    }

    /// Narrower scopes are more specific. Compared, never summed.
    pub(crate) const fn specificity(self) -> i64 {
        match self {
            Self::Global => 0,
            Self::User => 1,
            Self::Project => 2,
            Self::Principal => 3,
            Self::Session => 4,
        }
    }
}

pub(crate) const ALL_RULE_SCOPE_KINDS: &[RuleScopeKind] = &[
    RuleScopeKind::Global,
    RuleScopeKind::User,
    RuleScopeKind::Project,
    RuleScopeKind::Principal,
    RuleScopeKind::Session,
];

/// The global scope's key. Empty rather than absent, so the unique index is total.
pub(crate) const GLOBAL_RULE_SCOPE_KEY: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RuleScope {
    kind: RuleScopeKind,
    key: String,
}

impl RuleScope {
    pub(crate) fn global() -> Self {
        Self {
            kind: RuleScopeKind::Global,
            key: GLOBAL_RULE_SCOPE_KEY.to_string(),
        }
    }

    pub(crate) fn scoped(kind: RuleScopeKind, key: &str) -> Result<Self, RuleIdentityError> {
        let refuse = || RuleIdentityError::new(RuleIdentifierKind::ScopeKey, key);
        if kind == RuleScopeKind::Global
            || key.is_empty()
            || key.len() > MAX_SCOPE_KEY_CHARACTERS
            || key.contains('\0')
        {
            return Err(refuse());
        }
        Ok(Self {
            kind,
            key: key.to_string(),
        })
    }

    /// Rebuilds a scope from the two columns it is stored as, refusing any pair the constructors
    /// could not have produced.
    pub(crate) fn parse(kind: &str, key: &str) -> Result<Self, RuleIdentityError> {
        let Some(kind) = RuleScopeKind::parse(kind) else {
            return Err(RuleIdentityError::new(RuleIdentifierKind::ScopeKind, kind));
        };
        match kind {
            RuleScopeKind::Global if key == GLOBAL_RULE_SCOPE_KEY => Ok(Self::global()),
            RuleScopeKind::Global => Err(RuleIdentityError::new(RuleIdentifierKind::ScopeKey, key)),
            other => Self::scoped(other, key),
        }
    }

    pub(crate) const fn kind(&self) -> RuleScopeKind {
        self.kind
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}
