use chrono::{DateTime, Utc};

use super::memory::{
    MemoryAudience, MemoryId, MemoryProvenance, MemoryScope, MemorySource, MemoryType,
    MEMORY_AUDIENCE_MAX_AGENTS, MEMORY_CONTENT_MAX_CHARS, MEMORY_DESCRIPTION_MAX_CHARS,
    MEMORY_NAME_MAX_CHARS,
};
use super::policy::RevisionConflict;

/// A proposed new memory. Automatic extraction produces these; only approval turns one into an
/// active record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateMemoryCandidate {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) scope: MemoryScope,
    pub(crate) audience: MemoryAudience,
}

/// A proposed correction to an existing memory.
///
/// Carries the target's revision because a proposal written against one version of a memory must
/// not silently apply to a different one the user edited in the meantime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateMemoryCandidate {
    pub(crate) target_id: MemoryId,
    pub(crate) expected_target_revision: u64,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) content: Option<String>,
}

/// A proposal that an existing memory should stop being used. Archive rather than delete: the model
/// proposing removal is not evidence strong enough to destroy the user's record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveMemoryCandidate {
    pub(crate) target_id: MemoryId,
    pub(crate) expected_target_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCandidateOperation {
    Create(CreateMemoryCandidate),
    Update(UpdateMemoryCandidate),
    Archive(ArchiveMemoryCandidate),
}

impl MemoryCandidateOperation {
    pub(crate) fn target_id(&self) -> Option<&MemoryId> {
        match self {
            Self::Create(_) => None,
            Self::Update(update) => Some(&update.target_id),
            Self::Archive(archive) => Some(&archive.target_id),
        }
    }

    pub(crate) fn expected_target_revision(&self) -> Option<u64> {
        match self {
            Self::Create(_) => None,
            Self::Update(update) => Some(update.expected_target_revision),
            Self::Archive(archive) => Some(archive.expected_target_revision),
        }
    }

    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            Self::Create(_) => "create",
            Self::Update(_) => "update",
            Self::Archive(_) => "archive",
        }
    }

    /// Whether this proposal is worth queueing for a human to read.
    ///
    /// The same bounds an active record is held to, applied before persistence rather than at
    /// approval. A proposal that could never be approved is not a proposal — queueing it would
    /// spend a reviewer's attention to reach a rejection this already knows.
    ///
    /// The reason is a stable code rather than a message: it is what the caller logs, and a
    /// rejected proposal is precisely the text nobody has approved.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Create(create) => {
                bounded("candidate-name", &create.name, 1, MEMORY_NAME_MAX_CHARS)?;
                bounded(
                    "candidate-description",
                    &create.description,
                    1,
                    MEMORY_DESCRIPTION_MAX_CHARS,
                )?;
                bounded(
                    "candidate-content",
                    &create.content,
                    1,
                    MEMORY_CONTENT_MAX_CHARS,
                )?;
                // `Untyped` exists for records migrated from a store that had no types. Nothing
                // proposed today may claim it, because there is no pre-governance file to blame.
                if matches!(create.memory_type, MemoryType::Untyped) {
                    return Err("candidate-untyped");
                }
                if let MemoryAudience::SelectedAgents { agent_ids } = &create.audience {
                    if agent_ids.is_empty() || agent_ids.len() > MEMORY_AUDIENCE_MAX_AGENTS {
                        return Err("candidate-audience");
                    }
                }
                Ok(())
            }
            Self::Update(update) => {
                // A correction that corrects nothing is a malformed proposal, not an empty one:
                // approving it would bump a revision and change no text.
                if update.name.is_none() && update.description.is_none() && update.content.is_none()
                {
                    return Err("candidate-empty-update");
                }
                if let Some(name) = update.name.as_deref() {
                    bounded("candidate-name", name, 1, MEMORY_NAME_MAX_CHARS)?;
                }
                if let Some(description) = update.description.as_deref() {
                    bounded(
                        "candidate-description",
                        description,
                        1,
                        MEMORY_DESCRIPTION_MAX_CHARS,
                    )?;
                }
                if let Some(content) = update.content.as_deref() {
                    bounded("candidate-content", content, 1, MEMORY_CONTENT_MAX_CHARS)?;
                }
                Ok(())
            }
            // Nothing to bound: an archive proposal is two identities and no text.
            Self::Archive(_) => Ok(()),
        }
    }
}

fn bounded(
    reason: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), &'static str> {
    let length = value.trim().chars().count();
    if length < minimum || value.chars().count() > maximum {
        return Err(reason);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateReviewStatus {
    Pending,
    Approved,
    Rejected,
}

/// One reviewable proposal.
///
/// It is a separate record from `MemoryRecord` on purpose: a candidate has no place in the active
/// store, so it cannot be reached by anything that enumerates active memories, and there is no
/// status field an approval path could forget to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryCandidate {
    pub(crate) id: MemoryId,
    pub(crate) operation: MemoryCandidateOperation,
    pub(crate) source: MemorySource,
    pub(crate) provenance: MemoryProvenance,
    pub(crate) status: CandidateReviewStatus,
    pub(crate) created_at: DateTime<Utc>,
}

impl MemoryCandidate {
    /// Approval fails when the target moved. Without this, an extraction proposal written minutes
    /// ago could overwrite an edit the user made since.
    pub(crate) fn check_target_revision(
        &self,
        current_revision: u64,
    ) -> Result<(), RevisionConflict> {
        match self.operation.expected_target_revision() {
            None => Ok(()),
            Some(expected) if expected == current_revision => Ok(()),
            Some(expected) => Err(RevisionConflict {
                expected,
                current: current_revision,
            }),
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        matches!(self.status, CandidateReviewStatus::Pending)
    }
}

/// What a reviewer can do with a pending candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReviewAction {
    Approve,
    ApproveWithEdits {
        name: Option<String>,
        description: Option<String>,
        content: Option<String>,
        memory_type: Option<MemoryType>,
        scope: Option<MemoryScope>,
        audience: Option<MemoryAudience>,
    },
    Reject,
    MarkSensitiveAndArchive,
    MergeInto {
        target_id: MemoryId,
        expected_target_revision: u64,
    },
}

impl ReviewAction {
    /// Whether this action can produce or modify an active record. Rejection is the only action
    /// that never touches active state, which is what makes it safe to run without a revision.
    pub(crate) fn mutates_active_state(&self) -> bool {
        !matches!(self, Self::Reject)
    }
}

/// What a completed review did. Rejected candidates keep only bounded audit metadata; their
/// proposed content is dropped so a rejected extraction does not linger as a second copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewOutcome {
    pub(crate) candidate_id: MemoryId,
    pub(crate) status: CandidateReviewStatus,
    pub(crate) resulting_memory_id: Option<MemoryId>,
    pub(crate) retained_content: bool,
}
