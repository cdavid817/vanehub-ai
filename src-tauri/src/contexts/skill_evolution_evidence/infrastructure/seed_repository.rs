use rusqlite::{params, OptionalExtension, Row, Transaction, TransactionBehavior};

use super::seed_builder::{build_seeds, SeedProjection, SeedSignalRow, SEED_BUILDER_V1};
use super::{EvidenceRepositoryError, SqliteEvolutionEvidenceRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeedRebuildOutcome {
    pub(crate) signal_set_revision: u64,
    pub(crate) seed_count: usize,
    pub(crate) rebuilt: bool,
}

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn rebuild_dirty_seeds(
        &self,
    ) -> Result<SeedRebuildOutcome, EvidenceRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let (state, revision): (String, i64) = transaction.query_row(
            "SELECT state_value, revision FROM evolution_pipeline_state WHERE state_key = 'signal_set'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if state != "dirty" {
            let seed_count = count_seeds(&transaction)?;
            return Ok(SeedRebuildOutcome {
                signal_set_revision: revision as u64,
                seed_count,
                rebuilt: false,
            });
        }
        let projections = build_seeds(load_active_signals(&transaction)?)
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        replace_seeds(&transaction, revision, &projections)?;
        transaction.execute(
            "UPDATE evolution_pipeline_state SET state_value = 'clean', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE state_key = 'signal_set' AND revision = ?1",
            [revision],
        )?;
        transaction.commit()?;
        Ok(SeedRebuildOutcome {
            signal_set_revision: revision as u64,
            seed_count: projections.len(),
            rebuilt: true,
        })
    }

    pub(crate) fn supersede_signal(
        &self,
        signal_id: &str,
        replacement_signal_id: Option<&str>,
    ) -> Result<bool, EvidenceRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let changed = transaction.execute(
            "UPDATE evolution_signals SET lineage_status = 'superseded', superseded_by_signal_id = ?2 WHERE signal_id = ?1 AND lineage_status = 'active'",
            params![signal_id, replacement_signal_id],
        )?;
        if changed != 0 {
            transaction.execute(
                "UPDATE evolution_pipeline_state SET state_value = 'dirty', revision = revision + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE state_key = 'signal_set'",
                [],
            )?;
        }
        transaction.commit()?;
        Ok(changed != 0)
    }
}

fn load_active_signals(
    transaction: &Transaction<'_>,
) -> Result<Vec<SeedSignalRow>, EvidenceRepositoryError> {
    let mut statement = transaction.prepare(
        r#"SELECT signal_id, workspace, category, polarity, severity, attribution_strength,
                  occurred_at, extractor_version, sanitizer_version, task_fingerprint_version,
                  task_fingerprint, discriminator, safe_summary, source_agent_id,
                  (SELECT source_id FROM evolution_signal_source_links link WHERE link.signal_id = signal.signal_id AND source_kind = 'run' LIMIT 1),
                  (SELECT source_id FROM evolution_signal_source_links link WHERE link.signal_id = signal.signal_id AND source_kind = 'attempt' LIMIT 1)
           FROM evolution_signals signal
           WHERE lineage_status = 'active'
           ORDER BY occurred_at, signal_id"#,
    )?;
    let mut signals = statement
        .query_map([], map_signal)?
        .collect::<Result<Vec<_>, _>>()?;
    for signal in &mut signals {
        let (verified, human) = load_targets(transaction, &signal.signal_id)?;
        signal.verified_targets = verified;
        signal.human_targets = human;
    }
    Ok(signals)
}

fn map_signal(row: &Row<'_>) -> rusqlite::Result<SeedSignalRow> {
    Ok(SeedSignalRow {
        signal_id: row.get(0)?,
        workspace: row.get(1)?,
        category: row.get(2)?,
        polarity: row.get(3)?,
        severity: row.get(4)?,
        attribution: row.get(5)?,
        occurred_at: row.get(6)?,
        extractor_version: row.get::<_, u16>(7)?,
        sanitizer_version: row.get::<_, u16>(8)?,
        fingerprint_version: row.get::<_, u16>(9)?,
        fingerprint: row.get(10)?,
        discriminator: row.get(11)?,
        safe_summary: row.get(12)?,
        source_agent: row.get(13)?,
        run_id: row.get(14)?,
        attempt_id: row.get(15)?,
        verified_targets: Vec::new(),
        human_targets: Vec::new(),
    })
}

fn load_targets(
    transaction: &Transaction<'_>,
    signal_id: &str,
) -> Result<(Vec<String>, Vec<String>), EvidenceRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT skill_id || '@' || skill_revision, targeting_eligibility FROM evolution_signal_skill_associations WHERE signal_id = ?1 ORDER BY skill_id, skill_revision",
    )?;
    let rows = statement.query_map([signal_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut verified = Vec::new();
    let mut human = Vec::new();
    for row in rows {
        let (target, eligibility) = row?;
        match eligibility.as_str() {
            "automated_consideration" => verified.push(target),
            "human_review_only" => human.push(target),
            _ => {}
        }
    }
    Ok((verified, human))
}

fn replace_seeds(
    transaction: &Transaction<'_>,
    revision: i64,
    projections: &[SeedProjection],
) -> Result<(), EvidenceRepositoryError> {
    transaction.execute("DELETE FROM evolution_candidate_seeds", [])?;
    for seed in projections {
        transaction.execute(
            r#"INSERT INTO evolution_candidate_seeds (
                seed_id, seed_version, grouping_key_hash, workspace, category, readiness,
                readiness_reason, safe_summary, first_occurred_at, last_occurred_at,
                independent_run_count, signal_set_revision, has_recovery, lineage_summary_json,
                lineage_signal_count, lineage_truncated_count, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))"#,
            params![seed.seed_id, i64::from(SEED_BUILDER_V1), seed.grouping_hash, seed.workspace, seed.category, seed.readiness, seed.readiness_reason, seed.safe_summary, seed.first_occurred_at, seed.last_occurred_at, seed.independent_run_count as i64, revision, seed.has_recovery, seed.lineage_json, seed.lineage_signal_count as i64, seed.lineage_truncated_count as i64],
        )?;
        for (order, signal_id) in seed.signal_ids.iter().enumerate() {
            transaction.execute(
                "INSERT INTO evolution_candidate_seed_signals (seed_id, signal_id, lineage_order) VALUES (?1, ?2, ?3)",
                params![seed.seed_id, signal_id, order as i64],
            )?;
        }
    }
    Ok(())
}

fn count_seeds(transaction: &Transaction<'_>) -> Result<usize, EvidenceRepositoryError> {
    let count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM evolution_candidate_seeds",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(count as usize)
}
