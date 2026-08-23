// The dispatch engine that consumes these lands with Task Group 7; Task Group 3 lands the storage
// they are written through. Dead-code analysis walks from live roots, so the chain reads as unused
// until that root exists.
#![cfg_attr(not(test), allow(dead_code))]

//! Validated identities for Hook subjects, the snapshots their definitions came from, and the
//! bounded vocabulary an execution row is allowed to say.
//!
//! Two of these are load-bearing beyond "a newtype is tidier than a `String`".
//!
//! `SnapshotRef` is deliberately **opaque**. A definition revision names the snapshot it came
//! from, but `extension_platform` owns snapshots, and an enforced reference across that boundary
//! would let one subdomain's deletions reach into another's evidence. So this subdomain validates
//! the *shape* of the reference and never resolves it; resolution is a read through a projection
//! port, and the result is a reconciliation verdict rather than a foreign key.
//!
//! `HookOutcomeCode` is the redaction floor. An execution row records *that* a Hook failed and
//! under which stable code, never what it said: the grammar admits only lower_snake_case, which
//! makes "just put the error message in the outcome" fail at the constructor rather than being
//! caught by a reviewer who happens to be paying attention. A message contains prompts, paths, and
//! whatever a Hook's stderr felt like emitting, and none of that belongs in a durable row.

use super::HookIdentifierKind;

const MAX_GLOBAL_ID_CHARACTERS: usize = 160;
const MAX_OPAQUE_ID_CHARACTERS: usize = 128;
const MAX_SCOPE_KEY_CHARACTERS: usize = 256;
const MAX_OUTCOME_CODE_CHARACTERS: usize = 64;
/// SHA-256, rendered lower-case hex.
const DIGEST_CHARACTERS: usize = 64;

/// Truncates untrusted text before it enters a diagnostic, so a hostile value cannot make the
/// rejection itself unbounded.
fn bounded(value: &str) -> String {
    value.chars().take(MAX_GLOBAL_ID_CHARACTERS).collect()
}

/// Why a value could not be read as an identity.
///
/// Carries the offending text, bounded, because "some hook id was invalid" is not something an
/// operator can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookIdentityError {
    pub(crate) kind: HookIdentifierKind,
    pub(crate) value: String,
}

impl HookIdentityError {
    fn new(kind: HookIdentifierKind, value: &str) -> Self {
        Self {
            kind,
            value: bounded(value),
        }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.kind.code()
    }
}

/// The stable identity of one Hook, for as long as any evidence mentions it.
///
/// Survives every upgrade: a binding references this, never a definition revision, so a snapshot
/// that goes away cannot take a user's enablement with it.
///
/// The grammar admits `:` and `.` because contribution ids are namespaced (`ext::acme.git-guardian::pre-commit`)
/// and this subdomain must be able to hold one without knowing how it was composed. It validates
/// the shape and nothing else — what the segments *mean* is `extension_platform`'s business.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HookGlobalId(String);

impl HookGlobalId {
    pub(crate) fn parse(value: &str) -> Result<Self, HookIdentityError> {
        let acceptable = |character: char| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '.' | ':')
        };
        if value.is_empty()
            || value.len() > MAX_GLOBAL_ID_CHARACTERS
            || value.starts_with(['-', '_', '.', ':'])
            || value.ends_with(['-', '_', '.', ':'])
            || !value.chars().all(acceptable)
        {
            return Err(HookIdentityError::new(
                HookIdentifierKind::HookGlobal,
                value,
            ));
        }
        Ok(Self(value.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A snapshot, as named by something outside this subdomain.
///
/// Validated as text, never resolved here. See the module header.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SnapshotRef(String);

impl SnapshotRef {
    pub(crate) fn parse(value: &str) -> Result<Self, HookIdentityError> {
        if is_opaque_id(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(HookIdentityError::new(
                HookIdentifierKind::SnapshotRef,
                value,
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// One recorded execution. Host-generated, never parsed from anything an extension supplied.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HookExecutionId(String);

impl HookExecutionId {
    pub(crate) fn parse(value: &str) -> Result<Self, HookIdentityError> {
        if is_opaque_id(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(HookIdentityError::new(
                HookIdentifierKind::HookExecution,
                value,
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The digest of a definition's canonical form.
///
/// What makes "the same definition, recorded twice" distinguishable from "two different
/// definitions claiming the same identity" without storing the definition body twice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DefinitionDigest(String);

impl DefinitionDigest {
    pub(crate) fn parse(value: &str) -> Result<Self, HookIdentityError> {
        if value.len() == DIGEST_CHARACTERS
            && value
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            Ok(Self(value.to_string()))
        } else {
            Err(HookIdentityError::new(
                HookIdentifierKind::DefinitionDigest,
                value,
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A stable code an execution row is allowed to carry.
///
/// The redaction floor: see the module header. Nothing here is free text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HookOutcomeCode(String);

impl HookOutcomeCode {
    pub(crate) fn parse(value: &str) -> Result<Self, HookIdentityError> {
        if !value.is_empty()
            && value.len() <= MAX_OUTCOME_CODE_CHARACTERS
            && !value.starts_with('_')
            && !value.ends_with('_')
            && value.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
        {
            Ok(Self(value.to_string()))
        } else {
            Err(HookIdentityError::new(
                HookIdentifierKind::OutcomeCode,
                value,
            ))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a binding applies.
///
/// Modelled as a kind plus a key rather than one nullable column, because SQLite treats `NULL` as
/// distinct from every other `NULL` in a unique index: a `(hook, scope)` key with `NULL` meaning
/// "global" would admit unlimited global bindings for one Hook, each invisible to the others, and
/// whichever the reader happened to see first would decide whether the Hook ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HookScopeKind {
    Global,
    Project,
    Agent,
}

impl HookScopeKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
            Self::Agent => "agent",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "global" => Some(Self::Global),
            "project" => Some(Self::Project),
            "agent" => Some(Self::Agent),
            _ => None,
        }
    }
}

pub(crate) const ALL_HOOK_SCOPE_KINDS: &[HookScopeKind] = &[
    HookScopeKind::Global,
    HookScopeKind::Project,
    HookScopeKind::Agent,
];

/// The global scope's key. Empty rather than absent, so the unique index is total.
pub(crate) const GLOBAL_SCOPE_KEY: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HookScope {
    kind: HookScopeKind,
    key: String,
}

impl HookScope {
    pub(crate) fn global() -> Self {
        Self {
            kind: HookScopeKind::Global,
            key: GLOBAL_SCOPE_KEY.to_string(),
        }
    }

    /// A scope narrower than global. The key must be present and bounded; a scoped binding with no
    /// key would be a second spelling of "global" and the two would disagree.
    pub(crate) fn scoped(kind: HookScopeKind, key: &str) -> Result<Self, HookIdentityError> {
        if kind == HookScopeKind::Global {
            return Err(HookIdentityError::new(HookIdentifierKind::ScopeKey, key));
        }
        if key.is_empty() || key.len() > MAX_SCOPE_KEY_CHARACTERS || key.contains('\0') {
            return Err(HookIdentityError::new(HookIdentifierKind::ScopeKey, key));
        }
        Ok(Self {
            kind,
            key: key.to_string(),
        })
    }

    /// Rebuilds a scope from the two columns it is stored as, refusing any pair the constructors
    /// above could not have produced.
    pub(crate) fn parse(kind: &str, key: &str) -> Result<Self, HookIdentityError> {
        let Some(kind) = HookScopeKind::parse(kind) else {
            return Err(HookIdentityError::new(HookIdentifierKind::ScopeKind, kind));
        };
        match kind {
            HookScopeKind::Global if key == GLOBAL_SCOPE_KEY => Ok(Self::global()),
            HookScopeKind::Global => Err(HookIdentityError::new(HookIdentifierKind::ScopeKey, key)),
            other => Self::scoped(other, key),
        }
    }

    pub(crate) const fn kind(&self) -> HookScopeKind {
        self.kind
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

/// Application-generated opaque identifier. Not parsed from a manifest, so the rule only has to
/// exclude what would break a log line, a path, or a URL.
fn is_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_OPAQUE_ID_CHARACTERS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}
