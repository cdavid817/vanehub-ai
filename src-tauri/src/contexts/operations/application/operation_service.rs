use super::ApplicationError;
use crate::contexts::operations::domain::{OperationKind, OperationTask};
use serde_json::Value;
use std::sync::Arc;

pub(crate) trait OperationRepository: Send + Sync {
    fn insert(&self, operation: OperationTask) -> Result<(), ApplicationError>;

    fn update(
        &self,
        operation_id: &str,
        mutation: &mut dyn FnMut(&mut OperationTask),
    ) -> Result<OperationTask, ApplicationError>;

    fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError>;

    fn list(&self) -> Result<Vec<OperationTask>, ApplicationError>;
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

    pub(crate) fn complete(
        &self,
        operation_id: &str,
        result: Option<Value>,
    ) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.succeed(result.clone(), updated_at.clone());
        };
        self.repository.update(operation_id, &mut mutation)
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
        self.repository.update(operation_id, &mut mutation)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        let updated_at = self.clock.now();
        let mut mutation = |operation: &mut OperationTask| {
            operation.cancel(updated_at.clone());
        };
        self.repository.update(operation_id, &mut mutation)
    }

    pub(crate) fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.repository.get(operation_id)
    }

    pub(crate) fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
        self.repository.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::OperationStatus;
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRepository {
        operations: Mutex<BTreeMap<String, OperationTask>>,
    }

    impl OperationRepository for FakeRepository {
        fn insert(&self, operation: OperationTask) -> Result<(), ApplicationError> {
            self.operations
                .lock()
                .expect("operations")
                .insert(operation.id.clone(), operation);
            Ok(())
        }

        fn update(
            &self,
            operation_id: &str,
            mutation: &mut dyn FnMut(&mut OperationTask),
        ) -> Result<OperationTask, ApplicationError> {
            let mut operations = self.operations.lock().expect("operations");
            let operation = operations
                .get_mut(operation_id)
                .ok_or_else(|| ApplicationError::NotFound("operation not found".to_string()))?;
            mutation(operation);
            Ok(operation.clone())
        }

        fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
            self.operations
                .lock()
                .expect("operations")
                .get(operation_id)
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound("operation not found".to_string()))
        }

        fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
            Ok(self
                .operations
                .lock()
                .expect("operations")
                .values()
                .cloned()
                .collect())
        }
    }

    struct FakeClock {
        values: Mutex<VecDeque<String>>,
    }

    impl OperationClock for FakeClock {
        fn now(&self) -> String {
            self.values
                .lock()
                .expect("clock")
                .pop_front()
                .expect("clock value")
        }
    }

    struct FakeIds;

    impl OperationIdGenerator for FakeIds {
        fn next_id(&self, timestamp: &str) -> String {
            format!("op-{timestamp}-fixed")
        }
    }

    #[test]
    fn use_case_coordinates_deterministic_ids_timestamps_logs_and_results() {
        let repository = Arc::new(FakeRepository::default());
        let service = OperationService::new(
            repository,
            Arc::new(FakeClock {
                values: Mutex::new(
                    ["100", "101", "102", "103"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                ),
            }),
            Arc::new(FakeIds),
        );

        let started = service
            .start(
                OperationKind::Extension,
                None,
                Some("Installing".to_string()),
            )
            .expect("start");
        service
            .append_log(&started.id, "downloaded".to_string())
            .expect("append log");
        let completed = service
            .complete(&started.id, Some(serde_json::json!({ "ok": true })))
            .expect("complete");

        assert_eq!(completed.id, "op-100-fixed");
        assert_eq!(completed.created_at, "100");
        assert_eq!(completed.updated_at, "103");
        assert_eq!(completed.logs[0].timestamp, "101");
        assert_eq!(completed.logs[0].operation_id, completed.id);
        assert_eq!(completed.status, OperationStatus::Succeeded);
        assert_eq!(completed.result, Some(serde_json::json!({ "ok": true })));
    }
}
