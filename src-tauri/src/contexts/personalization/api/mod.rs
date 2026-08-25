//! The published boundary other contexts reach personalization through.
//!
//! Deliberately knows nothing about any consumer. An adapter that satisfies some other context's
//! port belongs at the composition boundary in `bootstrap`, not here: implementing a consumer's
//! trait from inside the provider inverts the dependency, and the architecture rules' own stated
//! repair is to "depend on a domain/application port and assemble its adapter in bootstrap".

#[cfg(test)]
mod compatibility_tests;

use std::sync::Arc;

use chrono::{DateTime, Utc};

use super::application::{
    CandidateSubmission, CandidateSubmissionOutcome, CandidateSubmissionService, CreateMemoryInput,
    LegacyAddressAliasPort, LegacySettingField, LegacySettingsCompatibility, LegacySettingsView,
    MaintenanceGatePort, MemoryApplicationService, MemoryHealthPort, MutationAdmission,
    PersonalizationApplicationError, PolicyResolutionService, ResolutionRequest, UpdateMemoryPatch,
    WorkspaceIdentityPort,
};
use super::domain::{
    EffectivePersonalizationSnapshot, LegacyAddressKey, MemoryAudience, MemoryId, MemoryProvenance,
    MemoryRecord, MemoryRuntimeHealth, MemoryScope, MemorySensitivity, MemorySource, MemoryStatus,
    MemoryType,
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
    /// The revision this body was read at. A caller holding a pinned reference compares it rather
    /// than trusting that the record has not moved since.
    pub(crate) revision: u64,
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
            revision: record.revision,
            name: record.name.clone(),
            description: record.description.clone(),
            memory_type: record.memory_type,
            content: record.content.clone(),
            source_agent_id: record
                .provenance
                .source_agent_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            // The raw folder, not the derived key. The contract this replaces carried a display
            // path, so a caller that shows it to a user must keep receiving one; the key exists for
            // comparison, and it is `None` for most of these anyway.
            source_workspace: record.provenance.legacy_folder.clone(),
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
    /// Resolves one immutable snapshot per generation. Published here rather than handed to runtime
    /// adapters directly, so every consumer reaches policy through the one boundary this context
    /// exposes and none of them can assemble a resolver of its own.
    resolver: Arc<PolicyResolutionService>,
    /// Turns proposals into reviewable candidates. Separate from `memories` on purpose: this
    /// service cannot write an active record, so no argument a runtime passes can cross from
    /// "the model suggested this" to "the user keeps this".
    candidates: Arc<CandidateSubmissionService>,
    /// Held across every read and every write, so a `Ready` answer cannot go stale between the
    /// check and the work it authorizes.
    gate: Arc<dyn MaintenanceGatePort>,
    health: Arc<dyn MemoryHealthPort>,
    settings: Arc<LegacySettingsCompatibility>,
    aliases: Arc<dyn LegacyAddressAliasPort>,
    /// Used only to derive a comparable key from the display path a pre-governance caller sends.
    /// The raw path is preserved regardless, so a failure to derive one loses nothing.
    workspace_identity: Arc<dyn WorkspaceIdentityPort>,
}

/// Everything the boundary is assembled from.
///
/// A named struct rather than a positional list: eight `Arc`s of which four are trait objects is
/// exactly the signature where two get swapped at one call site and the compiler agrees, and the
/// same shape is already used for the migration and maintenance services in this context.
pub(crate) struct PersonalizationApiParts {
    pub(crate) memories: Arc<MemoryApplicationService>,
    pub(crate) resolver: Arc<PolicyResolutionService>,
    pub(crate) candidates: Arc<CandidateSubmissionService>,
    pub(crate) gate: Arc<dyn MaintenanceGatePort>,
    pub(crate) health: Arc<dyn MemoryHealthPort>,
    pub(crate) settings: Arc<LegacySettingsCompatibility>,
    pub(crate) aliases: Arc<dyn LegacyAddressAliasPort>,
    pub(crate) workspace_identity: Arc<dyn WorkspaceIdentityPort>,
}

impl PersonalizationApi {
    pub(crate) fn new(parts: PersonalizationApiParts) -> Self {
        let PersonalizationApiParts {
            memories,
            resolver,
            candidates,
            gate,
            health,
            settings,
            aliases,
            workspace_identity,
        } = parts;
        Self {
            memories,
            resolver,
            candidates,
            gate,
            health,
            settings,
            aliases,
            workspace_identity,
        }
    }

    /// Whether stored memory is safe for a runtime to use, and if not, why.
    ///
    /// Fails closed on every uncertainty: an unreadable marker reads as failed, because the
    /// alternative is answering "is this data trustworthy" with a guess.
    pub(crate) fn memory_health(&self) -> MemoryRuntimeHealth {
        self.health.health()
    }

    pub(crate) fn memory_is_ready(&self) -> bool {
        self.memory_health().allows_memory_use()
    }

    /// Claims the directory for one operation, then refuses it if memory is unavailable.
    ///
    /// The order is the point. Taking the admission first is what makes the health answer binding:
    /// maintenance cannot begin while this admission lives, so `Ready` cannot become `Migrating`
    /// between the check below and the write it authorizes. Checking health first and admitting
    /// afterwards would leave exactly the window this exists to close.
    ///
    /// Typed rather than silent: a read that fails closed can honestly return nothing, but a write
    /// that quietly did nothing would let a caller believe a memory was saved.
    fn admit_write(&self) -> Result<Box<dyn MutationAdmission>> {
        let admission = self.gate.enter_mutation()?;
        if !self.memory_is_ready() {
            return Err(PersonalizationApplicationError::MaintenanceRequired);
        }
        Ok(admission)
    }

    /// The read counterpart. A read while maintenance owns the directory would see a half-migrated
    /// set, so it fails closed to the empty view the unavailable case already returns.
    fn admit_read(&self) -> Option<Box<dyn MutationAdmission>> {
        let admission = self.gate.enter_mutation().ok()?;
        self.memory_is_ready().then_some(admission)
    }

    /// One immutable snapshot for one generation or seat turn.
    ///
    /// The only way a runtime obtains policy. Taking it once at the start of a turn is what makes a
    /// settings change mid-turn reach the *next* turn rather than rewriting the one already planned
    /// around the old values.
    pub(crate) fn resolve_snapshot(
        &self,
        request: ResolutionRequest,
    ) -> Result<EffectivePersonalizationSnapshot> {
        self.resolver.resolve(request)
    }

    /// Queues proposals for review. Never writes an active memory.
    ///
    /// Held behind the same admission as a real write, because a proposal is written against
    /// revisions the snapshot pinned: submitting while migration owns the directory would queue an
    /// update whose expected revision the migration is in the middle of rewriting, and the review
    /// that later approved it would either conflict or apply to a record that had moved.
    ///
    /// Returns per-proposal outcomes rather than failing the batch. The callers are extraction and
    /// the model's own tool, both of which run behind a generation that has already answered.
    pub(crate) fn submit_memory_candidates(
        &self,
        submission: CandidateSubmission,
    ) -> Result<CandidateSubmissionOutcome> {
        let _admission = self.admit_write()?;
        self.candidates.submit(submission)
    }

    /// The dedicated policy in the shape the pre-governance settings page understands.
    ///
    /// Read-through: the policy is the source of truth from the moment migration completes, and the
    /// legacy rows are never consulted again. Available regardless of memory health — instructions
    /// are policy, not memory, and a migration that has not finished converting files says nothing
    /// about whether the user's instructions can be shown.
    pub(crate) fn legacy_settings(&self) -> Result<LegacySettingsView> {
        self.settings.view()
    }

    /// Write-through for one field of the pre-governance settings page.
    ///
    /// The expected revision is the one the caller's screen was rendered from, so a save from a
    /// stale copy is refused with a typed conflict rather than silently reverting another edit.
    pub(crate) fn save_legacy_setting(
        &self,
        field: LegacySettingField,
        expected_revision: u64,
    ) -> Result<LegacySettingsView> {
        self.settings.apply(field, expected_revision)
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
        // Fail closed: an incomplete or repair-required migration — or one running right now —
        // yields no memories rather than a partial set a caller would treat as the whole truth.
        let Some(_admission) = self.admit_read() else {
            return Ok(Vec::new());
        };
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

    /// The subset of the compatibility view a caller already holds handles for.
    ///
    /// Reads only the named records rather than the whole pool. The retrieval path resolves at most
    /// a page of hits per query, and answering it from a full snapshot would load and clone every
    /// memory body inside a generation.
    ///
    /// A handle with no record behind it is simply absent, which is what stops a deleted memory
    /// being surfaced again from an index row that outlived it.
    pub(crate) fn compatibility_memories_by_handle(
        &self,
        handles: &[String],
    ) -> Result<Vec<CompatibilityMemory>> {
        let Some(_admission) = self.admit_read() else {
            return Ok(Vec::new());
        };
        let mut memories = Vec::new();
        for handle in handles {
            let Some(id) = memory_id_from_file_name(handle) else {
                continue;
            };
            let Some(record) = self.memories.detail(&id)? else {
                continue;
            };
            if is_compatibility_visible(&record) {
                memories.push(CompatibilityMemory::from_record(&record));
            }
        }
        Ok(memories)
    }

    /// Creates, or updates the one record this legacy name identifies.
    ///
    /// Preserves the previous contract — saving under an existing name replaced that memory — but
    /// cannot implement it by searching for a name any more, because v2 permits duplicates and a
    /// search can return several. Resolution order, and none of it picks a record positionally:
    ///
    /// 1. a persisted `legacy address -> memory id` alias, addressed by stable id;
    /// 2. a stale alias whose target is gone is removed, and resolution continues as if absent;
    /// 3. no alias and exactly one visible record with this name: adopt it and persist the alias,
    ///    which is how a memory that predates the alias table acquires one;
    /// 4. no alias and no match: create, then persist the alias;
    /// 5. no alias and several matches: refuse with a typed ambiguity, because choosing the first,
    ///    the newest, or any sorted position would silently overwrite one of the user's memories.
    pub(crate) fn save_compatibility_memory(
        &self,
        input: CompatibilitySaveInput,
    ) -> Result<CompatibilityMemory> {
        // Refused rather than queued, and held for the whole operation rather than checked once. A
        // save accepted now would be written into a store whose derived views are mid-rebuild, and
        // the rebuild would then either miss it or resurrect a record the same run was removing.
        let _admission = self.admit_write()?;
        // A name that could never have been a v1 filename has no legacy identity, and therefore no
        // alias. That is correct rather than restrictive: nothing under v1 could have created it.
        let address = LegacyAddressKey::from_display_name(&input.name).ok();
        let existing = self.resolve_by_legacy_address(address.as_ref(), &input.name)?;

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
                    // A pre-governance caller sends a display path, not a key. Parsing it as a key
                    // succeeds only for the rare value that happens to hold no separator, so the
                    // raw value is kept alongside whatever the shared rule can derive — the same
                    // pairing migration records, because it is the same input from the same frozen
                    // contract.
                    source_workspace_key: input
                        .workspace
                        .as_deref()
                        .and_then(super::application::legacy_workspace_request)
                        .and_then(|request| self.workspace_identity.resolve(&request).ok())
                        .flatten()
                        .map(|identity| identity.key().clone()),
                    legacy_original_save_source: Some(if input.is_automatic {
                        super::domain::LegacyMemorySaveSource::Automatic
                    } else {
                        super::domain::LegacyMemorySaveSource::Explicit
                    }),
                    legacy_folder: input.workspace.clone(),
                    // Nothing was migrated: this record was created here, not read off disk.
                    legacy_source_relative_path: None,
                },
                sensitivity: MemorySensitivity::Normal,
            })?,
        };

        // Persisted after the write, not before: an alias pointing at a record that failed to be
        // created would send the next save to a memory that does not exist.
        if let Some(address) = address {
            self.aliases.put(
                &address,
                &coordinated.record.id,
                coordinated.record.updated_at,
            )?;
        }
        Ok(CompatibilityMemory::from_record(&coordinated.record))
    }

    /// Finds the single record a legacy address identifies, or explains why it cannot.
    fn resolve_by_legacy_address(
        &self,
        address: Option<&LegacyAddressKey>,
        name: &str,
    ) -> Result<Option<MemoryRecord>> {
        if let Some(address) = address {
            if let Some(memory_id) = self.aliases.get(address)? {
                match self.memories.detail(&memory_id)? {
                    Some(record) => return Ok(Some(record)),
                    // The alias outlived its target. Removing it here is what stops a deleted
                    // memory from permanently blocking a name from being reused.
                    None => {
                        self.aliases.remove(address)?;
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
        let _admission = self.admit_write()?;
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
        let _admission = self.admit_write()?;
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

/// Assembles the whole governed stack over one directory and database.
///
/// Lives at this context's api edge so a test in another module never has to reach for this
/// context's concrete persistence. Test-only: production assembly belongs in `bootstrap`.
#[cfg(test)]
pub(crate) fn build_for_tests(
    memory_root: std::path::PathBuf,
    database: crate::platform::database::NativeDatabase,
    retrieval_index: Arc<dyn super::application::RetrievalIndexPort>,
    clock: Arc<dyn super::application::ClockPort>,
) -> (PersonalizationApi, Arc<MemoryApplicationService>) {
    use super::application::{AgentCapabilityPort, LastKnownGoodPolicyCache};
    use super::domain::{AgentId, PersonalizationRuntimeCapabilities};
    use super::infrastructure::{
        DurableMemoryHealth, MaintenanceGate, MarkdownDerivedIndex, MarkdownMemoryRepository,
        SqliteCandidateRepository, SqliteLegacyAddressAlias, SqliteMemoryProjection,
        SqliteMigrationState, SqlitePolicyRepository, UuidMemoryIdGenerator,
    };

    /// Every Agent, fully capable. These fixtures are about memory and the compatibility view, not
    /// about which runtime can consume what, and an empty registry would fail every one of them
    /// closed for the wrong reason.
    struct EveryAgentCapable;

    impl AgentCapabilityPort for EveryAgentCapable {
        fn capabilities(
            &self,
            _agent_id: &AgentId,
        ) -> Result<Option<PersonalizationRuntimeCapabilities>> {
            Ok(Some(PersonalizationRuntimeCapabilities {
                supports_custom_instructions: true,
                supports_memory_index: true,
                supports_selected_memory_bodies: true,
                supports_automatic_extraction: true,
            }))
        }
    }

    let repository = Arc::new(
        MarkdownMemoryRepository::new(memory_root.clone(), Arc::new(UuidMemoryIdGenerator))
            .expect("memory repository"),
    );
    let service = Arc::new(MemoryApplicationService::new(
        repository.clone(),
        repository,
        Arc::new(SqliteMemoryProjection::new(database.clone())),
        Arc::new(MarkdownDerivedIndex::new(memory_root.clone())),
        retrieval_index,
        clock.clone(),
    ));
    let policies = Arc::new(SqlitePolicyRepository::new(database.clone()));
    let health = Arc::new(DurableMemoryHealth::new(Arc::new(
        SqliteMigrationState::new(database.clone()),
    )));
    let cache = Arc::new(LastKnownGoodPolicyCache::default());
    let resolver = Arc::new(PolicyResolutionService::new(
        policies.clone(),
        Arc::new(EveryAgentCapable),
        Arc::new(SqliteMemoryProjection::new(database.clone())),
        health.clone(),
        cache.clone(),
    ));
    let api = PersonalizationApi::new(PersonalizationApiParts {
        memories: service.clone(),
        resolver,
        candidates: Arc::new(CandidateSubmissionService::new(
            Arc::new(SqliteCandidateRepository::new(database.clone())),
            Arc::new(UuidMemoryIdGenerator),
            clock.clone(),
        )),
        gate: Arc::new(MaintenanceGate::new(&memory_root).expect("maintenance gate")),
        health,
        settings: Arc::new(LegacySettingsCompatibility::new(policies, clock, cache)),
        aliases: Arc::new(SqliteLegacyAddressAlias::new(database)),
        workspace_identity: Arc::new(
            super::application::WorkspaceIdentityResolver::for_this_platform(),
        ),
    });
    (api, service)
}
