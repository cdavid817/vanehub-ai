//! Wakes an in-flight loopback HTTP request once a human resolves the `Ask` decision it's
//! blocked on. Structurally analogous to `agent_runtime`'s own `pending_approvals`/
//! `await_approval`, but not the same registry: that one is private to `RuntimeAgentApiAdapter`
//! and keyed by native-agent generation. This one is `permissions`-owned infrastructure, keyed by
//! `ApprovalRequest::id`, and uses a `tokio::sync::oneshot` since the caller is an async axum
//! handler rather than a blocking native-agent thread.

use crate::contexts::permissions::domain::Effect;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Default)]
pub(crate) struct HookWaitRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<Effect>>>,
}

impl HookWaitRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Registers a wait for `request_id`, returning the receiver half to `.await`. Overwrites
    /// (and silently drops) any prior, unresolved registration under the same id — `request_id`
    /// is broker-generated fresh per call, so a collision would only happen if a caller reused
    /// one, which nothing in this codebase does.
    pub(crate) fn register(&self, request_id: &str) -> oneshot::Receiver<Effect> {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("hook wait registry mutex poisoned")
            .insert(request_id.to_string(), tx);
        rx
    }

    /// Delivers `effect` to whatever's waiting on `request_id`. Returns `false` (mirroring
    /// `AgentRuntimeApi::resolve_tool_approval`'s own shape) if nothing is registered — the
    /// request already resolved some other way, or never went through this registry at all (for
    /// example, it was a native-agent request, not a hook-bridge one).
    pub(crate) fn resolve(&self, request_id: &str, effect: Effect) -> bool {
        let sender = self
            .pending
            .lock()
            .expect("hook wait registry mutex poisoned")
            .remove(request_id);
        match sender {
            Some(sender) => sender.send(effect).is_ok(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_delivers_the_effect_to_the_registered_receiver() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");

        assert!(registry.resolve("req-1", Effect::Allow));

        assert_eq!(rx.await.unwrap(), Effect::Allow);
    }

    #[test]
    fn resolve_on_an_unregistered_id_returns_false() {
        let registry = HookWaitRegistry::new();
        assert!(!registry.resolve("never-registered", Effect::Deny));
    }

    #[tokio::test]
    async fn resolve_is_idempotent_the_second_call_returns_false() {
        let registry = HookWaitRegistry::new();
        let rx = registry.register("req-1");

        assert!(registry.resolve("req-1", Effect::Allow));
        assert!(!registry.resolve("req-1", Effect::Deny));

        assert_eq!(rx.await.unwrap(), Effect::Allow);
    }
}
