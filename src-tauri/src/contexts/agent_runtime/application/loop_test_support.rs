//! In-memory scope doubles for Loop application tests. They model the platform boundary's
//! contract (identity, manifests, diffs, sealing) without touching a filesystem; the real
//! boundary has its own sentinel tests in infrastructure.

use super::{
    AgentClockPort, AgentRegistryRepository, AgentRuntimeApplicationError, ApiAgentGateway,
    LoopAssessmentService, LoopCliCapabilityPort, LoopCliWitness, LoopExecutionAssessment,
    LoopGuardRole, LoopManifestChangeView, LoopManifestRef, LoopNativeCheckView,
    LoopPlatformWitness, LoopRootBinding, LoopRootIdentity, LoopScopeBinding, LoopScopeFailure,
    LoopScopeGuard, LoopScopePlatformPort, LoopSealReceipt, LoopVerificationCancellation,
    LOOP_SCOPE_RUNTIME_REVISION,
};
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopRequestedMode, LoopSideEffectChannel, LOOP_SCOPE_SCHEMA_VERSION,
};
use std::sync::{Arc, Mutex};

/// A platform whose manifests are named by a mutable "tree digest": tests move the digest to
/// simulate writes and list changes to simulate what a diff would find.
#[derive(Default)]
pub(crate) struct FakeScopePlatform {
    pub(crate) tree_digest: Mutex<String>,
    pub(crate) changes: Mutex<Vec<LoopManifestChangeView>>,
    pub(crate) native_check: Mutex<Option<LoopNativeCheckView>>,
    pub(crate) fail_capture: Mutex<Option<LoopScopeFailure>>,
    pub(crate) fail_seal: Mutex<Option<LoopScopeFailure>>,
    pub(crate) fail_root: Mutex<Option<LoopScopeFailure>>,
    pub(crate) captured: Mutex<Vec<String>>,
    pub(crate) sealed: Mutex<Vec<String>>,
}

impl FakeScopePlatform {
    pub(crate) fn new(tree_digest: &str) -> Arc<Self> {
        Arc::new(Self {
            tree_digest: Mutex::new(tree_digest.to_string()),
            ..Self::default()
        })
    }

    pub(crate) fn mutate_tree(&self, digest: &str) {
        *self.tree_digest.lock().expect("tree") = digest.to_string();
    }
}

impl LoopScopePlatformPort for FakeScopePlatform {
    fn witness(&self) -> LoopPlatformWitness {
        LoopPlatformWitness {
            platform: "test-unix".to_string(),
            safe_mutation: true,
            detail: "fake handle boundary".to_string(),
        }
    }

    fn bind_root(
        &self,
        _run_id: &str,
        worktree_path: &str,
        _cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopRootBinding, LoopScopeFailure> {
        if let Some(failure) = self.fail_root.lock().expect("fail root").clone() {
            return Err(failure);
        }
        Ok(LoopRootBinding {
            root: LoopRootIdentity {
                canonical_path: worktree_path.to_string(),
                device: 7,
                inode: 42,
                case_rule: "sensitive".to_string(),
            },
            baseline_manifest_id: "baseline".to_string(),
            baseline_digest: self.tree_digest.lock().expect("tree").clone(),
            baseline_entries: 3,
        })
    }

    fn verify_root(
        &self,
        _worktree_path: &str,
        root: &LoopRootIdentity,
    ) -> Result<(), LoopScopeFailure> {
        if let Some(failure) = self.fail_root.lock().expect("fail root").clone() {
            return Err(failure);
        }
        if root.device == 7 && root.inode == 42 {
            Ok(())
        } else {
            Err(LoopScopeFailure::new(
                "scope-root-changed",
                "fake root changed",
            ))
        }
    }

    fn capture_manifest(
        &self,
        _run_id: &str,
        _worktree_path: &str,
        manifest_id: &str,
        _cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopManifestRef, LoopScopeFailure> {
        if let Some(failure) = self.fail_capture.lock().expect("fail capture").clone() {
            return Err(failure);
        }
        self.captured
            .lock()
            .expect("captured")
            .push(manifest_id.to_string());
        Ok(LoopManifestRef {
            manifest_id: manifest_id.to_string(),
            digest: self.tree_digest.lock().expect("tree").clone(),
            entries: 3,
        })
    }

    fn diff_manifests(
        &self,
        _run_id: &str,
        _baseline_manifest_id: &str,
        _current_manifest_id: &str,
    ) -> Result<Vec<LoopManifestChangeView>, LoopScopeFailure> {
        Ok(self.changes.lock().expect("changes").clone())
    }

    fn native_check(
        &self,
        _run_id: &str,
        _worktree_path: &str,
        _root: &LoopRootIdentity,
        _baseline_manifest_id: &str,
        _current_manifest_id: &str,
    ) -> Result<LoopNativeCheckView, LoopScopeFailure> {
        Ok(self
            .native_check
            .lock()
            .expect("native check")
            .clone()
            .unwrap_or(LoopNativeCheckView {
                status: "passed".to_string(),
                findings: Vec::new(),
                inspected_files: 1,
                binary_excluded: 0,
                detail: None,
            }))
    }

    fn seal_contents(
        &self,
        _run_id: &str,
        _worktree_path: &str,
        manifest_id: &str,
        _cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopSealReceipt, LoopScopeFailure> {
        if let Some(failure) = self.fail_seal.lock().expect("fail seal").clone() {
            return Err(failure);
        }
        self.sealed
            .lock()
            .expect("sealed")
            .push(manifest_id.to_string());
        Ok(LoopSealReceipt {
            manifest_id: manifest_id.to_string(),
            digest: self.tree_digest.lock().expect("tree").clone(),
            copied_objects: 2,
            copied_bytes: 64,
        })
    }

    fn guard(
        &self,
        _binding: &LoopScopeBinding,
        role: LoopGuardRole,
    ) -> Result<Arc<dyn LoopScopeGuard>, LoopScopeFailure> {
        Ok(Arc::new(FakeGuard {
            role,
            writes: Mutex::new(Vec::new()),
        }))
    }
}

pub(crate) struct FakeGuard {
    role: LoopGuardRole,
    pub(crate) writes: Mutex<Vec<String>>,
}

impl LoopScopeGuard for FakeGuard {
    fn mediated(&self) -> bool {
        true
    }
    fn admit_channel(&self, channel: LoopSideEffectChannel) -> Result<(), String> {
        match (self.role, channel) {
            (LoopGuardRole::Verifier, _) => Err("verifier is read-only".to_string()),
            (LoopGuardRole::Worker, LoopSideEffectChannel::MediatedFile) => Ok(()),
            _ => Err(format!("{} not admitted", channel.as_str())),
        }
    }
    fn admit_write(&self, requested: &str) -> Result<(), String> {
        self.admit_channel(LoopSideEffectChannel::MediatedFile)?;
        if requested.starts_with("src/") {
            Ok(())
        } else {
            Err(format!("loop-scope: scope-outside-allowed: {requested}"))
        }
    }
    fn read(&self, _requested: &str) -> Result<Vec<u8>, String> {
        Ok(b"content".to_vec())
    }
    fn write(&self, requested: &str, _content: &[u8]) -> Result<(), String> {
        self.admit_write(requested)?;
        self.writes
            .lock()
            .expect("writes")
            .push(requested.to_string());
        Ok(())
    }
}

/// CLI witnesses for tests: `acp-*` ids are ACP-capable, `plain-*` ids are uncontained CLIs,
/// anything else is unknown.
#[derive(Default)]
pub(crate) struct FakeCli;

impl LoopCliCapabilityPort for FakeCli {
    fn cli_witness(&self, agent_id: &str) -> Option<LoopCliWitness> {
        if agent_id.starts_with("acp-") || agent_id.starts_with("plain-") {
            Some(LoopCliWitness {
                provider_id: agent_id.to_string(),
                adapter_revision: "test-v1".to_string(),
                executable: agent_id.to_string(),
                acp: agent_id.starts_with("acp-"),
                mapped_permission_hook: false,
            })
        } else {
            None
        }
    }
}

pub(crate) fn assessment_service(
    registry: Arc<dyn AgentRegistryRepository>,
    api_agents: Arc<dyn ApiAgentGateway>,
    platform: Arc<dyn LoopScopePlatformPort>,
    clock: Arc<dyn AgentClockPort>,
) -> LoopAssessmentService {
    LoopAssessmentService::new(registry, api_agents, Arc::new(FakeCli), platform, clock)
}

/// A binding whose witness matches `assessment`, as the orchestrator would have stored it.
pub(crate) fn test_binding(
    definition: &LoopDefinition,
    assessment: &LoopExecutionAssessment,
    tree_digest: &str,
) -> Result<LoopScopeBinding, AgentRuntimeApplicationError> {
    let scope = definition
        .scope()
        .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
    let mode = definition
        .requested_mode()
        .unwrap_or(LoopRequestedMode::PreventiveRequired);
    Ok(LoopScopeBinding {
        schema_version: LOOP_SCOPE_SCHEMA_VERSION,
        allowed_paths: scope.allowed_display(),
        protected_paths: scope.protected_display(),
        requested_mode: mode.as_str().to_string(),
        definition_version: definition.values().version,
        scope_digest: scope.digest(mode),
        root: LoopRootIdentity {
            canonical_path: "D:/project-loop".to_string(),
            device: 7,
            inode: 42,
            case_rule: "sensitive".to_string(),
        },
        base_commit: Some("abc123".to_string()),
        baseline_manifest_id: "baseline".to_string(),
        baseline_digest: tree_digest.to_string(),
        witness_digest: assessment.witness_digest.clone(),
        audit_receipt_id: None,
        bound_at: "2026-07-22T00:00:00Z".to_string(),
    })
}

pub(crate) fn scope_evidence(phase: &str, status: &str, digest: &str) -> super::LoopEvidenceView {
    super::LoopEvidenceView {
        id: format!("scope-{phase}-{status}"),
        run_id: "run-1".to_string(),
        iteration_id: Some("iteration-1".to_string()),
        kind: super::SCOPE_EVIDENCE_KIND.to_string(),
        status: status.to_string(),
        summary: format!("{phase} {status}"),
        operation_id: None,
        command_id: None,
        exit_code: None,
        duration_ms: None,
        details: Some(serde_json::json!({
            "phase": phase,
            "manifestId": format!("{phase}-1"),
            "manifestDigest": digest,
            "runtimeRevision": LOOP_SCOPE_RUNTIME_REVISION,
        })),
        created_at: "2026-07-22T00:00:00Z".to_string(),
    }
}
