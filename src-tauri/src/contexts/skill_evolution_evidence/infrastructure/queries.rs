use std::collections::BTreeMap;

use rusqlite::{params, Row};

use super::{
    EvidenceOverview, EvidencePageRequest, EvidencePipelineSummary, EvidenceQueryScope,
    EvidenceRepositoryError, EvidenceSeedLineage, EvidenceSeedSummary, EvidenceSignalSummary,
    SqliteEvolutionEvidenceRepository, EVIDENCE_RETENTION_DAYS, MAX_EVIDENCE_BYTES,
    MAX_SEEDS_PER_WORKSPACE, MAX_SIGNALS_PER_WORKSPACE,
};

pub(crate) const MAX_EVIDENCE_PAGE_SIZE: usize = 100;

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn query_overview(
        &self,
        request: &EvidencePageRequest,
    ) -> Result<EvidenceOverview, EvidenceRepositoryError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let limit = request.limit.clamp(1, MAX_EVIDENCE_PAGE_SIZE);
        let cursor = request.cursor.as_deref().unwrap_or("");
        let workspace = request.scope.workspace.as_deref();
        let skill = request.scope.skill_id.as_deref();
        let mut statement = connection.prepare(
            r#"SELECT signal.signal_id, signal.source_kind, signal.category, signal.polarity,
                      signal.severity, signal.attribution_strength, signal.attribution_rationale,
                      signal.source_fidelity, signal.source_agent_id, signal.extractor_id,
                      signal.extractor_version, signal.safe_summary, signal.occurred_at, signal.sanitizer_version,
                      signal.association_truncated_count, signal.source_link_truncated_count
               FROM evolution_signals signal
               WHERE signal.lineage_status = 'active'
                 AND (?1 IS NULL OR signal.workspace = ?1)
                 AND (?2 IS NULL OR EXISTS (
                    SELECT 1 FROM evolution_signal_skill_associations association
                    WHERE association.signal_id = signal.signal_id AND association.skill_id = ?2
                 ))
                 AND (?3 = '' OR signal.occurred_at || '|' || signal.signal_id < ?3)
               ORDER BY signal.occurred_at DESC, signal.signal_id DESC LIMIT ?4"#,
        )?;
        let signals = statement
            .query_map(
                params![workspace, skill, cursor, (limit + 1) as i64],
                map_signal,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = (signals.len() > limit).then(|| {
            let signal = &signals[limit - 1];
            format!("{}|{}", signal.occurred_at, signal.signal_id)
        });
        let signals = signals.into_iter().take(limit).collect::<Vec<_>>();
        let signal_count = scoped_signal_count(&connection, workspace, skill)?;
        let seed_count = scoped_seed_count(&connection, workspace, skill)?;
        let (first_occurred_at, last_occurred_at) =
            occurrence_range(&connection, workspace, skill)?;
        let seeds = scoped_seeds(&connection, workspace, skill, limit)?;
        let distributions = distributions(&connection, workspace, skill)?;
        Ok(EvidenceOverview {
            signal_count,
            seed_count,
            first_occurred_at,
            last_occurred_at,
            distributions,
            signals,
            seeds,
            next_cursor,
            pipeline: EvidencePipelineSummary {
                collection_enabled: true,
                status: "healthy".to_string(),
                queue_depth: 0,
                failure_count: pipeline_counter(&connection, "pipeline_failures")?,
            },
            retention_days: EVIDENCE_RETENTION_DAYS,
            signal_quota: MAX_SIGNALS_PER_WORKSPACE,
            seed_quota: MAX_SEEDS_PER_WORKSPACE,
            byte_quota: MAX_EVIDENCE_BYTES,
            dropped_count: pipeline_counter(&connection, "quota_evicted_signals")?,
            expired_count: pipeline_counter(&connection, "expired_signals")?,
        })
    }

    pub(crate) fn query_seed_lineage(
        &self,
        seed_id: &str,
        scope: &EvidenceQueryScope,
    ) -> Result<Option<EvidenceSeedLineage>, EvidenceRepositoryError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let seeds = scoped_seeds_by_id(
            &connection,
            scope.workspace.as_deref(),
            scope.skill_id.as_deref(),
            seed_id,
        )?;
        let Some(seed) = seeds.into_iter().next() else {
            return Ok(None);
        };
        let mut statement = connection.prepare(
            r#"SELECT signal.signal_id, signal.source_kind, signal.category, signal.polarity,
                      signal.severity, signal.attribution_strength, signal.attribution_rationale,
                      signal.source_fidelity, signal.source_agent_id, signal.extractor_id,
                      signal.extractor_version, signal.safe_summary, signal.occurred_at, signal.sanitizer_version,
                      signal.association_truncated_count, signal.source_link_truncated_count
               FROM evolution_candidate_seed_signals link
               JOIN evolution_signals signal ON signal.signal_id = link.signal_id
               WHERE link.seed_id = ?1 ORDER BY link.lineage_order LIMIT 100"#,
        )?;
        let signals = statement
            .query_map([seed_id], map_signal)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(EvidenceSeedLineage { seed, signals }))
    }
}

fn map_signal(row: &Row<'_>) -> rusqlite::Result<EvidenceSignalSummary> {
    Ok(EvidenceSignalSummary {
        signal_id: row.get(0)?,
        source_kind: row.get(1)?,
        category: row.get(2)?,
        polarity: row.get(3)?,
        severity: row.get(4)?,
        attribution: row.get(5)?,
        attribution_rationale: row.get(6)?,
        source_fidelity: row.get(7)?,
        source_agent_id: row.get(8)?,
        extractor_id: row.get(9)?,
        extractor_version: row.get(10)?,
        safe_summary: row.get(11)?,
        occurred_at: row.get(12)?,
        sanitizer_version: row.get(13)?,
        association_truncated_count: row.get::<_, i64>(14)?.max(0) as usize,
        source_link_truncated_count: row.get::<_, i64>(15)?.max(0) as usize,
    })
}

fn scoped_seed_count(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
) -> Result<usize, EvidenceRepositoryError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM evolution_candidate_seeds seed WHERE (?1 IS NULL OR seed.workspace = ?1) AND (?2 IS NULL OR EXISTS (SELECT 1 FROM evolution_candidate_seed_signals link JOIN evolution_signal_skill_associations a ON a.signal_id = link.signal_id WHERE link.seed_id = seed.seed_id AND a.skill_id = ?2))",
        params![workspace, skill], |row| row.get(0))?;
    Ok(count.max(0) as usize)
}

fn occurrence_range(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
) -> Result<(Option<String>, Option<String>), EvidenceRepositoryError> {
    connection.query_row(
        "SELECT MIN(signal.occurred_at), MAX(signal.occurred_at) FROM evolution_signals signal WHERE signal.lineage_status = 'active' AND (?1 IS NULL OR signal.workspace = ?1) AND (?2 IS NULL OR EXISTS (SELECT 1 FROM evolution_signal_skill_associations a WHERE a.signal_id = signal.signal_id AND a.skill_id = ?2))",
        params![workspace, skill], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(Into::into)
}

fn scoped_signal_count(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
) -> Result<usize, EvidenceRepositoryError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM evolution_signals signal WHERE signal.lineage_status = 'active' AND (?1 IS NULL OR signal.workspace = ?1) AND (?2 IS NULL OR EXISTS (SELECT 1 FROM evolution_signal_skill_associations a WHERE a.signal_id = signal.signal_id AND a.skill_id = ?2))",
        params![workspace, skill], |row| row.get(0))?;
    Ok(count.max(0) as usize)
}

fn scoped_seeds(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
    limit: usize,
) -> Result<Vec<EvidenceSeedSummary>, EvidenceRepositoryError> {
    scoped_seeds_query(connection, workspace, skill, None, limit)
}

fn scoped_seeds_by_id(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
    seed_id: &str,
) -> Result<Vec<EvidenceSeedSummary>, EvidenceRepositoryError> {
    scoped_seeds_query(connection, workspace, skill, Some(seed_id), 1)
}

fn scoped_seeds_query(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
    seed_id: Option<&str>,
    limit: usize,
) -> Result<Vec<EvidenceSeedSummary>, EvidenceRepositoryError> {
    let mut statement = connection.prepare(
        r#"SELECT seed.seed_id, seed.category, seed.readiness, seed.readiness_reason,
                  seed.safe_summary, seed.lineage_signal_count, seed.lineage_truncated_count,
                  seed.independent_run_count, seed.has_recovery, seed.first_occurred_at,
                  seed.last_occurred_at, seed.created_at FROM evolution_candidate_seeds seed
           WHERE (?1 IS NULL OR seed.workspace = ?1) AND (?2 IS NULL OR EXISTS (
              SELECT 1 FROM evolution_candidate_seed_signals link
              JOIN evolution_signal_skill_associations a ON a.signal_id = link.signal_id
              WHERE link.seed_id = seed.seed_id AND a.skill_id = ?2
           )) AND (?3 IS NULL OR seed.seed_id = ?3)
           ORDER BY seed.created_at DESC, seed.seed_id DESC LIMIT ?4"#,
    )?;
    let values = statement
        .query_map(params![workspace, skill, seed_id, limit as i64], |row| {
            Ok(EvidenceSeedSummary {
                seed_id: row.get(0)?,
                category: row.get(1)?,
                readiness: row.get(2)?,
                readiness_reason: row.get(3)?,
                safe_summary: row.get(4)?,
                signal_count: row.get::<_, i64>(5)?.max(0) as usize,
                truncated_signal_count: row.get::<_, i64>(6)?.max(0) as usize,
                independent_run_count: row.get::<_, i64>(7)?.max(0) as usize,
                has_recovery: row.get::<_, i64>(8)? != 0,
                first_occurred_at: row.get(9)?,
                last_occurred_at: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values)
}

fn distributions(
    connection: &rusqlite::Connection,
    workspace: Option<&str>,
    skill: Option<&str>,
) -> Result<BTreeMap<String, BTreeMap<String, usize>>, EvidenceRepositoryError> {
    let mut result = BTreeMap::new();
    for field in [
        "category",
        "polarity",
        "severity",
        "attribution_strength",
        "source_agent_id",
        "extractor_id",
        "source_kind",
        "source_fidelity",
    ] {
        let sql = format!("SELECT COALESCE(signal.{field}, 'unknown'), COUNT(*) FROM evolution_signals signal WHERE signal.lineage_status = 'active' AND (?1 IS NULL OR signal.workspace = ?1) AND (?2 IS NULL OR EXISTS (SELECT 1 FROM evolution_signal_skill_associations a WHERE a.signal_id = signal.signal_id AND a.skill_id = ?2)) GROUP BY signal.{field} ORDER BY signal.{field}");
        let mut statement = connection.prepare(&sql)?;
        let values = statement
            .query_map(params![workspace, skill], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        result.insert(
            field.to_string(),
            values
                .into_iter()
                .map(|(key, value)| (key, value.max(0) as usize))
                .collect(),
        );
    }
    let mut revisions = connection.prepare(
        "SELECT association.skill_revision, COUNT(DISTINCT association.signal_id) FROM evolution_signal_skill_associations association JOIN evolution_signals signal ON signal.signal_id = association.signal_id WHERE signal.lineage_status = 'active' AND (?1 IS NULL OR signal.workspace = ?1) AND (?2 IS NULL OR association.skill_id = ?2) GROUP BY association.skill_revision ORDER BY association.skill_revision",
    )?;
    let values = revisions
        .query_map(params![workspace, skill], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    result.insert(
        "skill_revision".to_string(),
        values
            .into_iter()
            .map(|(key, value)| (key, value.max(0) as usize))
            .collect(),
    );
    Ok(result)
}

fn pipeline_counter(
    connection: &rusqlite::Connection,
    key: &str,
) -> Result<usize, EvidenceRepositoryError> {
    let count = connection
        .query_row(
            "SELECT revision FROM evolution_pipeline_state WHERE state_key = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);
    Ok(count.max(0) as usize)
}
