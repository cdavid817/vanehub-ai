use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::indexing_service::truncate_for_embedding;
use super::{
    CodeIndexRepository, CodeRetrievalPort, EmbeddingPort, RetrievalConfigurationRepository,
    RetrievalDocumentRepository,
};
use crate::contexts::retrieval::domain::{
    code_embedding_identity, cosine_similarity, escape_fts_query, fuse_with_rrf, CodeIndexMode,
    CodeSearchHit, CodeSearchOutcome, CodeSearchQuery, Degradation, MatchedVia, RetrievalError,
    RetrievalScope, SourceKind,
};

pub(crate) struct CodeSearchService {
    workspace_id: String,
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    documents: Arc<dyn RetrievalDocumentRepository>,
    code_index: Arc<dyn CodeIndexRepository>,
    embeddings: Arc<dyn EmbeddingPort>,
}

impl CodeSearchService {
    pub(crate) fn new(
        workspace_id: String,
        configuration: Arc<dyn RetrievalConfigurationRepository>,
        documents: Arc<dyn RetrievalDocumentRepository>,
        code_index: Arc<dyn CodeIndexRepository>,
        embeddings: Arc<dyn EmbeddingPort>,
    ) -> Result<Self, RetrievalError> {
        RetrievalScope::Workspace(workspace_id.clone()).validate_for(SourceKind::WorkspaceFile)?;
        Ok(Self {
            workspace_id,
            configuration,
            documents,
            code_index,
            embeddings,
        })
    }

    fn vector_ranking(&self, text: &str, limit: usize) -> Option<Vec<String>> {
        if self
            .code_index
            .load_workspace(&self.workspace_id)
            .ok()??
            .mode
            == CodeIndexMode::Local
        {
            return None;
        }
        let configuration = self.configuration.load().ok()?;
        let (profile_id, model) = configuration.resolved_model()?;
        let generation = self
            .code_index
            .workspace_generation(&self.workspace_id)
            .ok()??;
        let confirmation = self
            .code_index
            .embedding_confirmation(&self.workspace_id)
            .ok()??;
        if confirmation.profile_id != profile_id
            || confirmation.model != model
            || confirmation.generation != generation
        {
            return None;
        }
        let query_vector = self
            .embeddings
            .embed(model, &[text.to_string()])
            .ok()?
            .into_iter()
            .next()?;
        let scope = RetrievalScope::Workspace(self.workspace_id.clone());
        let identity = code_embedding_identity(profile_id, model);
        let mut scored = self
            .documents
            .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &identity)
            .ok()?
            .into_iter()
            .filter_map(|(source_id, vector)| {
                cosine_similarity(&query_vector, &vector).map(|score| (source_id, score))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        Some(scored.into_iter().take(limit).map(|(id, _)| id).collect())
    }
}

impl CodeRetrievalPort for CodeSearchService {
    fn search_code(&self, query: &CodeSearchQuery) -> Result<CodeSearchOutcome, RetrievalError> {
        let text = truncate_for_embedding(&query.text);
        let over_fetch = query.limit.saturating_mul(4).max(query.limit);
        let scope = RetrievalScope::Workspace(self.workspace_id.clone());
        let local_mode = self
            .code_index
            .load_workspace(&self.workspace_id)?
            .ok_or(RetrievalError::InvalidScope)?
            .mode
            == CodeIndexMode::Local;
        let vector_ranking = self.vector_ranking(&text, over_fetch);
        let keyword_ranking = self
            .documents
            .keyword_candidates_scoped(
                SourceKind::WorkspaceFile,
                &scope,
                &escape_fts_query(&text),
                over_fetch,
            )
            .ok();
        let degraded = match (&vector_ranking, &keyword_ranking, local_mode) {
            (None, None, _) => return Err(RetrievalError::Unavailable),
            (None, Some(_), true) => None,
            (None, Some(_), false) => Some(Degradation::KeywordOnly),
            (Some(_), None, _) => Some(Degradation::VectorOnly),
            _ => None,
        };
        let vector_ids = vector_ranking.unwrap_or_default();
        let keyword_ids = keyword_ranking.unwrap_or_default();
        let in_vector = vector_ids.iter().cloned().collect::<HashSet<_>>();
        let in_keyword = keyword_ids.iter().cloned().collect::<HashSet<_>>();
        let fused = fuse_with_rrf(&[vector_ids, keyword_ids]);
        let wanted = fused
            .iter()
            .map(|(source_id, _)| source_id.clone())
            .collect::<Vec<_>>();
        let candidates = self
            .code_index
            .load_code_candidates(&self.workspace_id, &wanted)?
            .into_iter()
            .map(|candidate| (candidate.source_id.clone(), candidate))
            .collect::<HashMap<_, _>>();
        let hits = fused
            .into_iter()
            .filter_map(|(source_id, score)| {
                let candidate = candidates.get(&source_id)?;
                let matched_via = match (
                    in_vector.contains(source_id.as_str()),
                    in_keyword.contains(source_id.as_str()),
                ) {
                    (true, true) => MatchedVia::Both,
                    (true, false) => MatchedVia::Vector,
                    _ => MatchedVia::Keyword,
                };
                Some(CodeSearchHit {
                    file_path: candidate.file_path.clone(),
                    start_line: candidate.start_line,
                    end_line: candidate.end_line,
                    language: candidate.language.clone(),
                    symbol_name: candidate.symbol_name.clone(),
                    symbol_kind: candidate.symbol_kind.clone(),
                    snippet: candidate.snippet.clone(),
                    matched_via,
                    score,
                })
            })
            .take(query.limit)
            .collect();
        Ok(CodeSearchOutcome { hits, degraded })
    }
}

#[cfg(test)]
#[path = "../infrastructure/code_search_service_tests.rs"]
mod tests;
