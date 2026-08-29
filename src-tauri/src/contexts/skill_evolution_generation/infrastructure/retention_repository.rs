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
    for job_id in job_ids {
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO evolution_generation_governance_tombstones
             (tombstone_id,job_id,package_hash,artifact_hash,validation_report_hash,
              curator_candidate_id,final_status,source_purge_witness_hash,created_at_ms)
             SELECT 'tombstone:' || h.handoff_id,h.job_id,h.package_hash,d.content_hash,
                    v.report_hash,h.curator_candidate_id,h.status,?2,?3
             FROM evolution_generation_handoffs h
             JOIN evolution_generation_validations v ON v.validation_id=h.validation_id
             JOIN evolution_generated_drafts d ON d.draft_id=v.draft_id AND d.generation_attempt=v.draft_attempt
             WHERE h.job_id=?1 AND h.status IN ('delivered','duplicate')",
            params![job_id, witness_hash, now_ms],
        ).map_err(|_| GenerationPersistenceError::Storage)?;
        result.retained_tombstones += inserted as u64;
        transaction
            .execute(
                "DELETE FROM evolution_generation_handoffs WHERE job_id=?1",
                [job_id],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        result.removed_jobs += transaction
            .execute(
                "DELETE FROM evolution_generation_jobs WHERE job_id=?1",
                [job_id],
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
    for dossier_id in dossier_ids {
        let exports = transaction
            .execute(
                "DELETE FROM evolution_generation_exports WHERE dossier_id=?1",
                [dossier_id],
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        result.removed_export_manifests += exports as u64;
        result.exported_files_remain_user_managed |= exports > 0;
        result.removed_dossiers += transaction
            .execute(
                "DELETE FROM evolution_evidence_dossiers WHERE dossier_id=?1",
                [dossier_id],
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
