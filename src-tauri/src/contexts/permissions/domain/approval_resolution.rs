//! The immutable record of one approval decision, and the mutable state of delivering it.
//!
//! A resolution exists because the decision and its effect happen in two different places. The
//! decision is a row in SQLite; the effect is a native Agent or an HTTP waiter being released.
//! Those cannot be made atomic with each other, so the ordering has to be stated instead: commit
//! first, deliver second, and record what delivery did.
//!
//! The split between immutable and mutable is the whole design. `decision_effect`, `decider`,
//! `channel` and the identity fields are what somebody decided; they never change. `state`,
//! `delivery_attempts` and `last_error_code` are what happened when the system tried to act on
//! that decision, and only they move. A type that let the second kind of update rewrite the first
//! would make "the user approved this" a value the delivery layer could edit.

use super::effect::Effect;
use super::error::PermissionsDomainError;
use super::scope::Scope;

/// The identity a delivery carries.
///
/// Immutable and unique per approval request. A retry re-sends the same id, which is what lets the
/// receiving waiter apply a resolution at most once: without it, a retried delivery and a second
/// decision are indistinguishable to the side that has to act on them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ApprovalResolutionId(String);

impl ApprovalResolutionId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PermissionsDomainError> {
        let value = value.into();
        if value.is_empty() {
            return Err(PermissionsDomainError::RequiredValue("resolution_id"));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// A last-resort id for the emergency denial path, whose own id is derived from a request id
    /// that cannot be empty. Exists so that path has no `expect()` on an unreachable branch.
    pub(crate) fn emergency_fallback() -> Self {
        Self("emergency:unattributed".to_string())
    }
}

/// Who decided.
///
/// Kept apart from the effect because a `Deny` a human chose and a `Deny` a timeout produced are
/// the same effect and different facts — the first is an answer, the second is the absence of one,
/// and only the second may be produced without durable storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolutionDecider {
    Human,
    Timeout,
    /// The originating generation or waiter had already ended when delivery was reserved. The
    /// decision is recorded as evidence and never delivered.
    StaleGeneration,
    /// Storage was unavailable and the bounded approval timeout would otherwise have been
    /// violated. Can only ever carry `Deny`.
    EmergencyFailClosed,
}

impl ResolutionDecider {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Timeout => "timeout",
            Self::StaleGeneration => "stale_generation",
            Self::EmergencyFailClosed => "emergency_fail_closed",
        }
    }

    pub(crate) fn from_token(token: &str) -> Result<Self, PermissionsDomainError> {
        match token {
            "human" => Ok(Self::Human),
            "timeout" => Ok(Self::Timeout),
            "stale_generation" => Ok(Self::StaleGeneration),
            "emergency_fail_closed" => Ok(Self::EmergencyFailClosed),
            _ => Err(PermissionsDomainError::UnknownResolutionField("decider")),
        }
    }
}

/// Which waiter is blocked on this decision.
///
/// Recorded on the request rather than inferred at delivery time: a routed adapter that guessed
/// would have to try both channels and could release the wrong one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolutionChannel {
    NativeAgent,
    ClaudeHook,
}

impl ResolutionChannel {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::NativeAgent => "native_agent",
            Self::ClaudeHook => "claude_hook",
        }
    }

    pub(crate) fn from_token(token: &str) -> Result<Self, PermissionsDomainError> {
        match token {
            "native_agent" => Ok(Self::NativeAgent),
            "claude_hook" => Ok(Self::ClaudeHook),
            _ => Err(PermissionsDomainError::UnknownResolutionField("channel")),
        }
    }
}

/// How far delivery got.
///
/// Five states rather than a boolean, because "the decision did not reach the agent" has four
/// materially different causes and only one of them may ever be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalResolutionState {
    /// Durable, not yet delivered. The only state a delivery attempt may start from.
    Committed,
    /// The waiter acknowledged it. Terminal, and the only state that activates a grant.
    Delivered,
    /// Committed, then delivery failed. Retryable while the same reservation and generation
    /// remain valid; the grant stays inactive throughout.
    DeliveryFailed,
    /// The waiter or generation was already gone when delivery was reserved. Terminal, no effect
    /// was delivered, no grant exists.
    Stale,
    /// Found at startup in a state that cannot have a live waiter any more. Terminal by
    /// definition: the process that was waiting is gone.
    AbortedByRestart,
}

/// Every state, so storage can derive its predicates from the domain instead of hand-listing
/// tokens in SQL. A `state IN (...)` written by hand is a copy of a rule that will not be updated
/// when a state is added.
pub(crate) const ALL_RESOLUTION_STATES: [ApprovalResolutionState; 5] = [
    ApprovalResolutionState::Committed,
    ApprovalResolutionState::Delivered,
    ApprovalResolutionState::DeliveryFailed,
    ApprovalResolutionState::Stale,
    ApprovalResolutionState::AbortedByRestart,
];

impl ApprovalResolutionState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Delivered => "delivered",
            Self::DeliveryFailed => "delivery_failed",
            Self::Stale => "stale",
            Self::AbortedByRestart => "aborted_by_restart",
        }
    }

    pub(crate) fn from_token(token: &str) -> Result<Self, PermissionsDomainError> {
        match token {
            "committed" => Ok(Self::Committed),
            "delivered" => Ok(Self::Delivered),
            "delivery_failed" => Ok(Self::DeliveryFailed),
            "stale" => Ok(Self::Stale),
            "aborted_by_restart" => Ok(Self::AbortedByRestart),
            _ => Err(PermissionsDomainError::UnknownResolutionField("state")),
        }
    }

    /// Whether delivery can still be attempted from here.
    ///
    /// `Delivered`, `Stale` and `AbortedByRestart` are terminal for different reasons — it already
    /// worked, there was never anyone to tell, and whoever was waiting no longer exists — but a
    /// caller only ever needs to know whether to stop.
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Delivered | Self::Stale | Self::AbortedByRestart)
    }

    /// Whether a restart should reconcile this row.
    ///
    /// Exactly the two non-terminal states. A `committed` row means the decision never reached
    /// anyone before the process ended; a `delivery_failed` row means an attempt was in flight.
    /// Both had a waiter that cannot exist any more.
    pub(crate) fn needs_restart_reconciliation(self) -> bool {
        matches!(self, Self::Committed | Self::DeliveryFailed)
    }
}

/// The decision, as committed. Every field here is immutable for the life of the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalDecisionRecord {
    pub(crate) effect: Effect,
    pub(crate) scope: Scope,
    pub(crate) decider: ResolutionDecider,
    pub(crate) channel: ResolutionChannel,
}

impl ApprovalDecisionRecord {
    /// Rejects the two decisions that must never become durable.
    ///
    /// `Ask` is not a decision — committing one would record the absence of an answer as an answer.
    /// And an emergency fail-closed path exists precisely because storage is unavailable; letting
    /// it carry `Allow` would turn a storage outage into an authorization.
    pub(crate) fn new(
        effect: Effect,
        scope: Scope,
        decider: ResolutionDecider,
        channel: ResolutionChannel,
    ) -> Result<Self, PermissionsDomainError> {
        if matches!(effect, Effect::Ask) {
            return Err(PermissionsDomainError::UndecidedResolution);
        }
        if decider == ResolutionDecider::EmergencyFailClosed && effect != Effect::Deny {
            return Err(PermissionsDomainError::EmergencyResolutionMustDeny);
        }
        Ok(Self {
            effect,
            scope,
            decider,
            channel,
        })
    }
}

/// A committed resolution as storage holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApprovalResolution {
    pub(crate) id: ApprovalResolutionId,
    pub(crate) request_id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) decision: ApprovalDecisionRecord,
    pub(crate) state: ApprovalResolutionState,
    pub(crate) delivery_attempts: i64,
    /// A stable code, never a message. An error string from a transport would carry hostnames and
    /// paths into a table the redaction rules keep them out of.
    pub(crate) last_error_code: Option<String>,
}

impl ApprovalResolution {
    /// Whether a delivery carrying `resolution_id` is the one this row is waiting for.
    ///
    /// Both halves matter. A mismatched id is a delivery for some other decision; a terminal state
    /// means this decision already has an outcome, and applying it again would release a second
    /// execution for one approval.
    pub(crate) fn accepts_delivery_of(&self, resolution_id: &ApprovalResolutionId) -> bool {
        &self.id == resolution_id && !self.state.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(
        effect: Effect,
        decider: ResolutionDecider,
    ) -> Result<ApprovalDecisionRecord, PermissionsDomainError> {
        ApprovalDecisionRecord::new(
            effect,
            Scope::Session,
            decider,
            ResolutionChannel::NativeAgent,
        )
    }

    fn resolution(state: ApprovalResolutionState) -> ApprovalResolution {
        ApprovalResolution {
            id: ApprovalResolutionId::parse("res-1").expect("id"),
            request_id: "req-1".to_string(),
            principal_id: "principal-1".to_string(),
            session_id: "session-1".to_string(),
            generation_id: "generation-1".to_string(),
            decision: decision(Effect::Allow, ResolutionDecider::Human).expect("decision"),
            state,
            delivery_attempts: 0,
            last_error_code: None,
        }
    }

    #[test]
    fn an_undecided_effect_cannot_be_committed() {
        assert_eq!(
            decision(Effect::Ask, ResolutionDecider::Human),
            Err(PermissionsDomainError::UndecidedResolution)
        );
    }

    #[test]
    fn the_emergency_path_can_only_ever_carry_a_denial() {
        // The path exists because storage is unavailable. Letting it commit an Allow would turn a
        // storage outage into an authorization nobody granted.
        assert_eq!(
            decision(Effect::Allow, ResolutionDecider::EmergencyFailClosed),
            Err(PermissionsDomainError::EmergencyResolutionMustDeny)
        );
        assert!(decision(Effect::Deny, ResolutionDecider::EmergencyFailClosed).is_ok());
    }

    #[test]
    fn every_decider_and_channel_can_carry_an_ordinary_decision() {
        for decider in [
            ResolutionDecider::Human,
            ResolutionDecider::Timeout,
            ResolutionDecider::StaleGeneration,
        ] {
            assert!(decision(Effect::Allow, decider).is_ok());
            assert!(decision(Effect::Deny, decider).is_ok());
        }
    }

    #[test]
    fn a_resolution_id_is_required() {
        assert_eq!(
            ApprovalResolutionId::parse(""),
            Err(PermissionsDomainError::RequiredValue("resolution_id"))
        );
    }

    #[test]
    fn only_a_matching_id_in_a_non_terminal_state_accepts_delivery() {
        let other = ApprovalResolutionId::parse("res-2").expect("id");
        let mine = ApprovalResolutionId::parse("res-1").expect("id");

        assert!(resolution(ApprovalResolutionState::Committed).accepts_delivery_of(&mine));
        assert!(resolution(ApprovalResolutionState::DeliveryFailed).accepts_delivery_of(&mine));
        assert!(!resolution(ApprovalResolutionState::Committed).accepts_delivery_of(&other));

        for terminal in [
            ApprovalResolutionState::Delivered,
            ApprovalResolutionState::Stale,
            ApprovalResolutionState::AbortedByRestart,
        ] {
            assert!(
                !resolution(terminal).accepts_delivery_of(&mine),
                "{terminal:?} accepted a second delivery for one approval"
            );
        }
    }

    #[test]
    fn restart_reconciles_exactly_the_states_that_could_have_had_a_live_waiter() {
        assert!(ApprovalResolutionState::Committed.needs_restart_reconciliation());
        assert!(ApprovalResolutionState::DeliveryFailed.needs_restart_reconciliation());
        for terminal in [
            ApprovalResolutionState::Delivered,
            ApprovalResolutionState::Stale,
            ApprovalResolutionState::AbortedByRestart,
        ] {
            assert!(!terminal.needs_restart_reconciliation());
            assert!(terminal.is_terminal());
        }
    }

    #[test]
    fn stored_tokens_round_trip_and_unknown_text_is_refused() {
        for state in [
            ApprovalResolutionState::Committed,
            ApprovalResolutionState::Delivered,
            ApprovalResolutionState::DeliveryFailed,
            ApprovalResolutionState::Stale,
            ApprovalResolutionState::AbortedByRestart,
        ] {
            assert_eq!(
                ApprovalResolutionState::from_token(state.token()),
                Ok(state)
            );
        }
        for decider in [
            ResolutionDecider::Human,
            ResolutionDecider::Timeout,
            ResolutionDecider::StaleGeneration,
            ResolutionDecider::EmergencyFailClosed,
        ] {
            assert_eq!(ResolutionDecider::from_token(decider.token()), Ok(decider));
        }
        for channel in [
            ResolutionChannel::NativeAgent,
            ResolutionChannel::ClaudeHook,
        ] {
            assert_eq!(ResolutionChannel::from_token(channel.token()), Ok(channel));
        }

        // Refused rather than defaulted: a row storage cannot classify has to be visible, not
        // quietly read as the safest-looking neighbour.
        assert!(ApprovalResolutionState::from_token("in_flight").is_err());
        assert!(ResolutionDecider::from_token("policy").is_err());
        assert!(ResolutionChannel::from_token("mcp_callback").is_err());
    }
}
