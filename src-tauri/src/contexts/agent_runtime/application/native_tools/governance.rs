#![allow(dead_code)]

use std::collections::BTreeMap;

pub(crate) const NATIVE_TOOL_LIMIT_PROFILE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolErrorCode {
    InvalidInput,
    Ineligible,
    PermissionDenied,
    StaleApproval,
    Unavailable,
    Cancelled,
    DeadlineExceeded,
    LimitExceeded,
    IntegrityFailure,
    Conflict,
    ExternalFailure,
    InternalFailure,
}

impl NativeToolErrorCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::Ineligible => "ineligible",
            Self::PermissionDenied => "permission_denied",
            Self::StaleApproval => "stale_approval",
            Self::Unavailable => "unavailable",
            Self::Cancelled => "cancelled",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::LimitExceeded => "limit_exceeded",
            Self::IntegrityFailure => "integrity_failure",
            Self::Conflict => "conflict",
            Self::ExternalFailure => "external_failure",
            Self::InternalFailure => "internal_failure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolReadinessReasonCode {
    Disabled,
    MissingDependency,
    VersionMismatch,
    UnhealthyDependency,
    IsolationUnavailable,
    BackendUnavailable,
    PolicyUnavailable,
    DesktopRuntimeRequired,
}

impl NativeToolReadinessReasonCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::MissingDependency => "missing_dependency",
            Self::VersionMismatch => "version_mismatch",
            Self::UnhealthyDependency => "unhealthy_dependency",
            Self::IsolationUnavailable => "isolation_unavailable",
            Self::BackendUnavailable => "backend_unavailable",
            Self::PolicyUnavailable => "policy_unavailable",
            Self::DesktopRuntimeRequired => "desktop_runtime_required",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolLimitProfile {
    pub(crate) version: u16,
    pub(crate) max_input_bytes: u64,
    pub(crate) max_output_bytes: u64,
    pub(crate) max_duration_ms: u64,
    pub(crate) max_progress_events: u32,
    pub(crate) max_child_processes: u32,
    pub(crate) max_memory_bytes: Option<u64>,
    pub(crate) max_disk_bytes: Option<u64>,
}

impl NativeToolLimitProfile {
    pub(crate) fn bounded(
        max_input_bytes: u64,
        max_output_bytes: u64,
        max_duration_ms: u64,
        max_progress_events: u32,
    ) -> Self {
        Self {
            version: NATIVE_TOOL_LIMIT_PROFILE_VERSION,
            max_input_bytes,
            max_output_bytes,
            max_duration_ms,
            max_progress_events,
            max_child_processes: 0,
            max_memory_bytes: None,
            max_disk_bytes: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NativeToolOperation {
    BrowserRead,
    BrowserInteract,
    WebSearch,
    WebFetch,
    CodeExecute,
    OcrRead,
    ArtifactRead,
    ArtifactPublish,
    DelegationAnalyze,
    DelegationEdit,
    DelegationApply,
}

impl NativeToolOperation {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::BrowserRead => "browser.read",
            Self::BrowserInteract => "browser.interact",
            Self::WebSearch => "web.search",
            Self::WebFetch => "web.fetch",
            Self::CodeExecute => "code.execute",
            Self::OcrRead => "ocr.read",
            Self::ArtifactRead => "artifact.read",
            Self::ArtifactPublish => "artifact.publish",
            Self::DelegationAnalyze => "delegation.analyze",
            Self::DelegationEdit => "delegation.edit",
            Self::DelegationApply => "delegation.apply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolResourceKind {
    Workspace,
    BrowserOrigin,
    WebUrl,
    Runtime,
    Artifact,
    DelegationTarget,
    ChangeSet,
}

impl ToolResourceKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::BrowserOrigin => "browser_origin",
            Self::WebUrl => "web_url",
            Self::Runtime => "runtime",
            Self::Artifact => "artifact",
            Self::DelegationTarget => "delegation_target",
            Self::ChangeSet => "change_set",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalToolResource {
    pub(crate) kind: ToolResourceKind,
    pub(crate) canonical_id: String,
    pub(crate) attributes: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_codes_are_machine_readable() {
        assert_eq!(
            NativeToolErrorCode::StaleApproval.as_str(),
            "stale_approval"
        );
        assert_eq!(
            NativeToolReadinessReasonCode::DesktopRuntimeRequired.as_str(),
            "desktop_runtime_required"
        );
        assert_eq!(
            NativeToolOperation::DelegationApply.as_str(),
            "delegation.apply"
        );
        assert_eq!(ToolResourceKind::ChangeSet.as_str(), "change_set");
    }

    #[test]
    fn bounded_profile_is_versioned_and_has_no_implicit_process_budget() {
        let profile = NativeToolLimitProfile::bounded(10, 20, 30, 40);
        assert_eq!(profile.version, NATIVE_TOOL_LIMIT_PROFILE_VERSION);
        assert_eq!(profile.max_input_bytes, 10);
        assert_eq!(profile.max_output_bytes, 20);
        assert_eq!(profile.max_duration_ms, 30);
        assert_eq!(profile.max_progress_events, 40);
        assert_eq!(profile.max_child_processes, 0);
        assert_eq!(profile.max_memory_bytes, None);
        assert_eq!(profile.max_disk_bytes, None);
    }
}
