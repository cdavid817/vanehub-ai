use super::project_root::ProcessKey;
use crate::contexts::code_intelligence::domain::models::ProcessState;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationReason {
    ToolRequest,
    Prewarm { inventory: bool, manifest: bool },
}

impl ActivationReason {
    const fn should_activate(self) -> bool {
        match self {
            Self::ToolRequest => true,
            Self::Prewarm {
                inventory,
                manifest,
            } => inventory || manifest,
        }
    }

    const fn opens_request(self) -> bool {
        matches!(self, Self::ToolRequest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RejectionReason {
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LifecycleAction {
    Start(ProcessKey),
    Stop(ProcessKey),
    FailPending(ProcessKey),
    Reject(RejectionReason),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LifecyclePolicy {
    restart_budget: u32,
    initial_backoff: Duration,
    maximum_backoff: Duration,
    cooldown: Duration,
    idle_timeout: Duration,
}

impl Default for LifecyclePolicy {
    fn default() -> Self {
        Self::new(
            3,
            Duration::from_secs(1),
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(600),
        )
    }
}

impl LifecyclePolicy {
    pub(crate) const fn new(
        restart_budget: u32,
        initial_backoff: Duration,
        maximum_backoff: Duration,
        cooldown: Duration,
        idle_timeout: Duration,
    ) -> Self {
        Self {
            restart_budget,
            initial_backoff,
            maximum_backoff,
            cooldown,
            idle_timeout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessStatusSnapshot {
    pub(crate) state: ProcessState,
    pub(crate) active_requests: usize,
    pub(crate) document_leases: usize,
    pub(crate) restart_count: u32,
    pub(crate) restart_at: Option<Duration>,
    pub(crate) cooldown_until: Option<Duration>,
}

struct RegistryEntry {
    state: ProcessState,
    active_requests: usize,
    document_leases: usize,
    restart_count: u32,
    last_activity: Duration,
    restart_at: Option<Duration>,
    cooldown_until: Option<Duration>,
}

impl RegistryEntry {
    fn starting(now: Duration, active_requests: usize) -> Self {
        Self {
            state: ProcessState::Starting,
            active_requests,
            document_leases: 0,
            restart_count: 0,
            last_activity: now,
            restart_at: None,
            cooldown_until: None,
        }
    }

    fn snapshot(&self) -> ProcessStatusSnapshot {
        ProcessStatusSnapshot {
            state: self.state,
            active_requests: self.active_requests,
            document_leases: self.document_leases,
            restart_count: self.restart_count,
            restart_at: self.restart_at,
            cooldown_until: self.cooldown_until,
        }
    }
}

pub(crate) struct ProcessRegistry {
    policy: LifecyclePolicy,
    entries: HashMap<ProcessKey, RegistryEntry>,
}

impl ProcessRegistry {
    pub(crate) fn new(policy: LifecyclePolicy) -> Self {
        Self {
            policy,
            entries: HashMap::new(),
        }
    }

    pub(crate) fn acquire(
        &mut self,
        key: ProcessKey,
        reason: ActivationReason,
        now: Duration,
        authorized: bool,
    ) -> Vec<LifecycleAction> {
        if !authorized {
            return vec![LifecycleAction::Reject(RejectionReason::Untrusted)];
        }
        if !reason.should_activate() {
            return Vec::new();
        }
        if let Some(entry) = self.entries.get_mut(&key) {
            if reason.opens_request() {
                entry.active_requests = entry.active_requests.saturating_add(1);
                entry.last_activity = now;
            }
            return Vec::new();
        }
        let active_requests = usize::from(reason.opens_request());
        self.entries
            .insert(key.clone(), RegistryEntry::starting(now, active_requests));
        vec![LifecycleAction::Start(key)]
    }

    pub(crate) fn replace_configuration(
        &mut self,
        replacement: ProcessKey,
        now: Duration,
    ) -> Vec<LifecycleAction> {
        let stale = self
            .entries
            .keys()
            .filter(|key| key.same_instance_scope(&replacement) && *key != &replacement)
            .cloned()
            .collect::<Vec<_>>();
        let mut actions = Vec::with_capacity(stale.len() + 1);
        for key in stale {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.state = ProcessState::Stopping;
            }
            actions.push(LifecycleAction::Stop(key));
        }
        if !self.entries.contains_key(&replacement) {
            self.entries
                .insert(replacement.clone(), RegistryEntry::starting(now, 0));
            actions.push(LifecycleAction::Start(replacement));
        }
        actions
    }

    pub(crate) fn revoke_session(&mut self, session_root: &Path) -> Vec<LifecycleAction> {
        let keys = self
            .entries
            .keys()
            .filter(|key| key.session_root_ref() == session_root)
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| {
                let entry = self.entries.get_mut(&key)?;
                if entry.state == ProcessState::Stopping {
                    return None;
                }
                entry.state = ProcessState::Stopping;
                Some(LifecycleAction::Stop(key))
            })
            .collect()
    }

    pub(crate) fn mark_ready(&mut self, key: &ProcessKey, now: Duration) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.state = ProcessState::Ready;
            entry.last_activity = now;
            entry.restart_at = None;
        }
    }

    pub(crate) fn mark_initializing(&mut self, key: &ProcessKey, now: Duration) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.state = ProcessState::Initializing;
            entry.last_activity = now;
        }
    }

    pub(crate) fn release_request(&mut self, key: &ProcessKey, now: Duration) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.active_requests = entry.active_requests.saturating_sub(1);
            entry.last_activity = now;
        }
    }

    pub(crate) fn set_document_leases(
        &mut self,
        key: &ProcessKey,
        document_leases: usize,
        now: Duration,
    ) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.document_leases = document_leases;
            entry.last_activity = now;
        }
    }

    pub(crate) fn unexpected_exit(
        &mut self,
        key: &ProcessKey,
        now: Duration,
    ) -> Vec<LifecycleAction> {
        let Some(entry) = self.entries.get_mut(key) else {
            return Vec::new();
        };
        entry.active_requests = 0;
        entry.document_leases = 0;
        let actions = vec![LifecycleAction::FailPending(key.clone())];
        if entry.restart_count >= self.policy.restart_budget {
            entry.state = ProcessState::Failed;
            entry.restart_at = None;
            entry.cooldown_until = Some(now + self.policy.cooldown);
            return actions;
        }
        let backoff = exponential_backoff(
            self.policy.initial_backoff,
            self.policy.maximum_backoff,
            entry.restart_count,
        );
        entry.restart_count += 1;
        entry.state = ProcessState::Backoff;
        entry.restart_at = Some(now + backoff);
        entry.cooldown_until = None;
        actions
    }

    pub(crate) fn tick(&mut self, now: Duration) -> Vec<LifecycleAction> {
        let mut actions = Vec::new();
        for (key, entry) in &mut self.entries {
            let restart_due = entry.state == ProcessState::Backoff
                && entry.restart_at.is_some_and(|deadline| deadline <= now);
            let cooldown_due = entry.state == ProcessState::Failed
                && entry.cooldown_until.is_some_and(|deadline| deadline <= now);
            if restart_due || cooldown_due {
                if cooldown_due {
                    entry.restart_count = 0;
                }
                entry.state = ProcessState::Starting;
                entry.restart_at = None;
                entry.cooldown_until = None;
                entry.last_activity = now;
                actions.push(LifecycleAction::Start(key.clone()));
                continue;
            }
            let idle = entry.state == ProcessState::Ready
                && entry.active_requests == 0
                && entry.document_leases == 0
                && now.saturating_sub(entry.last_activity) >= self.policy.idle_timeout;
            if idle {
                entry.state = ProcessState::Stopping;
                actions.push(LifecycleAction::Stop(key.clone()));
            }
        }
        actions
    }

    pub(crate) fn status(&self, key: &ProcessKey) -> Option<ProcessStatusSnapshot> {
        self.entries.get(key).map(RegistryEntry::snapshot)
    }

    pub(crate) fn keys(&self) -> Vec<ProcessKey> {
        self.entries.keys().cloned().collect()
    }

    pub(crate) fn remove(&mut self, key: &ProcessKey) {
        self.entries.remove(key);
    }
}

fn exponential_backoff(initial: Duration, maximum: Duration, exponent: u32) -> Duration {
    let multiplier = 2_u32.checked_pow(exponent).unwrap_or(u32::MAX);
    initial.saturating_mul(multiplier).min(maximum)
}
