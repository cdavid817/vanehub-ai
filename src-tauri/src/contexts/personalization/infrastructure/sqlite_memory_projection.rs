use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{types::Value, Connection, Row, ToSql};

use sha2::{Digest, Sha256};

use crate::contexts::personalization::application::{
    AuthorizedMemoryRelation, EligibilityEnumerationBudget, MemoryEligibilityCriteria,
    MemoryProjectionPort, PersonalizationApplicationError, ResetCounts,
};
use crate::contexts::personalization::domain::{
    authority_fingerprint, AgentId, LegacyMemorySaveSource, MemoryAudience, MemoryCursor,
    MemoryEligibilitySummary, MemoryExclusionCount, MemoryId, MemoryOrder, MemoryPage, MemoryQuery,
    MemoryReadHandle, MemoryRecord, MemoryScope, MemoryScopeFilter, MemorySource, MemoryStatus,
    MemorySummary, MemoryType, PersonalizationExclusionReason, SnapshotMemoryRef, WorkspaceKey,
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
                "personalization projection holds an unreadable timestamp: {error}"
            ))
        })
}

/// The audience is stored as JSON because it is a list whose only consumer is the application
/// layer; giving it a join table would add a second write path to keep consistent with the
/// authoritative Markdown file for no query benefit.
fn audience_json(audience: &MemoryAudience) -> String {
    match audience {
        MemoryAudience::AllAgents => "\"all_agents\"".to_string(),
        MemoryAudience::SelectedAgents { agent_ids } => {
            let ids: Vec<&str> = agent_ids.iter().map(AgentId::as_str).collect();
            serde_json::json!({ "selected_agents": ids }).to_string()
        }
    }
}

fn audience_is_restricted(json: &str) -> bool {
    json != "\"all_agents\""
}

/// The inverse of `audience_json`, strict: anything but the two shapes this projection writes is
/// an error, never a guess at "all agents".
fn parse_audience_json(json: &str) -> Result<MemoryAudience> {
    if json == "\"all_agents\"" {
        return Ok(MemoryAudience::AllAgents);
    }
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| {
        PersonalizationApplicationError::Storage(format!(
            "personalization projection holds an unreadable audience: {error}"
        ))
    })?;
    let ids = value
        .get("selected_agents")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            PersonalizationApplicationError::Storage(
                "personalization projection holds an audience of unknown shape".to_string(),
            )
        })?;
    let agent_ids = ids
        .iter()
        .map(|id| {
            id.as_str()
                .ok_or_else(|| {
                    PersonalizationApplicationError::Storage(
                        "personalization projection holds a non-text audience member".to_string(),
                    )
                })
                .and_then(|id| AgentId::parse(id).map_err(PersonalizationApplicationError::from))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(MemoryAudience::SelectedAgents { agent_ids })
}

/// The one eligibility expression every governed query shares.
///
/// Bound parameters, in order: `?1` the stable Agent id, `?2` whether global scope is allowed,
/// `?3` whether the session is project-only, `?4` the one admitted workspace key or NULL.
///
/// Rows this build cannot classify -- an unknown status or scope kind, inconsistent workspace
/// columns, or an audience that is not one of the two shapes this projection writes -- come out
/// as `invalid_record` before any policy branch, so they can never fall through to `eligible`.
/// Audience membership is an exact `json_each` equality under BINARY collation: no LIKE, no
/// substring, no case folding, so `agent-1` never admits `agent-10` and `Agent` never admits
/// `agent`.
/// One keyset page of the complete eligibility relation. Params: ?1 agent, ?2 allow_global,
/// ?3 project_only, ?4 workspace, ?5 the last id of the previous page, ?6 the page size.
fn eligibility_page_statement() -> String {
    format!(
        "SELECT memory_id, revision, content_hash, status, scope_kind, workspace_key, \
                audience_json \
         FROM personalization_memory_projection \
         WHERE ({ELIGIBILITY_CLASSIFICATION}) = 'eligible' AND memory_id > ?5 \
         ORDER BY memory_id ASC LIMIT ?6"
    )
}

#[cfg(test)]
impl SqliteMemoryProjection {
    /// The planner's answer for one relation page, for the scale measurement notes.
    pub(crate) fn explain_eligibility_page(
        &self,
        criteria: &MemoryEligibilityCriteria,
        page_size: usize,
    ) -> Result<Vec<String>> {
        let conn = self.connection()?;
        let statement = format!("EXPLAIN QUERY PLAN {}", eligibility_page_statement());
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared
            .query_map(
                rusqlite::params![
                    criteria.agent_id.as_str(),
                    i64::from(criteria.allow_global),
                    i64::from(criteria.project_only),
                    criteria
                        .workspace
                        .as_ref()
                        .map(|key| key.as_str().to_string()),
                    "",
                    i64::try_from(page_size).unwrap_or(i64::MAX),
                ],
                |row| row.get::<_, String>(3),
            )
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)
    }
}

const ELIGIBILITY_CLASSIFICATION: &str = "\
    CASE \
      WHEN status NOT IN ('candidate', 'active', 'archived') THEN 'invalid_record' \
      WHEN scope_kind NOT IN ('global', 'workspace') THEN 'invalid_record' \
      WHEN scope_kind = 'global' AND workspace_key IS NOT NULL THEN 'invalid_record' \
      WHEN scope_kind = 'workspace' AND workspace_key IS NULL THEN 'invalid_record' \
      WHEN audience_json <> '\"all_agents\"' \
        AND NOT (json_valid(audience_json) \
                 AND json_type(audience_json, '$.selected_agents') = 'array') \
        THEN 'invalid_record' \
      WHEN status = 'candidate' THEN 'pending_candidate' \
      WHEN status <> 'active' THEN 'archived' \
      WHEN scope_kind = 'global' AND ?3 = 1 THEN 'project_only_session' \
      WHEN scope_kind = 'global' AND ?2 = 0 THEN 'global_memory_disabled' \
      WHEN scope_kind = 'workspace' AND (?4 IS NULL OR workspace_key IS NOT ?4) \
        THEN 'other_workspace' \
      WHEN audience_json <> '\"all_agents\"' \
        AND NOT EXISTS (SELECT 1 FROM json_each(audience_json, '$.selected_agents') \
                        WHERE json_each.type = 'text' AND json_each.value = ?1) \
        THEN 'agent_audience' \
      ELSE 'eligible' \
    END";

fn exclusion_reason(outcome: &str) -> Option<PersonalizationExclusionReason> {
    Some(match outcome {
        "pending_candidate" => PersonalizationExclusionReason::PendingCandidate,
        "archived" => PersonalizationExclusionReason::Archived,
        "invalid_record" => PersonalizationExclusionReason::InvalidRecord,
        "project_only_session" => PersonalizationExclusionReason::ProjectOnlySession,
        "global_memory_disabled" => PersonalizationExclusionReason::GlobalMemoryDisabled,
        "other_workspace" => PersonalizationExclusionReason::OtherWorkspace,
        "agent_audience" => PersonalizationExclusionReason::AgentAudience,
        _ => return None,
    })
}

fn read_snapshot_ref(row: &Row<'_>) -> rusqlite::Result<Result<SnapshotMemoryRef>> {
    let memory_id: String = row.get(0)?;
    let revision: i64 = row.get(1)?;
    let content_hash: String = row.get(2)?;
    let name: String = row.get(3)?;
    let description: String = row.get(4)?;
    let memory_type: String = row.get(5)?;
    let scope_kind: String = row.get(6)?;
    let workspace_key: Option<String> = row.get(7)?;
    let updated_at: String = row.get(8)?;
    let status: String = row.get(9)?;
    let audience: String = row.get(10)?;

    Ok((|| {
        Ok(SnapshotMemoryRef {
            id: MemoryId::parse(&memory_id)?,
            revision: u64::try_from(revision).unwrap_or_default(),
            content_hash,
            authority_fingerprint: row_authority_fingerprint(
                &status,
                &scope_kind,
                workspace_key.as_deref(),
                &audience,
            )?,
            name,
            description,
            memory_type: MemoryType::parse(&memory_type)?,
            // A hint for grouping and display, never an authorization input: eligibility was
            // decided by the query that produced this row.
            scope_hint: workspace_key.unwrap_or(scope_kind),
            updated_at: parse_timestamp(&updated_at)?,
        })
    })())
}

/// The authority digest for one projected row, computed from the same columns the domain reads.
fn row_authority_fingerprint(
    status: &str,
    scope_kind: &str,
    workspace_key: Option<&str>,
    audience_json: &str,
) -> Result<String> {
    let workspace_key = workspace_key.map(WorkspaceKey::parse).transpose()?;
    let scope = MemoryScope::from_parts(scope_kind, workspace_key.as_ref())?;
    Ok(authority_fingerprint(
        &MemoryStatus::parse(status)?,
        &scope,
        &parse_audience_json(audience_json)?,
    ))
}

/// A digest over what was eligible, in a fixed order.
///
/// Ids and revisions only. A memory edited between two generations changes its revision and
/// therefore this digest, which is what makes the next snapshot token differ; its text never
/// enters, because this value reaches diagnostics.
fn eligibility_digest(refs: &[SnapshotMemoryRef]) -> String {
    let mut ordered: Vec<String> = refs
        .iter()
        .map(|entry| format!("{}@{}", entry.id.as_str(), entry.revision))
        .collect();
    ordered.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"personalization-eligibility-v1");
    for entry in ordered {
        hasher.update(b"\x1f");
        hasher.update(entry.as_bytes());
    }
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Clone)]
pub(crate) struct SqliteMemoryProjection {
    database: NativeDatabase,
}

impl SqliteMemoryProjection {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

/// Every column a summary needs, and nothing else.
///
/// `content` is not among them and there is no join that could add it: the projection exists so a
/// list page never touches a memory body.
const SUMMARY_COLUMNS: &str = "memory_id, name, description, memory_type, scope_kind, \
     workspace_key, audience_json, status, source, source_agent_id, revision, updated_at";

fn read_summary(row: &Row<'_>) -> rusqlite::Result<Result<MemorySummary>> {
    let memory_id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: String = row.get(2)?;
    let memory_type: String = row.get(3)?;
    let scope_kind: String = row.get(4)?;
    let workspace_key: Option<String> = row.get(5)?;
    let audience: String = row.get(6)?;
    let status: String = row.get(7)?;
    let source: String = row.get(8)?;
    let source_agent_id: Option<String> = row.get(9)?;
    let revision: i64 = row.get(10)?;
    let updated_at: String = row.get(11)?;

    Ok((|| {
        Ok(MemorySummary {
            id: MemoryId::parse(&memory_id)?,
            name,
            description,
            memory_type: MemoryType::parse(&memory_type)?,
            scope_kind: match scope_kind.as_str() {
                "global" => "global",
                "workspace" => "workspace",
                other => {
                    return Err(PersonalizationApplicationError::Storage(format!(
                        "unknown projected memory scope kind {other:?}"
                    )))
                }
            },
            workspace_key: workspace_key
                .as_deref()
                .map(WorkspaceKey::parse)
                .transpose()?,
            audience_is_restricted: audience_is_restricted(&audience),
            status: MemoryStatus::parse(&status)?,
            source: MemorySource::parse(&source)?,
            source_agent_id: source_agent_id.as_deref().map(AgentId::parse).transpose()?,
            revision: u64::try_from(revision).unwrap_or_default(),
            updated_at: parse_timestamp(&updated_at)?,
        })
    })())
}

/// The value a cursor compares against for a given order.
fn sort_key_for(summary: &MemorySummary, order: MemoryOrder) -> String {
    match order {
        MemoryOrder::UpdatedDescending | MemoryOrder::UpdatedAscending => {
            timestamp(summary.updated_at)
        }
        MemoryOrder::NameAscending => summary.name.clone(),
    }
}

struct FilterSql {
    clauses: Vec<String>,
    bindings: Vec<Value>,
}

impl FilterSql {
    fn push(&mut self, clause: String, value: Value) {
        self.clauses.push(clause);
        self.bindings.push(value);
    }

    fn next_index(&self) -> usize {
        self.bindings.len() + 1
    }
}

fn build_filters(query: &MemoryQuery) -> FilterSql {
    let mut filter = FilterSql {
        clauses: Vec::new(),
        bindings: Vec::new(),
    };

    if let Some(search) = query
        .search
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        // Name and description only. Searching bodies from the projection would mean storing them
        // here, which is the thing this table exists to avoid.
        let index = filter.next_index();
        filter.clauses.push(format!(
            "(name LIKE ?{index} ESCAPE '\\' OR description LIKE ?{index} ESCAPE '\\')"
        ));
        filter
            .bindings
            .push(Value::Text(format!("%{}%", escape_like(search.trim()))));
    }

    match &query.scope {
        MemoryScopeFilter::Any => {}
        MemoryScopeFilter::GlobalOnly => {
            filter.push(
                format!("scope_kind = ?{}", filter.next_index()),
                Value::Text("global".to_string()),
            );
        }
        MemoryScopeFilter::Workspace { workspace_key } => {
            let index = filter.next_index();
            filter.clauses.push(format!(
                "(scope_kind = 'workspace' AND workspace_key = ?{index})"
            ));
            filter
                .bindings
                .push(Value::Text(workspace_key.as_str().to_string()));
        }
    }

    if !query.statuses.is_empty() {
        let placeholders = bind_list(&mut filter, query.statuses.iter().map(|s| s.as_str()));
        filter.clauses.push(format!("status IN ({placeholders})"));
    }
    if !query.memory_types.is_empty() {
        let placeholders = bind_list(&mut filter, query.memory_types.iter().map(|t| t.as_str()));
        filter
            .clauses
            .push(format!("memory_type IN ({placeholders})"));
    }
    if let Some(source_agent_id) = query.source_agent_id.as_ref() {
        filter.push(
            format!("source_agent_id = ?{}", filter.next_index()),
            Value::Text(source_agent_id.as_str().to_string()),
        );
    }
    if let Some(audience_agent_id) = query.audience_agent_id.as_ref() {
        // Either the memory is open to everyone, or the JSON list names exactly this Agent. An
        // exact `json_each` equality rather than a LIKE: a pattern match would admit prefixes,
        // wildcard characters inside an id, and case variants.
        let all_index = filter.next_index();
        filter
            .bindings
            .push(Value::Text("\"all_agents\"".to_string()));
        let named_index = filter.next_index();
        filter
            .bindings
            .push(Value::Text(audience_agent_id.as_str().to_string()));
        filter.clauses.push(format!(
            "(audience_json = ?{all_index} OR (json_valid(audience_json) AND EXISTS (\
                SELECT 1 FROM json_each(audience_json, '$.selected_agents') \
                WHERE json_each.type = 'text' AND json_each.value = ?{named_index})))"
        ));
    }
    filter
}

fn bind_list<'a>(filter: &mut FilterSql, values: impl Iterator<Item = &'a str>) -> String {
    let mut placeholders = Vec::new();
    for value in values {
        let index = filter.next_index();
        placeholders.push(format!("?{index}"));
        filter.bindings.push(Value::Text(value.to_string()));
    }
    placeholders.join(", ")
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn order_sql(order: MemoryOrder) -> (&'static str, &'static str, &'static str) {
    // (sort column, direction, comparison operator for the keyset predicate)
    match order {
        MemoryOrder::UpdatedDescending => ("updated_at", "DESC", "<"),
        MemoryOrder::UpdatedAscending => ("updated_at", "ASC", ">"),
        MemoryOrder::NameAscending => ("name", "ASC", ">"),
    }
}

impl MemoryProjectionPort for SqliteMemoryProjection {
    fn upsert(&self, record: &MemoryRecord, content_hash: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO personalization_memory_projection (
                 memory_id, file_name, name, description, memory_type, scope_kind, workspace_key,
                 audience_json, status, source, source_agent_id, source_session_id,
                 source_workspace_key, legacy_save_source, legacy_folder, legacy_source_path,
                 sensitivity, revision, content_hash, created_at, updated_at, verified_at,
                 last_used_at, use_count
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                       ?18, ?19, ?20, ?21, ?22, ?23, ?24)
             ON CONFLICT(memory_id) DO UPDATE SET
                 file_name = excluded.file_name,
                 name = excluded.name,
                 description = excluded.description,
                 memory_type = excluded.memory_type,
                 scope_kind = excluded.scope_kind,
                 workspace_key = excluded.workspace_key,
                 audience_json = excluded.audience_json,
                 status = excluded.status,
                 source = excluded.source,
                 source_agent_id = excluded.source_agent_id,
                 source_session_id = excluded.source_session_id,
                 source_workspace_key = excluded.source_workspace_key,
                 legacy_save_source = excluded.legacy_save_source,
                 legacy_folder = excluded.legacy_folder,
                 legacy_source_path = excluded.legacy_source_path,
                 sensitivity = excluded.sensitivity,
                 revision = excluded.revision,
                 content_hash = excluded.content_hash,
                 updated_at = excluded.updated_at,
                 verified_at = excluded.verified_at,
                 last_used_at = excluded.last_used_at,
                 use_count = excluded.use_count",
            rusqlite::params![
                record.id.as_str(),
                record.file_name(),
                record.name,
                record.description,
                record.memory_type.as_str(),
                record.scope.kind_str(),
                record.scope.workspace_key().map(WorkspaceKey::as_str),
                audience_json(&record.audience),
                record.status.as_str(),
                record.source.as_str(),
                record
                    .provenance
                    .source_agent_id
                    .as_ref()
                    .map(AgentId::as_str),
                record
                    .provenance
                    .source_session_id
                    .as_ref()
                    .map(|id| id.as_str()),
                record
                    .provenance
                    .source_workspace_key
                    .as_ref()
                    .map(WorkspaceKey::as_str),
                record
                    .provenance
                    .legacy_original_save_source
                    .map(LegacyMemorySaveSource::as_str),
                record.provenance.legacy_folder.as_deref(),
                record.provenance.legacy_source_relative_path.as_deref(),
                record.sensitivity.as_str(),
                i64::try_from(record.revision).unwrap_or(i64::MAX),
                content_hash,
                timestamp(record.created_at),
                timestamp(record.updated_at),
                record.verified_at.map(timestamp),
                record.last_used_at.map(timestamp),
                i64::try_from(record.use_count).unwrap_or(i64::MAX),
            ],
        )
        .map_err(storage)?;
        Ok(())
    }

    fn remove(&self, id: &MemoryId) -> Result<bool> {
        let conn = self.connection()?;
        let removed = conn
            .execute(
                "DELETE FROM personalization_memory_projection WHERE memory_id = ?1",
                [id.as_str()],
            )
            .map_err(storage)?;
        Ok(removed > 0)
    }

    fn list_page(&self, query: &MemoryQuery) -> Result<MemoryPage> {
        let conn = self.connection()?;
        let mut filter = build_filters(query);
        let (sort_column, direction, comparison) = order_sql(query.order);

        if let Some(cursor) = query.cursor.as_ref() {
            // Compare the pair, not just the sort column: two memories can share an updated
            // timestamp or a display name, and a single-column cursor would skip or repeat them.
            let key_index = filter.next_index();
            filter.bindings.push(Value::Text(cursor.sort_key.clone()));
            let id_index = filter.next_index();
            filter
                .bindings
                .push(Value::Text(cursor.id.as_str().to_string()));
            filter.clauses.push(format!(
                "({sort_column}, memory_id) {comparison} (?{key_index}, ?{id_index})"
            ));
        }

        let where_clause = if filter.clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", filter.clauses.join(" AND "))
        };
        // One extra row tells us whether another page exists without a second count query.
        let limit = query.page_size() + 1;
        let statement = format!(
            "SELECT {SUMMARY_COLUMNS} FROM personalization_memory_projection {where_clause} \
             ORDER BY {sort_column} {direction}, memory_id {direction} LIMIT {limit}"
        );

        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let bindings: Vec<&dyn ToSql> = filter
            .bindings
            .iter()
            .map(|value| value as &dyn ToSql)
            .collect();
        let rows = prepared
            .query_map(bindings.as_slice(), read_summary)
            .map_err(storage)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(storage)??);
        }

        let has_more = items.len() > query.page_size();
        if has_more {
            items.truncate(query.page_size());
        }
        // No cursor without a last row, expressed as a `filter` rather than an assertion: a page
        // that reported more results while holding none would be a bug, and answering it with
        // "no next page" is recoverable where a panic in a list query is not.
        let next_cursor = items.last().filter(|_| has_more).map(|last| MemoryCursor {
            sort_key: sort_key_for(last, query.order),
            id: last.id.clone(),
        });

        Ok(MemoryPage {
            items,
            next_cursor,
            total_matched: None,
        })
    }

    fn count_for_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        let conn = self.connection()?;
        let query = MemoryQuery {
            scope: scope.clone(),
            statuses: statuses.to_vec(),
            ..MemoryQuery::default()
        };
        let filter = build_filters(&query);
        let where_clause = if filter.clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", filter.clauses.join(" AND "))
        };
        let statement = format!(
            "SELECT COUNT(*), \
                    SUM(CASE WHEN scope_kind = 'global' THEN 1 ELSE 0 END), \
                    SUM(CASE WHEN scope_kind = 'workspace' THEN 1 ELSE 0 END), \
                    SUM(CASE WHEN status = 'candidate' THEN 1 ELSE 0 END) \
             FROM personalization_memory_projection {where_clause}"
        );
        let bindings: Vec<&dyn ToSql> = filter
            .bindings
            .iter()
            .map(|value| value as &dyn ToSql)
            .collect();
        let (matched, global, workspace, candidates): (i64, Option<i64>, Option<i64>, Option<i64>) =
            conn.query_row(&statement, bindings.as_slice(), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(storage)?;

        Ok(ResetCounts {
            matched: usize::try_from(matched).unwrap_or_default(),
            global: usize::try_from(global.unwrap_or_default()).unwrap_or_default(),
            workspace: usize::try_from(workspace.unwrap_or_default()).unwrap_or_default(),
            candidates: usize::try_from(candidates.unwrap_or_default()).unwrap_or_default(),
            // Malformed files are not projected — that is what makes them malformed. Only the
            // filesystem enumeration can count them, and it fills this in.
            malformed: 0,
        })
    }

    fn eligible_page(
        &self,
        criteria: &MemoryEligibilityCriteria,
    ) -> Result<MemoryEligibilitySummary> {
        let conn = self.connection()?;
        let workspace = criteria
            .workspace
            .as_ref()
            .map(|key| key.as_str().to_string());

        // One expression, evaluated once per row, that assigns exactly one outcome. Computing
        // eligibility and the exclusion counts separately is how the two drift until they stop
        // adding up, and the whole point of this summary is that they do.
        let counts_statement = format!(
            "SELECT {ELIGIBILITY_CLASSIFICATION} AS outcome, COUNT(*) \
             FROM personalization_memory_projection GROUP BY outcome"
        );
        let mut prepared = conn.prepare(&counts_statement).map_err(storage)?;
        let rows = prepared
            .query_map(
                rusqlite::params![
                    criteria.agent_id.as_str(),
                    i64::from(criteria.allow_global),
                    i64::from(criteria.project_only),
                    workspace,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(storage)?;

        let mut summary = MemoryEligibilitySummary::default();
        for row in rows {
            let (outcome, count) = row.map_err(storage)?;
            let count = usize::try_from(count).unwrap_or_default();
            summary.considered += count;
            match outcome.as_str() {
                "eligible" => summary.eligible_total = count,
                other => {
                    // An outcome this build cannot name still has to be counted, or the totals
                    // would quietly stop adding up. It is attributed to the record, never to the
                    // session: something here could not classify the row.
                    let reason = exclusion_reason(other)
                        .unwrap_or(PersonalizationExclusionReason::InvalidRecord);
                    summary
                        .exclusions
                        .push(MemoryExclusionCount { reason, count });
                }
            }
        }
        // Deterministic regardless of how SQLite grouped them, so two identical stores produce
        // identical summaries and therefore identical revision tokens.
        summary
            .exclusions
            .sort_by_key(|entry| entry.reason.as_str());

        let refs_statement = format!(
            "SELECT memory_id, revision, content_hash, name, description, memory_type, scope_kind, \
                    workspace_key, updated_at, status, audience_json \
             FROM personalization_memory_projection \
             WHERE ({ELIGIBILITY_CLASSIFICATION}) = 'eligible' \
             ORDER BY updated_at DESC, memory_id ASC LIMIT ?5"
        );
        let mut prepared = conn.prepare(&refs_statement).map_err(storage)?;
        let rows = prepared
            .query_map(
                rusqlite::params![
                    criteria.agent_id.as_str(),
                    i64::from(criteria.allow_global),
                    i64::from(criteria.project_only),
                    workspace,
                    i64::try_from(criteria.limit).unwrap_or(i64::MAX),
                ],
                read_snapshot_ref,
            )
            .map_err(storage)?;
        for row in rows {
            summary.refs.push(row.map_err(storage)??);
        }
        summary.truncated = summary.refs.len() < summary.eligible_total;
        summary.digest = eligibility_digest(&summary.refs);
        Ok(summary)
    }

    /// One read-only snapshot for the whole enumeration, so a save landing between two pages
    /// cannot produce a relation that mixes two states of the store. Released before any caller
    /// goes near a network call.
    fn eligible_authority(
        &self,
        criteria: &MemoryEligibilityCriteria,
        budget: EligibilityEnumerationBudget,
    ) -> Result<AuthorizedMemoryRelation> {
        let mut conn = self.connection()?;
        let snapshot = conn.transaction().map_err(storage)?;
        let workspace = criteria
            .workspace
            .as_ref()
            .map(|key| key.as_str().to_string());
        let considered: i64 = snapshot
            .query_row(
                "SELECT COUNT(*) FROM personalization_memory_projection",
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let page_size = budget.page_size.clamp(1, 10_000);
        let statement = eligibility_page_statement();
        let mut prepared = snapshot.prepare(&statement).map_err(storage)?;
        let mut entries: Vec<MemoryReadHandle> = Vec::new();
        let mut after = String::new();
        let mut complete = true;
        loop {
            let rows = prepared
                .query_map(
                    rusqlite::params![
                        criteria.agent_id.as_str(),
                        i64::from(criteria.allow_global),
                        i64::from(criteria.project_only),
                        workspace,
                        after,
                        i64::try_from(page_size).unwrap_or(i64::MAX),
                    ],
                    read_authority_row,
                )
                .map_err(storage)?;
            let mut page = Vec::new();
            for row in rows {
                page.push(row.map_err(storage)??);
            }
            let page_len = page.len();
            if let Some(last) = page.last() {
                after = last.id.as_str().to_string();
            }
            entries.extend(page);
            if entries.len() > budget.max_entries {
                // The relation is larger than this build is willing to hold. Reported as
                // incomplete rather than cut to the budget: a truncated relation is an
                // authorization set with eligible records silently missing from it.
                complete = false;
                entries.clear();
                break;
            }
            if page_len < page_size {
                break;
            }
        }
        drop(prepared);
        snapshot.finish().map_err(storage)?;
        Ok(AuthorizedMemoryRelation {
            entries,
            considered: usize::try_from(considered).unwrap_or_default(),
            complete,
        })
    }

    fn projected_ids(&self) -> Result<Vec<MemoryId>> {
        let conn = self.connection()?;
        collect_ids(
            &conn,
            "SELECT memory_id FROM personalization_memory_projection ORDER BY memory_id",
        )
    }

    fn clear(&self) -> Result<usize> {
        let conn = self.connection()?;
        conn.execute("DELETE FROM personalization_memory_projection", [])
            .map_err(storage)
    }
}

fn read_authority_row(row: &Row<'_>) -> rusqlite::Result<Result<MemoryReadHandle>> {
    let memory_id: String = row.get(0)?;
    let revision: i64 = row.get(1)?;
    let content_hash: String = row.get(2)?;
    let status: String = row.get(3)?;
    let scope_kind: String = row.get(4)?;
    let workspace_key: Option<String> = row.get(5)?;
    let audience: String = row.get(6)?;
    Ok((|| {
        Ok(MemoryReadHandle {
            id: MemoryId::parse(&memory_id)?,
            revision: u64::try_from(revision).unwrap_or_default(),
            content_hash,
            authority_fingerprint: row_authority_fingerprint(
                &status,
                &scope_kind,
                workspace_key.as_deref(),
                &audience,
            )?,
        })
    })())
}

fn collect_ids(conn: &Connection, statement: &str) -> Result<Vec<MemoryId>> {
    let mut prepared = conn.prepare(statement).map_err(storage)?;
    let rows = prepared
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage)?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(MemoryId::parse(&row.map_err(storage)?)?);
    }
    Ok(ids)
}
