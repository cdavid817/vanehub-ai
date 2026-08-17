#![allow(dead_code)]

use serde::{Deserialize, Serialize};

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }

        impl $name {
            pub(crate) const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }

            pub(crate) fn parse(value: &str) -> Option<Self> {
                match value { $($value => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

string_enum!(StoredToolOperationStatus {
    Queued => "queued",
    AwaitingApproval => "awaiting_approval",
    Running => "running",
    AwaitingHuman => "awaiting_human",
    Succeeded => "succeeded",
    Failed => "failed",
    Cancelled => "cancelled",
});

string_enum!(DelegationTarget {
    ClaudeCode => "claude_code",
    CodexCli => "codex_cli",
});

string_enum!(DelegationMode {
    Analyze => "analyze",
    Edit => "edit",
});

string_enum!(DelegationStatus {
    Queued => "queued",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
    Cancelled => "cancelled",
});

string_enum!(ChangeSetStatus {
    AwaitingApproval => "awaiting_approval",
    Preflighting => "preflighting",
    Applying => "applying",
    Verifying => "verifying",
    Succeeded => "succeeded",
    RolledBack => "rolled_back",
    ManualRecoveryRequired => "manual_recovery_required",
    Failed => "failed",
});

string_enum!(RecoveryStatus {
    NotRequired => "not_required",
    RolledBack => "rolled_back",
    ManualRecoveryRequired => "manual_recovery_required",
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredToolOperation {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) tool_name: String,
    pub(crate) status: StoredToolOperationStatus,
    pub(crate) progress_sequence: u32,
    pub(crate) progress_message: Option<String>,
    pub(crate) result_artifact_ids: Vec<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactRecord {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) content_hash: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) display_name: String,
    pub(crate) source_operation_id: Option<String>,
    pub(crate) source_artifact_ids: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) expires_at: Option<String>,
    pub(crate) publication_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DelegationRecord {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) task_hash: String,
    pub(crate) status: DelegationStatus,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DelegationAttemptRecord {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) delegation_id: String,
    pub(crate) attempt_number: u8,
    pub(crate) target: DelegationTarget,
    pub(crate) mode: DelegationMode,
    pub(crate) status: DelegationStatus,
    pub(crate) safe_summary: Option<String>,
    pub(crate) report_artifact_id: Option<String>,
    pub(crate) change_set_artifact_id: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) started_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileChangeKind {
    Add,
    Modify,
    Delete,
    Rename,
}

impl FileChangeKind {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Delete => "delete",
            Self::Rename => "rename",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "add" => Some(Self::Add),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            "rename" => Some(Self::Rename),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeSetFileRecord {
    pub(crate) path: String,
    pub(crate) change_kind: FileChangeKind,
    pub(crate) old_hash: Option<String>,
    pub(crate) new_hash: Option<String>,
    pub(crate) binary: bool,
    pub(crate) mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeSetRecord {
    pub(crate) contract_version: u16,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) repository_identity: String,
    pub(crate) base_commit: String,
    pub(crate) attempt_id: String,
    pub(crate) files: Vec<ChangeSetFileRecord>,
    pub(crate) warnings: Vec<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeSetApplyRecord {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) change_set_artifact_id: String,
    pub(crate) target_repository_identity: String,
    pub(crate) expected_base_commit: String,
    pub(crate) approval_input_hash: String,
    pub(crate) status: ChangeSetStatus,
    pub(crate) error_code: Option<String>,
    pub(crate) consumed_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryRecord {
    pub(crate) contract_version: u16,
    pub(crate) apply_attempt_id: String,
    pub(crate) status: RecoveryStatus,
    pub(crate) recovery_reference: Option<String>,
    pub(crate) safe_instructions: Vec<String>,
    pub(crate) updated_at: String,
}

pub(crate) trait NativeToolPersistencePort: Send + Sync {
    fn save_delegation(&self, record: &DelegationRecord) -> Result<(), ()>;
    fn save_delegation_attempt(&self, record: &DelegationAttemptRecord) -> Result<(), ()>;
    fn insert_change_set(&self, record: &ChangeSetRecord) -> Result<(), ()>;
    fn save_apply_attempt(&self, record: &ChangeSetApplyRecord) -> Result<(), ()>;
    fn is_change_set_available(&self, artifact_id: &str) -> Result<bool, ()>;
    fn save_recovery(&self, record: &RecoveryRecord) -> Result<(), ()>;
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
