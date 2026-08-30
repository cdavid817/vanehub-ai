use serde::{Deserialize, Serialize};

use super::{GeneratedArtifactKind, GenerationBudgetV1, GenerationConsentState};

pub(crate) const GENERATION_DISCLOSURE_VERSION_V1: &str = "generation-disclosure-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationPolicyV1 {
    pub(crate) schema_version: u16,
    pub(crate) workspace_id: String,
    pub(crate) consent_state: GenerationConsentState,
    pub(crate) disclosure_version: String,
    pub(crate) provider_profile_id: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) allowed_artifact_kinds: Vec<GeneratedArtifactKind>,
    pub(crate) job_budget: GenerationBudgetV1,
    pub(crate) daily_budget: GenerationDailyBudgetV1,
    pub(crate) retention: GenerationRetentionPolicyV1,
    pub(crate) consent_hash: String,
    pub(crate) policy_hash: String,
    pub(crate) revision: u64,
    pub(crate) updated_at_ms: i64,
}

impl GenerationPolicyV1 {
    pub(crate) fn default_disabled(workspace_id: String) -> Self {
        Self {
            schema_version: 1,
            workspace_id,
            consent_state: GenerationConsentState::Disabled,
            disclosure_version: GENERATION_DISCLOSURE_VERSION_V1.into(),
            provider_profile_id: None,
            model_id: None,
            allowed_artifact_kinds: vec![
                GeneratedArtifactKind::OverlayLearnBlock,
                GeneratedArtifactKind::OverlayExactPatch,
                GeneratedArtifactKind::NewSkill,
            ],
            job_budget: GenerationBudgetV1 {
                wall_time_ms: 180_000,
                model_calls: 3,
                tool_calls: 8,
                input_tokens: 48_000,
                output_tokens: 8_000,
                validation_repairs: 1,
            },
            daily_budget: GenerationDailyBudgetV1 {
                input_tokens: 250_000,
                output_tokens: 50_000,
                concurrent_workspace_jobs: 1,
                concurrent_global_jobs: 2,
            },
            retention: GenerationRetentionPolicyV1 {
                failed_job_days: 180,
                completed_package_days: 365,
            },
            consent_hash: String::new(),
            policy_hash: String::new(),
            revision: 0,
            updated_at_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationDailyBudgetV1 {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) concurrent_workspace_jobs: u8,
    pub(crate) concurrent_global_jobs: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationRetentionPolicyV1 {
    pub(crate) failed_job_days: u16,
    pub(crate) completed_package_days: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationProviderReadinessV1 {
    pub(crate) profile_id: String,
    pub(crate) model_id: String,
    pub(crate) provider_protocol: String,
    pub(crate) enabled: bool,
    pub(crate) credentials_available: bool,
    pub(crate) structured_json_supported: bool,
}
