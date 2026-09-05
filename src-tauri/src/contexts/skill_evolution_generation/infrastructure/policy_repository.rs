use crate::{
    contexts::skill_evolution_generation::{
        application::{policy_integrity_is_valid, GenerationPolicyError, GenerationPolicyPort},
        domain::{GenerationConsentState, GenerationPolicyV1},
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub(crate) struct SqliteGenerationPolicyRepository {
    database: NativeDatabase,
}

impl SqliteGenerationPolicyRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl GenerationPolicyPort for SqliteGenerationPolicyRepository {
    fn load(
        &self,
        workspace_id: &str,
    ) -> Result<Option<GenerationPolicyV1>, GenerationPolicyError> {
        if workspace_id.trim().is_empty() {
            return Err(GenerationPolicyError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationPolicyError::Storage)?;
        let stored = connection
            .query_row(
                "SELECT consent_state,policy_json,policy_hash,revision,updated_at_ms
                 FROM evolution_generation_policy WHERE workspace_id=?1",
                [workspace_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| GenerationPolicyError::Storage)?;
        let Some((state, json, hash, revision, updated_at_ms)) = stored else {
            return Ok(None);
        };
        let policy: GenerationPolicyV1 =
            serde_json::from_str(&json).map_err(|_| GenerationPolicyError::Storage)?;
        if consent_state_name(policy.consent_state) != state
            || policy.policy_hash != hash
            || policy.revision
                != u64::try_from(revision).map_err(|_| GenerationPolicyError::Storage)?
            || policy.updated_at_ms != updated_at_ms
            || policy.workspace_id != workspace_id
            || !policy_integrity_is_valid(&policy)
        {
            return Err(GenerationPolicyError::Storage);
        }
        Ok(Some(policy))
    }

    fn save(
        &self,
        policy: &GenerationPolicyV1,
        expected_revision: u64,
    ) -> Result<(), GenerationPolicyError> {
        if policy.revision != expected_revision + 1 || !policy_integrity_is_valid(policy) {
            return Err(GenerationPolicyError::InvalidInput);
        }
        let revision =
            i64::try_from(policy.revision).map_err(|_| GenerationPolicyError::InvalidInput)?;
        let expected =
            i64::try_from(expected_revision).map_err(|_| GenerationPolicyError::InvalidInput)?;
        let policy_json =
            super::canonical_json(policy).map_err(|_| GenerationPolicyError::Serialization)?;
        let job_budget = super::canonical_json(&policy.job_budget)
            .map_err(|_| GenerationPolicyError::Serialization)?;
        let daily_budget = super::canonical_json(&policy.daily_budget)
            .map_err(|_| GenerationPolicyError::Serialization)?;
        let retention = super::canonical_json(&policy.retention)
            .map_err(|_| GenerationPolicyError::Serialization)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationPolicyError::Storage)?;
        let changed = if expected_revision == 0 {
            connection.execute(
                "INSERT INTO evolution_generation_policy
                 (workspace_id,schema_version,consent_state,disclosure_version,provider_profile_id,
                  job_budget_json,daily_budget_json,retention_json,policy_json,policy_hash,revision,updated_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![policy.workspace_id,policy.schema_version,consent_state_name(policy.consent_state),
                    policy.disclosure_version,policy.provider_profile_id,job_budget,daily_budget,retention,
                    policy_json,policy.policy_hash,revision,policy.updated_at_ms],
            )
        } else {
            connection.execute(
                "UPDATE evolution_generation_policy SET consent_state=?1,disclosure_version=?2,
                 provider_profile_id=?3,job_budget_json=?4,daily_budget_json=?5,retention_json=?6,
                 policy_json=?7,policy_hash=?8,revision=?9,updated_at_ms=?10
                 WHERE workspace_id=?11 AND revision=?12",
                params![
                    consent_state_name(policy.consent_state),
                    policy.disclosure_version,
                    policy.provider_profile_id,
                    job_budget,
                    daily_budget,
                    retention,
                    policy_json,
                    policy.policy_hash,
                    revision,
                    policy.updated_at_ms,
                    policy.workspace_id,
                    expected
                ],
            )
        };
        match changed {
            Ok(1) => Ok(()),
            Ok(_) => Err(GenerationPolicyError::Conflict),
            Err(error)
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                Err(GenerationPolicyError::Conflict)
            }
            Err(_) => Err(GenerationPolicyError::Storage),
        }
    }
}

fn consent_state_name(state: GenerationConsentState) -> &'static str {
    match state {
        GenerationConsentState::Disabled => "disabled",
        GenerationConsentState::Enabled => "enabled",
        GenerationConsentState::Revoked => "revoked",
        GenerationConsentState::DisclosureStale => "disclosure_stale",
    }
}
