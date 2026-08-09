use std::sync::Arc;

use super::indexing_service::IndexGenerationGuard;
use super::{
    BatchOutcome, CodeIndexRepository, EmbeddingPort, IndexSourcePort, IndexingService,
    RetrievalDocumentRepository,
};
use crate::contexts::retrieval::domain::{
    code_embedding_identity, CodeEmbeddingConfirmation, CodeIndexPhase, RetrievalError,
    RetrievalScope, SourceKind,
};

pub(crate) fn prepare_code_embedding(
    repository: &dyn CodeIndexRepository,
    workspace_id: &str,
    profile_id: &str,
    model: &str,
    generation: u64,
) -> Result<bool, RetrievalError> {
    let confirmed = repository
        .embedding_confirmation(workspace_id)?
        .is_some_and(|confirmation| {
            confirmation.profile_id == profile_id
                && confirmation.model == model
                && confirmation.generation == generation
        });
    repository.set_workspace_phase(
        workspace_id,
        if confirmed {
            CodeIndexPhase::Embedding
        } else {
            CodeIndexPhase::AwaitingEmbeddingConfirmation
        },
    )?;
    Ok(confirmed)
}

pub(crate) fn confirm_code_embedding(
    repository: &dyn CodeIndexRepository,
    workspace_id: &str,
    profile_id: &str,
    model: &str,
    generation: u64,
) -> Result<CodeEmbeddingConfirmation, RetrievalError> {
    repository.confirm_embedding(workspace_id, profile_id, model, generation)
}

pub(crate) struct WorkspaceCodeEmbeddingService {
    indexing: IndexingService,
    request_model: String,
    embedding_identity: String,
}

impl WorkspaceCodeEmbeddingService {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        documents: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
        embeddings: Arc<dyn EmbeddingPort>,
        code_index: Arc<dyn CodeIndexRepository>,
        workspace_id: String,
        generation: u64,
        profile_id: String,
        model: String,
    ) -> Result<Self, RetrievalError> {
        let guard = Arc::new(WorkspaceEmbeddingGuard {
            repository: code_index,
            workspace_id: workspace_id.clone(),
            generation,
            profile_id: profile_id.clone(),
            model: model.clone(),
        });
        Ok(Self {
            indexing: IndexingService::new_scoped_with_guard(
                documents,
                source,
                embeddings,
                SourceKind::WorkspaceFile,
                RetrievalScope::Workspace(workspace_id),
                guard,
            )?,
            embedding_identity: code_embedding_identity(&profile_id, &model),
            request_model: model,
        })
    }

    pub(crate) fn process_pending_batch(&self) -> Result<BatchOutcome, RetrievalError> {
        self.indexing
            .process_pending_batch_with_identity(&self.request_model, &self.embedding_identity)
    }
}

struct WorkspaceEmbeddingGuard {
    repository: Arc<dyn CodeIndexRepository>,
    workspace_id: String,
    generation: u64,
    profile_id: String,
    model: String,
}

impl IndexGenerationGuard for WorkspaceEmbeddingGuard {
    fn is_current(&self, scope: &RetrievalScope) -> Result<bool, RetrievalError> {
        if scope.workspace_id() != Some(self.workspace_id.as_str()) {
            return Ok(false);
        }
        if self.repository.workspace_generation(&self.workspace_id)? != Some(self.generation) {
            return Ok(false);
        }
        Ok(self
            .repository
            .embedding_confirmation(&self.workspace_id)?
            .is_some_and(|confirmation| {
                confirmation.profile_id == self.profile_id
                    && confirmation.model == self.model
                    && confirmation.generation == self.generation
            }))
    }
}

#[cfg(test)]
#[path = "../infrastructure/code_embedding_tests.rs"]
mod tests;
