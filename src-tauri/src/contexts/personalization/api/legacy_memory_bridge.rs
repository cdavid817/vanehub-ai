use super::{CompatibilityMemory, CompatibilitySaveInput, PersonalizationApi};
use crate::contexts::agent_runtime::application::{
    AgentMemory, AgentMemoryPort, AgentRuntimeApplicationError, MemorySource, SaveMemoryInput,
};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::personalization::domain::MemoryType as GovernedMemoryType;

/// Satisfies `agent_runtime`'s pre-governance memory port from the governed v2 store.
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
