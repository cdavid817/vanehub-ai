use crate::contexts::operations::application::{
    ApplicationError, OperationClock, OperationIdGenerator, OperationRepository, OperationService,
};
use crate::contexts::operations::domain::OperationTask;
use crate::platform::clock::SystemClock;
use crate::platform::ids::MonotonicIdGenerator;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct InMemoryOperationRepository {
    operations: Mutex<BTreeMap<String, OperationTask>>,
}

impl OperationRepository for InMemoryOperationRepository {
    fn insert(&self, operation: OperationTask) -> Result<(), ApplicationError> {
        self.operations
            .lock()
            .map_err(lock_error)?
            .insert(operation.id.clone(), operation);
        Ok(())
    }

    fn update(
        &self,
        operation_id: &str,
        mutation: &mut dyn FnMut(&mut OperationTask),
    ) -> Result<OperationTask, ApplicationError> {
        let mut operations = self.operations.lock().map_err(lock_error)?;
        let operation = operations.get_mut(operation_id).ok_or_else(|| {
            ApplicationError::NotFound(format!("operation not found: {operation_id}"))
        })?;
        mutation(operation);
        Ok(operation.clone())
    }

    fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.operations
            .lock()
            .map_err(lock_error)?
            .get(operation_id)
            .cloned()
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("operation not found: {operation_id}"))
            })
    }

    fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
        Ok(self
            .operations
            .lock()
            .map_err(lock_error)?
            .values()
            .cloned()
            .collect())
    }
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ApplicationError {
    ApplicationError::Internal("operation registry lock is unavailable".to_string())
}

impl OperationClock for SystemClock {
    fn now(&self) -> String {
        self.unix_seconds()
    }
}

impl OperationIdGenerator for MonotonicIdGenerator {
    fn next_id(&self, timestamp: &str) -> String {
        self.next(timestamp)
    }
}

pub(crate) fn operation_service() -> OperationService {
    OperationService::new(
        Arc::new(InMemoryOperationRepository::default()),
        Arc::new(SystemClock),
        Arc::new(MonotonicIdGenerator::new("op")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::{OperationKind, OperationStatus};

    #[test]
    fn in_memory_adapter_preserves_legacy_id_shape_and_lifecycle() {
        let service = operation_service();
        let started = service
            .start(OperationKind::Sdk, Some("codex-sdk".to_string()), None)
            .expect("start");
        let failed = service
            .fail(&started.id, "install failed".to_string())
            .expect("fail");

        assert!(started.id.starts_with("op-"));
        assert_eq!(started.id.split('-').count(), 3);
        assert_eq!(failed.id, started.id);
        assert_eq!(failed.status, OperationStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some("install failed"));
        assert_eq!(service.list().expect("list"), vec![failed]);
    }
}
