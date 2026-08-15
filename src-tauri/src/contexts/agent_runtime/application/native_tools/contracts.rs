use super::{
    CanonicalToolResource, NativeToolErrorCode, NativeToolLimitProfile, NativeToolOperation,
    NativeToolReadinessReasonCode,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub(crate) const NATIVE_TOOL_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolExecutionMode {
    Plan,
    Execute,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeToolDefinition {
    pub(crate) contract_version: u16,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
    pub(crate) operations: Vec<NativeToolOperation>,
    pub(crate) plan_mode_compatible: bool,
    pub(crate) limit_profile: NativeToolLimitProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolEligibilityContext {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) canonical_workspace: Option<PathBuf>,
    pub(crate) execution_mode: NativeToolExecutionMode,
    pub(crate) readiness: BTreeMap<String, Option<NativeToolReadinessReasonCode>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolEligibility {
    Eligible,
    Ineligible {
        reason: NativeToolReadinessReasonCode,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValidatedNativeToolInput {
    pub(crate) value: Value,
    pub(crate) input_hash: String,
    pub(crate) operation: NativeToolOperation,
    pub(crate) resource: CanonicalToolResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolProgressPhase {
    Started,
    Updated,
    #[allow(dead_code)]
    AwaitingHuman,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeToolProgress {
    pub(crate) sequence: u32,
    pub(crate) phase: NativeToolProgressPhase,
    pub(crate) message: Option<String>,
    pub(crate) metadata: BTreeMap<String, Value>,
}

pub(crate) trait NativeToolProgressSink: std::fmt::Debug + Send + Sync {
    fn publish(&self, progress: NativeToolProgress);
}

#[derive(Debug, Clone)]
pub(crate) struct NativeToolExecutionContext {
    pub(crate) call_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) agent_id: String,
    pub(crate) canonical_workspace: Option<PathBuf>,
    pub(crate) deadline: Instant,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) progress: Arc<dyn NativeToolProgressSink>,
}

impl NativeToolExecutionContext {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn deadline_reached(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeToolResultStatus {
    Succeeded,
    Denied,
    Unavailable,
    Cancelled,
    Failed,
    LimitExceeded,
}

impl NativeToolResultStatus {
    #[allow(dead_code)]
    pub(crate) const fn is_terminal(self) -> bool {
        true
    }
}

/// Metadata key by which a native tool declares that its result includes an image, naming the
/// Artifact it just sealed. Deliberately an id rather than a new envelope field carrying bytes:
/// the envelope is constructed in more than ninety places, and an id keeps this additive and
/// opt-in per tool while keeping base64 out of both the persisted output and the stored operation
/// record (`add-onepiece-visual-tool-returns`).
pub(crate) const IMAGE_ARTIFACT_METADATA_KEY: &str = "image_artifact_id";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeToolResultEnvelope {
    pub(crate) contract_version: u16,
    pub(crate) status: NativeToolResultStatus,
    pub(crate) output: Option<Value>,
    pub(crate) error_code: Option<NativeToolErrorCode>,
    pub(crate) safe_error: Option<String>,
    pub(crate) truncated: bool,
    pub(crate) metadata: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Debug, Default)]
    struct CapturingProgressSink {
        events: Mutex<Vec<NativeToolProgress>>,
    }

    impl NativeToolProgressSink for CapturingProgressSink {
        fn publish(&self, progress: NativeToolProgress) {
            self.events.lock().expect("events").push(progress);
        }
    }

    #[test]
    fn execution_context_owns_cancellation_deadline_and_progress() {
        let sink = Arc::new(CapturingProgressSink::default());
        let cancelled = Arc::new(AtomicBool::new(false));
        let context = NativeToolExecutionContext {
            call_id: "call-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: Some(PathBuf::from("C:/workspace")),
            deadline: Instant::now() + Duration::from_secs(10),
            cancelled: cancelled.clone(),
            progress: sink.clone(),
        };

        assert!(!context.is_cancelled());
        assert!(!context.deadline_reached());
        context.progress.publish(NativeToolProgress {
            sequence: 1,
            phase: NativeToolProgressPhase::Started,
            message: None,
            metadata: BTreeMap::new(),
        });
        cancelled.store(true, Ordering::Release);
        assert!(context.is_cancelled());
        assert_eq!(sink.events.lock().expect("events").len(), 1);
    }

    #[test]
    fn result_envelope_has_a_versioned_terminal_contract() {
        let result = NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::LimitExceeded,
            output: None,
            error_code: Some(NativeToolErrorCode::LimitExceeded),
            safe_error: Some("Output limit reached.".to_owned()),
            truncated: true,
            metadata: BTreeMap::new(),
        };

        assert!(result.status.is_terminal());
        assert_eq!(result.contract_version, NATIVE_TOOL_CONTRACT_VERSION);
        assert_eq!(
            result.error_code.map(NativeToolErrorCode::as_str),
            Some("limit_exceeded")
        );
    }
}
