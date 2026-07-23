use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, CoordinationRepository,
};
use crate::contexts::agent_runtime::domain::{
    CoordinationPlan, CoordinationRun, CoordinationRunStatus,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::BTreeSet;

#[derive(Clone)]
pub(crate) struct SqliteCoordinationRepository {
    database: NativeDatabase,
}

impl SqliteCoordinationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn serialize(run: &CoordinationRun) -> Result<String, AgentRuntimeApplicationError> {
        serde_json::to_string(run).map_err(repository_error)
    }

    fn deserialize(snapshot: &str) -> Result<CoordinationRun, AgentRuntimeApplicationError> {
        let mut run =
            serde_json::from_str::<CoordinationRun>(snapshot).map_err(repository_error)?;
        let known_agents = run
            .plan
            .nodes
            .iter()
            .flat_map(|node| node.candidates().map(str::to_string))
            .collect::<BTreeSet<_>>();
        let plan = CoordinationPlan::rehydrate(
            run.plan.name.clone(),
            run.plan.project_path.clone(),
            run.plan.nodes.clone(),
            &known_agents,
        )?;
        if plan.topological_order != run.plan.topological_order {
            return Err(repository_error(
                "stored coordination topological order does not match its graph",
            ));
        }
        run.plan = plan;
        Ok(run)
    }

    fn find_in(
        connection: &Connection,
        run_id: &str,
    ) -> Result<Option<CoordinationRun>, AgentRuntimeApplicationError> {
        connection
            .query_row(
                "SELECT run_snapshot FROM coordination_runs WHERE id = ?1",
                [run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(repository_error)?
            .map(|snapshot| Self::deserialize(&snapshot))
            .transpose()
    }
}

impl CoordinationRepository for SqliteCoordinationRepository {
    fn insert(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                r#"
                INSERT INTO coordination_runs
                    (id, operation_id, status, cancel_requested, run_snapshot,
                     created_at, updated_at, completed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    run.id,
                    run.operation_id,
                    status(run.status),
                    run.cancel_requested,
                    Self::serialize(run)?,
                    run.created_at,
                    run.updated_at,
                    run.completed_at,
                ],
            )
            .map(|_| ())
            .map_err(repository_error)
    }

    fn save(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .database
            .connection()
            .map_err(repository_error)?
            .execute(
                r#"
                UPDATE coordination_runs
                SET operation_id = ?2,
                    status = ?3,
                    cancel_requested = ?4,
                    run_snapshot = ?5,
                    updated_at = ?6,
                    completed_at = ?7
                WHERE id = ?1
                "#,
                params![
                    run.id,
                    run.operation_id,
                    status(run.status),
                    run.cancel_requested,
                    Self::serialize(run)?,
                    run.updated_at,
                    run.completed_at,
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(repository_error(format!(
                "coordination run not found: {}",
                run.id
            )));
        }
        Ok(())
    }

    fn find(&self, run_id: &str) -> Result<Option<CoordinationRun>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        Self::find_in(&connection, run_id)
    }

    fn list(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let mut statement = connection
            .prepare("SELECT run_snapshot FROM coordination_runs ORDER BY created_at DESC, id")
            .map_err(repository_error)?;
        let runs = statement
            .query_map([], snapshot_row)
            .map_err(repository_error)?
            .map(|snapshot| {
                snapshot
                    .map_err(repository_error)
                    .and_then(|snapshot| Self::deserialize(&snapshot))
            })
            .collect();
        runs
    }

    fn request_cancel(
        &self,
        run_id: &str,
        updated_at: &str,
    ) -> Result<CoordinationRun, AgentRuntimeApplicationError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let mut run = Self::find_in(&transaction, run_id)?
            .ok_or_else(|| repository_error(format!("coordination run not found: {run_id}")))?;
        if run.status.is_terminal() {
            return Ok(run);
        }
        run.request_cancel(updated_at);
        transaction
            .execute(
                r#"
                UPDATE coordination_runs
                SET status = ?2,
                    cancel_requested = 1,
                    run_snapshot = ?3,
                    updated_at = ?4,
                    completed_at = ?5
                WHERE id = ?1
                "#,
                params![
                    run.id,
                    status(run.status),
                    Self::serialize(&run)?,
                    run.updated_at,
                    run.completed_at,
                ],
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(run)
    }

    fn is_cancel_requested(&self, run_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .query_row(
                "SELECT cancel_requested FROM coordination_runs WHERE id = ?1",
                [run_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(repository_error)?
            .ok_or_else(|| repository_error(format!("coordination run not found: {run_id}")))
    }

    fn list_recoverable(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let mut statement = connection
            .prepare(
                "SELECT run_snapshot FROM coordination_runs WHERE status IN ('queued', 'running') ORDER BY created_at, id",
            )
            .map_err(repository_error)?;
        let runs = statement
            .query_map([], snapshot_row)
            .map_err(repository_error)?
            .map(|snapshot| {
                snapshot
                    .map_err(repository_error)
                    .and_then(|snapshot| Self::deserialize(&snapshot))
            })
            .collect();
        runs
    }
}

fn snapshot_row(row: &Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}

fn status(status: CoordinationRunStatus) -> &'static str {
    match status {
        CoordinationRunStatus::Queued => "queued",
        CoordinationRunStatus::Running => "running",
        CoordinationRunStatus::Succeeded => "succeeded",
        CoordinationRunStatus::Failed => "failed",
        CoordinationRunStatus::Cancelled => "cancelled",
    }
}

fn repository_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Coordination(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{CoordinationNodeInput, CoordinationPlanInput};
    use crate::test_support::TempDirectory;

    fn run(database: &NativeDatabase) -> CoordinationRun {
        let known_agents = ["codex-cli"].into_iter().map(str::to_string).collect();
        let plan = CoordinationPlan::new(
            CoordinationPlanInput {
                name: "pipeline".to_string(),
                project_path: None,
                nodes: vec![CoordinationNodeInput {
                    id: "implement".to_string(),
                    primary_agent_id: "codex-cli".to_string(),
                    fallback_agent_ids: Vec::new(),
                    instruction: "implement".to_string(),
                    depends_on: Vec::new(),
                }],
            },
            &known_agents,
        )
        .expect("plan");
        let run = CoordinationRun::new(
            "run-1".to_string(),
            "operation-1".to_string(),
            plan,
            "2026-07-23T00:00:00Z".to_string(),
        )
        .expect("run");
        assert!(database.db_path.exists());
        run
    }

    #[test]
    fn repository_round_trips_and_cancels_atomically() {
        let directory = TempDirectory::new("coordination-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteCoordinationRepository::new(database.clone());
        let stored_run = run(&database);

        repository.insert(&stored_run).expect("insert");
        assert_eq!(repository.find("run-1").expect("find"), Some(stored_run));

        let cancelled = repository
            .request_cancel("run-1", "2026-07-23T00:01:00Z")
            .expect("cancel");
        assert_eq!(cancelled.status, CoordinationRunStatus::Cancelled);
        assert!(repository
            .is_cancel_requested("run-1")
            .expect("cancel requested"));
        assert!(repository
            .list_recoverable()
            .expect("recoverable")
            .is_empty());

        let mut completed = run(&database);
        completed.id = "run-2".to_string();
        completed.operation_id = "operation-2".to_string();
        completed.status = CoordinationRunStatus::Succeeded;
        completed.completed_at = Some("2026-07-23T00:02:00Z".to_string());
        repository.insert(&completed).expect("insert completed");
        let unchanged = repository
            .request_cancel("run-2", "2026-07-23T00:03:00Z")
            .expect("terminal cancel");
        assert_eq!(unchanged, completed);
        assert!(!repository
            .is_cancel_requested("run-2")
            .expect("terminal cancel flag"));
    }
}
