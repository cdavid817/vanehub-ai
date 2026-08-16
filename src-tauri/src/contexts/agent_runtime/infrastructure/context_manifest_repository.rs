use crate::contexts::agent_runtime::application::ContextManifestRepository;
use crate::contexts::agent_runtime::domain::{
    ContextEvidenceManifest, ContextEvidenceManifestPage, ContextEvidenceSummary, ContextRange,
    ContextReasonCode, ContextSourceKind, ContextSourceOutcome,
};
use crate::platform::database::{DatabaseError, NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_PAGE_SIZE: u32 = 100;
const HARD_LIMIT: i64 = 5_000;

#[derive(Clone)]
pub(crate) struct SqliteContextManifestRepository {
    database: NativeDatabase,
}

impl SqliteContextManifestRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

impl ContextManifestRepository for SqliteContextManifestRepository {
    fn save(&self, manifest: &ContextEvidenceManifest) -> Result<(), String> {
        validate_manifest(manifest)?;
        let selected = manifest
            .selected
            .iter()
            .map(StoredEvidence::from)
            .collect::<Vec<_>>();
        let rejected = manifest
            .rejected
            .iter()
            .map(|(id, reason)| (id, reason.as_str()))
            .collect::<Vec<_>>();
        let outcomes = manifest
            .source_outcomes
            .iter()
            .map(|(kind, outcome)| (kind.as_str(), outcome.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                r#"INSERT OR REPLACE INTO context_evidence_manifests (
                    generation_id, session_id, turn_id, recorded_at, policy_version,
                    evidence_budget, occupied_tokens, selected_json, rejected_json,
                    source_outcomes_json, duplicate_tokens_saved, collection_latency_bucket,
                    ranking_latency_bucket, compaction_triggered
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
                params![
                    manifest.generation_id,
                    manifest.session_id,
                    manifest.turn_id,
                    manifest.recorded_at,
                    manifest.policy_version,
                    to_i64(manifest.evidence_budget)?,
                    to_i64(manifest.occupied_tokens)?,
                    serde_json::to_string(&selected).map_err(storage_error)?,
                    serde_json::to_string(&rejected).map_err(storage_error)?,
                    serde_json::to_string(&outcomes).map_err(storage_error)?,
                    to_i64(manifest.duplicate_tokens_saved)?,
                    manifest.collection_latency_bucket,
                    manifest.ranking_latency_bucket,
                    manifest.compaction_triggered,
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"DELETE FROM context_evidence_manifests WHERE generation_id IN (
                    SELECT generation_id FROM context_evidence_manifests
                    ORDER BY recorded_at DESC, generation_id DESC LIMIT -1 OFFSET ?1
                )"#,
                [HARD_LIMIT],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn list(
        &self,
        session_id: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<ContextEvidenceManifestPage, String> {
        if !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err("invalid context manifest page size".to_string());
        }
        let connection = self.connection()?;
        let cursor_key = cursor
            .map(|id| {
                connection
                    .query_row(
                        "SELECT recorded_at, generation_id FROM context_evidence_manifests WHERE generation_id = ?1",
                        [id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or_else(|| "invalid context manifest cursor".to_string())
            })
            .transpose()?;
        let query_limit = i64::from(limit) + 1;
        let mut statement = connection.prepare(&format!(
            "{} WHERE (?1 IS NULL OR session_id = ?1) AND (?2 IS NULL OR recorded_at < ?2 OR (recorded_at = ?2 AND generation_id < ?3)) ORDER BY recorded_at DESC, generation_id DESC LIMIT ?4",
            manifest_select()
        )).map_err(storage_error)?;
        let (cursor_at, cursor_id) = cursor_key
            .map(|(at, id)| (Some(at), Some(id)))
            .unwrap_or((None, None));
        let rows = statement
            .query_map(
                params![session_id, cursor_at, cursor_id, query_limit],
                StoredManifest::read,
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let mut items = rows
            .into_iter()
            .map(StoredManifest::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor =
            (items.len() > limit as usize).then(|| items[limit as usize - 1].generation_id.clone());
        items.truncate(limit as usize);
        Ok(ContextEvidenceManifestPage { items, next_cursor })
    }

    fn get(&self, generation_id: &str) -> Result<Option<ContextEvidenceManifest>, String> {
        if generation_id.trim().is_empty() {
            return Err("generation id is required".to_string());
        }
        self.connection()?
            .query_row(
                &format!("{} WHERE generation_id = ?1", manifest_select()),
                [generation_id],
                StoredManifest::read,
            )
            .optional()
            .map_err(storage_error)?
            .map(StoredManifest::into_domain)
            .transpose()
    }
}

pub(crate) fn apply_context_manifest_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"CREATE TABLE IF NOT EXISTS context_evidence_manifests (
            generation_id TEXT PRIMARY KEY NOT NULL,
            session_id TEXT NOT NULL, turn_id TEXT NOT NULL, recorded_at TEXT NOT NULL,
            policy_version TEXT NOT NULL, evidence_budget INTEGER NOT NULL CHECK(evidence_budget >= 0),
            occupied_tokens INTEGER NOT NULL CHECK(occupied_tokens >= 0), selected_json TEXT NOT NULL,
            rejected_json TEXT NOT NULL, source_outcomes_json TEXT NOT NULL,
            duplicate_tokens_saved INTEGER NOT NULL CHECK(duplicate_tokens_saved >= 0),
            collection_latency_bucket TEXT NOT NULL, ranking_latency_bucket TEXT NOT NULL,
            compaction_triggered INTEGER NOT NULL CHECK(compaction_triggered IN (0, 1))
        );
        CREATE INDEX IF NOT EXISTS idx_context_evidence_session_recorded
        ON context_evidence_manifests(session_id, recorded_at DESC, generation_id DESC);"#,
    )?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct StoredEvidence {
    id: String,
    source_kind: String,
    source_ref: String,
    start_line: Option<u32>,
    end_line: Option<u32>,
    symbol: Option<String>,
    token_estimate: u64,
    safe_fingerprint: String,
    reasons: Vec<String>,
}

impl From<&ContextEvidenceSummary> for StoredEvidence {
    fn from(value: &ContextEvidenceSummary) -> Self {
        Self {
            id: value.id.clone(),
            source_kind: value.source_kind.as_str().to_string(),
            source_ref: value.source_ref.clone(),
            start_line: value.range.map(|range| range.start_line),
            end_line: value.range.map(|range| range.end_line),
            symbol: value.symbol.clone(),
            token_estimate: value.token_estimate,
            safe_fingerprint: value.safe_fingerprint.clone(),
            reasons: value
                .reasons
                .iter()
                .map(|reason| reason.as_str().to_string())
                .collect(),
        }
    }
}

struct StoredManifest {
    generation_id: String,
    session_id: String,
    turn_id: String,
    recorded_at: String,
    policy_version: String,
    evidence_budget: i64,
    occupied_tokens: i64,
    selected_json: String,
    rejected_json: String,
    outcomes_json: String,
    duplicate_tokens_saved: i64,
    collection_latency_bucket: String,
    ranking_latency_bucket: String,
    compaction_triggered: bool,
}

impl StoredManifest {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            generation_id: row.get(0)?,
            session_id: row.get(1)?,
            turn_id: row.get(2)?,
            recorded_at: row.get(3)?,
            policy_version: row.get(4)?,
            evidence_budget: row.get(5)?,
            occupied_tokens: row.get(6)?,
            selected_json: row.get(7)?,
            rejected_json: row.get(8)?,
            outcomes_json: row.get(9)?,
            duplicate_tokens_saved: row.get(10)?,
            collection_latency_bucket: row.get(11)?,
            ranking_latency_bucket: row.get(12)?,
            compaction_triggered: row.get(13)?,
        })
    }

    fn into_domain(self) -> Result<ContextEvidenceManifest, String> {
        let selected = serde_json::from_str::<Vec<StoredEvidence>>(&self.selected_json)
            .map_err(storage_error)?
            .into_iter()
            .map(StoredEvidence::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let rejected = serde_json::from_str::<Vec<(String, String)>>(&self.rejected_json)
            .map_err(storage_error)?
            .into_iter()
            .map(|(id, reason)| {
                ContextReasonCode::parse(&reason)
                    .map(|reason| (id, reason))
                    .ok_or_else(|| "invalid persisted context reason".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let outcomes = serde_json::from_str::<BTreeMap<String, String>>(&self.outcomes_json)
            .map_err(storage_error)?
            .into_iter()
            .map(|(kind, outcome)| -> Result<_, String> {
                Ok((
                    ContextSourceKind::parse(&kind)
                        .ok_or_else(|| "invalid persisted context source".to_string())?,
                    ContextSourceOutcome::parse(&outcome)
                        .ok_or_else(|| "invalid persisted context outcome".to_string())?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(ContextEvidenceManifest {
            session_id: self.session_id,
            turn_id: self.turn_id,
            generation_id: self.generation_id,
            recorded_at: self.recorded_at,
            policy_version: self.policy_version,
            evidence_budget: from_i64(self.evidence_budget)?,
            occupied_tokens: from_i64(self.occupied_tokens)?,
            selected,
            rejected,
            source_outcomes: outcomes,
            duplicate_tokens_saved: from_i64(self.duplicate_tokens_saved)?,
            collection_latency_bucket: self.collection_latency_bucket,
            ranking_latency_bucket: self.ranking_latency_bucket,
            compaction_triggered: self.compaction_triggered,
        })
    }
}

impl StoredEvidence {
    fn into_domain(self) -> Result<ContextEvidenceSummary, String> {
        let range = match (self.start_line, self.end_line) {
            (Some(start), Some(end)) => ContextRange::new(start, end),
            (None, None) => None,
            _ => return Err("invalid persisted context range".to_string()),
        };
        Ok(ContextEvidenceSummary {
            id: self.id,
            source_kind: ContextSourceKind::parse(&self.source_kind)
                .ok_or_else(|| "invalid persisted context source".to_string())?,
            source_ref: self.source_ref,
            range,
            symbol: self.symbol,
            token_estimate: self.token_estimate,
            safe_fingerprint: self.safe_fingerprint,
            reasons: self
                .reasons
                .into_iter()
                .map(|reason| {
                    ContextReasonCode::parse(&reason)
                        .ok_or_else(|| "invalid persisted context reason".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

fn manifest_select() -> &'static str {
    "SELECT generation_id, session_id, turn_id, recorded_at, policy_version, evidence_budget, occupied_tokens, selected_json, rejected_json, source_outcomes_json, duplicate_tokens_saved, collection_latency_bucket, ranking_latency_bucket, compaction_triggered FROM context_evidence_manifests"
}

fn validate_manifest(manifest: &ContextEvidenceManifest) -> Result<(), String> {
    if manifest.generation_id.trim().is_empty()
        || manifest.session_id.trim().is_empty()
        || manifest.turn_id.trim().is_empty()
        || manifest.selected.len() > 64
        || manifest.rejected.len() > 16
        || manifest.occupied_tokens > manifest.evidence_budget
    {
        return Err("invalid context evidence manifest".to_string());
    }
    Ok(())
}

fn to_i64(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "context manifest numeric overflow".to_string())
}
fn from_i64(value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "invalid persisted context numeric value".to_string())
}
fn storage_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "context_manifest_repository_tests.rs"]
mod tests;
