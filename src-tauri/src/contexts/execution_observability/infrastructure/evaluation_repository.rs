use crate::contexts::execution_observability::application::EvaluationRepositoryPort;
use crate::contexts::execution_observability::domain::{EvaluationArena, EvaluationAttempt};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};

const MAX_PAGE: usize = 100;
const MAX_SNAPSHOT_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub(crate) struct SqliteEvaluationRepository {
    database: NativeDatabase,
}

impl SqliteEvaluationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn save_terminal(
        &self,
        arena: &EvaluationArena,
        attempt: &EvaluationAttempt,
        timestamp: &str,
    ) -> Result<(), String> {
        if attempt.arena_id != arena.id || arena.attempts.len() > 8 {
            return Err("evaluation aggregate relationship is invalid".to_string());
        }
        let mut safe_arena = arena.clone();
        safe_arena.attempts.clear();
        let arena_json = safe_json(&safe_arena)?;
        let attempt_json = safe_json(attempt)?;
        let mut connection = self.database.connection().map_err(display)?;
        let transaction = connection.transaction().map_err(display)?;
        transaction
            .execute(
                "INSERT INTO evaluation_arenas (arena_id, operation_id, task_id, task_version, ranking_version, safe_snapshot_json, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?7) ON CONFLICT(arena_id) DO UPDATE SET safe_snapshot_json=excluded.safe_snapshot_json, updated_at=excluded.updated_at",
                params![arena.id, arena.operation_id, arena.task_id, arena.task_version, arena.ranking_version, arena_json, timestamp],
            )
            .map_err(display)?;
        transaction
            .execute(
                "INSERT INTO evaluation_attempts (attempt_id, arena_id, canonical_run_id, outcome, safe_snapshot_json, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?6) ON CONFLICT(attempt_id) DO UPDATE SET outcome=excluded.outcome, safe_snapshot_json=excluded.safe_snapshot_json, updated_at=excluded.updated_at",
                params![attempt.id, attempt.arena_id, attempt.canonical_run_id, outcome_name(attempt), attempt_json, timestamp],
            )
            .map_err(display)?;
        transaction
            .execute(
                "DELETE FROM evaluation_metrics WHERE attempt_id=?1",
                [&attempt.id],
            )
            .map_err(display)?;
        transaction
            .execute(
                "DELETE FROM evaluation_verifications WHERE attempt_id=?1",
                [&attempt.id],
            )
            .map_err(display)?;
        transaction
            .execute(
                "DELETE FROM evaluation_artifact_refs WHERE attempt_id=?1",
                [&attempt.id],
            )
            .map_err(display)?;
        for metric in &attempt.metrics {
            transaction.execute(
                "INSERT INTO evaluation_metrics (attempt_id,metric_name,value,unit,quality,source) VALUES (?1,?2,?3,?4,?5,?6)",
                params![attempt.id, metric.name, metric.value, metric.unit, serde_json::to_value(&metric.quality).map_err(display)?.as_str().unwrap_or("unavailable"), metric.source],
            ).map_err(display)?;
        }
        for check in &attempt.checks {
            transaction.execute(
                "INSERT INTO evaluation_verifications (attempt_id,check_id,passed,summary) VALUES (?1,?2,?3,?4)",
                params![attempt.id, check.check_id, check.passed, bounded(&check.summary, 1_000)],
            ).map_err(display)?;
        }
        for artifact_id in attempt.artifact_ids.iter().take(64) {
            transaction
                .execute(
                    "INSERT INTO evaluation_artifact_refs (attempt_id,artifact_id) VALUES (?1,?2)",
                    params![attempt.id, artifact_id],
                )
                .map_err(display)?;
        }
        transaction.commit().map_err(display)
    }

    pub(crate) fn get(&self, arena_id: &str) -> Result<Option<EvaluationArena>, String> {
        let connection = self.database.connection().map_err(display)?;
        let Some(json) = connection
            .query_row(
                "SELECT safe_snapshot_json FROM evaluation_arenas WHERE arena_id=?1",
                [arena_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(display)?
        else {
            return Ok(None);
        };
        let mut arena: EvaluationArena = serde_json::from_str(&json).map_err(display)?;
        let mut statement = connection
            .prepare("SELECT safe_snapshot_json FROM evaluation_attempts WHERE arena_id=?1 ORDER BY attempt_id")
            .map_err(display)?;
        arena.attempts = statement
            .query_map([arena_id], |row| row.get::<_, String>(0))
            .map_err(display)?
            .map(|row| {
                row.map_err(display)
                    .and_then(|value| serde_json::from_str(&value).map_err(display))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(arena))
    }

    pub(crate) fn list(&self, offset: usize, limit: usize) -> Result<Vec<EvaluationArena>, String> {
        let connection = self.database.connection().map_err(display)?;
        let mut statement = connection.prepare(
            "SELECT arena_id FROM evaluation_arenas ORDER BY created_at DESC, arena_id DESC LIMIT ?1 OFFSET ?2",
        ).map_err(display)?;
        let bounded_limit = i64::try_from(limit.clamp(1, MAX_PAGE)).map_err(display)?;
        let bounded_offset = i64::try_from(offset).map_err(display)?;
        let ids = statement
            .query_map(params![bounded_limit, bounded_offset], |row| {
                row.get::<_, String>(0)
            })
            .map_err(display)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(display)?;
        drop(statement);
        drop(connection);
        ids.iter()
            .map(|id| {
                self.get(id)?
                    .ok_or_else(|| "evaluation arena disappeared".to_string())
            })
            .collect()
    }

    pub(crate) fn retain_since(&self, cutoff: &str) -> Result<usize, String> {
        self.database
            .connection()
            .map_err(display)?
            .execute(
                "DELETE FROM evaluation_arenas WHERE updated_at < ?1",
                [cutoff],
            )
            .map_err(display)
    }
}

impl EvaluationRepositoryPort for SqliteEvaluationRepository {
    fn save_terminal(
        &self,
        arena: &EvaluationArena,
        attempt: &EvaluationAttempt,
        timestamp: &str,
    ) -> Result<(), String> {
        Self::save_terminal(self, arena, attempt, timestamp)
    }
    fn get(&self, arena_id: &str) -> Result<Option<EvaluationArena>, String> {
        Self::get(self, arena_id)
    }
    fn list(&self, offset: usize, limit: usize) -> Result<Vec<EvaluationArena>, String> {
        Self::list(self, offset, limit)
    }
    fn retain_since(&self, cutoff: &str) -> Result<usize, String> {
        Self::retain_since(self, cutoff)
    }
}

fn safe_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_string(value).map_err(display)?;
    if json.len() > MAX_SNAPSHOT_BYTES || json.contains("secret") || json.contains("api_key") {
        return Err("evaluation snapshot is unsafe or oversized".to_string());
    }
    Ok(json)
}

fn outcome_name(attempt: &EvaluationAttempt) -> String {
    serde_json::to_value(&attempt.outcome)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "benchmark_error".to_string())
}

fn bounded(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::domain::{
        EvaluationAgentSnapshot, EvaluationCheck, EvaluationMetric, EvaluationOutcome,
        MetricQuality, EVALUATION_RANKING_VERSION,
    };

    fn repository() -> (tempfile::TempDir, SqliteEvaluationRepository) {
        let directory = tempfile::tempdir().expect("temp");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (directory, SqliteEvaluationRepository::new(database))
    }

    fn arena(id: &str) -> EvaluationArena {
        let attempt = EvaluationAttempt {
            id: format!("attempt-{id}"),
            arena_id: id.into(),
            canonical_run_id: format!("run-{id}"),
            task_id: "task".into(),
            task_version: 1,
            agent: EvaluationAgentSnapshot {
                agent_id: "fake".into(),
                provider_id: "local".into(),
                model_id: None,
                interaction_mode: "fake".into(),
                configuration_fingerprint: "sha256-safe".into(),
            },
            outcome: EvaluationOutcome::Succeeded,
            checks: vec![EvaluationCheck {
                check_id: "tests".into(),
                passed: true,
                summary: "passed".into(),
            }],
            judge: None,
            metrics: vec![EvaluationMetric {
                name: "duration".into(),
                value: Some(10.0),
                unit: "ms".into(),
                quality: MetricQuality::Reported,
                source: "runtime".into(),
            }],
            context_evidence_manifest_id: Some("manifest-1".into()),
            artifact_ids: vec!["artifact-1".into()],
        };
        EvaluationArena {
            id: id.into(),
            operation_id: format!("operation-{id}"),
            task_id: "task".into(),
            task_version: 1,
            ranking_version: EVALUATION_RANKING_VERSION.into(),
            attempts: vec![attempt],
        }
    }

    #[test]
    fn terminal_write_round_trips_atomically_and_paginates() {
        let (_directory, repository) = repository();
        for (id, timestamp) in [("a", "2026-08-01T00:00:00Z"), ("b", "2026-08-02T00:00:00Z")] {
            let value = arena(id);
            repository
                .save_terminal(&value, &value.attempts[0], timestamp)
                .expect("save");
        }
        assert_eq!(repository.get("a").expect("get"), Some(arena("a")));
        assert_eq!(repository.list(0, 1).expect("page")[0].id, "b");
        assert_eq!(repository.list(1, 1).expect("page")[0].id, "a");
    }

    #[test]
    fn retention_cascades_and_unsafe_snapshots_are_rejected() {
        let (_directory, repository) = repository();
        let old = arena("old");
        repository
            .save_terminal(&old, &old.attempts[0], "2026-01-01T00:00:00Z")
            .expect("save");
        assert_eq!(
            repository
                .retain_since("2026-08-01T00:00:00Z")
                .expect("retain"),
            1
        );
        assert_eq!(repository.get("old").expect("get"), None);
        let mut unsafe_arena = arena("unsafe");
        unsafe_arena.attempts[0].agent.configuration_fingerprint = "secret-value".into();
        assert!(repository
            .save_terminal(
                &unsafe_arena,
                &unsafe_arena.attempts[0],
                "2026-08-01T00:00:00Z"
            )
            .is_err());
    }

    #[test]
    fn rejected_terminal_write_leaves_no_partial_arena() {
        let (_directory, repository) = repository();
        let value = arena("atomic");
        let mut mismatched = value.attempts[0].clone();
        mismatched.arena_id = "different".into();
        assert!(repository
            .save_terminal(&value, &mismatched, "2026-08-01T00:00:00Z")
            .is_err());
        assert_eq!(repository.get("atomic").expect("get"), None);
    }

    #[test]
    fn maximum_result_page_is_bounded_and_uses_hot_query_index() {
        let (_directory, repository) = repository();
        for index in 0..120 {
            let id = format!("perf-{index:03}");
            let value = arena(&id);
            repository
                .save_terminal(&value, &value.attempts[0], "2026-08-01T00:00:00Z")
                .expect("save fixture");
        }
        assert_eq!(
            repository.list(0, usize::MAX).expect("bounded page").len(),
            MAX_PAGE
        );
        let connection = repository.database.connection().expect("connection");
        let detail = connection
            .prepare("EXPLAIN QUERY PLAN SELECT arena_id FROM evaluation_arenas ORDER BY created_at DESC, arena_id DESC LIMIT 100 OFFSET 0")
            .expect("query plan")
            .query_map([], |row| row.get::<_, String>(3))
            .expect("plan rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("plan details")
            .join(" ");
        assert!(detail.contains("idx_evaluation_arenas_created"), "{detail}");
    }
}
