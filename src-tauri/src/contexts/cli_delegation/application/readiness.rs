use std::path::PathBuf;
use std::sync::Arc;

use super::readiness_support::{required_flags, version_class, VersionClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DelegationTarget {
    ClaudeCode,
    CodexCli,
}

impl DelegationTarget {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::CodexCli => "codex-cli",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DelegationMode {
    Analyze,
    Edit,
}

impl DelegationMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Edit => "edit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationAuthentication {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationProbeObservation {
    pub(crate) executable: PathBuf,
    pub(crate) executable_sha256: String,
    pub(crate) version: String,
    pub(crate) help: String,
    pub(crate) authentication: DelegationAuthentication,
}

pub(crate) trait DelegationProbePort: Send + Sync {
    fn probe(&self, target: DelegationTarget) -> Result<DelegationProbeObservation, ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelegationCapabilityDependencies {
    pub(crate) process_tree_control: bool,
    pub(crate) analyze_isolation: bool,
    pub(crate) edit_isolation: bool,
    pub(crate) artifact_storage: bool,
    pub(crate) codex_network_isolation_canary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationReadinessState {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationReadinessReason {
    Ready,
    ProbeFailed,
    AuthenticationUnavailable,
    VersionUnparseable,
    VersionBelowMinimum,
    VersionAboveReviewed,
    RequiredFlagsMissing,
    ProcessTreeControlUnavailable,
    AnalyzeIsolationUnavailable,
    EditIsolationUnavailable,
    ArtifactStorageUnavailable,
    ProviderChildNetworkIsolationUnavailable,
}

impl DelegationReadinessReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::ProbeFailed => "probe_failed",
            Self::AuthenticationUnavailable => "authentication_unavailable",
            Self::VersionUnparseable => "version_unparseable",
            Self::VersionBelowMinimum => "version_below_minimum",
            Self::VersionAboveReviewed => "version_above_reviewed",
            Self::RequiredFlagsMissing => "required_flags_missing",
            Self::ProcessTreeControlUnavailable => "process_tree_control_unavailable",
            Self::AnalyzeIsolationUnavailable => "analyze_isolation_unavailable",
            Self::EditIsolationUnavailable => "edit_isolation_unavailable",
            Self::ArtifactStorageUnavailable => "artifact_storage_unavailable",
            Self::ProviderChildNetworkIsolationUnavailable => {
                "provider_child_network_isolation_unavailable"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationReadiness {
    pub(crate) target: DelegationTarget,
    pub(crate) mode: DelegationMode,
    pub(crate) state: DelegationReadinessState,
    pub(crate) reason: DelegationReadinessReason,
    pub(crate) executable: Option<PathBuf>,
    pub(crate) executable_sha256: Option<String>,
    pub(crate) version: Option<String>,
}

pub(crate) struct DelegationReadinessService {
    probe: Arc<dyn DelegationProbePort>,
    dependencies: DelegationCapabilityDependencies,
}

impl DelegationReadinessService {
    pub(crate) fn new(
        probe: Arc<dyn DelegationProbePort>,
        dependencies: DelegationCapabilityDependencies,
    ) -> Self {
        Self {
            probe,
            dependencies,
        }
    }

    pub(crate) fn check(&self) -> Vec<DelegationReadiness> {
        [DelegationTarget::ClaudeCode, DelegationTarget::CodexCli]
            .into_iter()
            .flat_map(|target| {
                let observation = self.probe.probe(target);
                [DelegationMode::Analyze, DelegationMode::Edit]
                    .map(|mode| self.evaluate(target, mode, observation.as_ref().ok()))
            })
            .collect()
    }

    fn evaluate(
        &self,
        target: DelegationTarget,
        mode: DelegationMode,
        observation: Option<&DelegationProbeObservation>,
    ) -> DelegationReadiness {
        let Some(observation) = observation else {
            return blocked(target, mode, DelegationReadinessReason::ProbeFailed, None);
        };
        let reason = self.blocking_reason(target, mode, observation);
        let (state, reason) = match reason {
            Some(reason) => (DelegationReadinessState::Blocked, reason),
            None if version_class(target, &observation.version) == VersionClass::AboveReviewed => (
                DelegationReadinessState::Degraded,
                DelegationReadinessReason::VersionAboveReviewed,
            ),
            None => (
                DelegationReadinessState::Ready,
                DelegationReadinessReason::Ready,
            ),
        };
        DelegationReadiness {
            target,
            mode,
            state,
            reason,
            executable: Some(observation.executable.clone()),
            executable_sha256: Some(observation.executable_sha256.clone()),
            version: Some(observation.version.clone()),
        }
    }

    fn blocking_reason(
        &self,
        target: DelegationTarget,
        mode: DelegationMode,
        observation: &DelegationProbeObservation,
    ) -> Option<DelegationReadinessReason> {
        if cfg!(windows)
            && target == DelegationTarget::CodexCli
            && !self.dependencies.codex_network_isolation_canary
        {
            return Some(DelegationReadinessReason::ProviderChildNetworkIsolationUnavailable);
        }
        if observation.authentication == DelegationAuthentication::Unavailable {
            return Some(DelegationReadinessReason::AuthenticationUnavailable);
        }
        match version_class(target, &observation.version) {
            VersionClass::Unparseable => {
                return Some(DelegationReadinessReason::VersionUnparseable)
            }
            VersionClass::Below => return Some(DelegationReadinessReason::VersionBelowMinimum),
            VersionClass::AboveReviewed if mode == DelegationMode::Edit => {
                return Some(DelegationReadinessReason::VersionAboveReviewed)
            }
            VersionClass::Tested | VersionClass::AboveReviewed => {}
        }
        if !required_flags(target, mode)
            .iter()
            .all(|flag| observation.help.contains(flag))
        {
            return Some(DelegationReadinessReason::RequiredFlagsMissing);
        }
        if !self.dependencies.process_tree_control {
            return Some(DelegationReadinessReason::ProcessTreeControlUnavailable);
        }
        if !self.dependencies.analyze_isolation {
            return Some(DelegationReadinessReason::AnalyzeIsolationUnavailable);
        }
        if mode == DelegationMode::Edit && !self.dependencies.edit_isolation {
            return Some(DelegationReadinessReason::EditIsolationUnavailable);
        }
        if mode == DelegationMode::Edit && !self.dependencies.artifact_storage {
            return Some(DelegationReadinessReason::ArtifactStorageUnavailable);
        }
        None
    }
}

fn blocked(
    target: DelegationTarget,
    mode: DelegationMode,
    reason: DelegationReadinessReason,
    observation: Option<&DelegationProbeObservation>,
) -> DelegationReadiness {
    DelegationReadiness {
        target,
        mode,
        state: DelegationReadinessState::Blocked,
        reason,
        executable: observation.map(|value| value.executable.clone()),
        executable_sha256: observation.map(|value| value.executable_sha256.clone()),
        version: observation.map(|value| value.version.clone()),
    }
}

#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;
