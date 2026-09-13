use sha2::{Digest, Sha256};

use super::error::PersonalizationDomainError;
use super::memory::{
    eligibility, MemoryAudience, MemoryId, MemoryRecord, MemoryScope, MemoryStatus,
};
use super::policy::SessionPersonalizationMode;
use super::scope::{AgentId, SessionId, WorkspaceKey};
use super::snapshot::{
    EffectiveMemoryAccess, EffectivePersonalizationSnapshot, PersonalizationExclusionReason,
};

/// Bumped whenever the fields folded into a context fingerprint change, so a context minted by an
/// older build can never be mistaken for one of this build's.
pub(crate) const MEMORY_READ_CONTEXT_CONTRACT_VERSION: u32 = 1;

/// Who is reading: the stable Agent, the session, and the generation or seat turn that owns the
/// read. A seat in a group session is its own subject even when two seats share one Agent id,
/// which is what stops one seat borrowing another seat's authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MemoryReadSubject {
    pub(crate) agent_id: AgentId,
    pub(crate) session_id: SessionId,
    /// The generation or seat-turn identifier the context was minted for.
    pub(crate) generation_id: String,
    /// The seat this turn speaks for in a multi-seat session. `None` for a single-Agent session.
    pub(crate) seat_id: Option<String>,
}

/// Where the session's workspace stands, as three different answers rather than one `Option`.
///
/// "Explicitly no workspace" and "there is a workspace but it could not be resolved" lead to
/// opposite decisions: the first may still read global records, the second must read nothing,
/// because treating a failed resolution as absence is exactly how a project-only session would
/// fall open into the global pool.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum WorkspaceBinding {
    Absent,
    Resolved(WorkspaceKey),
    Unresolved,
}

impl WorkspaceBinding {
    pub(crate) fn key(&self) -> Option<&WorkspaceKey> {
        match self {
            Self::Resolved(key) => Some(key),
            Self::Absent | Self::Unresolved => None,
        }
    }

    fn as_fingerprint_part(&self) -> String {
        match self {
            Self::Absent => "absent".to_string(),
            Self::Resolved(key) => format!("resolved:{}", key.as_str()),
            Self::Unresolved => "unresolved".to_string(),
        }
    }
}

/// The trusted, frozen read authority for one generation.
///
/// Built only by the native personalization resolver from a resolved snapshot; nothing a model
/// sends can construct or alter one. Every memory read surface — index, selector, bodies, recall,
/// Context Engine — carries the same value, so all of them decide eligibility identically. The
/// fingerprint binds the fields to the process epoch, which is what lets the owning service
/// reject a value assembled anywhere else or carried over from a previous application run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MemoryReadContext {
    pub(crate) subject: MemoryReadSubject,
    pub(crate) workspace: WorkspaceBinding,
    pub(crate) session_mode: SessionPersonalizationMode,
    /// The snapshot revision token this context was frozen from. Diagnostics only.
    pub(crate) policy_revision: String,
    pub(crate) read: bool,
    pub(crate) global_allowed: bool,
    /// The one workspace whose records may be read. Always `None` unless `workspace` resolved.
    pub(crate) workspace_allowed: Option<WorkspaceKey>,
    pub(crate) maintenance_generation: u64,
    pub(crate) contract_version: u32,
    /// Digest over every field above plus the process epoch. Never derived by a consumer.
    pub(crate) fingerprint: String,
}

impl MemoryReadContext {
    /// Freezes a context from a resolved snapshot.
    ///
    /// The allowances are copied from the snapshot's resolved access rather than re-derived from
    /// policy, so a context can only ever be narrower than or equal to the snapshot that produced
    /// it. An unresolved workspace denies every read regardless of what policy says: the session
    /// has a workspace, and no scope decision can be made without knowing which one.
    pub(crate) fn freeze(
        snapshot: &EffectivePersonalizationSnapshot,
        workspace: WorkspaceBinding,
        generation_id: &str,
        seat_id: Option<&str>,
        maintenance_generation: u64,
        epoch: &str,
    ) -> Self {
        let access: &EffectiveMemoryAccess = &snapshot.memory_access;
        let unresolved = matches!(workspace, WorkspaceBinding::Unresolved);
        let temporary = matches!(
            snapshot.context.session_mode,
            SessionPersonalizationMode::Temporary
        );
        let read = access.read && !unresolved && !temporary;
        let allowance = access.readable_scopes();
        let workspace_allowed = if read {
            match (&workspace, allowance.workspace) {
                (WorkspaceBinding::Resolved(bound), Some(allowed)) if *bound == allowed => {
                    Some(allowed)
                }
                _ => None,
            }
        } else {
            None
        };
        let mut context = Self {
            subject: MemoryReadSubject {
                agent_id: snapshot.context.agent_id.clone(),
                session_id: snapshot.context.session_id.clone(),
                generation_id: generation_id.to_string(),
                seat_id: seat_id.map(str::to_string),
            },
            workspace,
            session_mode: snapshot.context.session_mode,
            policy_revision: snapshot.revision_token.clone(),
            read,
            global_allowed: read && allowance.global,
            workspace_allowed,
            maintenance_generation,
            contract_version: MEMORY_READ_CONTEXT_CONTRACT_VERSION,
            fingerprint: String::new(),
        };
        context.fingerprint = context.compute_fingerprint(epoch);
        context
    }

    /// Whether this context was minted by this process under this epoch and has not been altered.
    pub(crate) fn is_authentic(&self, epoch: &str) -> bool {
        self.contract_version == MEMORY_READ_CONTEXT_CONTRACT_VERSION
            && self.fingerprint == self.compute_fingerprint(epoch)
    }

    /// Whether anything at all may be read. A denied context is still a valid value: it is what a
    /// temporary session carries so every surface can ask one question and skip its work.
    pub(crate) fn permits_any_read(&self) -> bool {
        self.read && (self.global_allowed || self.workspace_allowed.is_some())
    }

    /// The safe, non-secret scope fingerprint diagnostics may record. Not the authenticity digest.
    pub(crate) fn scope_fingerprint(&self) -> String {
        short_digest(&[
            "memory-read-scope-v1",
            self.subject.agent_id.as_str(),
            self.session_mode.as_str(),
            &self.workspace.as_fingerprint_part(),
            &self.global_allowed.to_string(),
            &self
                .workspace_allowed
                .as_ref()
                .map(|key| key.as_str().to_string())
                .unwrap_or_default(),
        ])
    }

    /// The domain predicate every surface shares. Lifecycle first, then the frozen allowances,
    /// then exact audience membership; provenance never enters.
    pub(crate) fn admits(
        &self,
        record: &MemoryRecord,
    ) -> Result<(), PersonalizationExclusionReason> {
        if !self.read {
            return Err(match self.session_mode {
                SessionPersonalizationMode::Temporary => {
                    PersonalizationExclusionReason::TemporarySession
                }
                _ => PersonalizationExclusionReason::MemoryReadDisabled,
            });
        }
        let access = EffectiveMemoryAccess {
            read: true,
            global_memory: self.global_allowed,
            workspace: self.workspace_allowed.clone(),
            ..EffectiveMemoryAccess::denied()
        };
        eligibility(record, &access, &self.subject.agent_id)
    }

    fn compute_fingerprint(&self, epoch: &str) -> String {
        let seat = self.subject.seat_id.clone().unwrap_or_default();
        let workspace_allowed = self
            .workspace_allowed
            .as_ref()
            .map(|key| key.as_str().to_string())
            .unwrap_or_default();
        let parts = [
            "memory-read-context",
            &self.contract_version.to_string(),
            epoch,
            self.subject.agent_id.as_str(),
            self.subject.session_id.as_str(),
            &self.subject.generation_id,
            &seat,
            &self.workspace.as_fingerprint_part(),
            self.session_mode.as_str(),
            &self.policy_revision,
            &self.read.to_string(),
            &self.global_allowed.to_string(),
            &workspace_allowed,
            &self.maintenance_generation.to_string(),
        ];
        full_digest(&parts)
    }
}

/// One pinned record: the immutable id and the exact version the caller was shown.
///
/// A handle is what crosses from an index or a query result back into a read. Delivery compares
/// all three digests against the authoritative file, so an edit that forgot to advance the
/// revision, a scope change that left the body alone, or a replaced file all fail the same way.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MemoryReadHandle {
    pub(crate) id: MemoryId,
    pub(crate) revision: u64,
    pub(crate) content_hash: String,
    pub(crate) authority_fingerprint: String,
}

impl MemoryReadHandle {
    pub(crate) fn of(record: &MemoryRecord) -> Self {
        Self {
            id: record.id.clone(),
            revision: record.revision,
            content_hash: record.content_hash(),
            authority_fingerprint: authority_fingerprint(
                &record.status,
                &record.scope,
                &record.audience,
            ),
        }
    }

    pub(crate) fn matches(&self, record: &MemoryRecord) -> bool {
        self.id == record.id
            && self.revision == record.revision
            && self.content_hash == record.content_hash()
            && self.authority_fingerprint
                == authority_fingerprint(&record.status, &record.scope, &record.audience)
    }

    /// The v2 file name this handle addresses. The only mapping from a handle to a path, and it
    /// is derived from the immutable id alone, never from a caller-supplied path.
    pub(crate) fn source_id(&self) -> String {
        format!("{}.md", self.id)
    }

    /// Parses the file-name form an index row carries. Anything that is not exactly
    /// `<memory-id>.md` is rejected rather than joined onto a directory.
    pub(crate) fn id_from_source_id(
        source_id: &str,
    ) -> Result<MemoryId, PersonalizationDomainError> {
        let stem = source_id.strip_suffix(".md").unwrap_or(source_id);
        MemoryId::parse(stem)
    }
}

/// Digest over exactly the metadata that decides authorization: lifecycle, scope and audience.
///
/// Stored beside the id/revision/hash of every handle and every authorized-relation row, so a
/// record whose audience was narrowed without a body edit is detected at delivery even when the
/// projection that produced the candidate has not caught up.
pub(crate) fn authority_fingerprint(
    status: &MemoryStatus,
    scope: &MemoryScope,
    audience: &MemoryAudience,
) -> String {
    let mut parts: Vec<String> = vec![
        "memory-authority-v1".to_string(),
        status.as_str().to_string(),
        scope.kind_str().to_string(),
        scope
            .workspace_key()
            .map(|key| key.as_str().to_string())
            .unwrap_or_default(),
    ];
    match audience {
        MemoryAudience::AllAgents => parts.push("all_agents".to_string()),
        MemoryAudience::SelectedAgents { agent_ids } => {
            let mut ids: Vec<&str> = agent_ids.iter().map(AgentId::as_str).collect();
            ids.sort_unstable();
            parts.push(format!("selected:{}", ids.join("\u{1e}")));
        }
    }
    let borrowed: Vec<&str> = parts.iter().map(String::as_str).collect();
    full_digest(&borrowed)
}

fn full_digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\x1f");
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn short_digest(parts: &[&str]) -> String {
    full_digest(parts).chars().take(16).collect()
}
