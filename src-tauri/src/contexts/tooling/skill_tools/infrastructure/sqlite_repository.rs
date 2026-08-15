use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolPackageRef, SkillToolRevisionState,
    SkillToolStateRepository,
};
use crate::contexts::tooling::skill_tools::domain::{
    ContentHash, SkillToolDiagnosticSummary, SkillToolId, SkillToolIntegrity, SkillToolKey,
    SkillToolLifecycle, SkillToolOwnerId, SkillToolQuarantine, SkillToolRevision, SkillToolScope,
    SkillToolSourceScope, SkillToolTrustDecision, SkillToolTrustRecord, SkillToolValidationState,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Row};
use std::sync::Arc;

pub(crate) struct SqliteSkillToolRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteSkillToolRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, SkillToolApplicationError> {
        self.database
            .connection()
            .map_err(|error| SkillToolApplicationError::Storage(error.to_string()))
    }
}

const REVISION_COLUMNS: &str = "revision_witness, skill_id, scope, workspace_path, tool_id, \
     implementation_kind, base_revision, manifest_hash, implementation_hash, capability_digest, \
     validation_state, validation_code, enabled, quarantined, quarantine_reason, \
     consecutive_failures, diagnostic_summary, created_at, updated_at";

impl SkillToolStateRepository for SqliteSkillToolRepository {
    fn revision_states(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {REVISION_COLUMNS} FROM skill_tool_revisions \
                 WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3 \
                 ORDER BY tool_id ASC, created_at ASC"
            ))
            .map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    owner.owner.as_str(),
                    owner.source.scope.as_str(),
                    owner.source.storage_workspace_key()
                ],
                |row| Ok(read_revision_state(row)),
            )
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        // A row whose enums no longer parse is skipped rather than failing the whole read: a
        // single corrupt record must not make a Skill's tool inventory unreadable.
        Ok(rows.into_iter().flatten().collect())
    }

    fn revision_state(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        let connection = self.connection()?;
        let state = connection
            .query_row(
                &format!(
                    "SELECT {REVISION_COLUMNS} FROM skill_tool_revisions WHERE revision_witness = ?1"
                ),
                params![revision.as_str()],
                |row| Ok(read_revision_state(row)),
            )
            .optional()
            .map_err(storage)?;
        Ok(state.flatten())
    }

    fn record_discovered(
        &self,
        state: &SkillToolRevisionState,
    ) -> Result<(), SkillToolApplicationError> {
        let connection = self.connection()?;
        // `DO NOTHING` rather than an upsert: rediscovering a revision must not reset the trust,
        // enablement, or quarantine state an operator already decided for that exact content.
        connection
            .execute(
                "INSERT INTO skill_tool_revisions (\
                    revision_witness, skill_id, scope, workspace_path, tool_id, \
                    implementation_kind, base_revision, manifest_hash, implementation_hash, \
                    capability_digest, validation_state, validation_code, enabled, quarantined, \
                    quarantine_reason, consecutive_failures, diagnostic_summary, created_at, updated_at\
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19) \
                 ON CONFLICT(revision_witness) DO NOTHING",
                params![
                    state.key.revision.as_str(),
                    state.key.owner.as_str(),
                    state.key.source.scope.as_str(),
                    state.key.source.storage_workspace_key(),
                    state.key.tool.as_str(),
                    state.implementation_kind,
                    state.integrity.base_revision,
                    state.integrity.manifest_hash.as_str(),
                    state.integrity.implementation_hash.as_str(),
                    state.integrity.capability_digest,
                    state.lifecycle.validation.as_str(),
                    state.validation_code,
                    i64::from(state.lifecycle.enabled),
                    i64::from(state.lifecycle.quarantine.is_quarantined()),
                    state.lifecycle.quarantine.reason(),
                    i64::from(state.lifecycle.consecutive_failures),
                    state.diagnostics.to_storage_value(),
                    state.created_at,
                    state.updated_at,
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn save_lifecycle(
        &self,
        revision: &SkillToolRevision,
        lifecycle: &SkillToolLifecycle,
        validation_code: Option<&str>,
        diagnostics: &SkillToolDiagnosticSummary,
        updated_at: &str,
    ) -> Result<(), SkillToolApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE skill_tool_revisions SET validation_state = ?2, validation_code = ?3, \
                 enabled = ?4, quarantined = ?5, quarantine_reason = ?6, consecutive_failures = ?7, \
                 diagnostic_summary = ?8, updated_at = ?9 WHERE revision_witness = ?1",
                params![
                    revision.as_str(),
                    lifecycle.validation.as_str(),
                    validation_code,
                    i64::from(lifecycle.enabled),
                    i64::from(lifecycle.quarantine.is_quarantined()),
                    lifecycle.quarantine.reason(),
                    i64::from(lifecycle.consecutive_failures),
                    diagnostics.to_storage_value(),
                    updated_at,
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(SkillToolApplicationError::NotFound(
                revision.as_str().to_string(),
            ));
        }
        Ok(())
    }

    fn trust_record(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError> {
        let connection = self.connection()?;
        let record = connection
            .query_row(
                "SELECT revision_witness, base_revision, manifest_hash, implementation_hash, \
                 capability_digest, decision, actor, decided_at FROM skill_tool_trust \
                 WHERE revision_witness = ?1",
                params![revision.as_str()],
                |row| Ok(read_trust_record(row)),
            )
            .optional()
            .map_err(storage)?;
        Ok(record.flatten())
    }

    fn save_trust(
        &self,
        record: &SkillToolTrustRecord,
        decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|error| SkillToolApplicationError::Storage(error.to_string()))?;
        let transaction = connection.transaction().map_err(storage)?;
        // The trust row and the revision row it authorizes move together; a decision persisted
        // without its revision row would be trust for content nothing can observe.
        let owner: Option<(String, String, String, String)> = transaction
            .query_row(
                "SELECT skill_id, scope, workspace_path, tool_id FROM skill_tool_revisions \
                 WHERE revision_witness = ?1",
                params![record.revision.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(storage)?;
        let Some((skill_id, scope, workspace_path, tool_id)) = owner else {
            return Err(SkillToolApplicationError::NotFound(
                record.revision.as_str().to_string(),
            ));
        };
        transaction
            .execute(
                "INSERT INTO skill_tool_trust (\
                    revision_witness, skill_id, scope, workspace_path, tool_id, base_revision, \
                    manifest_hash, implementation_hash, capability_digest, decision, actor, decided_at\
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
                 ON CONFLICT(revision_witness) DO UPDATE SET \
                    decision = excluded.decision, actor = excluded.actor, \
                    decided_at = excluded.decided_at",
                params![
                    record.revision.as_str(),
                    skill_id,
                    scope,
                    workspace_path,
                    tool_id,
                    record.integrity.base_revision,
                    record.integrity.manifest_hash.as_str(),
                    record.integrity.implementation_hash.as_str(),
                    record.integrity.capability_digest,
                    decision.as_str(),
                    record.actor,
                    record.decided_at,
                ],
            )
            .map_err(storage)?;
        // Revoking also disables: leaving a revoked revision enabled would show an operator a
        // tool marked "on" that can never run.
        if decision == SkillToolTrustDecision::Revoked {
            transaction
                .execute(
                    "UPDATE skill_tool_revisions SET enabled = 0, updated_at = ?2 \
                     WHERE revision_witness = ?1",
                    params![record.revision.as_str(), record.decided_at],
                )
                .map_err(storage)?;
        }
        transaction.commit().map_err(storage)?;
        Ok(())
    }
}

fn read_revision_state(row: &Row<'_>) -> Option<SkillToolRevisionState> {
    let revision = SkillToolRevision::parse(&row.get::<_, String>(0).ok()?).ok()?;
    let owner = SkillToolOwnerId::parse(&row.get::<_, String>(1).ok()?).ok()?;
    let scope = SkillToolScope::parse(&row.get::<_, String>(2).ok()?)?;
    let workspace: String = row.get(3).ok()?;
    let source = SkillToolSourceScope::new(scope, Some(workspace.as_str())).ok()?;
    let tool = SkillToolId::parse(&row.get::<_, String>(4).ok()?).ok()?;
    let quarantined: i64 = row.get(13).ok()?;
    let quarantine_reason: Option<String> = row.get(14).ok()?;
    Some(SkillToolRevisionState {
        key: SkillToolKey::new(owner, source, tool, revision),
        implementation_kind: row.get(5).ok()?,
        integrity: SkillToolIntegrity {
            base_revision: row.get(6).ok()?,
            manifest_hash: ContentHash::parse(&row.get::<_, String>(7).ok()?).ok()?,
            implementation_hash: ContentHash::parse(&row.get::<_, String>(8).ok()?).ok()?,
            capability_digest: row.get(9).ok()?,
        },
        lifecycle: SkillToolLifecycle {
            validation: SkillToolValidationState::parse(&row.get::<_, String>(10).ok()?)?,
            trusted: false,
            enabled: row.get::<_, i64>(12).ok()? != 0,
            quarantine: if quarantined != 0 {
                SkillToolQuarantine::Quarantined {
                    reason: quarantine_reason.unwrap_or_default(),
                }
            } else {
                SkillToolQuarantine::Clear
            },
            consecutive_failures: u32::try_from(row.get::<_, i64>(15).ok()?).ok()?,
        },
        validation_code: row.get(11).ok()?,
        diagnostics: SkillToolDiagnosticSummary::from_storage_value(
            &row.get::<_, String>(16).ok()?,
        ),
        created_at: row.get(17).ok()?,
        updated_at: row.get(18).ok()?,
    })
}

fn read_trust_record(row: &Row<'_>) -> Option<SkillToolTrustRecord> {
    Some(SkillToolTrustRecord {
        revision: SkillToolRevision::parse(&row.get::<_, String>(0).ok()?).ok()?,
        integrity: SkillToolIntegrity {
            base_revision: row.get(1).ok()?,
            manifest_hash: ContentHash::parse(&row.get::<_, String>(2).ok()?).ok()?,
            implementation_hash: ContentHash::parse(&row.get::<_, String>(3).ok()?).ok()?,
            capability_digest: row.get(4).ok()?,
        },
        decision: SkillToolTrustDecision::parse(&row.get::<_, String>(5).ok()?)?,
        actor: row.get(6).ok()?,
        decided_at: row.get(7).ok()?,
    })
}

fn storage(error: rusqlite::Error) -> SkillToolApplicationError {
    SkillToolApplicationError::Storage(error.to_string())
}

/// Resolves persisted trust into the lifecycle's `trusted` flag.
///
/// Kept out of the row reader because trust is only meaningful against the observed integrity:
/// the stored decision alone must never be projected as "trusted" without re-checking what it was
/// bound to.
pub(crate) fn apply_trust(
    state: &mut SkillToolRevisionState,
    record: Option<&SkillToolTrustRecord>,
) {
    state.lifecycle.trusted =
        record.is_some_and(|record| record.authorizes(&state.key.revision, &state.integrity));
}
