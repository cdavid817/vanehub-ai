use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::contexts::personalization::application::{
    CandidateRepository, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    AgentId, ArchiveMemoryCandidate, CandidateReviewStatus, CreateMemoryCandidate, MemoryAudience,
    MemoryCandidate, MemoryCandidateOperation, MemoryId, MemoryProvenance, MemoryScope,
    MemorySource, MemoryType, SessionId, UpdateMemoryCandidate, WorkspaceKey,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| {
            PersonalizationApplicationError::Storage(format!(
                "personalization candidate holds an unreadable timestamp: {error}"
            ))
        })
}

fn review_status_str(status: CandidateReviewStatus) -> &'static str {
    match status {
        CandidateReviewStatus::Pending => "pending",
        CandidateReviewStatus::Approved => "approved",
        CandidateReviewStatus::Rejected => "rejected",
    }
}

fn parse_review_status(value: &str) -> Result<CandidateReviewStatus> {
    match value {
        "pending" => Ok(CandidateReviewStatus::Pending),
        "approved" => Ok(CandidateReviewStatus::Approved),
        "rejected" => Ok(CandidateReviewStatus::Rejected),
        other => Err(PersonalizationApplicationError::Storage(format!(
            "unknown candidate review status {other:?}"
        ))),
    }
}

fn audience_json(audience: &MemoryAudience) -> String {
    match audience {
        MemoryAudience::AllAgents => "\"all_agents\"".to_string(),
        MemoryAudience::SelectedAgents { agent_ids } => {
            let ids: Vec<&str> = agent_ids.iter().map(AgentId::as_str).collect();
            serde_json::json!({ "selected_agents": ids }).to_string()
        }
    }
}

fn parse_audience(value: &str) -> Result<MemoryAudience> {
    if value == "\"all_agents\"" {
        return Ok(MemoryAudience::AllAgents);
    }
    let parsed: serde_json::Value = serde_json::from_str(value).map_err(storage)?;
    let ids = parsed
        .get("selected_agents")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            PersonalizationApplicationError::Storage(
                "candidate audience JSON is not a recognized shape".to_string(),
            )
        })?;
    let mut agent_ids = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.as_str().ok_or_else(|| {
            PersonalizationApplicationError::Storage(
                "candidate audience JSON holds a non-string Agent id".to_string(),
            )
        })?;
        agent_ids.push(AgentId::parse(id)?);
    }
    Ok(MemoryAudience::SelectedAgents { agent_ids })
}

#[derive(Clone)]
pub(crate) struct SqliteCandidateRepository {
    database: NativeDatabase,
}

impl SqliteCandidateRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

const CANDIDATE_COLUMNS: &str = "candidate_id, operation_kind, target_memory_id, \
     expected_target_revision, proposed_name, proposed_description, proposed_memory_type, \
     proposed_content, proposed_scope_kind, proposed_workspace_key, proposed_audience_json, \
     source, source_agent_id, source_session_id, source_message_id, review_status, created_at";

fn read_candidate(row: &Row<'_>) -> rusqlite::Result<Result<MemoryCandidate>> {
    let candidate_id: String = row.get(0)?;
    let operation_kind: String = row.get(1)?;
    let target_memory_id: Option<String> = row.get(2)?;
    let expected_target_revision: Option<i64> = row.get(3)?;
    let proposed_name: Option<String> = row.get(4)?;
    let proposed_description: Option<String> = row.get(5)?;
    let proposed_memory_type: Option<String> = row.get(6)?;
    let proposed_content: Option<String> = row.get(7)?;
    let proposed_scope_kind: Option<String> = row.get(8)?;
    let proposed_workspace_key: Option<String> = row.get(9)?;
    let proposed_audience_json: Option<String> = row.get(10)?;
    let source: String = row.get(11)?;
    let source_agent_id: Option<String> = row.get(12)?;
    let source_session_id: Option<String> = row.get(13)?;
    let source_message_id: Option<String> = row.get(14)?;
    let review_status: String = row.get(15)?;
    let created_at: String = row.get(16)?;

    Ok((|| {
        let target = target_memory_id
            .as_deref()
            .map(MemoryId::parse)
            .transpose()?;
        let expected_revision = expected_target_revision
            .map(|value| u64::try_from(value).unwrap_or_default())
            .unwrap_or_default();

        let operation = match operation_kind.as_str() {
            "create" => {
                let workspace_key = proposed_workspace_key
                    .as_deref()
                    .map(WorkspaceKey::parse)
                    .transpose()?;
                MemoryCandidateOperation::Create(CreateMemoryCandidate {
                    name: proposed_name.unwrap_or_default(),
                    description: proposed_description.unwrap_or_default(),
                    memory_type: MemoryType::parse(
                        proposed_memory_type.as_deref().unwrap_or("untyped"),
                    )?,
                    // A pruned candidate keeps its metadata and loses its body; the empty string
                    // is what a reviewer sees, and the review UI refuses to approve it.
                    content: proposed_content.unwrap_or_default(),
                    scope: MemoryScope::from_parts(
                        proposed_scope_kind.as_deref().unwrap_or("global"),
                        workspace_key.as_ref(),
                    )?,
                    audience: parse_audience(
                        proposed_audience_json
                            .as_deref()
                            .unwrap_or("\"all_agents\""),
                    )?,
                })
            }
            "update" => MemoryCandidateOperation::Update(UpdateMemoryCandidate {
                target_id: target.ok_or_else(|| {
                    PersonalizationApplicationError::Storage(
                        "an update candidate has no target memory id".to_string(),
                    )
                })?,
                expected_target_revision: expected_revision,
                name: proposed_name,
                description: proposed_description,
                content: proposed_content,
            }),
            "archive" => MemoryCandidateOperation::Archive(ArchiveMemoryCandidate {
                target_id: target.ok_or_else(|| {
                    PersonalizationApplicationError::Storage(
                        "an archive candidate has no target memory id".to_string(),
                    )
                })?,
                expected_target_revision: expected_revision,
            }),
            other => {
                return Err(PersonalizationApplicationError::Storage(format!(
                    "unknown candidate operation kind {other:?}"
                )))
            }
        };

        Ok(MemoryCandidate {
            id: MemoryId::parse(&candidate_id)?,
            operation,
            source: MemorySource::parse(&source)?,
            provenance: MemoryProvenance {
                source_agent_id: source_agent_id.as_deref().map(AgentId::parse).transpose()?,
                source_session_id: source_session_id
                    .as_deref()
                    .map(SessionId::parse)
                    .transpose()?,
                source_message_id,
                source_workspace_key: None,
            },
            status: parse_review_status(&review_status)?,
            created_at: parse_timestamp(&created_at)?,
        })
    })())
}

impl CandidateRepository for SqliteCandidateRepository {
    fn insert(&self, candidate: &MemoryCandidate) -> Result<()> {
        let conn = self.connection()?;
        let (name, description, memory_type, content, scope_kind, workspace_key, audience) =
            match &candidate.operation {
                MemoryCandidateOperation::Create(create) => (
                    Some(create.name.clone()),
                    Some(create.description.clone()),
                    Some(create.memory_type.as_str().to_string()),
                    Some(create.content.clone()),
                    Some(create.scope.kind_str().to_string()),
                    create
                        .scope
                        .workspace_key()
                        .map(|key| key.as_str().to_string()),
                    Some(audience_json(&create.audience)),
                ),
                MemoryCandidateOperation::Update(update) => (
                    update.name.clone(),
                    update.description.clone(),
                    None,
                    update.content.clone(),
                    None,
                    None,
                    None,
                ),
                MemoryCandidateOperation::Archive(_) => (None, None, None, None, None, None, None),
            };

        conn.execute(
            "INSERT INTO personalization_memory_candidates (
                 candidate_id, operation_kind, target_memory_id, expected_target_revision,
                 proposed_name, proposed_description, proposed_memory_type, proposed_content,
                 proposed_scope_kind, proposed_workspace_key, proposed_audience_json,
                 source, source_agent_id, source_session_id, source_message_id,
                 review_status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                candidate.id.as_str(),
                candidate.operation.kind_str(),
                candidate.operation.target_id().map(MemoryId::as_str),
                candidate
                    .operation
                    .expected_target_revision()
                    .map(|revision| i64::try_from(revision).unwrap_or(i64::MAX)),
                name,
                description,
                memory_type,
                content,
                scope_kind,
                workspace_key,
                audience,
                candidate.source.as_str(),
                candidate
                    .provenance
                    .source_agent_id
                    .as_ref()
                    .map(AgentId::as_str),
                candidate
                    .provenance
                    .source_session_id
                    .as_ref()
                    .map(SessionId::as_str),
                candidate.provenance.source_message_id.as_deref(),
                review_status_str(candidate.status),
                timestamp(candidate.created_at),
            ],
        )
        .map_err(storage)?;
        Ok(())
    }

    fn get(&self, candidate_id: &MemoryId) -> Result<Option<MemoryCandidate>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {CANDIDATE_COLUMNS} FROM personalization_memory_candidates \
             WHERE candidate_id = ?1"
        );
        let row = conn
            .query_row(&statement, params![candidate_id.as_str()], |row| {
                read_candidate(row)
            })
            .optional()
            .map_err(storage)?;
        row.transpose()
    }

    fn list_pending(&self, limit: usize) -> Result<Vec<MemoryCandidate>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {CANDIDATE_COLUMNS} FROM personalization_memory_candidates \
             WHERE review_status = 'pending' ORDER BY created_at ASC, candidate_id ASC LIMIT ?1"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared
            .query_map(params![i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                read_candidate(row)
            })
            .map_err(storage)?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row.map_err(storage)??);
        }
        Ok(candidates)
    }

    fn count_pending(&self) -> Result<usize> {
        let conn = self.connection()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM personalization_memory_candidates \
                 WHERE review_status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        Ok(usize::try_from(count).unwrap_or_default())
    }

    fn mark_reviewed(
        &self,
        candidate_id: &MemoryId,
        status: CandidateReviewStatus,
        reviewed_at: DateTime<Utc>,
    ) -> Result<()> {
        if matches!(status, CandidateReviewStatus::Pending) {
            return Err(PersonalizationApplicationError::Storage(
                "a review outcome cannot be pending".to_string(),
            ));
        }
        let conn = self.connection()?;
        let changed = conn
            .execute(
                "UPDATE personalization_memory_candidates \
                 SET review_status = ?2, reviewed_at = ?3 WHERE candidate_id = ?1",
                params![
                    candidate_id.as_str(),
                    review_status_str(status),
                    timestamp(reviewed_at)
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(PersonalizationApplicationError::NotFound);
        }
        Ok(())
    }

    fn prune_reviewed(&self, retain: usize) -> Result<usize> {
        let conn = self.connection()?;
        // Drops the proposed body while keeping the audit row. A rejected extraction must not
        // linger as a second copy of text the user declined to keep, but the fact that it was
        // proposed and rejected is what makes a repeat proposal explicable.
        let pruned = conn
            .execute(
                "UPDATE personalization_memory_candidates
                 SET proposed_content = NULL
                 WHERE review_status <> 'pending'
                   AND proposed_content IS NOT NULL
                   AND candidate_id NOT IN (
                       SELECT candidate_id FROM personalization_memory_candidates
                       WHERE review_status <> 'pending'
                       ORDER BY reviewed_at DESC, candidate_id DESC
                       LIMIT ?1
                   )",
                params![i64::try_from(retain).unwrap_or(i64::MAX)],
            )
            .map_err(storage)?;
        Ok(pruned)
    }
}
