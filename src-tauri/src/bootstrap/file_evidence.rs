//! Asking what happened to one file, across two contexts that each hold half the answer.
//!
//! Workspaces knows where a session's workspace is. Execution observability knows what was recorded
//! against a file, but only under a digest of `(workspace, relative path)` — the journal has never
//! held a path and this does not change that. Neither context can answer alone, and neither should
//! learn the other's half, so the composition lives here with the rest of the cross-context
//! assembly.
//!
//! The digest is the same function the producer uses, imported rather than reproduced. A
//! fingerprint computed two ways is a query that silently returns nothing: no error, no missing
//! table, just an empty answer that reads as "this file was never touched".

use super::code_intelligence::workspace_path_fingerprint;
use crate::contexts::execution_observability::api::evidence::{
    EvidenceFileMutationId, EvidenceSessionId, FileEvidenceLinkPort, FileEvidenceLinkQuery,
    FileEvidenceLinks,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use std::path::Path;
use std::sync::Arc;

/// The two halves, joined.
#[derive(Clone)]
pub(crate) struct SessionFileEvidence {
    workspaces: WorkspaceApi,
    links: Arc<dyn FileEvidenceLinkPort>,
}

impl SessionFileEvidence {
    pub(crate) fn new(workspaces: WorkspaceApi, links: Arc<dyn FileEvidenceLinkPort>) -> Self {
        Self { workspaces, links }
    }

    /// What is retained about one file, or nothing when the question cannot be asked.
    ///
    /// A session with no local workspace has no root to hash against, and a remote one is hashed
    /// against a root this machine cannot canonicalise. Both answer "nothing linked" rather than
    /// failing: the panel's question is whether to offer an action, and an error would put a
    /// failure notice on screen for a file that is simply not one an agent has touched here.
    pub(crate) fn links_for(
        &self,
        session_id: &str,
        relative_path: &str,
    ) -> Result<FileEvidenceLinks, String> {
        let Some(root) = self
            .workspaces
            .resolve_session_root(session_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(FileEvidenceLinks::default());
        };
        let fingerprint = workspace_path_fingerprint(Path::new(&root), relative_path);
        let (Ok(session), Ok(mutation)) = (
            EvidenceSessionId::parse(session_id.to_string()),
            EvidenceFileMutationId::parse(fingerprint),
        ) else {
            return Ok(FileEvidenceLinks::default());
        };
        self.links
            .file_evidence_links(&FileEvidenceLinkQuery {
                session_id: session,
                file_mutation_id: mutation,
            })
            .map_err(|error| error.to_string())
    }
}
