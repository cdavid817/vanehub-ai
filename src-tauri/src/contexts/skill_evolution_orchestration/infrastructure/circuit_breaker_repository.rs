use crate::{
    contexts::skill_evolution_orchestration::domain::{
        acknowledge_breaker, canonical_hash, open_breaker, record_breaker_health,
        should_open_workspace_breaker, AutoApplyCircuitBreakerV1, AutomaticFailureSignalV1,
        BreakerHealthProbeV1, BreakerTransitionError, EvolutionActorProvenance,
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, TransactionBehavior};

use super::circuit_breaker_store_support::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CircuitBreakerRepositoryError {
    InvalidInput,
    Conflict,
    NotFound,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FailureRecordOutcome {
    pub(crate) duplicate: bool,
    pub(crate) breaker: Option<AutoApplyCircuitBreakerV1>,
}

#[derive(Clone)]
pub(crate) struct SqliteCircuitBreakerRepository {
    database: NativeDatabase,
}

impl SqliteCircuitBreakerRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn record_failure(
        &self,
        signal: &AutomaticFailureSignalV1,
    ) -> Result<FailureRecordOutcome, CircuitBreakerRepositoryError> {
        validate_signal(signal)?;
        let failure_id = canonical_hash(&(
            "automatic-failure-v1",
            &signal.workspace_id,
            &signal.source_run_id,
            &signal.source_application_id,
            signal.category.code(),
        ))
        .map_err(|_| CircuitBreakerRepositoryError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        let changed = transaction
            .execute(
                "INSERT OR IGNORE INTO evolution_auto_application_failures
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    failure_id,
                    signal.workspace_id,
                    signal.source_run_id,
                    signal.source_application_id,
                    signal.category.code(),
                    signal.occurred_at_ms,
                ],
            )
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        if changed == 0 {
            let breaker = load(&transaction, &signal.workspace_id, None)?;
            transaction
                .commit()
                .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
            return Ok(FailureRecordOutcome {
                duplicate: true,
                breaker,
            });
        }
        let failures = application_failure_times(&transaction, signal)?;
        let current = load(&transaction, &signal.workspace_id, None)?;
        let breaker =
            if should_open_workspace_breaker(signal, &failures_before_current(&failures, signal)) {
                let current = current.unwrap_or_else(|| closed_breaker(&signal.workspace_id));
                let opened = open_breaker(&current, signal).map_err(map_transition)?;
                persist(&transaction, &opened, current.revision)?;
                Some(opened)
            } else {
                current
            };
        transaction
            .commit()
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        Ok(FailureRecordOutcome {
            duplicate: false,
            breaker,
        })
    }

    pub(crate) fn record_health(
        &self,
        workspace_id: &str,
        skill_id: Option<&str>,
        expected_revision: u64,
        probe: &BreakerHealthProbeV1,
    ) -> Result<AutoApplyCircuitBreakerV1, CircuitBreakerRepositoryError> {
        if probe.workspace_id != workspace_id
            || probe.skill_id.as_deref() != skill_id
            || probe.proof_hash.is_empty()
        {
            return Err(CircuitBreakerRepositoryError::InvalidInput);
        }
        self.transition(workspace_id, skill_id, expected_revision, |current| {
            record_breaker_health(current, probe.passed, &probe.version, probe.checked_at_ms)
        })
    }

    pub(crate) fn acknowledge(
        &self,
        workspace_id: &str,
        skill_id: Option<&str>,
        expected_revision: u64,
        actor: EvolutionActorProvenance,
        now_ms: i64,
    ) -> Result<AutoApplyCircuitBreakerV1, CircuitBreakerRepositoryError> {
        self.transition(workspace_id, skill_id, expected_revision, |current| {
            acknowledge_breaker(current, actor, now_ms)
        })
    }

    fn transition(
        &self,
        workspace_id: &str,
        skill_id: Option<&str>,
        expected_revision: u64,
        transition: impl FnOnce(
            &AutoApplyCircuitBreakerV1,
        ) -> Result<AutoApplyCircuitBreakerV1, BreakerTransitionError>,
    ) -> Result<AutoApplyCircuitBreakerV1, CircuitBreakerRepositoryError> {
        validate_scope(workspace_id, skill_id)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        let current = load(&transaction, workspace_id, skill_id)?
            .ok_or(CircuitBreakerRepositoryError::NotFound)?;
        if current.revision != expected_revision {
            return Err(CircuitBreakerRepositoryError::Conflict);
        }
        let next = transition(&current).map_err(map_transition)?;
        persist(&transaction, &next, current.revision)?;
        transaction
            .commit()
            .map_err(|_| CircuitBreakerRepositoryError::Storage)?;
        Ok(next)
    }
}
