use super::ApplicationError;
use crate::contexts::operations::domain::{
    OperationKind, OperationProgress, OperationRecoveryEvidence, OperationTask,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

pub(crate) trait OperationRepository: Send + Sync {
    fn insert(&self, operation: OperationTask) -> Result<(), ApplicationError>;

    fn update(
        &self,
        operation_id: &str,
        mutation: &mut dyn FnMut(&mut OperationTask),
    ) -> Result<OperationTask, ApplicationError>;

    fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError>;

    fn list(&self) -> Result<Vec<OperationTask>, ApplicationError>;

    fn list_recovery_evidence(
        &self,
        execution_run_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationRecoveryEvidence>, ApplicationError> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|operation| operation.execution_run_id.as_deref() == Some(execution_run_id))
            .take(limit)
            .map(|operation| OperationRecoveryEvidence {
                operation_id: operation.id,
                execution_run_id: execution_run_id.to_string(),
                status: operation.status,
            })
            .collect())
    }
}

pub(crate) trait OperationClock: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait OperationIdGenerator: Send + Sync {
    fn next_id(&self, timestamp: &str) -> String;
}

#[derive(Clone)]
pub(crate) struct OperationService {
    repository: Arc<dyn OperationRepository>,
    clock: Arc<dyn OperationClock>,
    ids: Arc<dyn OperationIdGenerator>,
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl OperationService {
    pub(crate) fn new(
        repository: Arc<dyn OperationRepository>,
        clock: Arc<dyn OperationClock>,
        ids: Arc<dyn OperationIdGenerator>,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn start(
        &self,
        kind: OperationKind,
        related_entity_id: Option<String>,
        message: Option<String>,
    ) -> Result<OperationTask, ApplicationError> {
        let now = self.clock.now();
        let operation = OperationTask::start(
            self.ids.next_id(&now),
            kind,
            related_entity_id,
            message,
            now,
        );
        self.repository.insert(operation.clone())?;
        self.cancellations
            .lock()
            .map_err(|_| cancellation_registry_error())?
            .insert(operation.id.clone(), Arc::new(AtomicBool::new(false)));
        Ok(operation)
    }

    pub(crate) fn append_log(
        &self,
        operation_id: &str,
        line: String,
    ) -> Result<OperationTask, ApplicationError> {
        let log_timestamp = self.clock.now();
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.append_log(line.clone(), log_timestamp.clone(), updated_at.clone());
        };
        self.repository.update(operation_id, &mut mutation)
    }

    pub(crate) fn report_progress(
        &self,
        operation_id: &str,
        progress: OperationProgress,
    ) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.report_progress(progress.clone(), updated_at.clone());
        };
        self.repository.update(operation_id, &mut mutation)
    }

    pub(crate) fn correlate_execution(
        &self,
        operation_id: &str,
        run_id: String,
        trace_id: String,
    ) -> Result<OperationTask, ApplicationError> {
        let mut mutation = |operation: &mut OperationTask| {
            operation.correlate_execution(run_id.clone(), trace_id.clone());
        };
        self.repository.update(operation_id, &mut mutation)
    }

    pub(crate) fn complete(
        &self,
        operation_id: &str,
        result: Option<Value>,
    ) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.succeed(result.clone(), updated_at.clone());
        };
        let operation = self.repository.update(operation_id, &mut mutation)?;
        self.remove_cancellation(operation_id)?;
        Ok(operation)
    }

    pub(crate) fn cancellation_flag(
        &self,
        operation_id: &str,
    ) -> Result<Arc<AtomicBool>, ApplicationError> {
        self.cancellations
            .lock()
            .map_err(|_| cancellation_registry_error())?
            .get(operation_id)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound(operation_id.to_string()))
    }

    pub(crate) fn fail(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.fail(error.clone(), updated_at.clone());
        };
        let operation = self.repository.update(operation_id, &mut mutation)?;
        self.remove_cancellation(operation_id)?;
        Ok(operation)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.cancel(updated_at.clone());
        };
        let operation = self.repository.update(operation_id, &mut mutation)?;
        if let Some(cancellation) = self
            .cancellations
            .lock()
            .map_err(|_| cancellation_registry_error())?
            .remove(operation_id)
        {
            cancellation.store(true, Ordering::SeqCst);
        }
        Ok(operation)
    }

    pub(crate) fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.repository.get(operation_id)
    }

    pub(crate) fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
        self.repository.list()
    }

    pub(crate) fn list_recovery_evidence(
        &self,
        execution_run_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationRecoveryEvidence>, ApplicationError> {
        self.repository
            .list_recovery_evidence(execution_run_id, limit)
    }

    fn remove_cancellation(&self, operation_id: &str) -> Result<(), ApplicationError> {
        self.cancellations
            .lock()
            .map_err(|_| cancellation_registry_error())?
            .remove(operation_id);
        Ok(())
    }
}

fn cancellation_registry_error() -> ApplicationError {
    ApplicationError::Internal("operation cancellation registry unavailable".to_string())
}

#[cfg(test)]
#[path = "operation_service_tests.rs"]
mod tests;
