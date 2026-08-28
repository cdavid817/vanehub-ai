use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::contexts::personalization::application::{
    PersonalizationApplicationError, PolicyRepository,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PatchPolicyResult, PersonalizationLayers,
    PersonalizationPolicyPatch, PersonalizationPolicyRecord, PersonalizationPolicyScope,
    PolicyLayerState, PolicyResolutionBundle, PolicyToggle, WorkspaceKey,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// The primary key. Composite rather than the bare scope key so that when policy sets become user
/// visible, only the `scope_key` uniqueness has to be relaxed, not the identity of every row.
fn row_id(policy_set_id: &str, scope_key: &str) -> String {
    format!("{policy_set_id}::{scope_key}")
}

#[derive(Clone)]
pub(crate) struct SqlitePolicyRepository {
    database: NativeDatabase,
}

impl SqlitePolicyRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }

    /// Lets the schema test assert on tables and indexes directly. Production code goes through
    /// the port; nothing outside a test may reach the connection.
    #[cfg(test)]
    pub(crate) fn raw_connection_for_tests(&self) -> Result<PooledSqlite> {
        self.connection()
    }
}

const SELECT_COLUMNS: &str = "scope_kind, workspace_key, agent_id, policy_set_id, \
     instruction_merge_mode, about_user, style_rules, memory_read_mode, explicit_save_mode, \
     automatic_extraction_mode, global_memory_access_mode, revision";

/// Rebuilds a record from a row, refusing anything the domain would not accept.
///
/// A row that fails here is corruption, not a value to interpret generously: silently coercing an
/// unknown toggle to `enabled` would be a fail-open on exactly the switch that governs memory.
fn read_record(
    row: &Row<'_>,
) -> rusqlite::Result<
    std::result::Result<PersonalizationPolicyRecord, PersonalizationApplicationError>,
> {
    let scope_kind: String = row.get(0)?;
    let workspace_key: Option<String> = row.get(1)?;
    let agent_id: Option<String> = row.get(2)?;
    let policy_set_id: String = row.get(3)?;
    let merge_mode: String = row.get(4)?;
    let about_user: String = row.get(5)?;
    let style_rules: String = row.get(6)?;
    let memory_read: String = row.get(7)?;
    let explicit_save: String = row.get(8)?;
    let automatic_extraction: String = row.get(9)?;
    let global_memory: String = row.get(10)?;
    let revision: i64 = row.get(11)?;

    Ok(build_record(
        &scope_kind,
        workspace_key.as_deref(),
        agent_id.as_deref(),
        &policy_set_id,
        &merge_mode,
        about_user,
        style_rules,
        [
            memory_read.as_str(),
            explicit_save.as_str(),
            automatic_extraction.as_str(),
            global_memory.as_str(),
        ],
        revision,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_record(
    scope_kind: &str,
    workspace_key: Option<&str>,
    agent_id: Option<&str>,
    policy_set_id: &str,
    merge_mode: &str,
    about_user: String,
    style_rules: String,
    toggles: [&str; 4],
    revision: i64,
) -> Result<PersonalizationPolicyRecord> {
    let workspace_key = workspace_key.map(WorkspaceKey::parse).transpose()?;
    let agent_id = agent_id.map(AgentId::parse).transpose()?;
    let scope = PersonalizationPolicyScope::from_parts(
        scope_kind,
        workspace_key.as_ref(),
        agent_id.as_ref(),
    )?;

    let mut record = PersonalizationPolicyRecord::inheriting(scope);
    record.set_policy_set_id(policy_set_id.to_string());
    record.set_instruction_merge_mode(InstructionMergeMode::parse(merge_mode)?);
    record.set_about_user(about_user);
    record.set_style_rules(style_rules);
    record.set_memory_read_mode(PolicyToggle::parse(toggles[0])?);
    record.set_explicit_save_mode(PolicyToggle::parse(toggles[1])?);
    record.set_automatic_extraction_mode(PolicyToggle::parse(toggles[2])?);
    record.set_global_memory_access_mode(PolicyToggle::parse(toggles[3])?);
    // A negative revision cannot come from this writer; treating it as 0 would let a corrupted row
    // silently win every expected-revision check.
    record.set_revision(u64::try_from(revision).map_err(|_| {
        PersonalizationApplicationError::Storage(
            "personalization policy revision is negative".to_string(),
        )
    })?);
    Ok(record)
}

fn load_by_scope_key(
    conn: &Connection,
    scope_key: &str,
) -> Result<Option<PersonalizationPolicyRecord>> {
    let statement = format!(
        "SELECT {SELECT_COLUMNS} FROM personalization_policy_overrides WHERE scope_key = ?1"
    );
    let row = conn
        .query_row(&statement, params![scope_key], read_record)
        .optional()
        .map_err(storage)?;
    row.transpose()
}

fn write_record(
    conn: &Connection,
    record: &PersonalizationPolicyRecord,
    now: DateTime<Utc>,
) -> Result<()> {
    let scope = record.scope();
    let scope_key = scope.scope_key();
    let now = timestamp(now);
    conn.execute(
        "INSERT INTO personalization_policy_overrides (
             id, policy_set_id, scope_key, scope_kind, workspace_key, agent_id,
             instruction_merge_mode, about_user, style_rules, memory_read_mode,
             explicit_save_mode, automatic_extraction_mode, global_memory_access_mode,
             revision, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)
         ON CONFLICT(scope_key) DO UPDATE SET
             policy_set_id = excluded.policy_set_id,
             instruction_merge_mode = excluded.instruction_merge_mode,
             about_user = excluded.about_user,
             style_rules = excluded.style_rules,
             memory_read_mode = excluded.memory_read_mode,
             explicit_save_mode = excluded.explicit_save_mode,
             automatic_extraction_mode = excluded.automatic_extraction_mode,
             global_memory_access_mode = excluded.global_memory_access_mode,
             revision = excluded.revision,
             updated_at = excluded.updated_at",
        params![
            row_id(record.policy_set_id(), &scope_key),
            record.policy_set_id(),
            scope_key,
            scope.scope_kind(),
            scope.workspace_key().map(WorkspaceKey::as_str),
            scope.agent_id().map(AgentId::as_str),
            record.instruction_merge_mode().as_str(),
            record.about_user(),
            record.style_rules(),
            record.memory_read_mode().as_str(),
            record.explicit_save_mode().as_str(),
            record.automatic_extraction_mode().as_str(),
            record.global_memory_access_mode().as_str(),
            i64::try_from(record.revision()).unwrap_or(i64::MAX),
            now,
        ],
    )
    .map_err(storage)?;
    Ok(())
}

impl PolicyRepository for SqlitePolicyRepository {
    fn load(
        &self,
        scope: &PersonalizationPolicyScope,
    ) -> Result<Option<PersonalizationPolicyRecord>> {
        let conn = self.connection()?;
        load_by_scope_key(&conn, &scope.scope_key())
    }

    fn load_resolution_bundle(
        &self,
        scopes: &[PersonalizationPolicyScope],
    ) -> Result<PolicyResolutionBundle> {
        if scopes.is_empty() {
            return Ok(PolicyResolutionBundle { layers: Vec::new() });
        }
        let conn = self.connection()?;
        // One statement, which SQLite evaluates against one consistent view of the database. Four
        // round trips would let a save land between two of them and produce a bundle that mixes
        // revisions — the state an immutable snapshot exists to rule out.
        let keys: Vec<String> = scopes.iter().map(|scope| scope.scope_key()).collect();
        let placeholders = (1..=keys.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let statement = format!(
            "SELECT {SELECT_COLUMNS} FROM personalization_policy_overrides              WHERE scope_key IN ({placeholders})"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let bindings: Vec<&dyn rusqlite::ToSql> =
            keys.iter().map(|key| key as &dyn rusqlite::ToSql).collect();
        let rows = prepared
            .query_map(bindings.as_slice(), read_record)
            .map_err(storage)?;

        let mut found: Vec<PersonalizationPolicyRecord> = Vec::new();
        for row in rows {
            found.push(row.map_err(storage)??);
        }

        // Every requested key gets an entry. A key the query proved has no row is `Absent`, which
        // is a finding; a key that was never asked for is simply not in the bundle. Collapsing
        // those two into "missing" is what would let a failed read be cached as "no override".
        Ok(PolicyResolutionBundle {
            layers: scopes
                .iter()
                .map(|scope| {
                    let state = found
                        .iter()
                        .find(|record| record.scope() == scope)
                        .cloned()
                        .map(PolicyLayerState::Present)
                        .unwrap_or(PolicyLayerState::Absent);
                    (scope.clone(), state)
                })
                .collect(),
        })
    }

    fn load_layers(
        &self,
        agent_id: &AgentId,
        workspace_key: Option<&WorkspaceKey>,
    ) -> Result<PersonalizationLayers> {
        let conn = self.connection()?;
        // One statement rather than four round trips: a save landing between two of them would
        // produce a snapshot mixing revisions, which is precisely what the immutable snapshot is
        // supposed to rule out.
        let mut wanted = vec![
            PersonalizationPolicyScope::Global,
            PersonalizationPolicyScope::Agent {
                agent_id: agent_id.clone(),
            },
        ];
        if let Some(workspace_key) = workspace_key {
            wanted.push(PersonalizationPolicyScope::Workspace {
                workspace_key: workspace_key.clone(),
            });
            wanted.push(PersonalizationPolicyScope::WorkspaceAgent {
                workspace_key: workspace_key.clone(),
                agent_id: agent_id.clone(),
            });
        }
        let keys: Vec<String> = wanted.iter().map(|scope| scope.scope_key()).collect();
        let placeholders = (1..=keys.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let statement = format!(
            "SELECT {SELECT_COLUMNS} FROM personalization_policy_overrides \
             WHERE scope_key IN ({placeholders})"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let bindings: Vec<&dyn rusqlite::ToSql> =
            keys.iter().map(|key| key as &dyn rusqlite::ToSql).collect();
        let rows = prepared
            .query_map(bindings.as_slice(), read_record)
            .map_err(storage)?;

        let mut layers = PersonalizationLayers::default();
        for row in rows {
            let record = row.map_err(storage)??;
            match record.scope() {
                PersonalizationPolicyScope::Global => layers.global = Some(record),
                PersonalizationPolicyScope::Agent { .. } => layers.agent = Some(record),
                PersonalizationPolicyScope::Workspace { .. } => layers.workspace = Some(record),
                PersonalizationPolicyScope::WorkspaceAgent { .. } => {
                    layers.workspace_agent = Some(record)
                }
            }
        }
        Ok(layers)
    }

    fn list_all(&self) -> Result<Vec<PersonalizationPolicyRecord>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {SELECT_COLUMNS} FROM personalization_policy_overrides ORDER BY scope_key"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared.query_map([], read_record).map_err(storage)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(storage)??);
        }
        Ok(records)
    }

    fn seed_default_global(&self, now: DateTime<Utc>) -> Result<PersonalizationPolicyRecord> {
        let mut conn = self.connection()?;
        let transaction = conn.transaction().map_err(storage)?;
        let scope_key = PersonalizationPolicyScope::Global.scope_key();
        if let Some(existing) = load_by_scope_key(&transaction, &scope_key)? {
            // Leave an existing row alone, revision included. Startup must never reset a policy
            // the user or the legacy migration already established.
            return Ok(existing);
        }
        let record = PersonalizationPolicyRecord::default_global();
        write_record(&transaction, &record, now)?;
        transaction.commit().map_err(storage)?;
        Ok(record)
    }

    fn patch(
        &self,
        scope: &PersonalizationPolicyScope,
        expected_revision: Option<u64>,
        patch: PersonalizationPolicyPatch,
        now: DateTime<Utc>,
    ) -> Result<PatchPolicyResult> {
        let mut conn = self.connection()?;
        // IMMEDIATE so the write lock is taken before the read. With a deferred transaction two
        // savers can both read revision N and both promote to N+1 — expected-revision checking
        // that silently degrades to last-response-wins.
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let scope_key = scope.scope_key();
        let current = load_by_scope_key(&transaction, &scope_key)?;

        let base = match (current, expected_revision) {
            (Some(current), expected) => {
                if let Err(conflict) = current.check_expected_revision(expected) {
                    let _ = conflict;
                    return Ok(PatchPolicyResult::Conflict { current });
                }
                current
            }
            (None, None) | (None, Some(0)) => {
                if matches!(scope, PersonalizationPolicyScope::Global) {
                    PersonalizationPolicyRecord::default_global()
                } else {
                    PersonalizationPolicyRecord::inheriting(scope.clone())
                }
            }
            (None, Some(_)) => {
                // The caller expects a revision from a row that does not exist. Creating it would
                // discard whatever they were actually editing against.
                return Err(PersonalizationApplicationError::NotFound);
            }
        };

        let updated = base.apply(patch)?;
        write_record(&transaction, &updated, now)?;
        transaction.commit().map_err(storage)?;
        Ok(PatchPolicyResult::Updated(updated))
    }

    fn delete(&self, scope: &PersonalizationPolicyScope) -> Result<bool> {
        let conn = self.connection()?;
        let removed = conn
            .execute(
                "DELETE FROM personalization_policy_overrides WHERE scope_key = ?1",
                params![scope.scope_key()],
            )
            .map_err(storage)?;
        Ok(removed > 0)
    }
}
