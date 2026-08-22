// The activation flow that creates generations lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! One activation of one installation's runtime, and the single pointer that says which is live.
//!
//! A generation is created once and never edited: it records which snapshot was activated and
//! when. What changes is which one an installation is *currently* running, and that is one pointer
//! per installation, moved by compare-and-swap. Two rows claiming to be active is not a state this
//! design has a meaning for, so the primary key makes it unrepresentable rather than the code
//! having to check.

use super::{InstallationId, RuntimeGenerationId, SnapshotId};

/// One activation, frozen at the moment it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeGenerationRecord {
    pub(crate) generation: RuntimeGenerationId,
    pub(crate) installation: InstallationId,
    pub(crate) snapshot: SnapshotId,
    pub(crate) started_at: String,
}

/// Which generation an installation is running now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveGeneration {
    pub(crate) installation: InstallationId,
    pub(crate) generation: RuntimeGenerationId,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

/// Why a generation could not be recorded or made active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeGenerationError {
    /// Someone else moved the pointer since the caller read it.
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    /// The generation named does not belong to the installation being pointed at, or does not
    /// exist. Refused by the database's composite reference; reported here so a caller does not
    /// have to read a foreign-key message.
    UnknownGeneration,
    /// The installation named does not exist.
    UnknownInstallation,
    Storage(String),
}

impl RuntimeGenerationError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "runtime_generation_stale_revision",
            Self::UnknownGeneration => "unknown_runtime_generation",
            Self::UnknownInstallation => "unknown_installation",
            Self::Storage(_) => "runtime_generation_storage_failure",
        }
    }
}

pub(crate) fn all_runtime_generation_errors() -> Vec<RuntimeGenerationError> {
    vec![
        RuntimeGenerationError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        RuntimeGenerationError::UnknownGeneration,
        RuntimeGenerationError::UnknownInstallation,
        RuntimeGenerationError::Storage(String::new()),
    ]
}
