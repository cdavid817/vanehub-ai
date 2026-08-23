// The dispatch engine that reads these lands with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What one Hook is, in one snapshot — and what a second recording of the same pair means.
//!
//! Identity is split in two, and the split is the whole point. A **subject** is the stable
//! `<hook-global-id>` and lives for as long as any evidence mentions it. A **definition revision**
//! is `(subject, snapshot)` and is immutable. A user's binding references the *subject*.
//!
//! The alternative — a binding pointing at a versioned definition row — fails in three ways at
//! once: an upgrade orphans every binding or forces user state to be rewritten, a definition that
//! is momentarily unavailable takes the binding down with it, and rolling back to a previous
//! snapshot silently resurrects whatever enablement that snapshot happened to ship with.
//!
//! ## Recording the same revision twice
//!
//! Reinstalling a snapshot re-records its definitions, so recording must be idempotent — but only
//! for the *same* definition. `(subject, snapshot)` naming two different digests is not a
//! duplicate; it is two incompatible answers to "what does this Hook do in this snapshot", and
//! taking the later one would let a rebuild change what an already-installed snapshot means.
//! Neither is silently discarded: the conflict carries both digests, because which digest is bound
//! and which was offered is the entire content of the finding.

use super::{DefinitionDigest, HookGlobalId, SnapshotRef};

/// When a Hook is dispatched.
///
/// A closed set: an event this build does not know is a definition it cannot honour, and admitting
/// it as free text would mean storing a Hook that silently never fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    ExtensionActivated,
    ExtensionDeactivated,
}

impl HookEvent {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::ExtensionActivated => "extension_activated",
            Self::ExtensionDeactivated => "extension_deactivated",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_HOOK_EVENTS
            .iter()
            .copied()
            .find(|event| event.as_str() == value)
    }
}

pub(crate) const ALL_HOOK_EVENTS: &[HookEvent] = &[
    HookEvent::SessionStart,
    HookEvent::SessionEnd,
    HookEvent::UserPromptSubmit,
    HookEvent::PreToolUse,
    HookEvent::PostToolUse,
    HookEvent::ExtensionActivated,
    HookEvent::ExtensionDeactivated,
];

/// Where a subject came from.
///
/// Built-ins are seeded by the host and exist before any extension is installed; extension
/// subjects appear when a snapshot contributing them is recorded. Kept because the two have
/// different removal rules and a subject that cannot say which it is would have to be guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HookOrigin {
    Builtin,
    Extension,
}

impl HookOrigin {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Extension => "extension",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "builtin" => Some(Self::Builtin),
            "extension" => Some(Self::Extension),
            _ => None,
        }
    }
}

/// The stable identity a binding attaches to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookSubject {
    pub(crate) hook: HookGlobalId,
    pub(crate) origin: HookOrigin,
    pub(crate) first_seen_at: String,
}

/// What one Hook is, in one snapshot. Immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookDefinitionRevision {
    pub(crate) hook: HookGlobalId,
    pub(crate) snapshot: SnapshotRef,
    pub(crate) event: HookEvent,
    pub(crate) digest: DefinitionDigest,
    pub(crate) recorded_at: String,
}

/// What recording a revision would mean, given whatever is already recorded for the pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DefinitionOutcome {
    /// Nothing was recorded for this pair. The revision binds it.
    Recorded,
    /// The same definition, recorded again. Reinstalling a snapshot is not a conflict.
    AlreadyRecorded,
    /// The pair is bound to a different definition. Refused; both digests are reported.
    Conflict(DefinitionContentConflict),
}

impl DefinitionOutcome {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Recorded => "hook_definition_recorded",
            Self::AlreadyRecorded => "hook_definition_already_recorded",
            Self::Conflict(_) => "hook_definition_content_conflict",
        }
    }

    /// Whether the Hook may be dispatched from this snapshot.
    ///
    /// A conflicted pair has two answers to what the Hook does, and running either is a guess.
    pub(crate) const fn admits_dispatch(&self) -> bool {
        matches!(self, Self::Recorded | Self::AlreadyRecorded)
    }
}

/// The same `(subject, snapshot)`, twice, with different definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefinitionContentConflict {
    pub(crate) recorded_digest: DefinitionDigest,
    pub(crate) offered_digest: DefinitionDigest,
    pub(crate) recorded_event: HookEvent,
    pub(crate) recorded_at: String,
}

impl DefinitionContentConflict {
    pub(crate) const fn code(&self) -> &'static str {
        "hook_definition_content_conflict"
    }
}

/// Decides what recording a revision means against whatever already holds the pair.
///
/// A pure comparison, so the rule lives in one place and a repository only has to report what it
/// found. The digest is the whole comparison: `event` is *part of* what the digest covers, so a
/// revision that changed only its event still lands here as a conflict rather than as a silent
/// re-registration under a different trigger.
pub(crate) fn decide_definition(
    offered: &HookDefinitionRevision,
    recorded: Option<&HookDefinitionRevision>,
) -> DefinitionOutcome {
    let Some(recorded) = recorded else {
        return DefinitionOutcome::Recorded;
    };
    if recorded.digest == offered.digest {
        return DefinitionOutcome::AlreadyRecorded;
    }
    DefinitionOutcome::Conflict(DefinitionContentConflict {
        recorded_digest: recorded.digest.clone(),
        offered_digest: offered.digest.clone(),
        recorded_event: recorded.event,
        recorded_at: recorded.recorded_at.clone(),
    })
}
