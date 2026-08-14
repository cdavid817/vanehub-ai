use super::{DelegationMode, DelegationTarget};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DelegationCircuitKey {
    pub(crate) target: DelegationTarget,
    pub(crate) mode: DelegationMode,
    pub(crate) executable_sha256: String,
    pub(crate) adapter_version: String,
    pub(crate) policy_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationCircuitFailure {
    ProtocolIntegrity,
    SandboxIntegrity,
    ProcessTreeIntegrity,
    CleanupIntegrity,
    Authentication,
    ProviderRefusal,
    TaskFailure,
    ModelQuality,
    ProjectTestFailure,
}

impl DelegationCircuitFailure {
    const fn trips_circuit(self) -> bool {
        matches!(
            self,
            Self::ProtocolIntegrity
                | Self::SandboxIntegrity
                | Self::ProcessTreeIntegrity
                | Self::CleanupIntegrity
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationCircuitState {
    Closed,
    Open {
        failure_count: u8,
        retry_after_millis: u64,
    },
}

#[derive(Debug, Clone, Copy)]
struct CircuitObservation {
    failure_count: u8,
    last_failure_millis: u64,
    retry_after_millis: u64,
}

pub(crate) struct DelegationCircuitBreaker {
    threshold: u8,
    observation_window_millis: u64,
    cooldown_millis: u64,
    observations: BTreeMap<DelegationCircuitKey, CircuitObservation>,
}

impl DelegationCircuitBreaker {
    pub(crate) fn new(threshold: u8, observation_window_millis: u64, cooldown_millis: u64) -> Self {
        Self {
            threshold: threshold.max(1),
            observation_window_millis,
            cooldown_millis,
            observations: BTreeMap::new(),
        }
    }

    pub(crate) fn record(
        &mut self,
        key: DelegationCircuitKey,
        failure: DelegationCircuitFailure,
        now_millis: u64,
    ) -> DelegationCircuitState {
        if !failure.trips_circuit() {
            return self.state(&key, now_millis);
        }
        let observation = self.observations.entry(key).or_insert(CircuitObservation {
            failure_count: 0,
            last_failure_millis: now_millis,
            retry_after_millis: 0,
        });
        if now_millis.saturating_sub(observation.last_failure_millis)
            > self.observation_window_millis
        {
            observation.failure_count = 0;
        }
        observation.failure_count = observation.failure_count.saturating_add(1);
        observation.last_failure_millis = now_millis;
        if observation.failure_count >= self.threshold {
            observation.retry_after_millis = now_millis.saturating_add(self.cooldown_millis);
        }
        circuit_state(observation, self.threshold, now_millis)
    }

    pub(crate) fn state(
        &self,
        key: &DelegationCircuitKey,
        now_millis: u64,
    ) -> DelegationCircuitState {
        self.observations
            .get(key)
            .map_or(DelegationCircuitState::Closed, |observation| {
                circuit_state(observation, self.threshold, now_millis)
            })
    }

    pub(crate) fn record_compatible_success(&mut self, key: &DelegationCircuitKey) {
        self.observations.remove(key);
    }
}

fn circuit_state(
    observation: &CircuitObservation,
    threshold: u8,
    now_millis: u64,
) -> DelegationCircuitState {
    if observation.failure_count >= threshold && now_millis < observation.retry_after_millis {
        DelegationCircuitState::Open {
            failure_count: observation.failure_count,
            retry_after_millis: observation.retry_after_millis,
        }
    } else {
        DelegationCircuitState::Closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(
        target: DelegationTarget,
        mode: DelegationMode,
        fingerprint: &str,
    ) -> DelegationCircuitKey {
        DelegationCircuitKey {
            target,
            mode,
            executable_sha256: fingerprint.to_owned(),
            adapter_version: "1".to_owned(),
            policy_revision: "policy-1".to_owned(),
        }
    }

    #[test]
    fn integrity_failures_open_only_the_exact_target_mode_and_fingerprint() {
        let affected = key(DelegationTarget::CodexCli, DelegationMode::Edit, "hash-a");
        let analyze = key(
            DelegationTarget::CodexCli,
            DelegationMode::Analyze,
            "hash-a",
        );
        let changed_binary = key(DelegationTarget::CodexCli, DelegationMode::Edit, "hash-b");
        let mut breaker = DelegationCircuitBreaker::new(3, 60_000, 30_000);
        for now in [1, 2, 3] {
            breaker.record(
                affected.clone(),
                DelegationCircuitFailure::ProtocolIntegrity,
                now,
            );
        }
        assert!(matches!(
            breaker.state(&affected, 4),
            DelegationCircuitState::Open {
                failure_count: 3,
                retry_after_millis: 30_003
            }
        ));
        assert_eq!(breaker.state(&analyze, 4), DelegationCircuitState::Closed);
        assert_eq!(
            breaker.state(&changed_binary, 4),
            DelegationCircuitState::Closed
        );
        assert_eq!(
            breaker.state(&affected, 30_003),
            DelegationCircuitState::Closed
        );
    }

    #[test]
    fn ordinary_outcomes_do_not_trip_and_compatible_success_recovers() {
        let affected = key(
            DelegationTarget::ClaudeCode,
            DelegationMode::Analyze,
            "hash-a",
        );
        let mut breaker = DelegationCircuitBreaker::new(2, 60_000, 30_000);
        for failure in [
            DelegationCircuitFailure::Authentication,
            DelegationCircuitFailure::ProviderRefusal,
            DelegationCircuitFailure::TaskFailure,
            DelegationCircuitFailure::ModelQuality,
            DelegationCircuitFailure::ProjectTestFailure,
        ] {
            assert_eq!(
                breaker.record(affected.clone(), failure, 1),
                DelegationCircuitState::Closed
            );
        }
        breaker.record(
            affected.clone(),
            DelegationCircuitFailure::SandboxIntegrity,
            2,
        );
        breaker.record(
            affected.clone(),
            DelegationCircuitFailure::CleanupIntegrity,
            3,
        );
        assert!(matches!(
            breaker.state(&affected, 4),
            DelegationCircuitState::Open { .. }
        ));
        breaker.record_compatible_success(&affected);
        assert_eq!(breaker.state(&affected, 4), DelegationCircuitState::Closed);
    }
}
