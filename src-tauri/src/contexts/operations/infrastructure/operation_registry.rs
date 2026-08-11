use crate::contexts::operations::application::{
    ApplicationError, OperationClock, OperationIdGenerator, OperationRepository, OperationService,
};
use crate::contexts::operations::domain::{
    OperationRecoveryEvidence, OperationStatus, OperationTask,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use crate::platform::ids::MonotonicIdGenerator;
use rusqlite::params;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

struct InMemoryOperationRepository {
    operations: Mutex<BTreeMap<String, OperationTask>>,
    database: Option<NativeDatabase>,
}

impl InMemoryOperationRepository {
    #[cfg(test)]
    fn in_memory() -> Self {
        Self {
            operations: Mutex::new(BTreeMap::new()),
            database: None,
        }
    }

    fn persistent(database: NativeDatabase) -> Self {
        Self {
            operations: Mutex::new(BTreeMap::new()),
            database: Some(database),
        }
    }

    fn persist_recovery_evidence(&self, operation: &OperationTask) -> Result<(), ApplicationError> {
        let (Some(database), Some(execution_run_id)) =
            (&self.database, operation.execution_run_id.as_deref())
        else {
            return Ok(());
        };
        database
            .connection()
            .map_err(operation_storage_error)?
            .execute(
                r#"
                INSERT INTO operation_recovery_evidence (
                    operation_id, execution_run_id, status, updated_at
                ) VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(operation_id) DO UPDATE SET
                    execution_run_id = excluded.execution_run_id,
                    status = excluded.status,
                    updated_at = excluded.updated_at
                "#,
                params![
                    operation.id,
                    execution_run_id,
                    operation.status.as_str(),
                    operation.updated_at
                ],
            )
            .map_err(operation_storage_error)?;
        Ok(())
    }
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
        let operation = operations.get(operation_id).ok_or_else(|| {
            ApplicationError::NotFound(format!("operation not found: {operation_id}"))
        })?;
        let mut updated = operation.clone();
        mutation(&mut updated);
        self.persist_recovery_evidence(&updated)?;
        operations.insert(operation_id.to_string(), updated.clone());
        Ok(updated)
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

    fn list_recovery_evidence(
        &self,
        execution_run_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationRecoveryEvidence>, ApplicationError> {
        let Some(database) = &self.database else {
            return Ok(self
                .operations
                .lock()
                .map_err(lock_error)?
                .values()
                .filter(|operation| operation.execution_run_id.as_deref() == Some(execution_run_id))
                .take(limit)
                .map(|operation| OperationRecoveryEvidence {
                    operation_id: operation.id.clone(),
                    execution_run_id: execution_run_id.to_string(),
                    status: operation.status,
                })
                .collect());
        };
        let connection = database.connection().map_err(operation_storage_error)?;
        let mut statement = connection
            .prepare(
                "SELECT operation_id, execution_run_id, status \
                 FROM operation_recovery_evidence \
                 WHERE execution_run_id = ?1 ORDER BY updated_at, operation_id LIMIT ?2",
            )
            .map_err(operation_storage_error)?;
        let rows = statement
            .query_map(params![execution_run_id, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(operation_storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(operation_storage_error)?;
        rows.into_iter()
            .map(|(operation_id, execution_run_id, status)| {
                let status = OperationStatus::parse(&status).ok_or_else(|| {
                    ApplicationError::Internal(
                        "operation recovery evidence contains an invalid status".to_string(),
                    )
                })?;
                Ok(OperationRecoveryEvidence {
                    operation_id,
                    execution_run_id,
                    status,
                })
            })
            .collect()
    }
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ApplicationError {
    ApplicationError::Internal("operation registry lock is unavailable".to_string())
}

fn operation_storage_error(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::infrastructure("operation_storage", error.to_string())
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

#[cfg(test)]
pub(crate) fn operation_service() -> OperationService {
    OperationService::new(
        Arc::new(InMemoryOperationRepository::in_memory()),
        Arc::new(SystemClock),
        Arc::new(MonotonicIdGenerator::new("op")),
    )
}

pub(crate) fn persistent_operation_service(database: NativeDatabase) -> OperationService {
    OperationService::new(
        Arc::new(InMemoryOperationRepository::persistent(database)),
        Arc::new(SystemClock),
        Arc::new(MonotonicIdGenerator::new("op")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::{OperationKind, OperationStatus};
    use crate::test_support::TempDirectory;

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

    #[test]
    fn recovery_evidence_survives_database_reopen() {
        let directory = TempDirectory::new("operation-recovery-evidence-reopen");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let service = persistent_operation_service(database);
        let operation = service
            .start(OperationKind::Agent, Some("agent-1".to_string()), None)
            .expect("start operation");
        service
            .correlate_execution(
                &operation.id,
                "run-reopen".to_string(),
                "trace-reopen".to_string(),
            )
            .expect("correlate operation");
        service
            .complete(&operation.id, None)
            .expect("complete operation");
        drop(service);

        let reopened =
            NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
        let recovered = persistent_operation_service(reopened)
            .list_recovery_evidence("run-reopen", 2)
            .expect("read recovery evidence");

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].operation_id, operation.id);
        assert_eq!(recovered[0].execution_run_id, "run-reopen");
        assert_eq!(recovered[0].status, OperationStatus::Succeeded);
    }
}
