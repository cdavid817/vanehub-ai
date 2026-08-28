use crate::contexts::personalization::api::PersonalizationApi;

use std::time::SystemTime;

use crate::contexts::agent_runtime::application::{
    AgentCandidateOutcome, AgentCandidateSubmission, AgentMemoryAccess, AgentMemoryBody,
    AgentMemoryDelivery, AgentMemoryProposal, AgentMemoryRef, AgentPersonalizationSnapshot,
    AgentPersonalizationSnapshotPort, AgentProposalOrigin, AgentRuntimeApplicationError,
    GenerationPersonalizationContext,
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

/// Satisfies `agent_runtime`'s personalization boundary from the governed policy.
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
            // Translated at the boundary rather than shared as one type: sessions records what
            // the user chose and personalization decides what it means, and a mode this build
            // cannot parse resolves as temporary rather than standard — the narrow reading is the
            // only safe one when the stored value is not understood.
            session_mode: session_mode_from(&context.personalization_mode),
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
            source_message_id: submission.source_message_id.clone(),
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

/// The mode the session recorded, as personalization understands it.
///
/// Translated at the boundary rather than shared as one type: sessions records what the user chose
/// and personalization decides what it means, and one context's taxonomy must not become the
/// other's dependency.
///
/// A value this build cannot parse resolves as temporary rather than standard. The narrow reading
/// is the only safe one: a mode written by a newer build is more likely to mean "retain less" than
/// "retain everything", and defaulting the other way would silently widen a session the user asked
/// to keep narrow.
pub(super) fn session_mode_from(stored: &str) -> SessionPersonalizationMode {
    match stored {
        "project-only" => SessionPersonalizationMode::ProjectOnly,
        "temporary" => SessionPersonalizationMode::Temporary,
        "standard" | "" => SessionPersonalizationMode::Standard,
        _ => SessionPersonalizationMode::Temporary,
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
