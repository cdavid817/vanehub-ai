//! Turning a human decision into executable authority, in an order that cannot be undone wrongly.
//!
//! The previous flow released the waiting Agent first and wrote the grant and audit afterwards,
//! through two repositories with nothing joining them. That ordering has no safe failure: once a
//! tool has run, no rollback reaches it, so any storage failure after delivery left an action
//! performed with no record of why. And because finalization began by removing the pending request,
//! the failure was not even retryable.
//!
//! This use case states the order instead:
//!
//! ```text
//! claim  →  reserve  →  commit  →  deliver  →  acknowledge  →  activate
//! ```
//!
//! Each arrow is chosen for what it makes impossible. The claim makes two callers unable to write
//! competing decisions. The reservation proves the waiter is still there *without* resuming it, so
//! a stale generation is discovered before anything durable is written. The commit is one
//! transaction, so `Allow` cannot reach anyone before its evidence exists. Delivery is outside that
//! transaction because it has to be — and the acknowledgement is what finally activates the
//! remembered grant, so an approval that never actually arrived cannot authorize the next attempt.

use super::approval_broker::{ApprovalBroker, ApprovalClaim};
use super::error::PermissionsApplicationError;
use super::ports::{
    ApprovalResolutionRepository, AuditDecider, AuditRecord, NewApprovalResolution,
    PendingGrantIntent, PermissionsClockPort, PermissionsDiagnosticsPort, PermissionsIdPort,
    ResolutionCommit,
};
use crate::contexts::permissions::domain::{
    ApprovalDecision, ApprovalDecisionRecord, ApprovalRequest, ApprovalResolution,
    ApprovalResolutionId, ApprovalResolutionState, CanonicalGrantKey, Effect, PersistedEffect,
    RememberedScope, ResolutionChannel, ResolutionDecider, Scope,
};
use std::sync::Arc;

/// A live waiter, proven current and held.
///
/// Opaque on purpose: the only thing a caller may do with it is hand it back to `deliver`. A
/// reservation that exposed the generation or the channel would invite a caller to make its own
/// decision about whether the waiter is still valid, which is the check this type exists to own.
pub(crate) struct DeliveryReservation {
    pub(crate) token: String,
}

/// What the waiter did with a delivered resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryAcknowledgement {
    /// The waiter applied this resolution and resumed.
    Applied,
    /// The waiter had already applied this same resolution id. A retry, not a second execution —
    /// which is exactly what the immutable id is for.
    AlreadyApplied,
    /// The waiter disappeared between reservation and delivery. The decision stays durable; no
    /// effect was delivered.
    WaiterGone,
}

/// How a decision reaches whoever is blocked on it.
///
/// A consuming-side port: `permissions` declares what it needs, and the adapters that satisfy it
/// live at the boundary with `agent_runtime` and with the hook bridge. The context never imports
/// another context's repositories or generation internals to do this.
pub(crate) trait ApprovalDeliveryPort: Send + Sync {
    /// Proves the originating waiter and generation are still current and holds them, **without
    /// resuming execution**. Returning `None` means the waiter is gone — a stale generation, and
    /// the whole reason this step comes before the commit.
    fn reserve(
        &self,
        request: &ApprovalRequest,
    ) -> Result<Option<DeliveryReservation>, PermissionsApplicationError>;

    /// Resumes the reserved waiter with the immutable resolution id. Must be idempotent per id:
    /// the same resolution delivered twice releases one execution.
    fn deliver(
        &self,
        reservation: &DeliveryReservation,
        request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
    ) -> Result<DeliveryAcknowledgement, PermissionsApplicationError>;
}

/// What resolving an approval did.
///
/// Every variant is something the frontend renders differently, and none of them is an error
/// string. `DeliveryFailed` in particular is not a failure of the decision — the decision is
/// durable — it is a failure to tell anyone about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolveOutcome {
    /// The waiter applied the decision and the grant, if any, is now active.
    Delivered {
        resolution_id: String,
        effect: Effect,
    },
    /// The waiter or generation had already ended. Committed as evidence, never delivered, no
    /// grant.
    StaleGeneration { resolution_id: String },
    /// Durable but undelivered. Retryable; the grant stays inactive.
    DeliveryFailed {
        resolution_id: String,
        error_code: &'static str,
    },
    /// Another caller owns this request right now.
    Resolving { resolution_id: String },
    /// This request already has an immutable answer. A retry after an ambiguous response lands
    /// here rather than producing a second decision.
    AlreadyResolved {
        resolution_id: String,
        state: ApprovalResolutionState,
    },
    /// No pending request and no durable resolution.
    NotFound,
    /// Storage was unavailable and the bounded approval timeout would otherwise have been
    /// violated, so a denial was released with no row behind it.
    ///
    /// Carries no resolution id, because there is no resolution — that is the whole point. It can
    /// never be reinterpreted as an approval: the next attempt goes through a fresh evaluation.
    DeniedFailClosed { reason: &'static str },
}

impl ResolveOutcome {
    /// The stable token the frontend switches on.
    pub(crate) fn token(&self) -> &'static str {
        match self {
            Self::Delivered { .. } => "delivered",
            Self::StaleGeneration { .. } => "stale",
            Self::DeliveryFailed { .. } => "delivery_failed",
            Self::Resolving { .. } => "resolving",
            Self::AlreadyResolved { .. } => "already_resolved",
            Self::NotFound => "not_found",
            Self::DeniedFailClosed { .. } => "denied_fail_closed",
        }
    }

    /// Whether the waiting Agent or hook actually received this decision.
    ///
    /// The one question every caller of the old `finalize(delivered: bool)` was really asking,
    /// answered from the outcome instead of being passed in by a command that had already guessed.
    ///
    /// The command returns [`token`] rather than this, because the frontend has to tell
    /// `delivery_failed` from `stale` from `already_resolved`; this is the coarse reading the tests
    /// assert with.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn reached_the_waiter(&self) -> bool {
        matches!(self, Self::Delivered { .. })
    }
}

pub(crate) struct ResolveApprovalUseCase {
    broker: ApprovalBroker,
    resolutions: Arc<dyn ApprovalResolutionRepository>,
    delivery: Arc<dyn ApprovalDeliveryPort>,
    diagnostics: Arc<dyn PermissionsDiagnosticsPort>,
    clock: Arc<dyn PermissionsClockPort>,
    ids: Arc<dyn PermissionsIdPort>,
}

impl ResolveApprovalUseCase {
    pub(crate) fn new(
        broker: ApprovalBroker,
        resolutions: Arc<dyn ApprovalResolutionRepository>,
        delivery: Arc<dyn ApprovalDeliveryPort>,
        diagnostics: Arc<dyn PermissionsDiagnosticsPort>,
        clock: Arc<dyn PermissionsClockPort>,
        ids: Arc<dyn PermissionsIdPort>,
    ) -> Self {
        Self {
            broker,
            resolutions,
            delivery,
            diagnostics,
            clock,
            ids,
        }
    }

    /// A human's Approve or Deny.
    pub(crate) fn resolve(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
        scope: Scope,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        self.run(
            request_id,
            decision.as_effect(),
            scope,
            ResolutionDecider::Human,
        )
    }

    /// A timeout's fail-closed denial, through the same single-winner path.
    ///
    /// Deliberately not a separate flow. The sweep and a human clicking Deny are two callers of one
    /// decision, and giving the sweep its own path is how they would come to disagree about which
    /// of them won.
    pub(crate) fn resolve_timed_out(
        &self,
        request_id: &str,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        self.run(
            request_id,
            Effect::Deny,
            Scope::Once,
            ResolutionDecider::Timeout,
        )
    }

    fn run(
        &self,
        request_id: &str,
        effect: Effect,
        scope: Scope,
        decider: ResolutionDecider,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        let resolution_id = ApprovalResolutionId::parse(self.ids.next_id("resolution"))?;
        let request = match self.broker.claim(request_id, resolution_id.as_str()) {
            ApprovalClaim::Claimed(request) => *request,
            ApprovalClaim::AlreadyClaimed { resolution_id } => {
                return Ok(ResolveOutcome::Resolving { resolution_id })
            }
            // Not pending is not the same as not known. A retry whose first attempt already
            // committed lands here, and answering "not found" would invite the caller to decide
            // again.
            ApprovalClaim::NotPending => return self.existing_outcome(request_id),
        };

        let reservation = match self.delivery.reserve(&request) {
            Ok(Some(reservation)) => reservation,
            // Discovered before anything durable exists, which is the point of reserving first.
            Ok(None) => return self.commit_stale(&request, &resolution_id, effect, scope),
            Err(error) => {
                self.broker.revert_claim(request_id, resolution_id.as_str());
                return Err(error);
            }
        };

        let commit = self.build_commit(&request, &resolution_id, effect, scope, decider)?;
        let committed = match self.resolutions.commit_resolution(&commit) {
            Ok(committed) => committed,
            Err(error) => {
                // Nothing became durable, so the decision is still the user's to make.
                self.broker.revert_claim(request_id, resolution_id.as_str());
                // Except when leaving it unmade would break the bounded approval timeout. Then a
                // denial goes out with no row behind it, rather than a provider waiting forever.
                if decider == ResolutionDecider::Timeout {
                    return Ok(self.deny_fail_closed(&request, &reservation));
                }
                return Err(error);
            }
        };
        self.broker
            .mark_committed(request_id, resolution_id.as_str());

        // Checked against what storage actually holds, not against what was sent to it. A row that
        // is not this resolution, or is already terminal, must not be delivered: doing so would
        // release an execution for a decision this call did not make.
        if !committed.accepts_delivery_of(&resolution_id) {
            self.broker
                .release_committed(request_id, resolution_id.as_str());
            return Ok(existing_outcome_of(&committed));
        }

        // Everything from here on is after the point of no return for the *record*, so no failure
        // may be reported as "the decision did not happen".
        let acknowledgement = self
            .delivery
            .deliver(&reservation, &request, &resolution_id, effect);
        let outcome = match acknowledgement {
            Ok(DeliveryAcknowledgement::Applied) | Ok(DeliveryAcknowledgement::AlreadyApplied) => {
                self.resolutions
                    .acknowledge_delivery_and_activate(&resolution_id, &self.clock.now())?;
                ResolveOutcome::Delivered {
                    resolution_id: resolution_id.as_str().to_string(),
                    effect,
                }
            }
            Ok(DeliveryAcknowledgement::WaiterGone) => {
                self.record_failure(&resolution_id, "delivery_waiter_gone")?
            }
            Err(_) => self.record_failure(&resolution_id, "delivery_failed")?,
        };
        self.broker
            .release_committed(request_id, resolution_id.as_str());
        Ok(outcome)
    }

    /// Commits the decision as stale: evidence that a decision was made, and nothing else.
    ///
    /// No grant intent and no delivery. The user did decide, and that is worth recording; what they
    /// decided about is gone.
    fn commit_stale(
        &self,
        request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
        scope: Scope,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        let mut commit = self.build_commit(
            request,
            resolution_id,
            effect,
            scope,
            ResolutionDecider::StaleGeneration,
        )?;
        commit.grant_intent = None;
        commit.resolution.state = ApprovalResolutionState::Stale;
        commit.audit.outcome_reason = Some("approval_generation_ended");
        if let Err(error) = self.resolutions.commit_resolution(&commit) {
            self.broker
                .revert_claim(&request.id, resolution_id.as_str());
            return Err(error);
        }
        self.broker
            .mark_committed(&request.id, resolution_id.as_str());
        self.broker
            .release_committed(&request.id, resolution_id.as_str());
        Ok(ResolveOutcome::StaleGeneration {
            resolution_id: resolution_id.as_str().to_string(),
        })
    }

    /// Releases the waiter with a denial that has no durable record behind it.
    ///
    /// Reachable only from the timeout path, and only after the resolution transaction failed. The
    /// alternative is a provider blocked forever on a decision the database cannot accept, and
    /// between "denied, unrecorded" and "waiting, unbounded" the first is the one that fails safe.
    ///
    /// Three things make it safe rather than merely convenient. It can only ever carry `Deny` —
    /// `ApprovalDecisionRecord` refuses to build an emergency `Allow`, and nothing here constructs
    /// one anyway. It writes no grant, so nothing it does can authorize a later attempt. And it
    /// removes the pending entry rather than reverting it, because the waiter has been released:
    /// leaving the request on offer would let a human "approve" something that was already denied.
    fn deny_fail_closed(
        &self,
        request: &ApprovalRequest,
        reservation: &DeliveryReservation,
    ) -> ResolveOutcome {
        const REASON: &str = "resolution_storage_unavailable";
        // A synthetic id so the waiter's own at-most-once guard still applies. It is never stored,
        // which is exactly why this outcome carries no resolution id back to the caller.
        let emergency_id = ApprovalResolutionId::parse(format!("emergency:{}", request.id))
            .unwrap_or_else(|_| {
                debug_assert!(false, "an approval request id is never empty");
                ApprovalResolutionId::emergency_fallback()
            });
        let _ = self
            .delivery
            .deliver(reservation, request, &emergency_id, Effect::Deny);
        self.broker.discard_pending(&request.id);
        self.diagnostics.approval_denied_fail_closed(
            &request.id,
            &request.session_id,
            &request.generation_id,
            REASON,
        );
        ResolveOutcome::DeniedFailClosed { reason: REASON }
    }

    fn record_failure(
        &self,
        resolution_id: &ApprovalResolutionId,
        error_code: &'static str,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        self.resolutions
            .record_delivery_failure(resolution_id, error_code, &self.clock.now())?;
        Ok(ResolveOutcome::DeliveryFailed {
            resolution_id: resolution_id.as_str().to_string(),
            error_code,
        })
    }

    /// What the durable ledger says about a request that is no longer pending.
    fn existing_outcome(
        &self,
        request_id: &str,
    ) -> Result<ResolveOutcome, PermissionsApplicationError> {
        Ok(match self.resolutions.find_by_request_id(request_id)? {
            Some(resolution) => existing_outcome_of(&resolution),
            None => ResolveOutcome::NotFound,
        })
    }

    fn build_commit(
        &self,
        request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
        scope: Scope,
        decider: ResolutionDecider,
    ) -> Result<ResolutionCommit, PermissionsApplicationError> {
        // Skills and delegation are single-use whatever the caller asked for. Enforced here rather
        // than trusted from the request, so a new caller cannot widen it by passing a broader
        // scope.
        let scope = if request.action.as_str() == "delegation.apply" || request.skill.is_some() {
            Scope::Once
        } else {
            scope
        };
        let channel = ResolutionChannel::NativeAgent;
        let decision = ApprovalDecisionRecord::new(effect, scope, decider, channel)?;
        let now = self.clock.now();

        Ok(ResolutionCommit {
            resolution: NewApprovalResolution {
                id: resolution_id.clone(),
                request_id: request.id.clone(),
                principal_id: request.principal_id.clone(),
                session_id: request.session_id.clone(),
                generation_id: request.generation_id.clone(),
                call_id_hash: correlation_hash(&request.call_id),
                action: request.action.clone(),
                resource: request.resource.clone(),
                risk_level: request.risk_level,
                decision,
                state: ApprovalResolutionState::Committed,
                now: now.clone(),
            },
            audit: AuditRecord {
                id: self.ids.next_id("audit"),
                principal_id: request.principal_id.clone(),
                session_id: request.session_id.clone(),
                generation_id: request.generation_id.clone(),
                action: request.action.clone(),
                resource: request.resource.clone(),
                effect,
                risk_level: request.risk_level,
                decider: audit_decider_for(decider),
                channel: "native_agent",
                resolution_id: Some(resolution_id.as_str().to_string()),
                outcome_reason: None,
                created_at: now.clone(),
            },
            grant_intent: self.grant_intent(request, resolution_id, effect, scope, &now),
        })
    }

    /// What this decision should remember, if anything.
    ///
    /// `None` for every unrememberable combination, and the domain is what says which those are:
    /// `RememberedScope::parse` refuses `Once` and `PersistedEffect::parse` refuses `Ask`. Neither
    /// is re-checked here, so a new scope or effect cannot become persistable by being forgotten at
    /// this call site.
    fn grant_intent(
        &self,
        request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
        scope: Scope,
        now: &str,
    ) -> Option<PendingGrantIntent> {
        // A request carries both a session and a project; the scope alone decides which owns the
        // grant, so the other is cleared rather than passed through.
        let binding = match scope {
            Scope::Once => return None,
            Scope::Session => {
                RememberedScope::parse(scope, Some(request.session_id.as_str()), None)
            }
            Scope::Project => {
                RememberedScope::parse(scope, None, Some(request.project_key.as_str()))
            }
            Scope::Global => RememberedScope::parse(scope, None, None),
        }
        .ok()?;
        let effect = PersistedEffect::parse(effect).ok()?;
        let key = CanonicalGrantKey::new(
            request.principal_id.clone(),
            request.action.clone(),
            request.resource.clone(),
            binding,
        )
        .ok()?;
        Some(PendingGrantIntent {
            id: self.ids.next_id("grant"),
            key,
            effect,
            resolution_id: resolution_id.as_str().to_string(),
            now: now.to_string(),
        })
    }
}

fn existing_outcome_of(resolution: &ApprovalResolution) -> ResolveOutcome {
    let resolution_id = resolution.id.as_str().to_string();
    match resolution.state {
        ApprovalResolutionState::Stale => ResolveOutcome::StaleGeneration { resolution_id },
        state => ResolveOutcome::AlreadyResolved {
            resolution_id,
            state,
        },
    }
}

fn audit_decider_for(decider: ResolutionDecider) -> AuditDecider {
    match decider {
        ResolutionDecider::Human => AuditDecider::Human,
        ResolutionDecider::Timeout => AuditDecider::Timeout,
        ResolutionDecider::StaleGeneration => AuditDecider::StaleGeneration,
        ResolutionDecider::EmergencyFailClosed => AuditDecider::EmergencyFailClosed,
    }
}

/// A bounded correlation value for a provider-chosen call id.
///
/// FNV-1a rather than `DefaultHasher`, because this value is written to a durable table and read
/// back across builds: `DefaultHasher`'s output is explicitly not guaranteed stable between Rust
/// releases, which would silently break correlation for every row written before an upgrade.
///
/// Not a security boundary and not reversible-by-design — the point is that the raw call id, which
/// is provider-chosen and can carry request content, never lands in the ledger.
fn correlation_hash(call_id: &str) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let digest = call_id.as_bytes().iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    });
    format!("fnv1a:{digest:016x}")
}

#[cfg(test)]
#[path = "resolve_approval_tests.rs"]
mod tests;
