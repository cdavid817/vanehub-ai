use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Transaction, TransactionBehavior};

use super::deletion::delete_signal_set;
use super::{EvidenceRepositoryError, SqliteEvolutionEvidenceRepository};

pub(crate) const EVIDENCE_RETENTION_DAYS: i64 = 90;
pub(crate) const MAX_SIGNALS_PER_WORKSPACE: usize = 10_000;
pub(crate) const MAX_SEEDS_PER_WORKSPACE: usize = 2_000;
pub(crate) const MAX_EVIDENCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvidenceGovernancePolicy {
    pub(crate) retention_days: i64,
    pub(crate) signals_per_workspace: usize,
    pub(crate) seeds_per_workspace: usize,
    pub(crate) evidence_bytes: usize,
}

impl Default for EvidenceGovernancePolicy {
    fn default() -> Self {
        Self {
            retention_days: EVIDENCE_RETENTION_DAYS,
            signals_per_workspace: MAX_SIGNALS_PER_WORKSPACE,
            seeds_per_workspace: MAX_SEEDS_PER_WORKSPACE,
            evidence_bytes: MAX_EVIDENCE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvidenceMaintenanceOutcome {
    pub(crate) expired_signals: usize,
    pub(crate) quota_evicted_signals: usize,
    pub(crate) quota_evicted_seeds: usize,
    pub(crate) estimated_bytes: usize,
}

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn maintain(
        &self,
        now: DateTime<Utc>,
        policy: EvidenceGovernancePolicy,
    ) -> Result<EvidenceMaintenanceOutcome, EvidenceRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cutoff = (now - Duration::days(policy.retention_days)).to_rfc3339();
        let expired = expired_signal_ids(&transaction, &cutoff)?;
        let expired_signals = delete_signal_set(&transaction, &expired)?.signals;
        delete_expired_feedback(&transaction, &cutoff)?;

        let quota_ids = workspace_overflow_ids(&transaction, policy.signals_per_workspace)?;
        let quota_evicted_signals = delete_signal_set(&transaction, &quota_ids)?.signals;
        let quota_evicted_seeds = trim_workspace_seeds(&transaction, policy.seeds_per_workspace)?;
        let mut estimated_bytes = estimate_bytes(&transaction)?;
        let mut byte_evicted = 0;
        if estimated_bytes > policy.evidence_bytes {
            let candidates = all_retention_ids(&transaction)?;
            for signal_id in candidates {
                if estimated_bytes <= policy.evidence_bytes {
                    break;
                }
                byte_evicted += delete_signal_set(&transaction, &[signal_id])?.signals;
                estimated_bytes = estimate_bytes(&transaction)?;
            }
        }
        record_counter(&transaction, "expired_signals", expired_signals)?;
        record_counter(
            &transaction,
            "quota_evicted_signals",
            quota_evicted_signals + byte_evicted,
        )?;
        transaction.commit()?;
        Ok(EvidenceMaintenanceOutcome {
            expired_signals,
            quota_evicted_signals: quota_evicted_signals + byte_evicted,
            quota_evicted_seeds,
            estimated_bytes,
        })
    }
}

fn workspace_overflow_ids(
    transaction: &Transaction<'_>,
    limit: usize,
) -> Result<Vec<String>, EvidenceRepositoryError> {
    let mut workspaces = transaction.prepare(
        "SELECT workspace, COUNT(*) FROM evolution_signals GROUP BY workspace HAVING COUNT(*) > ?1 ORDER BY workspace",
    )?;
    let rows = workspaces.query_map([limit as i64], |row| {
        Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
    })?;
    let values = rows.collect::<Result<Vec<_>, _>>()?;
    let mut ids = Vec::new();
    for (workspace, count) in values {
        let count = usize::try_from(count).unwrap_or_default();
        ids.extend(workspace_retention_ids(
            transaction,
            workspace.as_deref(),
            count.saturating_sub(limit),
        )?);
    }
    Ok(ids)
}

fn retention_order_sql() -> &'static str {
    r#"WHERE workspace IS ?1 ORDER BY
       CASE
         WHEN category = 'explicit_feedback' AND safe_summary LIKE 'corrected:%' THEN 1
         WHEN attribution_strength = 'verified' AND category IN ('retry_recovery', 'verification_outcome') THEN 1
         WHEN attribution_strength = 'verified' AND category IN ('execution_failure', 'delegation_outcome') THEN 2
         WHEN attribution_strength = 'correlated' THEN 3
         WHEN category = 'explicit_feedback' OR polarity = 'neutral' THEN 4
         ELSE 5
       END DESC, occurred_at, signal_id"#
}

fn workspace_retention_ids(
    transaction: &Transaction<'_>,
    workspace: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, EvidenceRepositoryError> {
    let sql = format!(
        "SELECT signal_id FROM evolution_signals {} LIMIT ?2",
        retention_order_sql()
    );
    let mut statement = transaction.prepare(&sql)?;
    let rows = statement.query_map(params![workspace, limit as i64], |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn expired_signal_ids(
    transaction: &Transaction<'_>,
    cutoff: &str,
) -> Result<Vec<String>, EvidenceRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT signal_id FROM evolution_signals WHERE occurred_at < ?1 ORDER BY occurred_at, signal_id",
    )?;
    let rows = statement.query_map([cutoff], |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn all_retention_ids(
    transaction: &Transaction<'_>,
) -> Result<Vec<String>, EvidenceRepositoryError> {
    let mut statement = transaction.prepare(
        r#"SELECT signal_id FROM evolution_signals ORDER BY
           CASE
             WHEN category = 'explicit_feedback' AND safe_summary LIKE 'corrected:%' THEN 1
             WHEN attribution_strength = 'verified' AND category IN ('retry_recovery', 'verification_outcome') THEN 1
             WHEN attribution_strength = 'verified' AND category IN ('execution_failure', 'delegation_outcome') THEN 2
             WHEN attribution_strength = 'correlated' THEN 3
             WHEN category = 'explicit_feedback' OR polarity = 'neutral' THEN 4
             ELSE 5
           END DESC, occurred_at, signal_id"#,
    )?;
    let rows = statement.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn trim_workspace_seeds(
    transaction: &Transaction<'_>,
    limit: usize,
) -> Result<usize, EvidenceRepositoryError> {
    let mut statement = transaction.prepare("SELECT workspace, COUNT(*) FROM evolution_candidate_seeds GROUP BY workspace HAVING COUNT(*) > ?1 ORDER BY workspace")?;
    let groups = statement
        .query_map([limit as i64], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut deleted = 0;
    for (workspace, count) in groups {
        let count = usize::try_from(count).unwrap_or_default();
        deleted += transaction.execute(
            "DELETE FROM evolution_candidate_seeds WHERE seed_id IN (SELECT seed_id FROM evolution_candidate_seeds WHERE workspace IS ?1 ORDER BY CASE readiness WHEN 'ineligible' THEN 4 WHEN 'collecting' THEN 3 WHEN 'human_review_only' THEN 2 ELSE 1 END DESC, created_at, seed_id LIMIT ?2)",
            params![workspace, count.saturating_sub(limit) as i64],
        )?;
    }
    Ok(deleted)
}

fn estimate_bytes(transaction: &Transaction<'_>) -> Result<usize, EvidenceRepositoryError> {
    let signals: i64 = transaction.query_row("SELECT COALESCE(SUM(length(source_event_id)+length(discriminator)+length(safe_summary)+length(task_fingerprint)), 0) FROM evolution_signals", [], |row| row.get(0))?;
    let seeds: i64 = transaction.query_row("SELECT COALESCE(SUM(length(grouping_key_hash)+length(safe_summary)+length(lineage_summary_json)), 0) FROM evolution_candidate_seeds", [], |row| row.get(0))?;
    Ok((signals + seeds).max(0) as usize)
}

fn delete_expired_feedback(
    transaction: &Transaction<'_>,
    cutoff: &str,
) -> Result<(), EvidenceRepositoryError> {
    transaction.execute(
        "DELETE FROM evolution_feedback_current WHERE updated_at < ?1",
        [cutoff],
    )?;
    transaction.execute(
        "DELETE FROM evolution_feedback_events WHERE created_at < ?1",
        [cutoff],
    )?;
    Ok(())
}

fn record_counter(
    transaction: &Transaction<'_>,
    key: &str,
    increment: usize,
) -> Result<(), EvidenceRepositoryError> {
    transaction.execute(
        "INSERT INTO evolution_pipeline_state (state_key, state_value, revision, updated_at) VALUES (?1, 'counter', ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) ON CONFLICT(state_key) DO UPDATE SET revision = revision + excluded.revision, updated_at = excluded.updated_at",
        params![key, increment as i64],
    )?;
    Ok(())
}
