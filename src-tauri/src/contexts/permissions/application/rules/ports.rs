// See the domain's `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! What the authorization-rule store may be asked to do.
//!
//! Two repositories, because a rule set is immutable content and the active pointer is mutable
//! state under compare-and-swap. One repository holding both would offer an `update_rule_set` that
//! nothing should ever be able to call.
//!
//! Neither touches `agent_principals` or `permission_grants`. Templates stay with the existing
//! PDP, grants stay with the Approval Broker, and this store knows about neither — which is what
//! makes "the rule set changed" incapable of altering a template assignment or deleting a grant.

use crate::contexts::permissions::domain::rules::{
    ActiveRuleSet, ActiveRuleSetError, AuthorizationRule, RuleSet, RuleSetDigest, RuleSetId,
    RuleSetOutcome,
};

/// Immutable rule sets.
pub(crate) trait RuleSetRepository: Send + Sync {
    /// Stores a set, reporting what it meant against what is already there.
    ///
    /// Never overwrites. An id held by different contents yields `Conflict` and the stored set is
    /// untouched; identical contents under a different id yield `AlreadyRecorded` naming the id
    /// already in storage, because a caller recompiling the same rules gets a fresh id each time
    /// and needs the stored one back to activate anything.
    fn record(
        &self,
        rule_set_id: &RuleSetId,
        digest: &RuleSetDigest,
        rules: &[AuthorizationRule],
        created_at: &str,
    ) -> Result<RuleSetOutcome, String>;

    /// Reads a set back whole.
    ///
    /// Fails rather than skipping a row it cannot parse. A rule set with one unreadable rule
    /// silently dropped would be a rule set missing exactly one `Deny`, and nothing downstream
    /// could tell that had happened.
    fn rule_set(&self, rule_set_id: &RuleSetId) -> Result<Option<RuleSet>, String>;

    fn by_digest(&self, digest: &RuleSetDigest) -> Result<Option<RuleSetId>, String>;
}

/// The single pointer that says which rule set is in force.
pub(crate) trait ActiveRuleSetRepository: Send + Sync {
    fn active(&self) -> Result<ActiveRuleSet, ActiveRuleSetError>;

    /// Moves the pointer, refusing the write if someone else moved it since `expected_revision`
    /// was read.
    fn activate(
        &self,
        rule_set_id: &RuleSetId,
        expected_revision: i64,
        updated_at: &str,
    ) -> Result<ActiveRuleSet, ActiveRuleSetError>;
}
