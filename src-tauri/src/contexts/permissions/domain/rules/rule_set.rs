// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! An immutable set of rules, and the bytes that give it an identity.
//!
//! ## Why the digest cannot depend on row order
//!
//! A rule set is read back out of SQLite, and SQLite makes no promise about row order without an
//! `ORDER BY` — and even with one, adding an index or a column can change what the planner does.
//! If the digest depended on that, the same rules would hash differently on two machines, "is this
//! the set I reviewed?" would be unanswerable, and the active pointer would look like it had moved
//! when nothing had changed.
//!
//! So the encoding sorts. Every rule is encoded on its own, the encodings are sorted as bytes, and
//! the sorted sequence is hashed. Sorting encodings rather than rules means the order does not
//! depend on which field anyone thought to sort by.
//!
//! ## Why every field is length-prefixed
//!
//! Concatenating fields without a length would let two different rules produce identical bytes —
//! `("ab", "c")` and `("a", "bc")` — and two rules sharing an identity is exactly what a digest
//! exists to make impossible.
//!
//! The hashing itself is not here. The domain produces the bytes; infrastructure hashes them,
//! which is the same split the rest of this repository uses and keeps the domain free of any
//! dependency on a hash implementation.

use super::{AuthorizationRule, RuleSetDigest, RuleSetId};

/// Length-prefixed encoding of one string.
fn push_field(buffer: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    // A fixed-width length, so the prefix itself cannot be confused with content.
    buffer.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    buffer.extend_from_slice(bytes);
}

/// The canonical bytes of one rule.
fn canonical_rule_bytes(rule: &AuthorizationRule) -> Vec<u8> {
    let mut buffer = Vec::new();
    push_field(&mut buffer, rule.source.as_str());
    push_field(&mut buffer, rule.source_id.as_str());
    push_field(&mut buffer, rule.rule_id.as_str());
    push_field(&mut buffer, rule.scope.kind().as_str());
    push_field(&mut buffer, rule.scope.key());
    push_field(&mut buffer, rule.operation.as_str());
    push_field(&mut buffer, &rule.matcher.as_str());
    push_field(&mut buffer, rule.effect.as_str());
    push_field(&mut buffer, &rule.allowed_scopes.as_str());
    push_field(&mut buffer, &rule.priority.to_string());
    // Absent and present-but-empty are different states, so they encode differently.
    match rule.expires_at.as_deref() {
        Some(expiry) => {
            push_field(&mut buffer, "expires");
            push_field(&mut buffer, expiry);
        }
        None => push_field(&mut buffer, "never"),
    }
    push_field(&mut buffer, rule.provenance.as_str());
    buffer
}

/// The canonical bytes of a whole rule set. Independent of the order the rules arrive in.
pub(crate) fn canonical_rule_set_bytes(rules: &[AuthorizationRule]) -> Vec<u8> {
    let mut encodings: Vec<Vec<u8>> = rules.iter().map(canonical_rule_bytes).collect();
    encodings.sort_unstable();

    let mut buffer = Vec::new();
    // The count is inside the digest, so a set cannot be confused with a prefix of a larger one.
    buffer.extend_from_slice(&(encodings.len() as u64).to_be_bytes());
    for encoding in encodings {
        buffer.extend_from_slice(&(encoding.len() as u64).to_be_bytes());
        buffer.extend_from_slice(&encoding);
    }
    buffer
}

/// An immutable set of rules, published as one unit.
///
/// Published as a unit because the rules only mean anything together: activating half of a set
/// would apply a `Deny` without the `Ask` that was meant to accompany it, or the reverse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleSet {
    pub(crate) rule_set_id: RuleSetId,
    pub(crate) content_digest: RuleSetDigest,
    pub(crate) rules: Vec<AuthorizationRule>,
    pub(crate) created_at: String,
}

/// What recording a rule set would mean against what is already stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuleSetOutcome {
    /// Nothing held this identity or these contents. The set is stored.
    Recorded,
    /// The same contents are already stored, under `existing`.
    ///
    /// Carries the id because a caller recompiling the same rules gets a fresh id each time, and
    /// what it needs back is the id already in storage — otherwise it would activate an id that
    /// does not exist.
    AlreadyRecorded { existing: RuleSetId },
    /// The id is held by different contents.
    Conflict(RuleSetContentConflict),
}

impl RuleSetOutcome {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Recorded => "rule_set_recorded",
            Self::AlreadyRecorded { .. } => "rule_set_already_recorded",
            Self::Conflict(_) => "rule_set_content_conflict",
        }
    }

    /// Whether the caller has a rule set it may activate.
    pub(crate) const fn admits_activation(&self) -> bool {
        matches!(self, Self::Recorded | Self::AlreadyRecorded { .. })
    }

    /// The id to activate, when there is one.
    pub(crate) fn activatable(&self, offered: &RuleSetId) -> Option<RuleSetId> {
        match self {
            Self::Recorded => Some(offered.clone()),
            Self::AlreadyRecorded { existing } => Some(existing.clone()),
            Self::Conflict(_) => None,
        }
    }
}

/// One rule-set id, two different contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleSetContentConflict {
    pub(crate) stored_digest: RuleSetDigest,
    pub(crate) offered_digest: RuleSetDigest,
    pub(crate) stored_at: String,
}

impl RuleSetContentConflict {
    pub(crate) const fn code(&self) -> &'static str {
        "rule_set_content_conflict"
    }
}

/// Which rule set is in force.
///
/// One row for the whole application. The rules carry their own Global/User/Project/Principal/
/// Session boundaries, so "which set is live" is a single question — a pointer per scope would let
/// two scopes run rule generations that were never reviewed together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveRuleSet {
    /// `None` before anything has been activated.
    ///
    /// Deliberately not an empty rule set: "no rules have been published" and "a set was published
    /// that happens to contain nothing" are different facts, and a fabricated empty set would make
    /// the second unrepresentable while claiming a digest nobody produced.
    pub(crate) rule_set_id: Option<RuleSetId>,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

/// Why the active pointer could not be moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveRuleSetError {
    /// Someone else moved the pointer since the caller read it.
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    /// The rule set named does not exist. Refused by the database's reference; reported here so a
    /// caller does not have to read a foreign-key message.
    UnknownRuleSet,
    Storage(String),
}

impl ActiveRuleSetError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "active_rule_set_stale_revision",
            Self::UnknownRuleSet => "unknown_rule_set",
            Self::Storage(_) => "active_rule_set_storage_failure",
        }
    }
}

pub(crate) fn all_active_rule_set_errors() -> Vec<ActiveRuleSetError> {
    vec![
        ActiveRuleSetError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        ActiveRuleSetError::UnknownRuleSet,
        ActiveRuleSetError::Storage(String::new()),
    ]
}
