use super::policy::{InstructionMergeMode, SessionPersonalizationMode};
use super::scope::{
    AgentId, AgentRuntimeKind, PersonalizationPolicyScope, SessionId, WorkspaceIdentity,
    WorkspaceKey,
};

/// Everything a generation knows about itself before any policy is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonalizationResolutionContext {
    pub(crate) agent_id: AgentId,
    pub(crate) session_id: SessionId,
    pub(crate) workspace: Option<WorkspaceIdentity>,
    pub(crate) runtime_kind: AgentRuntimeKind,
    pub(crate) session_mode: SessionPersonalizationMode,
}

/// What a runtime adapter declares it can actually consume.
///
/// The UI derives available controls from this rather than assuming every Agent supports selected
/// memory bodies or automatic extraction. A capability the runtime does not have wins over a
/// policy value that says otherwise, because an enabled policy cannot make a CLI accept an
/// injection mechanism it has no place to put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PersonalizationRuntimeCapabilities {
    pub(crate) supports_custom_instructions: bool,
    pub(crate) supports_memory_index: bool,
    pub(crate) supports_selected_memory_bodies: bool,
    pub(crate) supports_automatic_extraction: bool,
}

impl PersonalizationRuntimeCapabilities {
    /// What a runtime with no declared capabilities gets: nothing. An adapter that forgets to
    /// declare must fail closed rather than inherit OnePiece's full surface.
    pub(crate) fn none() -> Self {
        Self {
            supports_custom_instructions: false,
            supports_memory_index: false,
            supports_selected_memory_bodies: false,
            supports_automatic_extraction: false,
        }
    }
}

/// One included instruction segment, with enough provenance for the preview to explain why it is
/// present. Carries the scope key rather than a display label so the explanation cannot drift from
/// the row that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedInstructionSegment {
    pub(crate) scope_kind: &'static str,
    pub(crate) scope_key: String,
    pub(crate) about_user: String,
    pub(crate) style_rules: String,
}

impl ResolvedInstructionSegment {
    pub(crate) fn from_scope(
        scope: &PersonalizationPolicyScope,
        about_user: &str,
        style_rules: &str,
    ) -> Option<Self> {
        if about_user.is_empty() && style_rules.is_empty() {
            return None;
        }
        Some(Self {
            scope_kind: scope.scope_kind(),
            scope_key: scope.scope_key(),
            about_user: about_user.to_string(),
            style_rules: style_rules.to_string(),
        })
    }
}

/// The four memory dimensions after precedence, session mode, and capabilities have all been
/// applied. `workspace` is the only scope a workspace memory may match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveMemoryAccess {
    pub(crate) read: bool,
    pub(crate) explicit_save: bool,
    pub(crate) automatic_extraction: bool,
    pub(crate) global_memory: bool,
    pub(crate) workspace: Option<WorkspaceKey>,
}

impl EffectiveMemoryAccess {
    /// Every dimension off. Used both by the fail-closed snapshot and by temporary mode.
    pub(crate) fn denied() -> Self {
        Self {
            read: false,
            explicit_save: false,
            automatic_extraction: false,
            global_memory: false,
            workspace: None,
        }
    }

    pub(crate) fn denies_everything(&self) -> bool {
        !self.read && !self.explicit_save && !self.automatic_extraction && !self.global_memory
    }
}

/// Why something the user might expect to be present is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersonalizationExclusionReason {
    /// A global memory in a project-only session.
    ProjectOnlySession,
    /// Any memory in a temporary session.
    TemporarySession,
    /// A workspace memory whose key is not the active workspace.
    OtherWorkspace,
    /// The current Agent is not in the memory's audience.
    AgentAudience,
    /// Still awaiting review; never injected.
    PendingCandidate,
    Archived,
    /// Effective global-memory access resolved disabled.
    GlobalMemoryDisabled,
    /// Effective memory read resolved disabled.
    MemoryReadDisabled,
    /// The runtime does not declare the capability this would require.
    RuntimeCapability,
    /// Migration or reconciliation has not established a safe generation.
    UnsafeMaintenanceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonalizationExclusion {
    pub(crate) reason: PersonalizationExclusionReason,
    pub(crate) count: usize,
}

/// A safe, code-shaped diagnostic. Never carries instruction text, memory bodies, credentials, or
/// filesystem paths — these travel to the frontend and into logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersonalizationWarningCode {
    /// Persistence could not be read; a previously validated policy is being used instead.
    UsingLastKnownGoodPolicy,
    /// No validated policy exists; instructions omitted and memory denied.
    NoValidatedPolicy,
    /// Migration has not completed, so memory stays unavailable.
    MigrationIncomplete,
    /// Derived state diverged from the authoritative records.
    RepairRequired,
    /// A stored override exists for a dimension this runtime cannot use.
    UnsupportedCapabilityOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PersonalizationWarning {
    pub(crate) code: PersonalizationWarningCode,
}

impl PersonalizationWarning {
    pub(crate) fn new(code: PersonalizationWarningCode) -> Self {
        Self { code }
    }
}

/// The immutable result of resolving policy for one generation or Agent seat turn.
///
/// Immutable is the point: a policy saved while a generation is in flight applies to the next
/// generation, never to this one. Rebuilding the prompt from a newer revision mid-flight would
/// make a single turn's behavior unexplainable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePersonalizationSnapshot {
    pub(crate) revision_token: String,
    pub(crate) context: PersonalizationResolutionContext,
    pub(crate) effective_instruction_mode: InstructionMergeMode,
    pub(crate) instruction_segments: Vec<ResolvedInstructionSegment>,
    pub(crate) memory_access: EffectiveMemoryAccess,
    pub(crate) exclusions: Vec<PersonalizationExclusion>,
    pub(crate) warnings: Vec<PersonalizationWarning>,
}

impl EffectivePersonalizationSnapshot {
    /// What a generation gets when no validated policy can be loaded: no user instructions, no
    /// long-term memory in any direction, and a warning. The generation still runs — a
    /// personalization failure must not take down the Agent — but it never falls open into memory.
    pub(crate) fn fail_closed(
        context: PersonalizationResolutionContext,
        code: PersonalizationWarningCode,
    ) -> Self {
        Self {
            revision_token: FAIL_CLOSED_REVISION_TOKEN.to_string(),
            context,
            effective_instruction_mode: InstructionMergeMode::Disabled,
            instruction_segments: Vec::new(),
            memory_access: EffectiveMemoryAccess::denied(),
            exclusions: Vec::new(),
            warnings: vec![PersonalizationWarning::new(code)],
        }
    }

    pub(crate) fn has_user_instructions(&self) -> bool {
        !self.instruction_segments.is_empty()
    }
}

/// A recognizable non-hash token so a diagnostic never has to guess whether a snapshot came from
/// real policy or from the fallback.
pub(crate) const FAIL_CLOSED_REVISION_TOKEN: &str = "fail-closed";
