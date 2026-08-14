#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolLogIdentity {
    pub(crate) operation_id: String,
    pub(crate) call_id: String,
    pub(crate) execution_run_id: String,
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) tool_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolLogEventKind {
    Started,
    Progress,
    AwaitingApproval,
    Denied,
    Completed,
    Cancelled,
    LimitExceeded,
    ReadinessUnavailable,
    ExternalProcessFailed,
    Failed,
}

impl NativeToolLogEventKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Progress => "progress",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Denied => "denied",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::LimitExceeded => "limit_exceeded",
            Self::ReadinessUnavailable => "readiness_unavailable",
            Self::ExternalProcessFailed => "external_process_failed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeToolSafeLogMetadata {
    pub(crate) operation: Option<String>,
    pub(crate) outcome: Option<String>,
    pub(crate) reason_code: Option<String>,
    pub(crate) resource_hash: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) mode: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) observed_count: Option<u64>,
    pub(crate) limit_count: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NativeToolPrivateLogData {
    pub(crate) raw_input: Option<String>,
    pub(crate) raw_output: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) credential: Option<String>,
    pub(crate) external_process_error: Option<String>,
    pub(crate) environment: std::collections::BTreeMap<String, String>,
    pub(crate) headers: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeToolLogEvent {
    pub(crate) identity: NativeToolLogIdentity,
    pub(crate) kind: NativeToolLogEventKind,
    pub(crate) safe: NativeToolSafeLogMetadata,
    pub(crate) private: NativeToolPrivateLogData,
}
