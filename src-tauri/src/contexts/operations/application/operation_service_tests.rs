use super::*;
use crate::contexts::operations::domain::OperationStatus;
use std::collections::{BTreeMap, VecDeque};

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
    let service = OperationService::new(
        Arc::new(FakeRepository::default()),
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

#[test]
fn cancellation_signals_the_shared_flag_without_repository_polling() {
    let service = OperationService::new(
        Arc::new(FakeRepository::default()),
        Arc::new(FakeClock {
            values: Mutex::new(["100", "101"].into_iter().map(str::to_string).collect()),
        }),
        Arc::new(FakeIds),
    );
    let started = service
        .start(OperationKind::Mcp, None, Some("Testing".to_string()))
        .expect("start");
    let cancellation = service
        .cancellation_flag(&started.id)
        .expect("cancellation flag");
    assert!(!cancellation.load(Ordering::SeqCst));

    service.cancel(&started.id).expect("cancel");

    assert!(cancellation.load(Ordering::SeqCst));
    assert!(matches!(
        service.cancellation_flag(&started.id),
        Err(ApplicationError::NotFound(_))
    ));
}
