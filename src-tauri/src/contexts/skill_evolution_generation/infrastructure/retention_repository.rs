use crate::contexts::skill_evolution_generation::application::GenerationRetentionCutoffsV1;
use rusqlite::{params, Connection, Transaction};

use super::GenerationPersistenceError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GenerationPurgeResultV1 {
    pub(crate) removed_jobs: u64,
    pub(crate) removed_dossiers: u64,
    pub(crate) retained_tombstones: u64,
    pub(crate) removed_export_manifests: u64,
    pub(crate) exported_files_remain_user_managed: bool,
}

pub(crate) struct GenerationRetentionRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationRetentionRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn purge_source_evidence(
        &self,
        source_id: &str,
        source_revision: Option<&str>,
        purge_witness_hash: &str,
        now_ms: i64,
    ) -> Result<GenerationPurgeResultV1, GenerationPersistenceError> {
        if source_id.trim().is_empty() || purge_witness_hash.trim().is_empty() || now_ms < 0 {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let jobs = source_job_ids(&transaction, source_id, source_revision)?;
        let dossiers = source_dossier_ids(&transaction, source_id, source_revision)?;
        let mut result = purge_jobs(&transaction, &jobs, purge_witness_hash, now_ms)?;
        purge_dossiers(&transaction, &dossiers, &mut result)?;
        transaction
            .commit()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        Ok(result)
    }

    pub(crate) fn apply_retention(
        &self,
        cutoffs: GenerationRetentionCutoffsV1,
        now_ms: i64,
    ) -> Result<GenerationPurgeResultV1, GenerationPersistenceError> {
        if now_ms < 0 {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let jobs = retention_job_ids(&transaction, cutoffs)?;
        let dossiers = job_dossier_ids(&transaction, &jobs)?;
        let mut result = purge_jobs(
            &transaction,
            &jobs,
            "sha256:generation-retention-policy-v1",
            now_ms,
        )?;
        purge_dossiers(&transaction, &dossiers, &mut result)?;
        transaction
            .commit()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        Ok(result)
    }
}

fn source_job_ids(
    transaction: &Transaction<'_>,
    source_id: &str,
    revision: Option<&str>,
) -> Result<Vec<String>, GenerationPersistenceError> {
    collect_ids(
        transaction,
        "SELECT DISTINCT job_id FROM evolution_generation_job_sources
         WHERE source_id=?1 AND (?2 IS NULL OR source_revision=?2) ORDER BY job_id",
        params![source_id, revision],
    )
}

fn source_dossier_ids(
    transaction: &Transaction<'_>,
    source_id: &str,
    revision: Option<&str>,
) -> Result<Vec<String>, GenerationPersistenceError> {
    collect_ids(
        transaction,
        "SELECT DISTINCT dossier_id FROM evolution_evidence_dossier_links
         WHERE link_kind='evidence' AND linked_id=?1 AND (?2 IS NULL OR linked_revision=?2)
         ORDER BY dossier_id",
        params![source_id, revision],
    )
}

fn retention_job_ids(
    transaction: &Transaction<'_>,
    cutoffs: GenerationRetentionCutoffsV1,
) -> Result<Vec<String>, GenerationPersistenceError> {
    collect_ids(
        transaction,
        "SELECT job_id FROM evolution_generation_jobs
         WHERE (status IN ('failed','cancelled') AND updated_at_ms < ?1)
            OR (status='completed' AND updated_at_ms < ?2)
         ORDER BY job_id",
        params![
            cutoffs.failed_cancelled_before_ms,
            cutoffs.completed_package_before_ms
        ],
    )
}

fn job_dossier_ids(
    transaction: &Transaction<'_>,
    job_ids: &[String],
) -> Result<Vec<String>, GenerationPersistenceError> {
    let mut dossiers = Vec::new();
    for job_id in job_ids {
        let mut values = collect_ids(
            transaction,
            "SELECT dossier_id FROM evolution_evidence_dossier_links
             WHERE link_kind='job' AND linked_id=?1 ORDER BY dossier_id",
            [job_id],
        )?;
        dossiers.append(&mut values);
    }
    dossiers.sort();
    dossiers.dedup();
    Ok(dossiers)
}

fn purge_jobs(
    transaction: &Transaction<'_>,
    job_ids: &[String],
    witness_hash: &str,
    now_ms: i64,
) -> Result<GenerationPurgeResultV1, GenerationPersistenceError> {
    let mut result = GenerationPurgeResultV1::default();
    // Set-based per chunk rather than one statement per job: a retention sweep can select many
    // jobs, and per-row DML turns it into an N+1 storm.
    for chunk in job_ids.chunks(IN_CHUNK) {
        let plain = placeholders_from(0, chunk.len());
        let ids: Vec<&dyn rusqlite::ToSql> =
            chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let shifted = placeholders_from(2, chunk.len());
        let mut tombstone_values: Vec<&dyn rusqlite::ToSql> = vec![&witness_hash, &now_ms];
        tombstone_values.extend(chunk.iter().map(|id| id as &dyn rusqlite::ToSql));
        let inserted = transaction.execute(
            &format!(
                "INSERT OR IGNORE INTO evolution_generation_governance_tombstones
                 (tombstone_id,job_id,package_hash,artifact_hash,validation_report_hash,
                  curator_candidate_id,final_status,source_purge_witness_hash,created_at_ms)
                 SELECT 'tombstone:' || h.handoff_id,h.job_id,h.package_hash,d.content_hash,
                        v.report_hash,h.curator_candidate_id,h.status,?1,?2
                 FROM evolution_generation_handoffs h
                 JOIN evolution_generation_validations v ON v.validation_id=h.validation_id
                 JOIN evolution_generated_drafts d ON d.draft_id=v.draft_id AND d.generation_attempt=v.draft_attempt
                 WHERE h.status IN ('delivered','duplicate') AND h.job_id IN ({shifted})"
            ),
            &tombstone_values[..],
        ).map_err(|_| GenerationPersistenceError::Storage)?;
        result.retained_tombstones += inserted as u64;
        transaction
            .execute(
                &format!("DELETE FROM evolution_generation_handoffs WHERE job_id IN ({plain})"),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        // Quarantine rows and the supersession self-references have no ON DELETE clause, so they
        // must be removed or detached first or the job DELETE aborts the whole purge transaction
        // on the foreign key — wedging retention and privacy purges permanently.
        transaction
            .execute(
                &format!(
                    "DELETE FROM evolution_generated_skill_quarantine WHERE job_id IN ({plain})"
                ),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        transaction
            .execute(
                &format!(
                    "UPDATE evolution_generation_jobs SET supersedes_job_id=NULL
                     WHERE supersedes_job_id IN ({plain})"
                ),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        transaction
            .execute(
                &format!(
                    "UPDATE evolution_generation_jobs SET superseded_by_job_id=NULL
                     WHERE superseded_by_job_id IN ({plain})"
                ),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        result.removed_jobs += transaction
            .execute(
                &format!("DELETE FROM evolution_generation_jobs WHERE job_id IN ({plain})"),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?
            as u64;
    }
    Ok(result)
}

fn purge_dossiers(
    transaction: &Transaction<'_>,
    dossier_ids: &[String],
    result: &mut GenerationPurgeResultV1,
) -> Result<(), GenerationPersistenceError> {
    for chunk in dossier_ids.chunks(IN_CHUNK) {
        let plain = placeholders_from(0, chunk.len());
        let ids: Vec<&dyn rusqlite::ToSql> =
            chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let exports = transaction
            .execute(
                &format!("DELETE FROM evolution_generation_exports WHERE dossier_id IN ({plain})"),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        result.removed_export_manifests += exports as u64;
        result.exported_files_remain_user_managed |= exports > 0;
        result.removed_dossiers += transaction
            .execute(
                &format!("DELETE FROM evolution_evidence_dossiers WHERE dossier_id IN ({plain})"),
                &ids[..],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?
            as u64;
    }
    Ok(())
}

fn collect_ids<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    sql: &str,
    params: P,
) -> Result<Vec<String>, GenerationPersistenceError> {
    let mut statement = transaction
        .prepare(sql)
        .map_err(|_| GenerationPersistenceError::Storage)?;
    let values = statement
        .query_map(params, |row| row.get(0))
        .map_err(|_| GenerationPersistenceError::Storage)?
        .collect::<Result<_, _>>()
        .map_err(|_| GenerationPersistenceError::Storage)?;
    Ok(values)
}

/// Chunked IN-lists keep statements bounded and plans cacheable however many rows a sweep selects.
const IN_CHUNK: usize = 500;

fn placeholders_from(preceding: usize, count: usize) -> String {
    (0..count)
        .map(|index| format!("?{}", preceding + index + 1))
        .collect::<Vec<_>>()
        .join(",")
}
