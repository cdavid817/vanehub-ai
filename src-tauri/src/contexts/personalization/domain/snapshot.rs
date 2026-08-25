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

/// Which user-authored field a segment carries.
///
/// One segment per field rather than one per layer, because a preview has to be able to say which
/// *field* survived: a layer that replaced the style rules and left the description alone produces
/// two segments with different provenance, and merging them would lose that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionField {
    AboutUser,
    StyleRules,
}

impl InstructionField {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AboutUser => "about_user",
            Self::StyleRules => "style_rules",
        }
    }
}

/// What the merge state machine did with a layer's fields.
///
/// Recorded per segment so a preview can explain the outcome rather than restate the stored mode:
/// "appended by the workspace layer" and "replaced by the Agent layer" look identical in a final
/// text and completely different to a user trying to find where a sentence came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionMergeAction {
    Appended,
    Replaced,
}

impl InstructionMergeAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Appended => "appended",
            Self::Replaced => "replaced",
        }
    }
}

/// Why a layer's field is not in the final instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionExclusionReason {
    /// The field was empty at this layer, so there was nothing to contribute.
    EmptyField,
    /// A later layer replaced everything below it.
    ReplacedByHigherLayer,
    /// A later layer disabled personalization instructions entirely.
    DisabledByHigherLayer,
    /// This layer inherits, so it contributes nothing of its own.
    InheritedLayer,
    /// The runtime does not accept custom instructions at all.
    RuntimeCapability,
}

impl InstructionExclusionReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::EmptyField => "empty_field",
            Self::ReplacedByHigherLayer => "replaced_by_higher_layer",
            Self::DisabledByHigherLayer => "disabled_by_higher_layer",
            Self::InheritedLayer => "inherited_layer",
            Self::RuntimeCapability => "runtime_capability",
        }
    }
}

/// Where one surviving instruction field came from.
///
/// Carries provenance rather than only text: the scope that authored it, the revision that scope
/// was at, and what the merge did. Without those, a user reading a preview can see *what* is being
/// sent but has no way to find *which* setting to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedInstructionSegment {
    pub(crate) field: InstructionField,
    pub(crate) scope_kind: &'static str,
    pub(crate) scope_key: String,
    /// The revision the authoring layer was at when this snapshot was taken. A later edit produces
    /// a different snapshot rather than mutating this one.
    pub(crate) policy_revision: u64,
    pub(crate) merge_action: InstructionMergeAction,
    pub(crate) text: String,
}

/// One field that did not survive, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExcludedInstructionSegment {
    pub(crate) field: InstructionField,
    pub(crate) scope_kind: &'static str,
    pub(crate) scope_key: String,
    pub(crate) policy_revision: u64,
    pub(crate) reason: InstructionExclusionReason,
}

impl ResolvedInstructionSegment {
    /// The segments one layer contributes, and the ones it does not.
    ///
    /// An empty field is reported as excluded rather than dropped: "you set nothing here" and "this
    /// layer never came up" are different answers, and a preview that showed neither would leave a
    /// user hunting through scopes for text that does not exist.
    pub(crate) fn from_scope(
        scope: &PersonalizationPolicyScope,
        policy_revision: u64,
        merge_action: InstructionMergeAction,
        about_user: &str,
        style_rules: &str,
    ) -> (Vec<Self>, Vec<ExcludedInstructionSegment>) {
        let mut included = Vec::new();
        let mut excluded = Vec::new();
        for (field, text) in [
            (InstructionField::AboutUser, about_user),
            (InstructionField::StyleRules, style_rules),
        ] {
            if text.is_empty() {
                excluded.push(ExcludedInstructionSegment {
                    field,
                    scope_kind: scope.scope_kind(),
                    scope_key: scope.scope_key(),
                    policy_revision,
                    reason: InstructionExclusionReason::EmptyField,
                });
                continue;
            }
            included.push(Self {
                field,
                scope_kind: scope.scope_kind(),
                scope_key: scope.scope_key(),
                policy_revision,
                merge_action,
                text: text.to_string(),
            });
        }
        (included, excluded)
    }

    /// Restates this segment as an exclusion, for when a higher layer removes it.
    pub(crate) fn excluded_by(
        &self,
        reason: InstructionExclusionReason,
    ) -> ExcludedInstructionSegment {
        ExcludedInstructionSegment {
            field: self.field,
            scope_kind: self.scope_kind,
            scope_key: self.scope_key.clone(),
            policy_revision: self.policy_revision,
            reason,
        }
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
    /// The Agent is not in the registry, so no capabilities can be established for it.
    UnknownAgent,
    /// A project-only session reached resolution with no workspace to be isolated to.
    WorkspaceRequired,
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
    /// Every user-authored field that did not make it, with the reason. Present so a preview can
    /// answer "why is my instruction not being used" without the user diffing scopes by hand.
    pub(crate) excluded_instruction_segments: Vec<ExcludedInstructionSegment>,
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
            excluded_instruction_segments: Vec::new(),
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
