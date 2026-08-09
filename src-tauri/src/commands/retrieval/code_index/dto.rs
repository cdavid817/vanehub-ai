use crate::contexts::retrieval::domain::{
    CodeEmbeddingConfirmation, CodeIndexAuditEntry, CodeIndexConfigurationUpdate, CodeIndexMode,
    CodeIndexStatus, CodeLanguage, CodeWorkspace, RetrievalError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeIndexConfigurationInput {
    pub(crate) enabled: bool,
    pub(crate) mode: String,
    pub(crate) selected_roots: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) exclusion_patterns: Vec<String>,
    pub(crate) max_file_bytes: u64,
}

impl TryFrom<CodeIndexConfigurationInput> for CodeIndexConfigurationUpdate {
    type Error = RetrievalError;

    fn try_from(input: CodeIndexConfigurationInput) -> Result<Self, Self::Error> {
        let languages = input
            .languages
            .iter()
            .map(|language| {
                CodeLanguage::parse(language).ok_or_else(|| {
                    RetrievalError::Validation("unsupported code language".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mode = CodeIndexMode::parse(&input.mode)
            .ok_or_else(|| RetrievalError::Validation("unsupported code index mode".to_string()))?;
        CodeIndexConfigurationUpdate {
            enabled: input.enabled,
            mode,
            selected_roots: input.selected_roots,
            languages,
            exclusion_patterns: input.exclusion_patterns,
            max_file_bytes: input.max_file_bytes,
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeIndexStatusDto {
    pub(crate) phase: String,
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
    pub(crate) last_failure_category: Option<String>,
    pub(crate) updated_at: String,
}

impl From<CodeIndexStatus> for CodeIndexStatusDto {
    fn from(status: CodeIndexStatus) -> Self {
        Self {
            phase: status.phase.as_str().to_string(),
            total_files: status.total_files,
            processed_files: status.processed_files,
            failed_files: status.failed_files,
            total_chunks: status.total_chunks,
            processed_chunks: status.processed_chunks,
            pending_chunks: status.pending_chunks,
            indexed_chunks: status.indexed_chunks,
            failed_chunks: status.failed_chunks,
            redaction_count: status.redaction_count,
            estimated_embedding_requests: status.estimated_embedding_requests,
            last_failure_category: status
                .last_failure_category
                .map(|category| category.as_str().to_string()),
            updated_at: status.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeIndexWorkspaceDto {
    pub(crate) workspace_id: String,
    pub(crate) canonical_root: String,
    pub(crate) display_name: String,
    pub(crate) origin: String,
    pub(crate) enabled: bool,
    pub(crate) mode: String,
    pub(crate) selected_roots: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) exclusion_patterns: Vec<String>,
    pub(crate) max_file_bytes: u64,
    pub(crate) index_version: String,
    pub(crate) generation: u64,
    pub(crate) status: CodeIndexStatusDto,
}

impl CodeIndexWorkspaceDto {
    pub(crate) fn new(workspace: CodeWorkspace, status: CodeIndexStatus) -> Self {
        Self {
            workspace_id: workspace.workspace_id,
            canonical_root: workspace.canonical_root,
            display_name: workspace.display_name,
            origin: workspace.origin.as_str().to_string(),
            enabled: workspace.enabled,
            mode: workspace.mode.as_str().to_string(),
            selected_roots: workspace.selected_roots,
            languages: workspace.languages,
            exclusion_patterns: workspace.exclusion_patterns,
            max_file_bytes: workspace.max_file_bytes,
            index_version: workspace.index_version,
            generation: workspace.generation,
            status: status.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeEmbeddingConfirmationDto {
    pub(crate) profile_id: String,
    pub(crate) model: String,
    pub(crate) generation: u64,
}

impl From<CodeEmbeddingConfirmation> for CodeEmbeddingConfirmationDto {
    fn from(value: CodeEmbeddingConfirmation) -> Self {
        Self {
            profile_id: value.profile_id,
            model: value.model,
            generation: value.generation,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeIndexAuditDto {
    pub(crate) audit_id: u64,
    pub(crate) workspace_id: String,
    pub(crate) relative_path: Option<String>,
    pub(crate) event: String,
    pub(crate) reason: Option<String>,
    pub(crate) item_count: u64,
    pub(crate) created_at: String,
}

impl From<CodeIndexAuditEntry> for CodeIndexAuditDto {
    fn from(value: CodeIndexAuditEntry) -> Self {
        Self {
            audit_id: value.audit_id,
            workspace_id: value.workspace_id,
            relative_path: value.relative_path,
            event: value.event.as_str().to_string(),
            reason: value.reason.map(|reason| reason.as_str().to_string()),
            item_count: value.item_count,
            created_at: value.created_at,
        }
    }
}

pub(crate) fn safe_error(error: RetrievalError) -> String {
    error.category().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configuration(languages: &[&str]) -> CodeIndexConfigurationInput {
        CodeIndexConfigurationInput {
            enabled: true,
            mode: "local".to_string(),
            selected_roots: vec!["src".to_string()],
            languages: languages
                .iter()
                .map(|language| (*language).to_string())
                .collect(),
            exclusion_patterns: vec!["**/*.generated.ts".to_string()],
            max_file_bytes: 100 * 1024,
        }
    }

    #[test]
    fn configuration_input_rejects_unknown_languages() {
        let error = CodeIndexConfigurationUpdate::try_from(configuration(&["rust", "brainfuck"]))
            .expect_err("unknown languages must not cross the command boundary");

        assert_eq!(safe_error(error), "validation");
    }

    #[test]
    fn command_errors_only_expose_categories() {
        let error = RetrievalError::Storage(
            "database path and statement details must remain private".to_string(),
        );

        assert_eq!(safe_error(error), "storage");
    }
}
