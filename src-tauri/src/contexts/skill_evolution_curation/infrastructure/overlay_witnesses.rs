use crate::contexts::tooling::skills::api::{
    OverlayScope, OverlayScopeStatus, OverlaySummary, OverlayTrustState,
};

pub(super) struct OverlayStateWitnesses {
    pub(super) pin: String,
    pub(super) trust: String,
    pub(super) conflict: String,
}

pub(super) fn overlay_state_witnesses(
    summary: &OverlaySummary,
    scope: OverlayScope,
) -> OverlayStateWitnesses {
    let current = summary.scopes.iter().find(|entry| entry.scope == scope);
    OverlayStateWitnesses {
        pin: format!("pin-v1:{}", summary.pinned),
        trust: current.map_or_else(
            || "trust-v1:none".to_string(),
            |entry| {
                format!(
                    "trust-v1:{}:{}:{}",
                    entry.revision,
                    trust_name(entry.trust),
                    status_name(entry.status)
                )
            },
        ),
        conflict: current.map_or_else(
            || format!("conflict-v1:0:{}", summary.needs_reconcile),
            |entry| {
                format!(
                    "conflict-v1:{}:{}:{}",
                    entry.conflict_count, entry.needs_reconcile, summary.needs_reconcile
                )
            },
        ),
    }
}

fn trust_name(value: OverlayTrustState) -> &'static str {
    match value {
        OverlayTrustState::Trusted => "trusted",
        OverlayTrustState::Untrusted => "untrusted",
    }
}

fn status_name(value: OverlayScopeStatus) -> &'static str {
    match value {
        OverlayScopeStatus::Applied => "applied",
        OverlayScopeStatus::Untrusted => "untrusted",
        OverlayScopeStatus::NeedsReconciliation => "needs_reconciliation",
        OverlayScopeStatus::BlockedByEarlierScope => "blocked_by_earlier_scope",
        OverlayScopeStatus::IntegrityFailure => "integrity_failure",
    }
}
