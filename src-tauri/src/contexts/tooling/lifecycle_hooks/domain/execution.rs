// The dispatch engine that appends these lands with Task Group 7; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! That a Hook ran, when, and how it ended — and nothing about what it saw or said.
//!
//! ## What a row may contain
//!
//! An execution record has no payload field, no message field, and no path field. Not "a redacted
//! one" — none. A repository with a free-text column has to be trusted to redact, every caller has
//! to remember to, and the one that forgets writes a prompt into a durable row that outlives the
//! session it came from. Removing the column removes the obligation: the only thing an outcome can
//! say is a `HookOutcomeCode`, whose grammar is lower_snake_case, so a stderr dump does not fit
//! through it and the attempt fails at the constructor.
//!
//! ## Ordering
//!
//! `sequence` is monotonic per subject, assigned as `MAX + 1` under a write lock. Timestamps are
//! not an ordering: two executions inside the same clock tick tie, the clock can go backwards, and
//! "which of these ran first" is exactly the question an execution log exists to answer.
//!
//! Retention is what makes monotonicity safe to state. Because it keeps at least one row —
//! `HookExecutionRetention` cannot be constructed with a window of zero — `MAX` never returns to
//! nothing for a subject that has ever run, so a sequence is never reissued.
//!
//! ## Retention
//!
//! Pruning removes **only terminal rows**, and only those outside the window. A pending or running
//! execution is not old, it is *unfinished*; deleting one turns a Hook that is still going into a
//! Hook that never happened, and the completion that arrives afterwards has nothing to attach to.
//! That is the one deletion an execution log must never make, so it is expressed as a property of
//! the status rather than as a `WHERE` clause someone has to notice.

use super::{HookExecutionId, HookGlobalId, HookOutcomeCode};

/// How far along one execution is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HookExecutionStatus {
    /// Accepted for dispatch, not started.
    Pending,
    /// Started, not finished.
    Running,
    Succeeded,
    Failed,
    TimedOut,
    /// Refused before it ran, by authorization or by a capability gate.
    Denied,
}

impl HookExecutionStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Denied => "denied",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_HOOK_EXECUTION_STATUSES
            .iter()
            .copied()
            .find(|status| status.as_str() == value)
    }

    /// Whether this execution is over.
    ///
    /// The single predicate retention is allowed to prune by. Adding a status without deciding
    /// which side of this line it falls on is a compile error, which is the point of matching
    /// exhaustively rather than listing the terminal ones.
    pub(crate) const fn is_terminal(self) -> bool {
        match self {
            Self::Pending | Self::Running => false,
            Self::Succeeded | Self::Failed | Self::TimedOut | Self::Denied => true,
        }
    }
}

pub(crate) const ALL_HOOK_EXECUTION_STATUSES: &[HookExecutionStatus] = &[
    HookExecutionStatus::Pending,
    HookExecutionStatus::Running,
    HookExecutionStatus::Succeeded,
    HookExecutionStatus::Failed,
    HookExecutionStatus::TimedOut,
    HookExecutionStatus::Denied,
];

/// One execution, as it is stored.
///
/// See the module header for why there is nowhere here to put a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookExecutionRecord {
    pub(crate) execution: HookExecutionId,
    pub(crate) hook: HookGlobalId,
    /// Assigned by the repository, monotonic per subject. Zero on an unappended record.
    pub(crate) sequence: i64,
    pub(crate) status: HookExecutionStatus,
    /// Present only once the execution is over, and only ever a stable code.
    pub(crate) outcome: Option<HookOutcomeCode>,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
}

/// How many executions to keep per subject.
///
/// A window of zero would let retention empty a subject's history, and `MAX(sequence) + 1` would
/// then reissue a sequence number that a previous execution already used. Refusing zero at
/// construction is what makes "sequence is monotonic" true rather than usually true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HookExecutionRetention(usize);

/// What the product keeps when nothing says otherwise.
pub(crate) const DEFAULT_HOOK_EXECUTION_RETENTION: usize = 200;

impl HookExecutionRetention {
    pub(crate) const fn new(keep: usize) -> Option<Self> {
        if keep == 0 {
            return None;
        }
        Some(Self(keep))
    }

    pub(crate) const fn keep(self) -> usize {
        self.0
    }
}

impl Default for HookExecutionRetention {
    fn default() -> Self {
        Self(DEFAULT_HOOK_EXECUTION_RETENTION)
    }
}

/// Why an execution could not be appended or pruned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HookExecutionError {
    /// No subject with this id.
    UnknownSubject,
    /// An execution id that is already recorded. Immutable rows are not re-appended, and silently
    /// treating this as an update would let a finished execution be rewritten.
    DuplicateExecution,
    Storage(String),
}

impl HookExecutionError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::UnknownSubject => "unknown_hook_subject_for_execution",
            Self::DuplicateExecution => "duplicate_hook_execution",
            Self::Storage(_) => "hook_execution_storage_failure",
        }
    }
}

pub(crate) fn all_hook_execution_errors() -> Vec<HookExecutionError> {
    vec![
        HookExecutionError::UnknownSubject,
        HookExecutionError::DuplicateExecution,
        HookExecutionError::Storage(String::new()),
    ]
}
