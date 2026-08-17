use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolOwnerInput {
    pub(crate) skill_id: String,
    pub(crate) scope: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolRevisionInput {
    pub(crate) revision: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolTrustInput {
    pub(crate) revision: String,
    pub(crate) trusted: bool,
    pub(crate) actor: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolEnablementInput {
    pub(crate) revision: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolQuarantineInput {
    pub(crate) revision: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolDiagnosticDto {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolRevisionDto {
    pub(crate) skill_id: String,
    pub(crate) tool_id: String,
    pub(crate) canonical_id: String,
    pub(crate) revision: String,
    pub(crate) source_scope: String,
    pub(crate) workspace_path: Option<String>,
    pub(crate) implementation_kind: String,
    pub(crate) base_revision: String,
    pub(crate) manifest_hash: String,
    pub(crate) implementation_hash: String,
    pub(crate) capability_digest: String,
    pub(crate) capability_diff: Option<SkillToolCapabilityDiffDto>,
    pub(crate) validation: String,
    pub(crate) validation_code: Option<String>,
    pub(crate) trusted: bool,
    pub(crate) enabled: bool,
    pub(crate) quarantined: bool,
    pub(crate) quarantine_reason: Option<String>,
    pub(crate) consecutive_failures: u32,
    pub(crate) diagnostics: Vec<SkillToolDiagnosticDto>,
    pub(crate) runtime_support: String,
    pub(crate) enforcement_strength: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillToolCapabilityDiffDto {
    pub(crate) previous_digest: Option<String>,
    pub(crate) current_digest: String,
    pub(crate) added: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) changed: bool,
}
