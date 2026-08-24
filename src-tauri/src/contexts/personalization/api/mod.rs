//! The published boundary. Composition lives here, so this module is the one place allowed to know
//! both this context's internals and another context's contracts.

/// Transitional: satisfies `agent_runtime`'s pre-governance memory port from the governed store.
///
/// Published by the context taking ownership rather than implemented inside the context being
/// taken over from. Placing it there would have meant permanently raising that subtree's line
/// ceilings for code with a scheduled deletion date — the snapshot runtime adapters remove this
/// module wholesale.
#[cfg(test)]
mod legacy_alias_tests;
mod legacy_memory_bridge;
#[cfg(test)]
mod legacy_memory_bridge_tests;

pub(crate) use legacy_memory_bridge::LegacyMemoryPortBridge;

use std::sync::Arc;

use chrono::{DateTime, Utc};

use super::application::{
    CreateMemoryInput, MemoryApplicationService, MigrationJournalPort, MigrationStatePort,
    PersonalizationApplicationError, UpdateMemoryPatch,
};
use super::domain::{
    LegacySourceId, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, MigrationJournalEntry,
    MigrationStage,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// One governed memory, flattened for a caller that predates scope and audience.
///
/// Deliberately not `MemoryRecord`: the compatibility surface must not hand out a type whose new
/// fields a legacy caller would then start depending on. It carries the v2 file name as the id
/// because that is the handle the old port passes back for delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompatibilityMemory {
    pub(crate) file_name: String,
    pub(crate) id: MemoryId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) source_agent_id: Option<String>,
    pub(crate) source_workspace: Option<String>,
    pub(crate) is_automatic: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl CompatibilityMemory {
    fn from_record(record: &MemoryRecord) -> Self {
        Self {
            file_name: record.file_name(),
            id: record.id.clone(),
            name: record.name.clone(),
            description: record.description.clone(),
            memory_type: record.memory_type,
            content: record.content.clone(),
            source_agent_id: record
                .provenance
                .source_agent_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            source_workspace: record
                .provenance
                .source_workspace_key
                .as_ref()
                .map(|key| key.as_str().to_string()),
            is_automatic: matches!(
                record.source,
                MemorySource::OnePieceAutomatic
                    | MemorySource::CliAutomatic
                    | MemorySource::ModelMemoryTool
            ),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// A save from a caller that has no scope, audience, or revision to offer.
pub(crate) struct CompatibilitySaveInput {
    pub(crate) agent_id: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) content: String,
    pub(crate) is_automatic: bool,
}

/// The published boundary other contexts reach personalization through.
///
/// Everything here is deliberately narrow. The full policy-resolved surface arrives with the
/// snapshot runtime adapters; until then this exposes only what a pre-governance caller can
/// express, so nothing can accidentally depend on half-built policy semantics.
#[derive(Clone)]
pub(crate) struct PersonalizationApi {
    memories: Arc<MemoryApplicationService>,
    migration_state: Arc<dyn MigrationStatePort>,
    journal: Arc<dyn MigrationJournalPort>,
}

impl PersonalizationApi {
    pub(crate) fn new(
        memories: Arc<MemoryApplicationService>,
        migration_state: Arc<dyn MigrationStatePort>,
        journal: Arc<dyn MigrationJournalPort>,
    ) -> Self {
        Self {
            memories,
            migration_state,
            journal,
        }
    }

    /// Whether stored memory is safe for a runtime to use.
    ///
    /// Fails closed on every uncertainty: an unreadable migration row reads as not ready, because
    /// the alternative is answering "is this data trustworthy" with a guess.
    pub(crate) fn memory_is_ready(&self) -> bool {
        match self.migration_state.load() {
            Ok(state) => state.is_complete() && !state.repair_required,
            Err(_) => false,
        }
    }

    /// The pre-governance view: active, global, all-Agents.
    ///
    /// Exactly the set that existed before scopes did, so a legacy caller sees what it saw before
    /// and nothing more. A workspace-scoped or audience-restricted memory is invisible here by
    /// design — a caller that cannot express a scope must not receive a scoped record.
    ///
    /// Reads bodies, which the previous store's `list_all` also did; this is not a new cost. It
    /// disappears when the runtime adapters take snapshots instead.
    pub(crate) fn compatibility_memories(&self) -> Result<Vec<CompatibilityMemory>> {
        if !self.memory_is_ready() {
            // Fail closed: an incomplete or repair-required migration yields no memories rather
            // than a partial set a caller would treat as the whole truth.
            return Ok(Vec::new());
        }
        let mut memories: Vec<CompatibilityMemory> = self
            .memories
            .all_records()?
            .iter()
            .filter(|record| is_compatibility_visible(record))
            .map(CompatibilityMemory::from_record)
            .collect();
        // Newest first, with a stable tie-break: a migration writes many records at one timestamp,
        // and an unstable order there would reshuffle the injected index on every generation.
        memories.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(memories)
    }

    /// Creates, or updates the one record this legacy name identifies.
    ///
    /// Preserves the previous contract — saving under an existing name replaced that memory — but
    /// cannot implement it by searching for a name any more, because v2 permits duplicates and a
    /// search can return several. Resolution order, and none of it picks a record positionally:
    ///
    /// 1. a persisted `legacy source identity -> memory id` alias, addressed by stable id;
    /// 2. a stale alias whose target is gone is removed, and resolution continues as if absent;
    /// 3. no alias and exactly one visible record with this name: adopt it and persist the alias,
    ///    which is how a memory that predates the journal acquires one;
    /// 4. no alias and no match: create, then persist the alias;
    /// 5. no alias and several matches: refuse with a typed ambiguity, because choosing the first,
    ///    the newest, or any sorted position would silently overwrite one of the user's memories.
    pub(crate) fn save_compatibility_memory(
        &self,
        input: CompatibilitySaveInput,
    ) -> Result<CompatibilityMemory> {
        // A name that could never have been a v1 filename has no legacy identity, and therefore no
        // alias. That is correct rather than restrictive: nothing under v1 could have created it.
        let legacy_id = LegacySourceId::from_display_name(&input.name).ok();
        let existing = self.resolve_by_legacy_identity(legacy_id.as_ref(), &input.name)?;

        let source = if input.is_automatic {
            MemorySource::OnePieceAutomatic
        } else {
            MemorySource::ExplicitUser
        };

        let coordinated = match existing {
            Some(record) => self.memories.update(
                &record.id,
                record.revision,
                UpdateMemoryPatch {
                    description: Some(input.description),
                    memory_type: input.memory_type,
                    content: Some(input.content),
                    ..UpdateMemoryPatch::default()
                },
            )?,
            None => self.memories.create(CreateMemoryInput {
                name: input.name,
                description: input.description,
                // A legacy caller that supplies no type gets `untyped`, which only a
                // legacy-migration record may carry — so the source is recorded accordingly rather
                // than inventing a type the caller did not choose.
                memory_type: input.memory_type.unwrap_or(MemoryType::Untyped),
                content: input.content,
                scope: MemoryScope::Global,
                audience: MemoryAudience::AllAgents,
                status: MemoryStatus::Active,
                source: if input.memory_type.is_none() {
                    MemorySource::LegacyMigration
                } else {
                    source
                },
                provenance: MemoryProvenance {
                    source_agent_id: input
                        .agent_id
                        .as_deref()
                        .and_then(|id| super::domain::AgentId::parse(id).ok()),
                    source_session_id: None,
                    source_message_id: None,
                    source_workspace_key: input
                        .workspace
                        .as_deref()
                        .and_then(|key| super::domain::WorkspaceKey::parse(key).ok()),
                },
                sensitivity: MemorySensitivity::Normal,
            })?,
        };

        // Persisted after the write, not before: an alias pointing at a record that failed to be
        // created would send the next save to a memory that does not exist.
        if let Some(legacy_id) = legacy_id {
            self.journal.upsert(
                &MigrationJournalEntry {
                    legacy_source_id: legacy_id,
                    memory_id: Some(coordinated.record.id.clone()),
                    // Not a migrated record: it was authored through the compatibility surface.
                    // `Completed` records that its identity is settled and nothing is pending.
                    stage: MigrationStage::Completed,
                    legacy_backup_path: None,
                    legacy_content_hash: None,
                    last_error_code: None,
                },
                coordinated.record.updated_at,
            )?;
        }
        Ok(CompatibilityMemory::from_record(&coordinated.record))
    }

    /// Finds the single record a legacy name identifies, or explains why it cannot.
    fn resolve_by_legacy_identity(
        &self,
        legacy_id: Option<&LegacySourceId>,
        name: &str,
    ) -> Result<Option<MemoryRecord>> {
        if let Some(legacy_id) = legacy_id {
            if let Some(entry) = self.journal.get(legacy_id)? {
                if let Some(memory_id) = entry.memory_id.filter(|_| entry.stage.has_usable_memory())
                {
                    match self.memories.detail(&memory_id)? {
                        Some(record) => return Ok(Some(record)),
                        // The alias outlived its target. Removing it here is what stops a deleted
                        // memory from permanently blocking a name from being reused.
                        None => {
                            self.journal.remove(legacy_id)?;
                        }
                    }
                }
            }
        }

        let matches: Vec<MemoryRecord> = self
            .memories
            .all_records()?
            .into_iter()
            .filter(is_compatibility_visible_ref)
            .filter(|record| record.name == name)
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.into_iter().next()),
            count => Err(PersonalizationApplicationError::AmbiguousLegacyName { matches: count }),
        }
    }

    /// Deletes by the v2 file name a compatibility listing handed out.
    pub(crate) fn delete_compatibility_memory(&self, file_name: &str) -> Result<bool> {
        let Some(id) = memory_id_from_file_name(file_name) else {
            // An unrecognized handle is not an error: the previous store treated deleting
            // something that is not there as the caller's desired end state.
            return Ok(false);
        };
        let outcome = self.memories.delete(&id, None)?;
        Ok(outcome.deleted_file)
    }

    /// Removes every memory the compatibility view can see.
    ///
    /// Deletes through the coordinated service one record at a time rather than through the scoped
    /// reset, so no confirmation token has to be fabricated on a caller's behalf. It therefore
    /// matches the previous behavior exactly, including leaving unparseable files alone; the
    /// complete reset arrives with the maintenance UI.
    pub(crate) fn delete_all_compatibility_memories(&self) -> Result<usize> {
        let mut removed = 0;
        for record in self.memories.all_records()? {
            if !is_compatibility_visible(&record) {
                continue;
            }
            if self.memories.delete(&record.id, None)?.deleted_file {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

/// The pre-governance visibility rule, in one place so listing, saving, and deleting cannot drift.
fn is_compatibility_visible(record: &MemoryRecord) -> bool {
    matches!(record.status, MemoryStatus::Active)
        && matches!(record.scope, MemoryScope::Global)
        && matches!(record.audience, MemoryAudience::AllAgents)
}

fn is_compatibility_visible_ref(record: &MemoryRecord) -> bool {
    is_compatibility_visible(record)
}

fn memory_id_from_file_name(file_name: &str) -> Option<MemoryId> {
    MemoryId::parse(file_name.strip_suffix(".md").unwrap_or(file_name)).ok()
}
