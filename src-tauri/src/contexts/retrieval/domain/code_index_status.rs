use super::{content_hash, CodeIndexPhase, FailureCategory};

pub(crate) fn code_embedding_identity(profile_id: &str, model: &str) -> String {
    let material = format!("{}:{profile_id}{}:{model}", profile_id.len(), model.len());
    format!("workspace-code:{}", content_hash(&material))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeEmbeddingConfirmation {
    pub(crate) profile_id: String,
    pub(crate) model: String,
    pub(crate) generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeIndexStatus {
    pub(crate) phase: CodeIndexPhase,
    pub(crate) total_files: u64,
    pub(crate) processed_files: u64,
    pub(crate) failed_files: u64,
    pub(crate) total_chunks: u64,
    pub(crate) processed_chunks: u64,
    pub(crate) pending_chunks: u64,
    pub(crate) indexed_chunks: u64,
    pub(crate) failed_chunks: u64,
    pub(crate) redaction_count: u64,
    pub(crate) estimated_embedding_requests: u64,
    pub(crate) last_failure_category: Option<FailureCategory>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeIndexAuditEvent {
    Admitted,
    Skipped,
    Indexed,
    Failed,
    Deleted,
    Rebuilt,
}

impl CodeIndexAuditEvent {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Skipped => "skipped",
            Self::Indexed => "indexed",
            Self::Failed => "failed",
            Self::Deleted => "deleted",
            Self::Rebuilt => "rebuilt",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "admitted" => Some(Self::Admitted),
            "skipped" => Some(Self::Skipped),
            "indexed" => Some(Self::Indexed),
            "failed" => Some(Self::Failed),
            "deleted" => Some(Self::Deleted),
            "rebuilt" => Some(Self::Rebuilt),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeIndexAuditReason {
    OutsideSelectedRoots,
    SensitiveFile,
    UserExcluded,
    LanguageDisabled,
    SizeLimit,
    Binary,
    Unreadable,
    Parse,
    Auth,
    InvalidRequest,
    RateLimit,
    Network,
    StaleGeneration,
}

impl CodeIndexAuditReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::OutsideSelectedRoots => "outside_selected_roots",
            Self::SensitiveFile => "sensitive_file",
            Self::UserExcluded => "user_excluded",
            Self::LanguageDisabled => "language_disabled",
            Self::SizeLimit => "size_limit",
            Self::Binary => "binary",
            Self::Unreadable => "unreadable",
            Self::Parse => "parse",
            Self::Auth => "auth",
            Self::InvalidRequest => "invalid_request",
            Self::RateLimit => "rate_limit",
            Self::Network => "network",
            Self::StaleGeneration => "stale_generation",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "outside_selected_roots" => Some(Self::OutsideSelectedRoots),
            "sensitive_file" => Some(Self::SensitiveFile),
            "user_excluded" => Some(Self::UserExcluded),
            "language_disabled" => Some(Self::LanguageDisabled),
            "size_limit" => Some(Self::SizeLimit),
            "binary" => Some(Self::Binary),
            "unreadable" => Some(Self::Unreadable),
            "parse" => Some(Self::Parse),
            "auth" => Some(Self::Auth),
            "invalid_request" => Some(Self::InvalidRequest),
            "rate_limit" => Some(Self::RateLimit),
            "network" => Some(Self::Network),
            "stale_generation" => Some(Self::StaleGeneration),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeIndexAuditEntry {
    pub(crate) audit_id: u64,
    pub(crate) workspace_id: String,
    pub(crate) relative_path: Option<String>,
    pub(crate) event: CodeIndexAuditEvent,
    pub(crate) reason: Option<CodeIndexAuditReason>,
    pub(crate) item_count: u64,
    pub(crate) created_at: String,
}
