use crate::contexts::operations::application::{
    ApplicationError, MissionControlQuery, MissionControlRepository,
};
use crate::contexts::operations::domain::AgentRun;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, ToSql};
use std::collections::BTreeMap;

pub(crate) struct SqliteMissionControlRepository {
    database: NativeDatabase,
}

impl SqliteMissionControlRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl MissionControlRepository for SqliteMissionControlRepository {
    fn query(
        &self,
        query: &MissionControlQuery,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<AgentRun>, ApplicationError> {
        let mut clauses = Vec::new();
        let mut values: Vec<Box<dyn ToSql>> = Vec::new();
        if let Some(runner) = query.runner.as_deref() {
            clauses.push("runner_kind = ?".into());
            values.push(Box::new(
                if runner == "remote" { "ssh" } else { runner }.to_string(),
            ));
        }
        if query.attention_only {
            clauses.push("(state IN ('waiting_approval', 'waiting_user', 'blocked', 'stuck', 'failed') OR EXISTS (SELECT 1 FROM agent_run_links attention_link WHERE attention_link.run_id = agent_runs.run_id AND attention_link.link_type = 'review'))".into());
        }
        if !query.states.is_empty() {
            let placeholders = std::iter::repeat_n("?", query.states.len())
                .collect::<Vec<_>>()
                .join(",");
            clauses.push(format!("state IN ({placeholders})"));
            for state in &query.states {
                values.push(Box::new(enum_text(state)?));
            }
        }
        if let Some(agent_id) = &query.agent_id {
            clauses.push("owner_id = ?".into());
            values.push(Box::new(agent_id.clone()));
        }
        if let Some(project_id) = &query.project_id {
            clauses.push("EXISTS (SELECT 1 FROM agent_run_links l WHERE l.run_id = agent_runs.run_id AND l.link_type = 'project' AND l.link_id = ?)".into());
            values.push(Box::new(project_id.clone()));
        }
        values.push(Box::new(limit as i64));
        values.push(Box::new(offset as i64));
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", clauses.join(" AND "))
        };
        let order = match query.sort.as_deref() {
            Some("oldest") => "created_at ASC, run_id ASC",
            Some("attention") => "CASE state WHEN 'waiting_approval' THEN 0 WHEN 'waiting_user' THEN 1 WHEN 'stuck' THEN 2 WHEN 'blocked' THEN 2 WHEN 'failed' THEN 3 ELSE 4 END, updated_at DESC, run_id",
            _ => "updated_at DESC, run_id",
        };
        // created_at is intentionally read from the safe snapshot while ordering uses indexed
        // updated_at unless oldest is explicitly requested.
        let order = if order.starts_with("created_at") {
            "json_extract(snapshot_json, '$.createdAt') ASC, run_id ASC"
        } else {
            order
        };
        let sql = format!(
            "SELECT snapshot_json FROM agent_runs{where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
        );
        let connection = self.database.connection().map_err(storage)?;
        let mut statement = connection.prepare(&sql).map_err(storage)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(values.iter().map(|value| value.as_ref())),
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        rows.into_iter().map(parse).collect()
    }

    fn counts(&self) -> Result<BTreeMap<String, usize>, ApplicationError> {
        let connection = self.database.connection().map_err(storage)?;
        let mut statement = connection
            .prepare("SELECT state, COUNT(*) FROM agent_runs GROUP BY state")
            .map_err(storage)?;
        let counts = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        Ok(counts
            .into_iter()
            .map(|(state, count)| (state, count.max(0) as usize))
            .collect())
    }

    fn get(&self, run_id: &str) -> Result<AgentRun, ApplicationError> {
        let connection = self.database.connection().map_err(storage)?;
        let value = connection
            .query_row(
                "SELECT snapshot_json FROM agent_runs WHERE run_id = ?1",
                params![run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?;
        value
            .ok_or_else(|| ApplicationError::NotFound(format!("run not found: {run_id}")))
            .and_then(parse)
    }
}

fn parse(value: String) -> Result<AgentRun, ApplicationError> {
    serde_json::from_str(&value).map_err(storage)
}
fn enum_text<T: serde::Serialize>(value: &T) -> Result<String, ApplicationError> {
    serde_json::to_string(value)
        .map(|text| text.trim_matches('"').to_string())
        .map_err(storage)
}
fn storage(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::infrastructure("mission_control_storage", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::{CreateAgentRun, MissionControlService};
    use crate::contexts::operations::domain::{
        RunOwner, RunRecoveryPolicy, RunRunner, RunRunnerKind, RunRunnerRecovery,
    };
    use crate::contexts::operations::infrastructure::persistent_run_service;
    use crate::test_support::TempDirectory;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingRepository {
        inner: SqliteMissionControlRepository,
        queries: AtomicUsize,
        counts: AtomicUsize,
        gets: AtomicUsize,
    }

    impl CountingRepository {
        fn new(database: NativeDatabase) -> Self {
            Self {
                inner: SqliteMissionControlRepository::new(database),
                queries: AtomicUsize::new(0),
                counts: AtomicUsize::new(0),
                gets: AtomicUsize::new(0),
            }
        }

        fn total_overview_queries(&self) -> usize {
            self.queries.load(Ordering::Relaxed) + self.counts.load(Ordering::Relaxed)
        }
    }

    impl MissionControlRepository for CountingRepository {
        fn query(
            &self,
            query: &MissionControlQuery,
            offset: usize,
            limit: usize,
        ) -> Result<Vec<AgentRun>, ApplicationError> {
            self.queries.fetch_add(1, Ordering::Relaxed);
            self.inner.query(query, offset, limit)
        }

        fn counts(&self) -> Result<BTreeMap<String, usize>, ApplicationError> {
            self.counts.fetch_add(1, Ordering::Relaxed);
            self.inner.counts()
        }

        fn get(&self, run_id: &str) -> Result<AgentRun, ApplicationError> {
            self.gets.fetch_add(1, Ordering::Relaxed);
            self.inner.get(run_id)
        }
    }

    #[test]
    fn overview_is_bounded_for_large_history_and_uses_safe_projection() {
        let directory = TempDirectory::new("mission-control-history");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let runs = persistent_run_service(database.clone());
        for index in 0..125 {
            runs.create(CreateAgentRun {
                id: Some(format!("018f0f17-4d6a-7e20-b41d-{index:012x}")),
                owner: RunOwner {
                    owner_type: "session_generation".into(),
                    owner_id: format!("agent-{index}"),
                },
                links: Vec::new(),
                parent_run_id: None,
                recovery_policy: RunRecoveryPolicy::NotRecoverable,
                runner: None,
                max_retries: 1,
                witness: "safe-witness".into(),
            })
            .expect("create run");
        }
        let service =
            MissionControlService::new(Arc::new(SqliteMissionControlRepository::new(database)));
        let overview = service
            .overview(MissionControlQuery {
                limit: Some(20),
                ..MissionControlQuery::default()
            })
            .expect("overview");
        assert_eq!(overview.active.items.len(), 20);
        assert_eq!(overview.active.next_cursor.as_deref(), Some("20"));
        assert!(overview
            .active
            .items
            .iter()
            .all(|run| run.title == "Agent Run" && run.tokens.is_none() && run.cost.is_none()));
    }

    #[test]
    fn rejects_invalid_cursor_without_querying_rows() {
        let directory = TempDirectory::new("mission-control-cursor");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let service =
            MissionControlService::new(Arc::new(SqliteMissionControlRepository::new(database)));
        let error = service
            .overview(MissionControlQuery {
                cursor: Some("../../logs".into()),
                ..MissionControlQuery::default()
            })
            .expect_err("invalid cursor");
        assert!(matches!(error, ApplicationError::Invalid(_)));
    }

    #[test]
    fn runner_filter_projects_metadata_and_preserves_legacy_missing_fields() {
        let directory = TempDirectory::new("mission-control-runner-filter");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let runs = persistent_run_service(database.clone());
        for (suffix, runner) in [
            ("000000000001", None),
            (
                "000000000002",
                Some(RunRunner {
                    kind: RunRunnerKind::Ssh,
                    target_id: "connection-1".into(),
                    target_revision: Some(7),
                    label: "Build host".into(),
                    host_label: Some("build.example.test".into()),
                    recovery: RunRunnerRecovery::InspectOnly,
                    capability_witness: "cap-v1".into(),
                    authority_witness: "authority-v7".into(),
                    recovery_reference: None,
                }),
            ),
        ] {
            runs.create(CreateAgentRun {
                id: Some(format!("018f0f17-4d6a-7e20-b41d-{suffix}")),
                owner: RunOwner {
                    owner_type: "agent_generation".into(),
                    owner_id: "codex-cli".into(),
                },
                links: Vec::new(),
                parent_run_id: None,
                recovery_policy: RunRecoveryPolicy::NotRecoverable,
                runner,
                max_retries: 0,
                witness: "created".into(),
            })
            .expect("create");
        }
        let service =
            MissionControlService::new(Arc::new(SqliteMissionControlRepository::new(database)));
        let ssh = service
            .overview(MissionControlQuery {
                runner: Some("ssh".into()),
                ..MissionControlQuery::default()
            })
            .expect("ssh overview");
        assert_eq!(ssh.active.items.len(), 1);
        assert_eq!(
            ssh.active.items[0]
                .runner
                .as_ref()
                .expect("runner")
                .target_revision,
            Some(7)
        );
        let legacy_alias = service
            .overview(MissionControlQuery {
                runner: Some("remote".into()),
                ..MissionControlQuery::default()
            })
            .expect("legacy alias");
        assert_eq!(legacy_alias.active.items.len(), 1);
        let all = service
            .overview(MissionControlQuery::default())
            .expect("all");
        assert!(all.active.items.iter().any(|run| run.runner.is_none()));
    }

    #[test]
    fn mission_control_newest_query_uses_existing_state_index() {
        let connection = rusqlite::Connection::open_in_memory().expect("database");
        crate::contexts::operations::infrastructure::apply_run_schema(&connection).expect("schema");
        crate::contexts::operations::infrastructure::apply_runner_projection_schema(&connection)
            .expect("runner projection");
        let plan = connection.prepare("EXPLAIN QUERY PLAN SELECT snapshot_json FROM agent_runs WHERE state = 'running' ORDER BY updated_at DESC, run_id LIMIT 20").expect("plan")
            .query_map([], |row| row.get::<_, String>(3)).expect("query").collect::<Result<Vec<_>, _>>().expect("rows");
        assert!(plan
            .iter()
            .any(|line| line.contains("idx_agent_runs_state")));
    }

    #[test]
    fn overview_query_count_is_constant_for_hundred_and_thousand_runs() {
        for run_count in [100_usize, 1_000] {
            let directory = TempDirectory::new(&format!("mission-control-{run_count}"));
            let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
            let runs = persistent_run_service(database.clone());
            for index in 0..run_count {
                runs.create(CreateAgentRun {
                    id: Some(format!("018f0f17-4d6a-7e20-b41d-{index:012x}")),
                    owner: RunOwner {
                        owner_type: "session_generation".into(),
                        owner_id: format!("agent-{index}"),
                    },
                    links: Vec::new(),
                    parent_run_id: None,
                    recovery_policy: RunRecoveryPolicy::NotRecoverable,
                    runner: None,
                    max_retries: 1,
                    witness: "safe-witness".into(),
                })
                .expect("create run");
            }
            let repository = Arc::new(CountingRepository::new(database));
            let service = MissionControlService::new(repository.clone());
            let overview = service
                .overview(MissionControlQuery {
                    limit: Some(100),
                    ..MissionControlQuery::default()
                })
                .expect("overview");
            assert_eq!(repository.total_overview_queries(), 4);
            assert_eq!(repository.gets.load(Ordering::Relaxed), 0);
            assert_eq!(overview.active.items.len(), 50);
            assert_eq!(overview.active.next_cursor.as_deref(), Some("50"));
            let run_id = &overview.active.items[0].run_id;
            service.detail(run_id).expect("lazy detail");
            assert_eq!(repository.gets.load(Ordering::Relaxed), 1);
            eprintln!(
                "MISSION_CONTROL_PERFORMANCE dataset=runs-{run_count} queries=4 rows=50 lazy_detail_queries=1"
            );
        }
    }
}
