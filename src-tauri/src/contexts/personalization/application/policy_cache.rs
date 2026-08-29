use std::collections::BTreeMap;
use std::sync::Mutex;

use super::error::PersonalizationApplicationError;
use crate::contexts::personalization::domain::{
    AgentId, PersonalizationPolicyScope, PolicyResolutionBundle, WorkspaceKey,
};

/// Exactly which resolution a cached bundle answers.
///
/// Every dimension that changes what was read is in the key, because a bundle answers one question
/// and reusing it for a different one would be worse than not caching at all. In particular the
/// scope-key set is part of it: a bundle read for an installation with no workspace proves nothing
/// about a workspace override, and lending it to a workspace resolution would silently assert that
/// no such override exists.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PolicyCacheKey {
    pub(crate) policy_set_id: String,
    pub(crate) agent_id: AgentId,
    pub(crate) workspace: Option<WorkspaceKey>,
    /// Sorted, so two callers asking for the same scopes in different orders share one entry.
    pub(crate) scope_keys: Vec<String>,
    /// The migration generation the bundle was read under. A migration or a schema change makes
    /// every earlier bundle unusable rather than merely stale.
    pub(crate) generation: u64,
}

impl PolicyCacheKey {
    pub(crate) fn new(
        policy_set_id: impl Into<String>,
        agent_id: AgentId,
        workspace: Option<WorkspaceKey>,
        scopes: &[PersonalizationPolicyScope],
        generation: u64,
    ) -> Self {
        let mut scope_keys: Vec<String> = scopes.iter().map(|scope| scope.scope_key()).collect();
        scope_keys.sort();
        Self {
            policy_set_id: policy_set_id.into(),
            agent_id,
            workspace,
            scope_keys,
            generation,
        }
    }
}

/// The last policy bundle that was read *and validated*, per exact context.
///
/// # What may be cached
///
/// Only the durable policy rows, including the finding that a scope has no override. Nothing else:
/// not the resolved snapshot, not memory eligibility, not memory summaries, not migration health,
/// not capabilities, not the session mode, not the workspace display path. Every one of those is
/// read fresh on each resolution, which is what stops a cached "memory was enabled" from surviving
/// a store that has since gone into repair.
///
/// # When it may be used
///
/// Only for a read that failed transiently — the database was locked, the file was momentarily
/// unavailable. A bundle that could not be *validated* is a different situation entirely: a schema
/// mismatch, a corrupted value, an enum this build does not recognize, or a failed invariant means
/// the stored policy is not something this build can vouch for, and answering from an older copy
/// would be asserting something about data we just failed to read correctly. Those fail closed.
#[derive(Default)]
pub(crate) struct LastKnownGoodPolicyCache {
    entries: Mutex<BTreeMap<PolicyCacheKey, PolicyResolutionBundle>>,
}

impl LastKnownGoodPolicyCache {
    /// Records a bundle that was read and validated.
    pub(crate) fn remember(&self, key: PolicyCacheKey, bundle: PolicyResolutionBundle) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(key, bundle);
        }
    }

    /// The bundle for this exact context, if one was ever validated for it.
    pub(crate) fn recall(&self, key: &PolicyCacheKey) -> Option<PolicyResolutionBundle> {
        self.entries.lock().ok()?.get(key).cloned()
    }

    /// Drops everything after a successful policy write.
    ///
    /// All of it rather than the written scope alone: a bundle for a workspace-Agent context
    /// contains the global row too, so a global edit invalidates entries whose keys never mention
    /// it. Scoped invalidation would be an optimization that has to know the containment rule, and
    /// getting it wrong means serving a policy the user has already changed.
    pub(crate) fn invalidate(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

/// Whether a failed read is the kind a previously validated bundle may stand in for.
///
/// Storage failures are transient by nature — a lock, a momentarily missing file, a busy database —
/// and are exactly what last-known-good exists for. Everything else means the stored data is not
/// something this build can vouch for, and using an older copy would assert a fact about data that
/// just failed to validate.
pub(crate) fn is_transient_read_failure(error: &PersonalizationApplicationError) -> bool {
    matches!(
        error,
        PersonalizationApplicationError::Storage(_)
            | PersonalizationApplicationError::MaintenanceBusy
    )
}
