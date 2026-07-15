use crate::tasks::models::{OperationKind, OperationLogEntry, OperationStatus, OperationTask};
use crate::AppError;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct TaskRegistry {
    counter: AtomicU64,
    tasks: Mutex<BTreeMap<String, OperationTask>>,
}

impl TaskRegistry {
    pub fn start(
        &self,
        kind: OperationKind,
        related_entity_id: Option<String>,
        message: Option<String>,
    ) -> Result<OperationTask, AppError> {
        let now = now_string();
        let id = format!("op-{}-{}", now, self.counter.fetch_add(1, Ordering::Relaxed) + 1);
        let task = OperationTask {
            id: id.clone(),
            kind,
            status: OperationStatus::Running,
            related_entity_id,
            message,
            logs: Vec::new(),
            result: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.tasks
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?
            .insert(id, task.clone());
        Ok(task)
    }

    pub fn append_log(&self, operation_id: &str, line: String) -> Result<OperationTask, AppError> {
        self.update(operation_id, |task| {
            task.logs.push(OperationLogEntry {
                operation_id: operation_id.to_string(),
                line,
                timestamp: now_string(),
            });
        })
    }

    pub fn complete(&self, operation_id: &str, result: Option<Value>) -> Result<OperationTask, AppError> {
        self.update(operation_id, |task| {
            task.status = OperationStatus::Succeeded;
            task.result = result;
            task.error = None;
        })
    }

    pub fn fail(&self, operation_id: &str, error: String) -> Result<OperationTask, AppError> {
        self.update(operation_id, |task| {
            task.status = OperationStatus::Failed;
            task.error = Some(error);
        })
    }

    pub fn get(&self, operation_id: &str) -> Result<OperationTask, AppError> {
        self.tasks
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?
            .get(operation_id)
            .cloned()
            .ok_or_else(|| AppError::Validation(format!("operation not found: {operation_id}")))
    }

    pub fn list(&self) -> Result<Vec<OperationTask>, AppError> {
        Ok(self
            .tasks
            .lock()
            .map_err(|err| AppError::Storage(err.to_string()))?
            .values()
            .cloned()
            .collect())
    }

    fn update(&self, operation_id: &str, update: impl FnOnce(&mut OperationTask)) -> Result<OperationTask, AppError> {
        let mut tasks = self.tasks.lock().map_err(|err| AppError::Storage(err.to_string()))?;
        let task = tasks
            .get_mut(operation_id)
            .ok_or_else(|| AppError::Validation(format!("operation not found: {operation_id}")))?;
        update(task);
        task.updated_at = now_string();
        Ok(task.clone())
    }
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_registry_records_lifecycle() {
        let registry = TaskRegistry::default();
        let task = registry
            .start(OperationKind::Sdk, Some("claude-sdk".to_string()), Some("Installing".to_string()))
            .expect("start");

        assert!(matches!(task.status, OperationStatus::Running));

        let task = registry.append_log(&task.id, "npm install".to_string()).expect("log");
        assert_eq!(task.logs.len(), 1);

        let task = registry.complete(&task.id, None).expect("complete");
        assert!(matches!(task.status, OperationStatus::Succeeded));
    }
}
