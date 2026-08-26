//! What a file has been involved in, without the journal ever holding its path.
//!
//! A file-mutation observation is recorded under a digest of `(workspace, relative path)` rather
//! than under the path itself — that is Group 3's rule and it stands. Asking "what happened to this
//! file" therefore means computing the same digest and looking for it, which is why the caller
//! passes a mutation id rather than a path: this context has no workspace root to hash against, and
//! giving it one would put a location back into the journal's vocabulary.
//!
//! The answer is deliberately small. It says whether there is anything to look at and roughly how
//! much, so a panel can decide whether to offer an action; it does not carry the records, because a
//! reader who wants them is one click from the surface that already lists them properly.

use super::super::EvidenceApplicationError;
use crate::contexts::execution_observability::domain::{EvidenceFileMutationId, EvidenceSessionId};

/// How many correlated identifiers one answer carries.
///
/// A file touched by a long run correlates to more commands than a link list can show, and a reader
/// scanning for "which run was that" needs the first few rather than all of them.
pub(crate) const MAX_LINKED_IDENTIFIERS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileEvidenceLinkQuery {
    pub(crate) session_id: EvidenceSessionId,
    pub(crate) file_mutation_id: EvidenceFileMutationId,
}

/// What is retained about one file.
///
/// `observations` is the count of times a change to it was recorded, which is what decides whether
/// an action is worth offering at all. Zero is a real answer and the common one: most files in a
/// workspace were never touched by an agent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileEvidenceLinks {
    pub(crate) observations: u32,
    /// Runs that touched it, newest first, bounded.
    pub(crate) run_ids: Vec<String>,
    /// Commands correlated with those observations, newest first, bounded.
    pub(crate) command_ids: Vec<String>,
    /// Whether more identifiers exist than are listed. Distinct from `observations`, which counts
    /// events rather than distinct runs.
    pub(crate) truncated: bool,
}

pub(crate) trait FileEvidenceLinkPort: Send + Sync {
    fn file_evidence_links(
        &self,
        query: &FileEvidenceLinkQuery,
    ) -> Result<FileEvidenceLinks, EvidenceApplicationError>;
}
