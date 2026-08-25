use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::memory::{MemoryId, MemoryType};
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

/// How memory reaches a runtime, if at all.
///
/// Decided by capability, never by policy: a policy that permits reading cannot make a runtime
/// accept an injection mechanism it has no place to put. Widening this is the only thing
/// selected-body support does — it never makes a memory eligible that was not already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryDeliveryMode {
    /// Nothing is delivered. Either the runtime cannot take an index, or memory is unavailable.
    None,
    /// A bounded pointer list only: names and descriptions, no bodies.
    IndexOnly,
    /// The index, plus the bodies of whichever memories relevance selection picks.
    IndexWithSelectedBodies,
}

impl MemoryDeliveryMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::IndexOnly => "index_only",
            Self::IndexWithSelectedBodies => "index_with_selected_bodies",
        }
    }
}

/// Why memory is unavailable for this whole snapshot, when it is.
///
/// Whole-snapshot rather than per-record. When one of these holds, no record was considered at all,
/// and reporting per-record exclusion counts would imply an enumeration that never happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryBlockReason {
    /// Policy resolved memory reading to off.
    ReadDisabled,
    /// The session mode forbids long-term memory outright.
    SessionMode,
    /// The runtime declares no memory index.
    RuntimeCapability,
    /// Migration or repair has not established a safe generation.
    MaintenanceState,
    /// No validated policy could be established.
    NoValidatedPolicy,
}

impl MemoryBlockReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ReadDisabled => "read_disabled",
            Self::SessionMode => "session_mode",
            Self::RuntimeCapability => "runtime_capability",
            Self::MaintenanceState => "maintenance_state",
            Self::NoValidatedPolicy => "no_validated_policy",
        }
    }
}

/// Which scopes a memory may be read from, after every restriction.
///
/// Stated as an allowance rather than derived at each call site, so a caller cannot reconstruct it
/// differently: "global is permitted" and "this one workspace is permitted" are the only two
/// dimensions, and a project-only session is exactly the case where the first is false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryScopeAllowance {
    pub(crate) global: bool,
    /// The single workspace whose memories may be read. `None` means no workspace scope is
    /// readable at all, which is different from "any workspace".
    pub(crate) workspace: Option<WorkspaceKey>,
}

/// Where a new memory may be saved, when saving is permitted at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemorySaveConstraint {
    /// Saving is not permitted in this snapshot.
    Denied,
    /// The user may choose global or the active workspace.
    GlobalOrWorkspace { workspace: Option<WorkspaceKey> },
    /// Only the active workspace — a project-only session, where global is not offered at all.
    WorkspaceOnly { workspace: WorkspaceKey },
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
    /// Whether a completed exchange may produce a candidate at all. Separate from
    /// `automatic_extraction` because a temporary session forbids even proposing one, while an
    /// ordinary session with extraction off simply does not run the extractor.
    pub(crate) candidate_creation: bool,
    /// Whether anything may be written to the retrieval index. A temporary session leaves no trace
    /// there either, which a read-only check would not have covered.
    pub(crate) retrieval_write: bool,
    pub(crate) delivery: MemoryDeliveryMode,
    /// Present exactly when nothing may be read. `None` alongside `read: true` is the only
    /// combination a caller should ever act on.
    pub(crate) block_reason: Option<MemoryBlockReason>,
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
            candidate_creation: false,
            retrieval_write: false,
            delivery: MemoryDeliveryMode::None,
            block_reason: None,
        }
    }

    /// Denies everything and records why, in one step, so no caller can produce a denial with no
    /// explanation attached.
    pub(crate) fn blocked(reason: MemoryBlockReason) -> Self {
        Self {
            block_reason: Some(reason),
            ..Self::denied()
        }
    }

    /// Turns this into a denial, keeping an already-recorded reason.
    ///
    /// First reason wins: the earliest restriction to fire is the most fundamental one, and
    /// overwriting it with a later, narrower one would tell the user the wrong thing to fix.
    pub(crate) fn block(&mut self, reason: MemoryBlockReason) {
        let existing = self.block_reason;
        *self = Self::denied();
        self.block_reason = existing.or(Some(reason));
    }

    pub(crate) fn denies_everything(&self) -> bool {
        !self.read && !self.explicit_save && !self.automatic_extraction && !self.global_memory
    }

    /// Which scopes may be read from, after everything.
    pub(crate) fn readable_scopes(&self) -> MemoryScopeAllowance {
        if !self.read {
            return MemoryScopeAllowance {
                global: false,
                workspace: None,
            };
        }
        MemoryScopeAllowance {
            global: self.global_memory,
            workspace: self.workspace.clone(),
        }
    }

    /// Where a new memory may be saved.
    ///
    /// A project-only session is the case where global is not merely disallowed but not offered:
    /// the caller has no choice to present, which is different from presenting one that will fail.
    pub(crate) fn save_constraint(&self) -> MemorySaveConstraint {
        if !self.explicit_save {
            return MemorySaveConstraint::Denied;
        }
        match (self.global_memory, self.workspace.as_ref()) {
            (false, Some(workspace)) => MemorySaveConstraint::WorkspaceOnly {
                workspace: workspace.clone(),
            },
            (false, None) => MemorySaveConstraint::Denied,
            (true, workspace) => MemorySaveConstraint::GlobalOrWorkspace {
                workspace: workspace.cloned(),
            },
        }
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

impl PersonalizationExclusionReason {
    /// A stable code for diagnostics, and the key exclusion counts are ordered by so two identical
    /// stores produce identical summaries.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ProjectOnlySession => "project_only_session",
            Self::TemporarySession => "temporary_session",
            Self::OtherWorkspace => "other_workspace",
            Self::AgentAudience => "agent_audience",
            Self::PendingCandidate => "pending_candidate",
            Self::Archived => "archived",
            Self::GlobalMemoryDisabled => "global_memory_disabled",
            Self::MemoryReadDisabled => "memory_read_disabled",
            Self::RuntimeCapability => "runtime_capability",
            Self::UnsafeMaintenanceState => "unsafe_maintenance_state",
        }
    }
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

/// One eligible memory as the snapshot pins it.
///
/// Carries the id *and* the revision, which is what makes reading a body later safe: a consumer
/// fetches by id at the pinned revision, so an edit made mid-generation produces a conflict rather
/// than silently substituting different text into a turn that was planned around the old one.
/// Never carries a body — a snapshot is taken before token budgeting, and loading every body to
/// decide what fits would defeat the budgeting it feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotMemoryRef {
    pub(crate) id: MemoryId,
    pub(crate) revision: u64,
    /// Fingerprint of the body at that revision. Lets a later read prove it got the same text
    /// without having had the text here.
    pub(crate) content_hash: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    /// `global`, or the workspace this belongs to. A hint for display and grouping, never an
    /// authorization input — eligibility was already decided when this ref was produced.
    pub(crate) scope_hint: String,
    pub(crate) updated_at: DateTime<Utc>,
}

/// One primary exclusion reason and how many records it accounted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryExclusionCount {
    pub(crate) reason: PersonalizationExclusionReason,
    pub(crate) count: usize,
}

/// What eligibility found, bounded.
///
/// `considered` is every record the query looked at; `eligible_total` is how many passed. Each
/// excluded record is counted under exactly one reason — the first that applies, by a fixed
/// precedence — so `eligible_total + sum(exclusions) == considered` holds by construction rather
/// than by careful bookkeeping. Without that invariant a user reading "3 of 40 eligible" has no way
/// to know whether the other 37 are accounted for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MemoryEligibilitySummary {
    pub(crate) considered: usize,
    pub(crate) eligible_total: usize,
    /// Bounded: at most the delivery budget. `eligible_total` is the honest count regardless.
    pub(crate) refs: Vec<SnapshotMemoryRef>,
    /// True when `refs` holds fewer than `eligible_total`, so a reader never mistakes a page for
    /// the whole set.
    pub(crate) truncated: bool,
    pub(crate) exclusions: Vec<MemoryExclusionCount>,
    /// A digest over the eligible ids and revisions, in a fixed order. Part of the revision token,
    /// so a memory edited between two generations produces a different token.
    pub(crate) digest: String,
}

impl MemoryEligibilitySummary {
    /// The invariant every consumer relies on, stated where it can be asserted.
    pub(crate) fn is_balanced(&self) -> bool {
        let excluded: usize = self.exclusions.iter().map(|entry| entry.count).sum();
        self.eligible_total + excluded == self.considered
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
    /// What was eligible when this snapshot was taken. Frozen with it: a memory edited, archived or
    /// deleted afterwards does not change these counts or these refs, and the next snapshot is what
    /// reflects the new state.
    pub(crate) memory: MemoryEligibilitySummary,
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
            memory_access: EffectiveMemoryAccess::blocked(block_reason_for(code)),
            memory: MemoryEligibilitySummary::default(),
            exclusions: Vec::new(),
            warnings: vec![PersonalizationWarning::new(code)],
        }
    }

    /// Attaches the eligibility this snapshot was taken with, folding its digest into the token.
    ///
    /// Two steps because the two depend on each other in one direction only: which memories are
    /// eligible follows from the resolved access, and the token has to cover what was eligible. So
    /// access resolves first, eligibility is queried with it, and the token is finalized here —
    /// with the canonical encoding still living in one place.
    pub(crate) fn with_memory(mut self, memory: MemoryEligibilitySummary) -> Self {
        if self.revision_token != FAIL_CLOSED_REVISION_TOKEN {
            let mut hasher = Sha256::new();
            hasher.update(SNAPSHOT_TOKEN_VERSION.as_bytes());
            hasher.update(b"\x1fbase=");
            hasher.update(self.revision_token.as_bytes());
            hasher.update(b"\x1feligible=");
            hasher.update(memory.digest.as_bytes());
            let digest = hasher.finalize();
            self.revision_token = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        }
        self.memory = memory;
        self
    }

    pub(crate) fn has_user_instructions(&self) -> bool {
        !self.instruction_segments.is_empty()
    }
}

/// Why a fail-closed snapshot denies memory, from the warning that produced it.
fn block_reason_for(code: PersonalizationWarningCode) -> MemoryBlockReason {
    match code {
        PersonalizationWarningCode::MigrationIncomplete
        | PersonalizationWarningCode::RepairRequired => MemoryBlockReason::MaintenanceState,
        PersonalizationWarningCode::UnsupportedCapabilityOverride => {
            MemoryBlockReason::RuntimeCapability
        }
        PersonalizationWarningCode::WorkspaceRequired => MemoryBlockReason::SessionMode,
        // An unknown Agent and an unreadable policy are the same situation from memory's side:
        // nothing validated says this may be read.
        PersonalizationWarningCode::UnknownAgent
        | PersonalizationWarningCode::NoValidatedPolicy
        | PersonalizationWarningCode::UsingLastKnownGoodPolicy => {
            MemoryBlockReason::NoValidatedPolicy
        }
    }
}

/// Bumped whenever what the token hashes changes, so two encodings can never collide and be read
/// as the same snapshot.
pub(crate) const SNAPSHOT_TOKEN_VERSION: &str = "personalization-snapshot-v2";

/// A recognizable non-hash token so a diagnostic never has to guess whether a snapshot came from
/// real policy or from the fallback.
pub(crate) const FAIL_CLOSED_REVISION_TOKEN: &str = "fail-closed";
