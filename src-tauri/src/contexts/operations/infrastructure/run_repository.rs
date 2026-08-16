use crate::contexts::operations::application::{
    AgentRunRepository, AgentRunService, ApplicationError, OperationIdGenerator, RunClockPort,
    RunListFilter,
};
use crate::contexts::operations::domain::{AgentRun, RunEvent};
use crate::platform::clock::SystemClock;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Arc;

pub(crate) fn apply_run_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE agent_runs (
            run_id TEXT PRIMARY KEY,
            owner_type TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            parent_run_id TEXT,
            state TEXT NOT NULL,
            version INTEGER NOT NULL,
            updated_at TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            CHECK (length(run_id) BETWEEN 1 AND 128),
            CHECK (version > 0),
            CHECK (parent_run_id IS NULL OR parent_run_id <> run_id)
        );
        CREATE TABLE agent_run_events (
            run_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            state TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            event_json TEXT NOT NULL,
            PRIMARY KEY (run_id, sequence),
            FOREIGN KEY (run_id) REFERENCES agent_runs(run_id) ON DELETE CASCADE
        );
        CREATE TABLE agent_run_links (
            run_id TEXT NOT NULL,
            link_type TEXT NOT NULL,
            link_id TEXT NOT NULL,
            PRIMARY KEY (run_id, link_type, link_id),
            FOREIGN KEY (run_id) REFERENCES agent_runs(run_id) ON DELETE CASCADE,
            CHECK (length(link_type) BETWEEN 1 AND 128),
            CHECK (length(link_id) BETWEEN 1 AND 128)
        );
        CREATE INDEX idx_agent_runs_owner ON agent_runs(owner_type, owner_id, updated_at DESC);
        CREATE INDEX idx_agent_runs_parent ON agent_runs(parent_run_id, updated_at, run_id);
        CREATE INDEX idx_agent_runs_state ON agent_runs(state, updated_at, run_id);
        CREATE INDEX idx_agent_run_links_target ON agent_run_links(link_type, link_id, run_id);
        "#,
    )?;
    Ok(())
}

struct SqliteRunRepository {
    database: NativeDatabase,
}

impl SqliteRunRepository {
    fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl AgentRunRepository for SqliteRunRepository {
    fn insert(&self, run: &AgentRun, event: &RunEvent) -> Result<(), ApplicationError> {
        let mut connection = self.database.connection().map_err(storage)?;
        let transaction = connection.transaction().map_err(storage)?;
        transaction.execute(
            "INSERT INTO agent_runs (run_id, owner_type, owner_id, parent_run_id, state, version, updated_at, snapshot_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![run.id, run.owner.owner_type, run.owner.owner_id, run.parent_run_id, enum_text(&run.state)?, run.version as i64, run.updated_at, json(run)?],
        ).map_err(storage)?;
        for link in &run.links {
            transaction
                .execute(
                    "INSERT INTO agent_run_links (run_id, link_type, link_id) VALUES (?1, ?2, ?3)",
                    params![run.id, link.link_type, link.link_id],
                )
                .map_err(storage)?;
        }
        insert_event(&transaction, &run.id, event)?;
        transaction.commit().map_err(storage)
    }

    fn get(&self, id: &str) -> Result<AgentRun, ApplicationError> {
        let connection = self.database.connection().map_err(storage)?;
        let value: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM agent_runs WHERE run_id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        value
            .ok_or_else(|| ApplicationError::NotFound(format!("run not found: {id}")))
            .and_then(parse)
    }

    fn save(
        &self,
        expected: u64,
        run: &AgentRun,
        event: Option<&RunEvent>,
    ) -> Result<(), ApplicationError> {
        let mut connection = self.database.connection().map_err(storage)?;
        let transaction = connection.transaction().map_err(storage)?;
        let changed = transaction.execute(
            "UPDATE agent_runs SET state = ?1, version = ?2, updated_at = ?3, snapshot_json = ?4 WHERE run_id = ?5 AND version = ?6",
            params![enum_text(&run.state)?, run.version as i64, run.updated_at, json(run)?, run.id, expected as i64],
        ).map_err(storage)?;
        if changed != 1 {
            return Err(ApplicationError::Conflict);
        }
        if let Some(event) = event {
            insert_event(&transaction, &run.id, event)?;
        }
        transaction.commit().map_err(storage)
    }

    fn list(
        &self,
        filter: &RunListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<AgentRun>, ApplicationError> {
        let state = filter.state.as_ref().map(enum_text).transpose()?;
        let mut clauses = Vec::new();
        let mut parameters: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (column, value) in [
            ("owner_type", filter.owner_type.as_ref()),
            ("owner_id", filter.owner_id.as_ref()),
            ("parent_run_id", filter.parent_run_id.as_ref()),
            ("state", state.as_ref()),
        ] {
            if let Some(value) = value {
                clauses.push(format!("{column} = ?"));
                parameters.push(Box::new(value.clone()));
            }
        }
        parameters.push(Box::new(limit as i64));
        parameters.push(Box::new(offset as i64));
        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT snapshot_json FROM agent_runs{where_clause} \
             ORDER BY updated_at DESC, run_id LIMIT ? OFFSET ?"
        );
        let connection = self.database.connection().map_err(storage)?;
        let mut statement = connection.prepare(&sql).map_err(storage)?;
        let values = statement
            .query_map(
                rusqlite::params_from_iter(parameters.iter().map(|value| value.as_ref())),
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        values.into_iter().map(parse).collect()
    }

    fn children(&self, parent: &str) -> Result<Vec<AgentRun>, ApplicationError> {
        self.query("SELECT snapshot_json FROM agent_runs WHERE parent_run_id = ?1 ORDER BY updated_at, run_id", params![parent])
    }

    fn events(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ApplicationError> {
        let connection = self.database.connection().map_err(storage)?;
        let mut statement = connection.prepare("SELECT event_json FROM agent_run_events WHERE run_id = ?1 ORDER BY sequence LIMIT ?2 OFFSET ?3").map_err(storage)?;
        let values = statement
            .query_map(params![id, limit as i64, offset as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        values.into_iter().map(parse).collect()
    }
}

impl SqliteRunRepository {
    fn query<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<AgentRun>, ApplicationError> {
        let connection = self.database.connection().map_err(storage)?;
        let mut statement = connection.prepare(sql).map_err(storage)?;
        let values = statement
            .query_map(params, |row| row.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        values.into_iter().map(parse).collect()
    }
}

fn insert_event(
    connection: &Connection,
    id: &str,
    event: &RunEvent,
) -> Result<(), ApplicationError> {
    connection.execute("INSERT INTO agent_run_events (run_id, sequence, state, timestamp, event_json) VALUES (?1, ?2, ?3, ?4, ?5)", params![id, event.sequence as i64, enum_text(&event.state)?, event.timestamp, json(event)?]).map_err(storage)?;
    Ok(())
}
fn json<T: serde::Serialize>(value: &T) -> Result<String, ApplicationError> {
    serde_json::to_string(value).map_err(storage)
}
fn enum_text<T: serde::Serialize>(value: &T) -> Result<String, ApplicationError> {
    Ok(json(value)?.trim_matches('"').to_string())
}
fn parse<T: serde::de::DeserializeOwned>(value: String) -> Result<T, ApplicationError> {
    serde_json::from_str(&value).map_err(storage)
}
fn storage(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::infrastructure("agent_run_storage", error.to_string())
}

struct UuidRunIds;
impl OperationIdGenerator for UuidRunIds {
    fn next_id(&self, _: &str) -> String {
        uuid::Uuid::now_v7().to_string()
    }
}
pub(crate) fn persistent_run_service(database: NativeDatabase) -> AgentRunService {
    AgentRunService::new(
        Arc::new(SqliteRunRepository::new(database)),
        Arc::new(Rfc3339RunClock),
        Arc::new(UuidRunIds),
    )
}

struct Rfc3339RunClock;
impl RunClockPort for Rfc3339RunClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::CreateAgentRun;
    use crate::contexts::operations::domain::{RunLink, RunOwner, RunRecoveryPolicy, RunState};
    use crate::test_support::TempDirectory;

    #[test]
    fn snapshots_events_and_restart_reconciliation_survive_reopen() {
        let directory = TempDirectory::new("canonical-runs");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let service = persistent_run_service(database.clone());
        let run = service
            .create(CreateAgentRun {
                id: None,
                owner: RunOwner {
                    owner_type: "cli_generation".into(),
                    owner_id: "generation-1".into(),
                },
                links: vec![RunLink {
                    link_type: "session".into(),
                    link_id: "session-1".into(),
                }],
                parent_run_id: None,
                recovery_policy: RunRecoveryPolicy::NotRecoverable,
                max_retries: 1,
                witness: "created".into(),
            })
            .expect("create");
        assert_eq!(service.events(&run.id, 0, 10).expect("events").len(), 1);
        drop(service);
        drop(database);
        let reopened = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
        let service = persistent_run_service(reopened);
        assert_eq!(service.reconcile_after_restart().expect("reconcile"), 1);
        assert_eq!(service.get(&run.id).expect("run").state, RunState::Failed);
        assert_eq!(service.reconcile_after_restart().expect("idempotent"), 0);
        assert_eq!(service.events(&run.id, 0, 10).expect("events").len(), 2);
    }

    #[test]
    fn run_schema_failure_rolls_back_and_legacy_tables_are_untouched() {
        let mut connection = Connection::open_in_memory().expect("database");
        connection
            .execute("CREATE TABLE legacy_records (id TEXT PRIMARY KEY)", [])
            .expect("legacy");
        let transaction = connection.transaction().expect("transaction");
        transaction
            .execute("CREATE TABLE agent_run_events (conflict TEXT)", [])
            .expect("conflict");
        assert!(apply_run_schema(&transaction).is_err());
        transaction.rollback().expect("rollback");
        let run_table: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'agent_runs'",
                [],
                |row| row.get(0),
            )
            .expect("schema");
        assert_eq!(run_table, 0);
        connection
            .execute(
                "INSERT INTO legacy_records (id) VALUES ('still-readable')",
                [],
            )
            .expect("legacy remains writable");
    }

    #[test]
    fn owner_history_and_event_queries_use_declared_indexes() {
        let connection = Connection::open_in_memory().expect("database");
        apply_run_schema(&connection).expect("schema");
        let owner_plan = query_plan(
            &connection,
            "SELECT snapshot_json FROM agent_runs WHERE owner_type = ?1 AND owner_id = ?2 ORDER BY updated_at DESC, run_id LIMIT 10",
            &[&"plan_run", &"plan-1"],
        );
        let event_plan = query_plan(
            &connection,
            "SELECT event_json FROM agent_run_events WHERE run_id = ?1 ORDER BY sequence LIMIT 10",
            &[&"run-1"],
        );
        assert!(owner_plan
            .iter()
            .any(|line| line.contains("idx_agent_runs_owner")));
        assert!(event_plan
            .iter()
            .any(|line| line.contains("sqlite_autoindex_agent_run_events")));
    }

    fn query_plan(
        connection: &Connection,
        sql: &str,
        parameters: &[&dyn rusqlite::ToSql],
    ) -> Vec<String> {
        connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .expect("plan")
            .query_map(parameters, |row| row.get::<_, String>(3))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows")
    }
}
