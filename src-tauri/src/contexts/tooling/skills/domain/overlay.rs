#![cfg_attr(not(test), allow(dead_code))]

use super::{SkillDomainError, SkillId};

pub(crate) const OVERLAY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum OverlayScope {
    System,
    User,
    Project,
}

impl OverlayScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Project => "project",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Self::System),
            "user" => Some(Self::User),
            "project" => Some(Self::Project),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayMutationState {
    Active,
    Disabled,
    Reverted,
}

impl OverlayMutationState {
    pub(crate) fn disable(self) -> Result<Self, SkillDomainError> {
        match self {
            Self::Active => Ok(Self::Disabled),
            Self::Disabled => Ok(Self::Disabled),
            Self::Reverted => Err(SkillDomainError::InvalidOverlayTransition(
                "A reverted Overlay mutation cannot be disabled".to_string(),
            )),
        }
    }

    pub(crate) fn revert(self) -> Result<Self, SkillDomainError> {
        match self {
            Self::Active | Self::Disabled => Ok(Self::Reverted),
            Self::Reverted => Err(SkillDomainError::InvalidOverlayTransition(
                "An Overlay mutation cannot be reverted twice".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayConflictState {
    Active,
    Resolved,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayTrustState {
    Trusted,
    Untrusted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayOrigin {
    Local,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayTrust {
    state: OverlayTrustState,
    origin: OverlayOrigin,
    source_summary: Option<String>,
    reviewed_revision: Option<u64>,
    reviewed_content_hash: Option<String>,
}

impl OverlayTrust {
    pub(crate) fn trusted_local(revision: u64) -> Self {
        Self {
            state: OverlayTrustState::Trusted,
            origin: OverlayOrigin::Local,
            source_summary: None,
            reviewed_revision: Some(revision),
            reviewed_content_hash: None,
        }
    }

    pub(crate) fn untrusted_imported(source_summary: Option<String>) -> Self {
        Self {
            state: OverlayTrustState::Untrusted,
            origin: OverlayOrigin::Imported,
            source_summary,
            reviewed_revision: None,
            reviewed_content_hash: None,
        }
    }

    pub(crate) fn rehydrate(
        state: OverlayTrustState,
        origin: OverlayOrigin,
        source_summary: Option<String>,
        reviewed_revision: Option<u64>,
        reviewed_content_hash: Option<String>,
    ) -> Result<Self, SkillDomainError> {
        if source_summary
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SkillDomainError::InvalidOverlayValue(
                "Overlay trust source summary must not be empty".to_string(),
            ));
        }
        let valid = match (state, origin) {
            (OverlayTrustState::Trusted, OverlayOrigin::Local) => {
                reviewed_revision.is_some() && reviewed_content_hash.is_none()
            }
            (OverlayTrustState::Untrusted, OverlayOrigin::Imported) => {
                reviewed_revision.is_none() && reviewed_content_hash.is_none()
            }
            (OverlayTrustState::Trusted, OverlayOrigin::Imported) => {
                reviewed_revision.is_some()
                    && reviewed_content_hash
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
            }
            (OverlayTrustState::Untrusted, OverlayOrigin::Local) => false,
        };
        if !valid {
            return Err(SkillDomainError::InvalidOverlayValue(
                "Overlay trust metadata is inconsistent".to_string(),
            ));
        }
        Ok(Self {
            state,
            origin,
            source_summary,
            reviewed_revision,
            reviewed_content_hash,
        })
    }

    pub(crate) fn promote(
        &mut self,
        revision: u64,
        content_hash: &str,
    ) -> Result<(), SkillDomainError> {
        require_non_empty("reviewed content hash", content_hash)?;
        self.state = OverlayTrustState::Trusted;
        self.reviewed_revision = Some(revision);
        self.reviewed_content_hash = Some(content_hash.to_string());
        Ok(())
    }

    fn bind_local_revision(&mut self, revision: u64) {
        if self.origin == OverlayOrigin::Local {
            self.state = OverlayTrustState::Trusted;
            self.reviewed_revision = Some(revision);
            self.reviewed_content_hash = None;
        } else {
            self.state = OverlayTrustState::Untrusted;
            self.reviewed_revision = None;
            self.reviewed_content_hash = None;
        }
    }

    pub(crate) fn is_trusted_for_revision(&self, revision: u64) -> bool {
        self.state == OverlayTrustState::Trusted && self.reviewed_revision == Some(revision)
    }

    pub(crate) fn state(&self) -> OverlayTrustState {
        self.state
    }

    pub(crate) fn origin(&self) -> OverlayOrigin {
        self.origin
    }

    pub(crate) fn reviewed_content_hash(&self) -> Option<&str> {
        self.reviewed_content_hash.as_deref()
    }

    pub(crate) fn source_summary(&self) -> Option<&str> {
        self.source_summary.as_deref()
    }

    pub(crate) fn reviewed_revision(&self) -> Option<u64> {
        self.reviewed_revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayBaseWitness {
    pub(crate) base_identity: String,
    pub(crate) instruction_hash: String,
    pub(crate) package_hash: String,
}

impl OverlayBaseWitness {
    pub(crate) fn new(
        base_identity: &str,
        instruction_hash: &str,
        package_hash: &str,
    ) -> Result<Self, SkillDomainError> {
        Ok(Self {
            base_identity: required("base identity", base_identity)?,
            instruction_hash: required("base instruction hash", instruction_hash)?,
            package_hash: required("base package hash", package_hash)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayPatch {
    pub(crate) id: String,
    pub(crate) old_string: String,
    pub(crate) new_string: String,
    pub(crate) replace_all: bool,
    state: OverlayMutationState,
    pub(crate) creation_base_hash: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl OverlayPatch {
    pub(crate) fn new(
        id: &str,
        old_string: &str,
        new_string: &str,
        replace_all: bool,
        creation_base_hash: &str,
        created_at: &str,
    ) -> Result<Self, SkillDomainError> {
        require_non_empty("patch target", old_string)?;
        Ok(Self {
            id: required("patch id", id)?,
            old_string: old_string.to_string(),
            new_string: new_string.to_string(),
            replace_all,
            state: OverlayMutationState::Active,
            creation_base_hash: required("patch creation base hash", creation_base_hash)?,
            created_at: required("patch creation timestamp", created_at)?,
            updated_at: created_at.to_string(),
        })
    }

    pub(crate) fn state(&self) -> OverlayMutationState {
        self.state
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rehydrate(
        id: &str,
        old_string: &str,
        new_string: &str,
        replace_all: bool,
        state: OverlayMutationState,
        creation_base_hash: &str,
        created_at: &str,
        updated_at: &str,
    ) -> Result<Self, SkillDomainError> {
        let mut patch = Self::new(
            id,
            old_string,
            new_string,
            replace_all,
            creation_base_hash,
            created_at,
        )?;
        patch.state = state;
        patch.updated_at = required("patch update timestamp", updated_at)?;
        Ok(patch)
    }

    pub(crate) fn disable(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.disable()?;
        self.updated_at = required("patch update timestamp", updated_at)?;
        Ok(())
    }

    pub(crate) fn edit_for_reconciliation(
        &mut self,
        old_string: &str,
        new_string: &str,
        replace_all: bool,
        creation_base_hash: &str,
        updated_at: &str,
    ) -> Result<(), SkillDomainError> {
        if self.state != OverlayMutationState::Active {
            return Err(SkillDomainError::InvalidOverlayTransition(
                "Only an active Overlay patch can be edited during reconciliation".to_string(),
            ));
        }
        require_non_empty("patch target", old_string)?;
        self.old_string = old_string.to_string();
        self.new_string = new_string.to_string();
        self.replace_all = replace_all;
        self.creation_base_hash = required("patch creation base hash", creation_base_hash)?;
        self.updated_at = required("patch update timestamp", updated_at)?;
        Ok(())
    }

    pub(crate) fn revert(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.revert()?;
        self.updated_at = required("patch update timestamp", updated_at)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayLearnBlock {
    pub(crate) id: String,
    pub(crate) guidance: String,
    state: OverlayMutationState,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl OverlayLearnBlock {
    pub(crate) fn new(
        id: &str,
        guidance: &str,
        created_at: &str,
    ) -> Result<Self, SkillDomainError> {
        Ok(Self {
            id: required("learned-guidance id", id)?,
            guidance: required("learned guidance", guidance)?,
            state: OverlayMutationState::Active,
            created_at: required("learned-guidance creation timestamp", created_at)?,
            updated_at: created_at.to_string(),
        })
    }

    pub(crate) fn state(&self) -> OverlayMutationState {
        self.state
    }

    pub(crate) fn rehydrate(
        id: &str,
        guidance: &str,
        state: OverlayMutationState,
        created_at: &str,
        updated_at: &str,
    ) -> Result<Self, SkillDomainError> {
        let mut block = Self::new(id, guidance, created_at)?;
        block.state = state;
        block.updated_at = required("learned-guidance update timestamp", updated_at)?;
        Ok(block)
    }

    pub(crate) fn disable(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.disable()?;
        self.updated_at = required("learned-guidance update timestamp", updated_at)?;
        Ok(())
    }

    pub(crate) fn revert(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.revert()?;
        self.updated_at = required("learned-guidance update timestamp", updated_at)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayFile {
    pub(crate) id: String,
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) size: u64,
    pub(crate) content_hash: String,
    pub(crate) payload_ref: String,
    state: OverlayMutationState,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl OverlayFile {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: &str,
        logical_path: &str,
        media_type: &str,
        size: u64,
        content_hash: &str,
        payload_ref: &str,
        created_at: &str,
    ) -> Result<Self, SkillDomainError> {
        Ok(Self {
            id: required("Overlay file id", id)?,
            logical_path: required("Overlay logical path", logical_path)?,
            media_type: required("Overlay media type", media_type)?,
            size,
            content_hash: required("Overlay file content hash", content_hash)?,
            payload_ref: required("Overlay payload reference", payload_ref)?,
            state: OverlayMutationState::Active,
            created_at: required("Overlay file creation timestamp", created_at)?,
            updated_at: created_at.to_string(),
        })
    }

    pub(crate) fn state(&self) -> OverlayMutationState {
        self.state
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rehydrate(
        id: &str,
        logical_path: &str,
        media_type: &str,
        size: u64,
        content_hash: &str,
        payload_ref: &str,
        state: OverlayMutationState,
        created_at: &str,
        updated_at: &str,
    ) -> Result<Self, SkillDomainError> {
        let mut file = Self::new(
            id,
            logical_path,
            media_type,
            size,
            content_hash,
            payload_ref,
            created_at,
        )?;
        file.state = state;
        file.updated_at = required("Overlay file update timestamp", updated_at)?;
        Ok(file)
    }

    pub(crate) fn disable(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.disable()?;
        self.updated_at = required("Overlay file update timestamp", updated_at)?;
        Ok(())
    }

    pub(crate) fn revert(&mut self, updated_at: &str) -> Result<(), SkillDomainError> {
        self.state = self.state.revert()?;
        self.updated_at = required("Overlay file update timestamp", updated_at)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayConflict {
    id: String,
    mutation_id: String,
    pub(crate) reason: String,
    pub(crate) witnessed_base_hash: String,
    state: OverlayConflictState,
    resolution_revision: Option<u64>,
}

impl OverlayConflict {
    pub(crate) fn new(
        id: &str,
        mutation_id: &str,
        reason: &str,
        witnessed_base_hash: &str,
    ) -> Result<Self, SkillDomainError> {
        Ok(Self {
            id: required("Overlay conflict id", id)?,
            mutation_id: required("Overlay conflict mutation id", mutation_id)?,
            reason: required("Overlay conflict reason", reason)?,
            witnessed_base_hash: required("Overlay conflict base hash", witnessed_base_hash)?,
            state: OverlayConflictState::Active,
            resolution_revision: None,
        })
    }

    pub(crate) fn resolve(&mut self, revision: u64) -> Result<(), SkillDomainError> {
        self.finish(OverlayConflictState::Resolved, revision)
    }

    pub(crate) fn ignore(&mut self, revision: u64) -> Result<(), SkillDomainError> {
        self.finish(OverlayConflictState::Ignored, revision)
    }

    fn finish(
        &mut self,
        next_state: OverlayConflictState,
        revision: u64,
    ) -> Result<(), SkillDomainError> {
        if self.state != OverlayConflictState::Active {
            return Err(SkillDomainError::InvalidOverlayTransition(
                "Only an active Overlay conflict can be resolved or ignored".to_string(),
            ));
        }
        self.state = next_state;
        self.resolution_revision = Some(revision);
        Ok(())
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn mutation_id(&self) -> &str {
        &self.mutation_id
    }

    pub(crate) fn state(&self) -> OverlayConflictState {
        self.state
    }

    pub(crate) fn resolution_revision(&self) -> Option<u64> {
        self.resolution_revision
    }

    pub(crate) fn rehydrate(
        id: &str,
        mutation_id: &str,
        reason: &str,
        witnessed_base_hash: &str,
        state: OverlayConflictState,
        resolution_revision: Option<u64>,
    ) -> Result<Self, SkillDomainError> {
        let mut conflict = Self::new(id, mutation_id, reason, witnessed_base_hash)?;
        match state {
            OverlayConflictState::Active if resolution_revision.is_none() => Ok(conflict),
            OverlayConflictState::Resolved => {
                conflict.resolve(resolution_revision.ok_or_else(|| {
                    SkillDomainError::InvalidOverlayValue(
                        "A resolved Overlay conflict requires a resolution revision".to_string(),
                    )
                })?)?;
                Ok(conflict)
            }
            OverlayConflictState::Ignored => {
                conflict.ignore(resolution_revision.ok_or_else(|| {
                    SkillDomainError::InvalidOverlayValue(
                        "An ignored Overlay conflict requires a resolution revision".to_string(),
                    )
                })?)?;
                Ok(conflict)
            }
            OverlayConflictState::Active => Err(SkillDomainError::InvalidOverlayValue(
                "An active Overlay conflict cannot have a resolution revision".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayDocument {
    pub(crate) schema_version: u32,
    pub(crate) canonical_skill_id: SkillId,
    scope: OverlayScope,
    workspace_identity: Option<String>,
    revision: u64,
    pub(crate) base_witness: OverlayBaseWitness,
    trust: OverlayTrust,
    pub(crate) patches: Vec<OverlayPatch>,
    pub(crate) learn_blocks: Vec<OverlayLearnBlock>,
    pub(crate) files: Vec<OverlayFile>,
    pub(crate) conflicts: Vec<OverlayConflict>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    prior_revision_hash: Option<String>,
}

impl OverlayDocument {
    pub(crate) fn new(
        canonical_skill_id: SkillId,
        scope: OverlayScope,
        workspace_identity: Option<&str>,
        base_witness: OverlayBaseWitness,
        trust: OverlayTrust,
        created_at: &str,
    ) -> Result<Self, SkillDomainError> {
        let workspace_identity = validate_workspace(scope, workspace_identity)?;
        if !trust.is_trusted_for_revision(1) && trust.origin() == OverlayOrigin::Local {
            return Err(SkillDomainError::InvalidOverlayValue(
                "A local Overlay must trust its initial revision".to_string(),
            ));
        }
        Ok(Self {
            schema_version: OVERLAY_SCHEMA_VERSION,
            canonical_skill_id,
            scope,
            workspace_identity,
            revision: 1,
            base_witness,
            trust,
            patches: Vec::new(),
            learn_blocks: Vec::new(),
            files: Vec::new(),
            conflicts: Vec::new(),
            created_at: required("Overlay creation timestamp", created_at)?,
            updated_at: created_at.to_string(),
            prior_revision_hash: None,
        })
    }

    pub(crate) fn advance_revision(
        &mut self,
        prior_revision_hash: &str,
        updated_at: &str,
    ) -> Result<(), SkillDomainError> {
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(SkillDomainError::OverlayRevisionOverflow)?;
        self.prior_revision_hash = Some(required(
            "prior Overlay revision hash",
            prior_revision_hash,
        )?);
        self.updated_at = required("Overlay update timestamp", updated_at)?;
        self.revision = next_revision;
        self.trust.bind_local_revision(next_revision);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rehydrate(
        canonical_skill_id: SkillId,
        scope: OverlayScope,
        workspace_identity: Option<&str>,
        revision: u64,
        base_witness: OverlayBaseWitness,
        trust: OverlayTrust,
        patches: Vec<OverlayPatch>,
        learn_blocks: Vec<OverlayLearnBlock>,
        files: Vec<OverlayFile>,
        conflicts: Vec<OverlayConflict>,
        created_at: &str,
        updated_at: &str,
        prior_revision_hash: Option<&str>,
    ) -> Result<Self, SkillDomainError> {
        if revision == 0 {
            return Err(SkillDomainError::InvalidOverlayValue(
                "Overlay revision must be at least one".to_string(),
            ));
        }
        let prior_revision_hash = match (revision, prior_revision_hash) {
            (1, None) => None,
            (1, Some(_)) => {
                return Err(SkillDomainError::InvalidOverlayValue(
                    "Initial Overlay revision cannot have a prior revision hash".to_string(),
                ))
            }
            (_, Some(hash)) => Some(required("prior Overlay revision hash", hash)?),
            (_, None) => {
                return Err(SkillDomainError::InvalidOverlayValue(
                    "Advanced Overlay revision requires a prior revision hash".to_string(),
                ))
            }
        };
        if trust.state() == OverlayTrustState::Trusted && !trust.is_trusted_for_revision(revision) {
            return Err(SkillDomainError::InvalidOverlayValue(
                "Overlay trust must target the persisted revision".to_string(),
            ));
        }
        Ok(Self {
            schema_version: OVERLAY_SCHEMA_VERSION,
            canonical_skill_id,
            scope,
            workspace_identity: validate_workspace(scope, workspace_identity)?,
            revision,
            base_witness,
            trust,
            patches,
            learn_blocks,
            files,
            conflicts,
            created_at: required("Overlay creation timestamp", created_at)?,
            updated_at: required("Overlay update timestamp", updated_at)?,
            prior_revision_hash,
        })
    }

    pub(crate) fn scope(&self) -> OverlayScope {
        self.scope
    }

    pub(crate) fn workspace_identity(&self) -> Option<&str> {
        self.workspace_identity.as_deref()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn trust(&self) -> &OverlayTrust {
        &self.trust
    }

    pub(crate) fn quarantine_import(&mut self, source_summary: String) {
        self.trust = OverlayTrust::untrusted_imported(Some(source_summary));
    }

    pub(crate) fn promote_import(
        &mut self,
        reviewed_revision: u64,
        reviewed_content_hash: &str,
        updated_at: &str,
    ) -> Result<(), SkillDomainError> {
        if self.revision != reviewed_revision
            || self.trust.origin() != OverlayOrigin::Imported
            || self.trust.state() != OverlayTrustState::Untrusted
        {
            return Err(SkillDomainError::InvalidOverlayValue(
                "Overlay promotion must target the exact untrusted imported revision".to_string(),
            ));
        }
        self.trust
            .promote(reviewed_revision, reviewed_content_hash)?;
        self.updated_at = required("Overlay promotion timestamp", updated_at)?;
        Ok(())
    }

    pub(crate) fn prior_revision_hash(&self) -> Option<&str> {
        self.prior_revision_hash.as_deref()
    }
}

fn validate_workspace(
    scope: OverlayScope,
    workspace_identity: Option<&str>,
) -> Result<Option<String>, SkillDomainError> {
    match scope {
        OverlayScope::Project => workspace_identity
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| {
                SkillDomainError::InvalidOverlayValue(
                    "A Project Overlay requires a canonical workspace identity".to_string(),
                )
            }),
        OverlayScope::System | OverlayScope::User => {
            if workspace_identity.is_some() {
                Err(SkillDomainError::InvalidOverlayValue(
                    "Only a Project Overlay can have a workspace identity".to_string(),
                ))
            } else {
                Ok(None)
            }
        }
    }
}

fn required(label: &str, value: &str) -> Result<String, SkillDomainError> {
    require_non_empty(label, value)?;
    Ok(value.to_string())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), SkillDomainError> {
    if value.trim().is_empty() {
        Err(SkillDomainError::InvalidOverlayValue(format!(
            "{label} must not be empty"
        )))
    } else {
        Ok(())
    }
}
