//! Wakes an in-flight loopback HTTP request once a human resolves the `Ask` decision it's
//! blocked on. Structurally analogous to `agent_runtime`'s own `pending_approvals`/
//! `await_approval`, but not the same registry: that one is private to `RuntimeAgentApiAdapter`
//! and keyed by native-agent generation. This one is `permissions`-owned infrastructure, keyed by
//! `ApprovalRequest::id`, and uses a `tokio::sync::oneshot` since the caller is an async axum
//! handler rather than a blocking native-agent thread.
//!
//! It is deliberately not a second decision engine. It holds no policy, decides nothing, and its
//! whole vocabulary is: is somebody still waiting, apply this exact resolution once, and stop
//! waiting. Everything about *which* decision to apply belongs to `ResolveApprovalUseCase`.

use crate::contexts::permissions::domain::{ApprovalResolutionId, Effect};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use tokio::sync::oneshot;

/// What a delivery attempt found.
///
/// Three outcomes rather than a boolean, because a retry that finds the resolution already applied
/// and a delivery that finds nobody waiting are different facts: the first means the effect
/// happened, the second means it did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookDelivery {
    Applied,
    /// This exact resolution id was already delivered. Idempotent by design: a retry must not
    /// release a second tool execution.
    AlreadyApplied,
    /// No waiter — it timed out, disconnected, or was cancelled.
    WaiterGone,
}

/// One blocked loopback request, and the resolution it has already been given, if any.
struct HookWaiter {
    sender: Option<oneshot::Sender<Effect>>,
    /// Retained after the sender is consumed so a retried delivery of the same id is recognised.
    /// Without it, the second attempt would look like a vanished waiter and the caller would
    /// record a delivery failure for something that was in fact delivered.
    applied: Option<ApprovalResolutionId>,
}

#[derive(Default)]
pub(crate) struct HookWaitRegistry {
    pending: Mutex<HashMap<String, HookWaiter>>,
}

impl HookWaitRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Recovers the guard rather than aborting on a poisoned lock. A poisoned lock here means
    /// another thread panicked while holding it; aborting this one as well would strand every
    /// hook-bridge request currently blocked on an `Ask` decision, which is the opposite of
    /// what this registry exists to guarantee. The guarded map only ever sees whole-entry
    /// insert/remove/replace, so the recovered value is structurally sound.
    fn lock_pending(&self) -> MutexGuard<'_, HashMap<String, HookWaiter>> {
        self.pending.lock().unwrap_or_else(|poisoned| {
            debug_assert!(false, "hook wait registry mutex poisoned");
            poisoned.into_inner()
        })
    }

    /// Registers a wait for `request_id`, returning the receiver half to `.await`. Overwrites
    /// (and silently drops) any prior, unresolved registration under the same id — `request_id`
    /// is broker-generated fresh per call, so a collision would only happen if a caller reused
    /// one, which nothing in this codebase does.
    pub(crate) fn register(&self, request_id: &str) -> oneshot::Receiver<Effect> {
        let (tx, rx) = oneshot::channel();
        self.lock_pending().insert(
            request_id.to_string(),
            HookWaiter {
                sender: Some(tx),
                applied: None,
            },
        );
        rx
    }

    /// Whether a loopback request is still blocked on `request_id` and could receive a decision.
    ///
    /// This is the reservation: asked before a resolution is committed, so a hook waiter that timed
    /// out is discovered while the decision can still be recorded as stale rather than delivered to
    /// nobody. It deliberately does not consume the registration — proving somebody is there is not
    /// the same as releasing them, and releasing before the commit is the ordering this whole
    /// change exists to reverse.
    pub(crate) fn has_waiter(&self, request_id: &str) -> bool {
        self.lock_pending()
            .get(request_id)
            .is_some_and(|waiter| waiter.sender.is_some())
    }

    /// Delivers one immutable resolution to whatever is waiting on `request_id`.
    ///
    /// At most once per resolution id. The check and the send happen under one lock, so two retries
    /// arriving together cannot both decide they are the first.
    pub(crate) fn deliver(
        &self,
        request_id: &str,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
    ) -> HookDelivery {
        let mut pending = self.lock_pending();
        let Some(waiter) = pending.get_mut(request_id) else {
            return HookDelivery::WaiterGone;
        };
        if waiter.applied.as_ref() == Some(resolution_id) {
            return HookDelivery::AlreadyApplied;
        }
        // A different resolution id for a request that already has one is not a retry — it is a
        // second decision, and the waiter has already acted on the first.
        if waiter.applied.is_some() {
            return HookDelivery::WaiterGone;
        }
        let Some(sender) = waiter.sender.take() else {
            return HookDelivery::WaiterGone;
        };
        if sender.send(effect).is_err() {
            // The HTTP request went away between the reservation and now. Nothing was released.
            pending.remove(request_id);
            return HookDelivery::WaiterGone;
        }
        waiter.applied = Some(resolution_id.clone());
        HookDelivery::Applied
    }

    /// Stops a wait without delivering anything, releasing the blocked request to its own
    /// fail-closed default.
    ///
    /// Its own operation rather than a `deliver(Deny)`, because those are different facts: a
    /// cancelled request was never decided, and recording a denial for it would put a decision
    /// nobody made into the audit trail.
    ///
    /// No production caller yet — nothing in the resolution flow cancels a hook wait, and the
    /// facade deliberately does not surface it. It is here because `deliver` needs a counterpart
    /// that releases the waiter *without* a decision, and a registry that could only ever answer
    /// with one would push callers toward `deliver(Deny)` when they mean "never mind".
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn cancel(&self, request_id: &str) -> bool {
        self.lock_pending()
            .remove(request_id)
            .is_some_and(|waiter| waiter.sender.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolution(id: &str) -> ApprovalResolutionId {
        ApprovalResolutionId::parse(id).expect("id")
    }

    #[tokio::test]
    async fn delivery_reaches_the_registered_receiver() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");

        assert_eq!(
            registry.deliver("req-1", &resolution("res-1"), Effect::Allow),
            HookDelivery::Applied
        );

        assert_eq!(rx.await.unwrap(), Effect::Allow);
    }

    #[test]
    fn delivering_to_an_unregistered_id_reports_a_missing_waiter() {
        let registry = HookWaitRegistry::new();
        assert_eq!(
            registry.deliver("never-registered", &resolution("res-1"), Effect::Deny),
            HookDelivery::WaiterGone
        );
    }

    /// `claude-code-permission-hook`'s "The same resolution is delivered twice".
    #[tokio::test]
    async fn the_same_resolution_delivered_twice_releases_one_execution() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");
        let id = resolution("res-1");

        assert_eq!(
            registry.deliver("req-1", &id, Effect::Allow),
            HookDelivery::Applied
        );
        // Idempotent, and distinguishable from a vanished waiter: the caller must not record a
        // delivery failure for something that was in fact delivered.
        assert_eq!(
            registry.deliver("req-1", &id, Effect::Allow),
            HookDelivery::AlreadyApplied
        );

        assert_eq!(rx.await.unwrap(), Effect::Allow);
    }

    #[tokio::test]
    async fn a_different_resolution_cannot_overwrite_one_the_waiter_already_applied() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");
        registry.deliver("req-1", &resolution("res-1"), Effect::Allow);

        // Not a retry: a second decision for a request that already acted on the first.
        assert_eq!(
            registry.deliver("req-1", &resolution("res-2"), Effect::Deny),
            HookDelivery::WaiterGone
        );

        assert_eq!(rx.await.unwrap(), Effect::Allow);
    }

    /// `claude-code-permission-hook`'s "Hook waiter ended before reservation".
    #[test]
    fn a_reservation_reports_a_waiter_without_consuming_it() {
        let registry = HookWaitRegistry::new();
        assert!(!registry.has_waiter("req-1"));

        let _rx = registry.register("req-1");
        assert!(registry.has_waiter("req-1"));
        // Asked twice, because a reservation that consumed the waiter would release the hook
        // before its decision was committed — the ordering this change exists to reverse.
        assert!(registry.has_waiter("req-1"));

        registry.deliver("req-1", &resolution("res-1"), Effect::Allow);
        assert!(!registry.has_waiter("req-1"));
    }

    #[test]
    fn a_waiter_whose_request_went_away_is_reported_as_gone() {
        let registry = HookWaitRegistry::new();
        // The HTTP request disconnected: its receiver is dropped.
        drop(registry.register("req-1"));

        assert_eq!(
            registry.deliver("req-1", &resolution("res-1"), Effect::Allow),
            HookDelivery::WaiterGone
        );
        assert!(!registry.has_waiter("req-1"));
    }

    #[tokio::test]
    async fn cancelling_releases_the_request_without_recording_a_decision() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");

        assert!(registry.cancel("req-1"));

        // The blocked handler falls through to its own fail-closed default rather than being told
        // a decision that nobody made.
        assert!(rx.await.is_err());
        assert!(!registry.has_waiter("req-1"));
        assert!(!registry.cancel("req-1"));
    }
}
