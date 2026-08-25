use crate::contexts::personalization::api::{
    CompatibilityMemory, CompatibilitySaveInput, PersonalizationApi,
};

use std::time::SystemTime;

use crate::contexts::agent_runtime::application::{
    AgentCandidateOutcome, AgentCandidateSubmission, AgentMemory, AgentMemoryAccess,
    AgentMemoryBody, AgentMemoryDelivery, AgentMemoryPort, AgentMemoryProposal, AgentMemoryRef,
    AgentPersonalizationPort, AgentPersonalizationSnapshot, AgentPersonalizationSnapshotPort,
    AgentProposalOrigin, AgentRuntimeApplicationError, GenerationPersonalizationContext,
    MemorySource, PersonalizationSettings, SaveMemoryInput,
};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::personalization::application::{
    legacy_workspace_request, CandidateSubmission, ResolutionRequest, WorkspaceIdentityPort,
    WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::MemorySource as GovernedSource;
use crate::contexts::personalization::domain::MemoryType as GovernedMemoryType;
use crate::contexts::personalization::domain::{
    AgentId, ArchiveMemoryCandidate, CreateMemoryCandidate, InstructionField, MemoryAudience,
    MemoryCandidateOperation, MemoryDeliveryMode, MemoryId, MemoryProvenance, MemoryScope,
    SessionId, SessionPersonalizationMode, UpdateMemoryCandidate,
};

/// Satisfies `agent_runtime`'s pre-governance memory port from the governed v2 store.
///
/// Lives at the composition boundary rather than inside either context. Implementing a consumer's
/// trait from inside the provider would invert the dependency; implementing it inside the consumer
/// would make that context know the provider's internals. `bootstrap` is where the repository's own
/// architecture rules say to assemble an adapter over a published port.
///
/// # Why this exists
///
/// Migrating v1 files to v2 and deleting the sources leaves the old file store reading a directory
/// it does not understand. It would not fail loudly — that would be easier. `FileAgentMemoryStore`
/// strips quotes from frontmatter values and ignores unknown keys, so it partially reads a v2
/// file: it recovers name, description, and body, silently loses the type (v2 writes
/// `memory_type`, v1 reads `type`), and rejects outright any memory whose name contains a
/// character v1 forbids in a filename — which v2 now permits, because v2 names are not filenames.
/// The result would be a name-dependent subset appearing to work.
///
/// Three further hazards follow from leaving the old store wired:
///
/// - its `save` addresses memories by a name-derived filename, so saving over a migrated memory
///   would write a second, v1 file beside the v2 one — the dual-write this change exists to end;
/// - its `reconcile_index` rebuilds `MEMORY.md` from its own scan on every write, overwriting the
///   v2 index with links that resolve to nothing;
/// - that scan is still capped at 200, so a store larger than that would silently truncate.
///
/// This bridge removes all four by making the old port a projection of the new store. It is
/// deliberately narrow: no policy resolution, no scope, no candidates, no change to OnePiece
/// compaction or relevance selection, and no change to any CLI's internal context or native files.
/// It is removed when the snapshot runtime adapters land.
#[derive(Clone)]
pub(crate) struct LegacyMemoryPortBridge {
    personalization: PersonalizationApi,
}

impl LegacyMemoryPortBridge {
    pub(crate) fn new(personalization: PersonalizationApi) -> Self {
        Self { personalization }
    }
}

fn bridge_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Memory(error.to_string())
}

/// The two contexts keep separate type enums on purpose — a shared one would make either context's
/// taxonomy a dependency of the other's — so the boundary translates.
fn to_governed_type(value: RuntimeMemoryType) -> GovernedMemoryType {
    match value {
        RuntimeMemoryType::User => GovernedMemoryType::User,
        RuntimeMemoryType::Feedback => GovernedMemoryType::Feedback,
        RuntimeMemoryType::Project => GovernedMemoryType::Project,
        RuntimeMemoryType::Reference => GovernedMemoryType::Reference,
    }
}

fn to_runtime_type(value: GovernedMemoryType) -> Option<RuntimeMemoryType> {
    match value {
        GovernedMemoryType::User => Some(RuntimeMemoryType::User),
        GovernedMemoryType::Feedback => Some(RuntimeMemoryType::Feedback),
        GovernedMemoryType::Project => Some(RuntimeMemoryType::Project),
        GovernedMemoryType::Reference => Some(RuntimeMemoryType::Reference),
        // `untyped` is a real governed value with no legacy equivalent. `None` is exactly how the
        // old model expressed it, so this degrades rather than inventing a type.
        GovernedMemoryType::Untyped => None,
    }
}

fn to_agent_memory(memory: CompatibilityMemory) -> AgentMemory {
    AgentMemory {
        // The v2 file name, because that is the handle every downstream consumer — the management
        // view, the retrieval index — passes back to address this memory.
        id: memory.file_name,
        agent_id: memory.source_agent_id.unwrap_or_default(),
        folder: memory.source_workspace,
        name: memory.name,
        description: memory.description,
        memory_type: to_runtime_type(memory.memory_type),
        content: memory.content,
        source: if memory.is_automatic {
            MemorySource::Automatic
        } else {
            MemorySource::Explicit
        },
        created_at: memory.created_at.to_rfc3339(),
        // Governed records carry their own `updated_at`, which is what recency and staleness
        // already key on, so the filesystem timestamp the old store used is not needed.
        modified_at: None,
    }
}

impl AgentMemoryPort for LegacyMemoryPortBridge {
    fn save(&self, input: SaveMemoryInput<'_>) -> Result<(), AgentRuntimeApplicationError> {
        let content = input.content.trim();
        if content.is_empty() {
            return Err(bridge_error("Memory content is empty."));
        }
        // A caller that supplies neither is not a production path — every current writer supplies
        // both — so this refuses rather than deriving a name, which would put naming policy in the
        // compatibility layer.
        let (Some(name), Some(description)) = (input.name, input.description) else {
            return Err(bridge_error(
                "A governed memory requires a name and a description.",
            ));
        };

        self.personalization
            .save_compatibility_memory(CompatibilitySaveInput {
                agent_id: Some(input.agent_id.to_string()),
                workspace: input.folder.map(str::to_string),
                name: name.to_string(),
                description: description.to_string(),
                memory_type: input.memory_type.map(to_governed_type),
                content: content.to_string(),
                is_automatic: matches!(input.source, MemorySource::Automatic),
            })
            .map(|_| ())
            .map_err(bridge_error)
    }

    fn list_all(&self) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        Ok(self
            .personalization
            .compatibility_memories()
            .map_err(bridge_error)?
            .into_iter()
            .map(to_agent_memory)
            .collect())
    }

    fn delete(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.personalization
            .delete_compatibility_memory(memory_id)
            .map(|_| ())
            .map_err(bridge_error)
    }

    fn delete_all(&self) -> Result<(), AgentRuntimeApplicationError> {
        self.personalization
            .delete_all_compatibility_memories()
            .map(|_| ())
            .map_err(bridge_error)
    }
}

/// Satisfies `agent_runtime`'s personalization port from the governed policy.
///
/// Instructions and the memory switches come from the dedicated policy. Automatic context
/// compaction and the context-quality retention window stay on the desktop settings, because those
/// were never personalization and this change does not move them.
///
/// Fails closed on a policy it cannot read, and on memory that is not `Ready`: instructions are
/// omitted and memory is reported off rather than falling back to permissive defaults. Generation
/// continues either way — an unavailable policy makes an answer less personal, never absent.
#[derive(Clone)]
pub(crate) struct GovernedPersonalizationAdapter {
    personalization: PersonalizationApi,
    settings: DesktopSettingsApi,
    /// Turns the folder a session records into a stable workspace key, by the same rule migration
    /// uses. Two rules for one input is how the two surfaces would come to disagree about which
    /// workspace a session is in.
    workspace_identity: WorkspaceIdentityResolver,
}

impl GovernedPersonalizationAdapter {
    pub(crate) fn new(personalization: PersonalizationApi, settings: DesktopSettingsApi) -> Self {
        Self {
            personalization,
            settings,
            workspace_identity: WorkspaceIdentityResolver::for_this_platform(),
        }
    }
}

impl AgentPersonalizationPort for GovernedPersonalizationAdapter {
    fn settings(&self) -> Result<PersonalizationSettings, AgentRuntimeApplicationError> {
        let view = self
            .settings
            .get_settings()
            .map_err(|error| AgentRuntimeApplicationError::Personalization(error.to_string()))?
            .settings;

        let policy = self.personalization.legacy_settings().ok();
        let memory_ready = self.personalization.memory_is_ready();
        Ok(PersonalizationSettings {
            custom_instructions_about_user: policy
                .as_ref()
                .and_then(|policy| policy.settings.about_user.clone())
                .unwrap_or_default(),
            custom_instructions_style_rules: policy
                .as_ref()
                .and_then(|policy| policy.settings.style_rules.clone())
                .unwrap_or_default(),
            // An unreadable policy means no instructions are applied at all, not "apply the empty
            // ones": an enabled flag with empty text and a disabled flag are different states, and
            // only the second is honest about not knowing.
            custom_instructions_enabled: policy
                .as_ref()
                .and_then(|policy| policy.settings.custom_instructions_enabled)
                .unwrap_or(false),
            // Two conditions, and both must hold. A policy that permits memory says nothing about
            // whether the store behind it is safe to read.
            memory_enabled: memory_ready
                && policy
                    .as_ref()
                    .and_then(|policy| policy.settings.memory_enabled)
                    .unwrap_or(false),
            memory_tool_assisted_chats_enabled: memory_ready
                && policy
                    .as_ref()
                    .and_then(|policy| policy.settings.tool_assisted_extraction_enabled)
                    .unwrap_or(false),
            automatic_context_compaction_enabled: view.automatic_context_compaction_enabled(),
            context_quality_retention_days: view.context_quality_retention_days(),
        })
    }
}

impl AgentPersonalizationSnapshotPort for GovernedPersonalizationAdapter {
    /// One governed snapshot, translated into the shape the runtime speaks.
    ///
    /// Never fails the generation: an unresolvable policy yields a fail-closed snapshot, because an
    /// answer without personalization is still an answer and refusing to generate is not.
    fn snapshot(&self, context: GenerationPersonalizationContext) -> AgentPersonalizationSnapshot {
        let (Ok(agent_id), Ok(session_id)) = (
            AgentId::parse(&context.agent_id),
            SessionId::parse(&context.session_id),
        ) else {
            return AgentPersonalizationSnapshot::fail_closed("unresolvable_identity");
        };

        // The session's folder, resolved to a stable key the same way migration resolves a legacy
        // one. A display path is not a workspace identity, so a folder that does not name one root
        // yields no workspace rather than an invented key.
        let workspace = context
            .folder
            .as_deref()
            .and_then(legacy_workspace_request)
            .and_then(|request| self.workspace_identity.resolve(&request).ok())
            .flatten();

        let resolved = self.personalization.resolve_snapshot(ResolutionRequest {
            agent_id,
            session_id,
            workspace,
            // Sessions do not record a personalization mode yet, so every session is standard. The
            // resolver already applies the other two modes correctly; what is missing is the place
            // a user chooses one, which arrives with the session UI.
            session_mode: SessionPersonalizationMode::Standard,
            session_override: None,
        });
        let Ok(resolved) = resolved else {
            return AgentPersonalizationSnapshot::fail_closed("policy_unavailable");
        };

        let settings = self.settings.get_settings().ok();
        // The tool-assisted sub-policy, read from the same projection the flat settings surface
        // reads. It is not a second opinion about extraction: it narrows the tool-assisted turns
        // alone, and the runtime combines the two.
        let tool_assisted = self
            .personalization
            .legacy_settings()
            .ok()
            .and_then(|policy| policy.settings.tool_assisted_extraction_enabled)
            .unwrap_or(false);
        AgentPersonalizationSnapshot {
            revision_token: resolved.revision_token.clone(),
            instruction_block: instruction_block(&resolved),
            memory: memory_access(&resolved, tool_assisted),
            // Not personalization, and not moved by it. Absent settings fall back to the safe
            // values rather than to whatever the last read happened to be.
            automatic_context_compaction_enabled: settings
                .as_ref()
                .map(|view| view.settings.automatic_context_compaction_enabled())
                .unwrap_or(true),
            context_quality_retention_days: settings
                .as_ref()
                .map(|view| view.settings.context_quality_retention_days())
                .unwrap_or(30),
        }
    }

    fn pinned_bodies(
        &self,
        refs: &[AgentMemoryRef],
    ) -> Result<Vec<AgentMemoryBody>, AgentRuntimeApplicationError> {
        let handles: Vec<String> = refs.iter().map(|entry| entry.id.clone()).collect();
        let memories = self
            .personalization
            .compatibility_memories_by_handle(&handles)
            .map_err(bridge_error)?;
        Ok(refs
            .iter()
            .filter_map(|entry| {
                let memory = memories
                    .iter()
                    .find(|memory| memory.file_name == entry.id)?;
                // A record whose revision moved since the snapshot is absent rather than silently
                // newer: the body in the prompt has to be the body the index described.
                (memory.revision == entry.revision).then(|| AgentMemoryBody {
                    id: entry.id.clone(),
                    revision: entry.revision,
                    name: memory.name.clone(),
                    memory_type: to_runtime_type(memory.memory_type),
                    content: memory.content.clone(),
                    updated_at: entry.updated_at,
                })
            })
            .collect())
    }

    /// Translates one generation's proposals and hands them to the review queue.
    ///
    /// The scope of a proposal comes from the session, never from the model: a create proposed in
    /// a session with a resolvable workspace is proposed for that workspace, and one without is
    /// global. Letting a proposal name its own scope would let a model widen what it is allowed to
    /// affect by asking.
    fn propose_memories(
        &self,
        submission: AgentCandidateSubmission,
    ) -> Result<AgentCandidateOutcome, AgentRuntimeApplicationError> {
        let workspace = submission
            .folder
            .as_deref()
            .and_then(legacy_workspace_request)
            .and_then(|request| self.workspace_identity.resolve(&request).ok())
            .flatten();
        let provenance = MemoryProvenance {
            source_agent_id: AgentId::parse(&submission.agent_id).ok(),
            source_session_id: SessionId::parse(&submission.session_id).ok(),
            source_workspace_key: workspace.as_ref().map(|identity| identity.key().clone()),
            ..MemoryProvenance::default()
        };
        let scope = match workspace.as_ref() {
            Some(identity) => MemoryScope::Workspace {
                workspace_key: identity.key().clone(),
            },
            None => MemoryScope::Global,
        };
        let eligible_targets: Vec<MemoryId> = submission
            .eligible
            .iter()
            .filter_map(|entry| MemoryId::parse(entry.id.trim_end_matches(".md")).ok())
            .collect();
        let total = submission.proposals.len();
        let proposals: Vec<MemoryCandidateOperation> = submission
            .proposals
            .into_iter()
            .filter_map(|proposal| to_candidate_operation(proposal, &scope))
            .collect();
        // A proposal this boundary could not even translate is rejected here rather than being
        // dropped silently: the caller reports counts, and a batch that reported fewer than it
        // sent with nothing to account for the difference would read as a persistence bug.
        let untranslated = total - proposals.len();
        let outcome = self
            .personalization
            .submit_memory_candidates(CandidateSubmission {
                proposals,
                source: match submission.origin {
                    AgentProposalOrigin::AutomaticExtraction => GovernedSource::OnePieceAutomatic,
                    AgentProposalOrigin::ModelTool => GovernedSource::ModelMemoryTool,
                },
                provenance,
                eligible_targets,
            })
            .map_err(bridge_error)?;
        Ok(AgentCandidateOutcome {
            accepted: outcome.accepted_count(),
            rejected: outcome.rejected_count() + untranslated,
        })
    }
}

/// Turns one runtime proposal into the governed shape, or drops it.
///
/// `None` for a target id that is not a legal memory id at all — a create's own id is allocated by
/// the service, so only the two target-bearing kinds can fail here.
pub(super) fn to_candidate_operation(
    proposal: AgentMemoryProposal,
    scope: &MemoryScope,
) -> Option<MemoryCandidateOperation> {
    match proposal {
        AgentMemoryProposal::Create {
            name,
            description,
            memory_type,
            content,
        } => Some(MemoryCandidateOperation::Create(CreateMemoryCandidate {
            name,
            description,
            // A proposal that named no type is a project note by default rather than `Untyped`,
            // which exists only for records migrated from a store that had no types at all.
            memory_type: memory_type
                .map(to_governed_type)
                .unwrap_or(GovernedMemoryType::Project),
            content,
            scope: scope.clone(),
            // Narrowing to a subset of Agents is a user's decision made in review, not something a
            // proposing model may assert about the user's other Agents.
            audience: MemoryAudience::AllAgents,
        })),
        AgentMemoryProposal::Update {
            target_id,
            expected_revision,
            description,
            content,
        } => Some(MemoryCandidateOperation::Update(UpdateMemoryCandidate {
            target_id: MemoryId::parse(target_id.trim_end_matches(".md")).ok()?,
            expected_target_revision: expected_revision,
            // Renaming is not something an extraction may propose: the name is how a user finds a
            // memory again, and a silent rename reads as a memory having disappeared.
            name: None,
            description,
            content,
        })),
        AgentMemoryProposal::Archive {
            target_id,
            expected_revision,
        } => Some(MemoryCandidateOperation::Archive(ArchiveMemoryCandidate {
            target_id: MemoryId::parse(target_id.trim_end_matches(".md")).ok()?,
            expected_target_revision: expected_revision,
        })),
    }
}

/// The user-authored instruction block, rendered in the order policy resolved.
///
/// Style rules before the description of the user, matching what the previous flat settings
/// produced: style is a constraint on every response, the description is background, and a reader
/// who saw them swap would experience it as the setting having changed.
pub(super) fn instruction_block(
    snapshot: &crate::contexts::personalization::domain::EffectivePersonalizationSnapshot,
) -> Option<String> {
    let text_for = |field: InstructionField| -> Option<String> {
        let joined: Vec<&str> = snapshot
            .instruction_segments
            .iter()
            .filter(|segment| segment.field == field)
            .map(|segment| segment.text.as_str())
            .collect();
        (!joined.is_empty()).then(|| joined.join("\n\n"))
    };

    let mut sections = Vec::new();
    if let Some(style) = text_for(InstructionField::StyleRules) {
        sections.push(format!("### Response style\n{style}"));
    }
    if let Some(about) = text_for(InstructionField::AboutUser) {
        sections.push(format!("### About the user\n{about}"));
    }
    // Byte-identical to what the flat settings rendered, heading spacing included: a user whose
    // instructions have not changed must not see their prompt change across the migration, and a
    // provider prefix cache must not be invalidated by one.
    (!sections.is_empty()).then(|| format!("## Custom Instructions\n{}", sections.join("\n\n")))
}

/// What the runtime may do with memory, from what the snapshot resolved.
pub(super) fn memory_access(
    snapshot: &crate::contexts::personalization::domain::EffectivePersonalizationSnapshot,
    tool_assisted_extraction: bool,
) -> AgentMemoryAccess {
    let access = &snapshot.memory_access;
    AgentMemoryAccess {
        read: access.read,
        explicit_save: access.explicit_save,
        automatic_extraction: access.automatic_extraction,
        automatic_extraction_in_tool_assisted_turns: access.automatic_extraction
            && tool_assisted_extraction,
        candidate_creation: access.candidate_creation,
        retrieval_write: access.retrieval_write,
        delivery: match access.delivery {
            MemoryDeliveryMode::None => AgentMemoryDelivery::None,
            MemoryDeliveryMode::IndexOnly => AgentMemoryDelivery::IndexOnly,
            MemoryDeliveryMode::IndexWithSelectedBodies => {
                AgentMemoryDelivery::IndexWithSelectedBodies
            }
        },
        eligible: snapshot
            .memory
            .refs
            .iter()
            .map(|entry| AgentMemoryRef {
                // The v2 file name, which is the handle every other surface addresses this memory
                // by, including the body fetch above.
                id: format!("{}.md", entry.id),
                revision: entry.revision,
                name: entry.name.clone(),
                description: entry.description.clone(),
                memory_type: to_runtime_type(entry.memory_type),
                updated_at: Some(SystemTime::from(entry.updated_at)),
            })
            .collect(),
        eligible_total: snapshot.memory.eligible_total,
        blocked_reason: access
            .block_reason
            .map(|reason| reason.as_str().to_string()),
    }
}
