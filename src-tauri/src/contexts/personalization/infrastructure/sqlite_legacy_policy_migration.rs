use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, TransactionBehavior};

use crate::contexts::personalization::application::{
    LegacyPolicyMigrationPort, MigratedPolicy, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    AgentId, PersonalizationPolicyRecord, WorkspaceKey,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Bumped when policy migration completes. Non-zero means "the dedicated policy is authoritative";
/// zero means the legacy fields have not been read yet.
const POLICY_MIGRATION_GENERATION: i64 = 1;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

/// Commits the legacy policy migration atomically.
#[derive(Clone)]
pub(crate) struct SqliteLegacyPolicyMigration {
    database: NativeDatabase,
}

impl SqliteLegacyPolicyMigration {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

fn migration_generation(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT generation FROM personalization_migration_state WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .map_err(storage)
}

/// Writes one policy row inside the caller's transaction.
///
/// Deliberately not the repository's `patch`: that opens its own transaction, and a nested one
/// would either fail or silently commit half of this migration. Migration writes rows at their
/// initial revision rather than patching a base, so there is no expected-revision check to share.
fn insert_policy_row(
    conn: &Connection,
    record: &PersonalizationPolicyRecord,
    now: &str,
) -> Result<()> {
    let scope = record.scope();
    let scope_key = scope.scope_key();
    conn.execute(
        "INSERT INTO personalization_policy_overrides (
             id, policy_set_id, scope_key, scope_kind, workspace_key, agent_id,
             instruction_merge_mode, about_user, style_rules, memory_read_mode,
             explicit_save_mode, automatic_extraction_mode, global_memory_access_mode,
             revision, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)
         ON CONFLICT(scope_key) DO UPDATE SET
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
            format!("{}::{scope_key}", record.policy_set_id()),
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

impl LegacyPolicyMigrationPort for SqliteLegacyPolicyMigration {
    fn is_complete(&self) -> Result<bool> {
        let conn = self.connection()?;
        Ok(migration_generation(&conn)? >= POLICY_MIGRATION_GENERATION)
    }

    fn commit(&self, migrated: &MigratedPolicy, now: DateTime<Utc>) -> Result<bool> {
        let mut conn = self.connection()?;
        // IMMEDIATE so the "already migrated?" check and the write cannot be separated by another
        // process doing the same thing — two concurrent first startups would otherwise both read
        // generation 0 and both migrate.
        let transaction = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;

        if migration_generation(&transaction)? >= POLICY_MIGRATION_GENERATION {
            // Already done. Rolling back rather than committing keeps a repeated startup a true
            // no-op instead of a rewrite that would reset revisions the user has since advanced.
            return Ok(false);
        }

        insert_policy_row(&transaction, &migrated.global, &timestamp(now))?;
        if let Some(override_record) = migrated.onepiece_override.as_ref() {
            insert_policy_row(&transaction, override_record, &timestamp(now))?;
        }

        let changed = transaction
            .execute(
                "UPDATE personalization_migration_state
                 SET generation = ?1, started_at = COALESCE(started_at, ?2), completed_at = ?2,
                     last_error_code = NULL
                 WHERE id = 1 AND generation < ?1",
                params![POLICY_MIGRATION_GENERATION, timestamp(now)],
            )
            .map_err(storage)?;
        if changed == 0 {
            // The marker row is missing or already advanced. Committing the policy rows without a
            // marker would make the next startup migrate again over data that had already moved.
            return Err(PersonalizationApplicationError::Storage(
                "the personalization migration marker could not be advanced".to_string(),
            ));
        }

        transaction.commit().map_err(storage)?;
        Ok(true)
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
