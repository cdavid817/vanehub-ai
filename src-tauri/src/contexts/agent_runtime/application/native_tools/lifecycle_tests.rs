use super::execute_with_lifecycle;
use crate::contexts::agent_runtime::application::{
    CanonicalToolResource, NativeToolDefinition, NativeToolErrorCode, NativeToolExecutionContext,
    NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile, NativeToolOperation,
    NativeToolPermissionRequest, NativeToolProgress, NativeToolProgressPhase,
    NativeToolProgressSink, NativeToolResultEnvelope, NativeToolResultStatus, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct CapturingProgress {
    events: Mutex<Vec<NativeToolProgress>>,
}

impl NativeToolProgressSink for CapturingProgress {
    fn publish(&self, progress: NativeToolProgress) {
        self.events.lock().expect("events").push(progress);
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    Success,
    Oversized,
    InvalidProgress,
    ProgressFlood,
    Cancel,
    Panic,
}

struct Handler {
    definition: NativeToolDefinition,
    behavior: Behavior,
    cleanup_calls: Arc<AtomicU32>,
}

impl Handler {
    fn new(behavior: Behavior, output_bytes: u64, progress_events: u32) -> Self {
        Self {
            definition: NativeToolDefinition {
                contract_version: NATIVE_TOOL_CONTRACT_VERSION,
                name: "fixture".to_owned(),
                description: "fixture".to_owned(),
                input_schema: json!({"type": "object"}),
                operations: vec![NativeToolOperation::ArtifactRead],
                plan_mode_compatible: true,
                limit_profile: NativeToolLimitProfile::bounded(
                    1024,
                    output_bytes,
                    1_000,
                    progress_events,
                ),
            },
            behavior,
            cleanup_calls: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl NativeToolHandler for Handler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, _context: &ToolEligibilityContext) -> ToolEligibility {
        ToolEligibility::Eligible
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        Ok(validated(input.clone()))
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            action: Action::new("artifact.read"),
            resource: Resource::new("artifact/1"),
            operation: input.operation,
            canonical_resource: input.resource.clone(),
            input_hash: input.input_hash.clone(),
        }
    }

    fn execute(
        &self,
        _input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        match self.behavior {
            Behavior::Oversized => success(Some(json!({"value": "0123456789"}))),
            Behavior::InvalidProgress => {
                context.progress.publish(progress(1));
                context.progress.publish(progress(1));
                success(Some(json!({"ok": true})))
            }
            Behavior::ProgressFlood => {
                for sequence in 1..=10_000 {
                    context.progress.publish(progress(sequence));
                }
                success(Some(json!({"ok": true})))
            }
            Behavior::Cancel => {
                context.cancelled.store(true, Ordering::Release);
                success(None)
            }
            Behavior::Panic => panic!("private backend failure"),
            Behavior::Success => success(Some(json!({"ok": true}))),
        }
    }

    fn cleanup(&self, _context: &NativeToolExecutionContext) -> Result<(), NativeToolHandlerError> {
        self.cleanup_calls.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

fn context(progress: Arc<dyn NativeToolProgressSink>) -> NativeToolExecutionContext {
    NativeToolExecutionContext {
        call_id: "call-1".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        agent_id: "onepiece".to_owned(),
        canonical_workspace: Some(PathBuf::from("C:/workspace")),
        deadline: Instant::now() + Duration::from_secs(10),
        cancelled: Arc::new(AtomicBool::new(false)),
        progress,
    }
}

fn validated(value: Value) -> ValidatedNativeToolInput {
    ValidatedNativeToolInput {
        value,
        input_hash: "sha256:fixture".to_owned(),
        operation: NativeToolOperation::ArtifactRead,
        resource: CanonicalToolResource {
            kind: ToolResourceKind::Artifact,
            canonical_id: "artifact/1".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

fn progress(sequence: u32) -> NativeToolProgress {
    NativeToolProgress {
        sequence,
        phase: NativeToolProgressPhase::Updated,
        message: None,
        metadata: BTreeMap::new(),
    }
}

fn success(output: Option<Value>) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: 999,
        status: NativeToolResultStatus::Succeeded,
        output,
        error_code: None,
        safe_error: None,
        truncated: false,
        metadata: BTreeMap::new(),
    }
}

#[test]
fn lifecycle_bounds_progress_and_keeps_sequences_monotonic() {
    let captured = Arc::new(CapturingProgress::default());
    let handler = Handler::new(Behavior::InvalidProgress, 1024, 2);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(captured.clone()),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::LimitExceeded);
    assert_eq!(captured.events.lock().expect("events").len(), 1);
    assert_eq!(handler.cleanup_calls.load(Ordering::Acquire), 1);
}

#[test]
fn progress_flood_is_bounded_without_forwarding_excess_events() {
    let captured = Arc::new(CapturingProgress::default());
    let handler = Handler::new(Behavior::ProgressFlood, 1024, 64);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(captured.clone()),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::LimitExceeded);
    assert_eq!(captured.events.lock().expect("events").len(), 64);
    assert_eq!(handler.cleanup_calls.load(Ordering::Acquire), 1);
}

#[test]
fn lifecycle_truncates_oversized_output_without_partial_json() {
    let handler = Handler::new(Behavior::Oversized, 4, 2);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(Arc::new(CapturingProgress::default())),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::LimitExceeded);
    assert!(result.output.is_none());
    assert!(result.truncated);
    assert_eq!(result.contract_version, NATIVE_TOOL_CONTRACT_VERSION);
}

#[test]
fn lifecycle_propagates_cancellation_and_always_cleans_up() {
    let handler = Handler::new(Behavior::Cancel, 1024, 2);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(Arc::new(CapturingProgress::default())),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::Cancelled);
    assert_eq!(result.error_code, Some(NativeToolErrorCode::Cancelled));
    assert_eq!(handler.cleanup_calls.load(Ordering::Acquire), 1);
}

#[test]
fn lifecycle_maps_panics_to_safe_terminal_failures_and_cleans_up() {
    let handler = Handler::new(Behavior::Panic, 1024, 2);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(Arc::new(CapturingProgress::default())),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::Failed);
    assert_eq!(
        result.error_code,
        Some(NativeToolErrorCode::InternalFailure)
    );
    assert_eq!(
        result.safe_error.as_deref(),
        Some("The native tool failed unexpectedly.")
    );
    assert_eq!(handler.cleanup_calls.load(Ordering::Acquire), 1);
}

#[test]
fn lifecycle_normalizes_contract_version_on_success() {
    let handler = Handler::new(Behavior::Success, 1024, 2);
    let result = execute_with_lifecycle(
        &handler,
        validated(json!({})),
        context(Arc::new(CapturingProgress::default())),
        &handler.definition.limit_profile,
    );

    assert_eq!(result.status, NativeToolResultStatus::Succeeded);
    assert_eq!(result.contract_version, NATIVE_TOOL_CONTRACT_VERSION);
}
