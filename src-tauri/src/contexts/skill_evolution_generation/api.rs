//! Sanitized native facade for generation policy, jobs, and evidence projections.

mod export;
mod queries;
#[cfg(test)]
mod tests;

use crate::{
    contexts::agent_runtime::{
        application::ApiCredentialPort, infrastructure::OsApiCredentialAdapter,
    },
    contexts::skill_evolution_generation::{
        application::{
            update_generation_policy, GenerationPolicyChangeSource, GenerationPolicyChangeV1,
            GenerationPolicyError, GenerationPolicyPort,
        },
        domain::{
            DossierSectionPageRequest, GeneratedArtifactKind, GenerationConsentState,
            GenerationPolicyV1, GenerationProviderReadinessV1,
        },
        infrastructure::{GenerationDossierQuery, SqliteGenerationPolicyRepository},
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationApiError {
    InvalidRequest,
    NotFound,
    Conflict,
    ProviderUnavailable,
    CuratorUnavailable,
    Immutable,
    Storage,
}

impl GenerationApiError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "generation-invalid-request",
            Self::NotFound => "generation-not-found",
            Self::Conflict => "generation-conflict",
            Self::ProviderUnavailable => "generation-provider-unavailable",
            Self::CuratorUnavailable => "generation-curator-unavailable",
            Self::Immutable => "generation-job-immutable",
            Self::Storage => "generation-storage-failed",
        }
    }
}

#[derive(Clone)]
pub(crate) struct SkillEvolutionGenerationApi {
    database: NativeDatabase,
    policy: SqliteGenerationPolicyRepository,
    credentials: OsApiCredentialAdapter,
}

impl SkillEvolutionGenerationApi {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self {
            policy: SqliteGenerationPolicyRepository::new(database.clone()),
            credentials: OsApiCredentialAdapter::new(),
            database,
        }
    }

    pub(crate) fn policy(&self, workspace_id: &str) -> Result<Value, GenerationApiError> {
        validate_id(workspace_id)?;
        let policy = self
            .policy
            .load(workspace_id)
            .map_err(map_policy_error)?
            .unwrap_or_else(|| GenerationPolicyV1::default_disabled(workspace_id.into()));
        Ok(policy_json(&policy))
    }

    pub(crate) fn update_policy(
        &self,
        input: &GenerationPolicyUpdate<'_>,
        now_ms: i64,
    ) -> Result<Value, GenerationApiError> {
        validate_id(input.workspace_id)?;
        let readiness = input
            .enabled
            .then(|| self.provider_readiness(input))
            .transpose()?;
        let state = if input.enabled {
            GenerationConsentState::Enabled
        } else {
            GenerationConsentState::Disabled
        };
        let policy = update_generation_policy(
            &self.policy,
            &GenerationPolicyChangeV1 {
                workspace_id: input.workspace_id,
                expected_revision: input.expected_revision,
                requested_state: state,
                disclosure_acknowledgement: Some(input.disclosure_version),
                source: GenerationPolicyChangeSource::LocalInteractiveUser,
                allowed_artifact_kinds: Some(input.allowed_artifact_kinds),
            },
            readiness.as_ref(),
            now_ms,
        )
        .map_err(map_policy_error)?;
        Ok(policy_json(&policy))
    }

    pub(crate) fn dossier_section(
        &self,
        dossier_id: &str,
        ordinal: u8,
        cursor: Option<&str>,
        limit: u16,
    ) -> Result<Value, GenerationApiError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let query = GenerationDossierQuery::new(&connection);
        let page = query
            .section_page(&DossierSectionPageRequest {
                dossier_id,
                ordinal,
                cursor,
                limit,
            })
            .map_err(map_persistence_error)?;
        let links = query
            .source_links(dossier_id, None, 50)
            .map_err(map_persistence_error)?;
        let mut value = serde_json::to_value(page).map_err(|_| GenerationApiError::Storage)?;
        value
            .as_object_mut()
            .ok_or(GenerationApiError::Storage)?
            .insert(
                "sourceLinks".into(),
                serde_json::to_value(links.links).map_err(|_| GenerationApiError::Storage)?,
            );
        Ok(value)
    }

    pub(crate) fn cancel_job(
        &self,
        job_id: &str,
        now_ms: i64,
    ) -> Result<Value, GenerationApiError> {
        validate_id(job_id)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let changed = connection.execute(
            "UPDATE evolution_generation_jobs SET status='cancel_requested',cancel_requested_at_ms=?1,updated_at_ms=?1,revision=revision+1 WHERE job_id=?2 AND status IN ('requested','queued','running')",
            params![now_ms, job_id],
        ).map_err(|_| GenerationApiError::Storage)?;
        if changed == 0 {
            return match self.job_detail(job_id)? {
                Some(value) if value["status"] == "cancel_requested" => Ok(value),
                Some(_) => Err(GenerationApiError::Immutable),
                None => Err(GenerationApiError::NotFound),
            };
        }
        self.job_detail(job_id)?.ok_or(GenerationApiError::Storage)
    }

    pub(crate) fn handoff(&self, job_id: &str) -> Result<Value, GenerationApiError> {
        let detail = self
            .job_detail(job_id)?
            .ok_or(GenerationApiError::NotFound)?;
        if detail["status"] != "completed" {
            return Err(GenerationApiError::Immutable);
        }
        if !matches!(
            detail["handoffStatus"].as_str(),
            Some("delivered" | "duplicate")
        ) {
            return Err(GenerationApiError::CuratorUnavailable);
        }
        Ok(detail)
    }

    fn provider_readiness(
        &self,
        input: &GenerationPolicyUpdate<'_>,
    ) -> Result<GenerationProviderReadinessV1, GenerationApiError> {
        let profile_id = input
            .provider_profile_id
            .ok_or(GenerationApiError::ProviderUnavailable)?;
        let model_id = input
            .model_id
            .ok_or(GenerationApiError::ProviderUnavailable)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let row = connection.query_row(
            "SELECT interface_format,authentication_mode FROM onepiece_provider_profiles WHERE id=?1 AND model_id=?2 AND active=1 AND structured_output_capability='supported'",
            params![profile_id, model_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ).optional().map_err(|_| GenerationApiError::Storage)?
            .ok_or(GenerationApiError::ProviderUnavailable)?;
        let credentials_available = if row.1 == "required" {
            let scoped_key = format!("onepiece-profile:{profile_id}");
            self.credentials
                .fetch(&scoped_key)
                .map_err(|_| GenerationApiError::ProviderUnavailable)?
                .or(self
                    .credentials
                    .fetch("onepiece")
                    .map_err(|_| GenerationApiError::ProviderUnavailable)?)
                .is_some()
        } else {
            true
        };
        Ok(GenerationProviderReadinessV1 {
            profile_id: profile_id.into(),
            model_id: model_id.into(),
            provider_protocol: if row.0 == "anthropic" {
                "anthropic_messages"
            } else {
                "openai_responses"
            }
            .into(),
            enabled: true,
            credentials_available,
            structured_json_supported: true,
        })
    }
}

pub(crate) struct GenerationPolicyUpdate<'a> {
    pub(crate) workspace_id: &'a str,
    pub(crate) expected_revision: u64,
    pub(crate) enabled: bool,
    pub(crate) disclosure_version: &'a str,
    pub(crate) provider_profile_id: Option<&'a str>,
    pub(crate) model_id: Option<&'a str>,
    pub(crate) allowed_artifact_kinds: &'a [GeneratedArtifactKind],
}

fn policy_json(policy: &GenerationPolicyV1) -> Value {
    json!({
        "workspaceId": policy.workspace_id, "enabled": policy.consent_state == GenerationConsentState::Enabled,
        "disclosureVersion": policy.disclosure_version, "providerProfileId": policy.provider_profile_id,
        "modelId": policy.model_id, "allowedArtifactKinds": policy.allowed_artifact_kinds,
        "dailyModelCalls": policy.job_budget.model_calls,
        "dailyInputTokens": policy.daily_budget.input_tokens, "dailyOutputTokens": policy.daily_budget.output_tokens,
        "failedCancelledRetentionDays": policy.retention.failed_job_days,
        "completedPackageRetentionDays": policy.retention.completed_package_days,
        "revision": policy.revision, "policyHash": policy.policy_hash,
    })
}

fn validate_id(value: &str) -> Result<(), GenerationApiError> {
    if value.trim().is_empty() || value.len() > 512 {
        Err(GenerationApiError::InvalidRequest)
    } else {
        Ok(())
    }
}

fn map_policy_error(error: GenerationPolicyError) -> GenerationApiError {
    match error {
        GenerationPolicyError::Conflict | GenerationPolicyError::StalePolicy => {
            GenerationApiError::Conflict
        }
        GenerationPolicyError::ProviderUnavailable => GenerationApiError::ProviderUnavailable,
        GenerationPolicyError::Storage | GenerationPolicyError::Serialization => {
            GenerationApiError::Storage
        }
        _ => GenerationApiError::InvalidRequest,
    }
}

fn map_persistence_error(
    error: crate::contexts::skill_evolution_generation::infrastructure::GenerationPersistenceError,
) -> GenerationApiError {
    use crate::contexts::skill_evolution_generation::infrastructure::GenerationPersistenceError;
    match error {
        GenerationPersistenceError::InvalidInput => GenerationApiError::InvalidRequest,
        GenerationPersistenceError::Conflict => GenerationApiError::Conflict,
        GenerationPersistenceError::Immutable => GenerationApiError::Immutable,
        GenerationPersistenceError::Storage => GenerationApiError::Storage,
    }
}

pub(super) fn stable_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    format!(
        "{prefix}-{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
