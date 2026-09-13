//! The native implementation of `LoopScopePlatformPort` and the per-session scope guard.
//!
//! Host-owned evidence lives outside every worktree under the application data directory, so a
//! mediated Worker (which can only reach its bound root) cannot touch baselines or sealed
//! objects. Audit mode makes no such claim: its limitations are disclosed at start.

use super::loop_artifact_scan::{
    diff_manifests, scan_stable, ScanBudget, ScanFailure, WorkspaceManifest,
};
use super::loop_evidence_store::{
    EvidenceStoreError, LoopEvidenceStore, DEFAULT_OBJECT_BUDGET_BYTES,
};
use super::loop_native_check::patch_whitespace;
use super::loop_scope_fs::{
    platform_capability, relative_scope_path, ScopedFsError, ScopedWorkspaceRoot,
};
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopBackgroundPort, LoopCliCapabilityPort, LoopCliWitness,
    LoopGuardRole, LoopManifestChangeView, LoopManifestRef, LoopNativeCheckView,
    LoopPlatformWitness, LoopRootBinding, LoopRootIdentity, LoopScopeBinding, LoopScopeFailure,
    LoopScopeGuard, LoopScopePlatformPort, LoopSealReceipt, LoopVerificationCancellation,
};
use crate::contexts::agent_runtime::domain::{
    CaseRule, LoopRequestedMode, LoopScopeConfig, LoopSideEffectChannel,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct NativeLoopScopePlatform {
    store: LoopEvidenceStore,
    budget: ScanBudget,
}

impl NativeLoopScopePlatform {
    pub(crate) fn new(evidence_root: PathBuf) -> Self {
        Self {
            store: LoopEvidenceStore::new(evidence_root),
            budget: ScanBudget::default(),
        }
    }

    fn scan(
        &self,
        worktree_path: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<WorkspaceManifest, LoopScopeFailure> {
        let cancel = cancellation.signal();
        scan_stable(Path::new(worktree_path), &self.budget, &cancel).map_err(scan_failure)
    }
}

impl LoopScopePlatformPort for NativeLoopScopePlatform {
    fn witness(&self) -> LoopPlatformWitness {
        let capability = platform_capability();
        LoopPlatformWitness {
            platform: capability.platform.to_string(),
            safe_mutation: capability.safe_mutation,
            detail: capability.detail.to_string(),
        }
    }

    fn bind_root(
        &self,
        run_id: &str,
        worktree_path: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopRootBinding, LoopScopeFailure> {
        let root = ScopedWorkspaceRoot::open(Path::new(worktree_path)).map_err(fs_failure)?;
        let manifest = self.scan(worktree_path, cancellation)?;
        let manifest_id = "baseline".to_string();
        self.store
            .store_manifest(run_id, &manifest_id, &manifest)
            .map_err(store_failure)?;
        self.store
            .capture_objects(run_id, root.root(), &manifest, DEFAULT_OBJECT_BUDGET_BYTES)
            .map_err(store_failure)?;
        // The baseline must describe exactly the objects that were copied: a tree that moved
        // during capture is reported rather than bound.
        let recheck = self.scan(worktree_path, cancellation)?;
        if recheck.digest != manifest.digest {
            return Err(LoopScopeFailure::new(
                "scan-unstable",
                "the worktree changed while the baseline was being sealed",
            ));
        }
        let identity = root.identity();
        Ok(LoopRootBinding {
            root: LoopRootIdentity {
                canonical_path: root.root().to_string_lossy().to_string(),
                device: identity.device,
                inode: identity.inode,
                case_rule: root.case_rule().as_str().to_string(),
            },
            baseline_manifest_id: manifest_id,
            baseline_digest: manifest.digest,
            baseline_entries: manifest.entries.len(),
        })
    }

    fn verify_root(
        &self,
        worktree_path: &str,
        root: &LoopRootIdentity,
    ) -> Result<(), LoopScopeFailure> {
        let opened = ScopedWorkspaceRoot::open(Path::new(worktree_path)).map_err(fs_failure)?;
        let identity = opened.identity();
        if identity.device != root.device
            || identity.inode != root.inode
            || opened.root().to_string_lossy() != root.canonical_path
        {
            return Err(LoopScopeFailure::new(
                "scope-root-changed",
                "the worktree root no longer has its bound identity",
            ));
        }
        Ok(())
    }

    fn capture_manifest(
        &self,
        run_id: &str,
        worktree_path: &str,
        manifest_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopManifestRef, LoopScopeFailure> {
        let manifest = self.scan(worktree_path, cancellation)?;
        self.store
            .store_manifest(run_id, manifest_id, &manifest)
            .map_err(store_failure)?;
        Ok(LoopManifestRef {
            manifest_id: manifest_id.to_string(),
            digest: manifest.digest,
            entries: manifest.entries.len(),
        })
    }

    fn diff_manifests(
        &self,
        run_id: &str,
        baseline_manifest_id: &str,
        current_manifest_id: &str,
    ) -> Result<Vec<LoopManifestChangeView>, LoopScopeFailure> {
        let baseline = self
            .store
            .load_manifest(run_id, baseline_manifest_id)
            .map_err(store_failure)?;
        let current = self
            .store
            .load_manifest(run_id, current_manifest_id)
            .map_err(store_failure)?;
        Ok(diff_manifests(&baseline, &current)
            .into_iter()
            .map(|change| LoopManifestChangeView {
                path: change.path,
                kind: change.kind.as_str().to_string(),
            })
            .collect())
    }

    fn native_check(
        &self,
        run_id: &str,
        worktree_path: &str,
        root: &LoopRootIdentity,
        baseline_manifest_id: &str,
        current_manifest_id: &str,
    ) -> Result<LoopNativeCheckView, LoopScopeFailure> {
        self.verify_root(worktree_path, root)?;
        let opened = ScopedWorkspaceRoot::open(Path::new(worktree_path)).map_err(fs_failure)?;
        let baseline = self
            .store
            .load_manifest(run_id, baseline_manifest_id)
            .map_err(store_failure)?;
        let current = self
            .store
            .load_manifest(run_id, current_manifest_id)
            .map_err(store_failure)?;
        let outcome = patch_whitespace(run_id, &self.store, &opened, &baseline, &current);
        Ok(LoopNativeCheckView {
            status: outcome.status.as_str().to_string(),
            findings: outcome.findings,
            inspected_files: outcome.inspected_files,
            binary_excluded: outcome.binary_excluded,
            detail: outcome.detail,
        })
    }

    fn seal_contents(
        &self,
        run_id: &str,
        worktree_path: &str,
        manifest_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopSealReceipt, LoopScopeFailure> {
        let manifest = self.scan(worktree_path, cancellation)?;
        let receipt = self
            .store
            .capture_objects(
                run_id,
                Path::new(worktree_path),
                &manifest,
                DEFAULT_OBJECT_BUDGET_BYTES,
            )
            .map_err(store_failure)?;
        // Copying takes time; the tree must still describe the same bytes afterwards, and every
        // object the manifest names must be readable and intact before it is called sealed.
        let recheck = self.scan(worktree_path, cancellation)?;
        if recheck.digest != manifest.digest {
            return Err(LoopScopeFailure::new(
                "scan-unstable",
                "the worktree changed while its contents were being sealed",
            ));
        }
        self.store
            .verify_objects(run_id, &manifest)
            .map_err(store_failure)?;
        self.store
            .store_manifest(run_id, manifest_id, &manifest)
            .map_err(store_failure)?;
        Ok(LoopSealReceipt {
            manifest_id: manifest_id.to_string(),
            digest: manifest.digest,
            copied_objects: receipt.copied_objects,
            copied_bytes: receipt.copied_bytes,
        })
    }

    fn guard(
        &self,
        binding: &LoopScopeBinding,
        role: LoopGuardRole,
    ) -> Result<Arc<dyn LoopScopeGuard>, LoopScopeFailure> {
        let scope = binding
            .scope()
            .map_err(|error| LoopScopeFailure::new("scope-binding-invalid", error.to_string()))?;
        let mode = binding.mode().ok_or_else(|| {
            LoopScopeFailure::new("scope-binding-invalid", "unknown requested mode")
        })?;
        let root = ScopedWorkspaceRoot::open(Path::new(&binding.root.canonical_path));
        let mediated = platform_capability().safe_mutation;
        let root = match root {
            Ok(root) => {
                let identity = root.identity();
                if identity.device != binding.root.device || identity.inode != binding.root.inode {
                    return Err(LoopScopeFailure::new(
                        "scope-root-changed",
                        "the worktree root no longer has its bound identity",
                    ));
                }
                Some(root)
            }
            Err(ScopedFsError::Unsupported(_)) if !mediated => None,
            Err(error) => return Err(fs_failure(error)),
        };
        // Without the handle boundary the guard can only refuse; audit mode then relies on the
        // artifact validation it disclosed, and strict mode was never admitted here.
        Ok(Arc::new(NativeLoopScopeGuard {
            role,
            mode,
            scope,
            root_path: PathBuf::from(&binding.root.canonical_path),
            root,
            case: if binding.root.case_rule == CaseRule::InsensitiveAscii.as_str() {
                CaseRule::InsensitiveAscii
            } else {
                CaseRule::Sensitive
            },
        }))
    }
}

pub(crate) struct NativeLoopScopeGuard {
    role: LoopGuardRole,
    mode: LoopRequestedMode,
    scope: LoopScopeConfig,
    root_path: PathBuf,
    root: Option<ScopedWorkspaceRoot>,
    case: CaseRule,
}

impl LoopScopeGuard for NativeLoopScopeGuard {
    fn mediated(&self) -> bool {
        self.root.is_some()
    }

    fn admit_channel(&self, channel: LoopSideEffectChannel) -> Result<(), String> {
        // Verifiers are read-only on every channel in both modes. Workers keep only the
        // mediated file channel; the audit-only channels (terminal, opaque CLI internals) are
        // admitted solely for ACP-driven CLI Workers, whose assessment disclosed them.
        let admitted = match (self.role, channel) {
            (LoopGuardRole::Verifier, _) => false,
            (LoopGuardRole::Worker, LoopSideEffectChannel::MediatedFile) => true,
            (
                LoopGuardRole::Worker,
                LoopSideEffectChannel::Terminal | LoopSideEffectChannel::CliInternal,
            ) => self.mode == LoopRequestedMode::ArtifactAudited,
            (LoopGuardRole::Worker, _) => false,
        };
        if admitted {
            Ok(())
        } else {
            Err(format!(
                "loop-scope: the {} channel is not admitted for the Loop {} role under {}",
                channel.as_str(),
                self.role.as_str(),
                self.mode.as_str()
            ))
        }
    }

    fn admit_write(&self, requested: &str) -> Result<(), String> {
        self.admit_channel(LoopSideEffectChannel::MediatedFile)?;
        let path =
            relative_scope_path(&self.root_path, requested).map_err(|error| error.to_string())?;
        self.scope
            .admit_mutation(&path, self.case)
            .map_err(|rejection| format!("loop-scope: {}", rejection.message()))
    }

    fn read(&self, requested: &str) -> Result<Vec<u8>, String> {
        let root = self.root.as_ref().ok_or_else(|| {
            "loop-scope: mediated reads are unavailable on this platform".to_string()
        })?;
        let path =
            relative_scope_path(&self.root_path, requested).map_err(|error| error.to_string())?;
        root.read_file(&path).map_err(|error| error.to_string())
    }

    fn write(&self, requested: &str, content: &[u8]) -> Result<(), String> {
        self.admit_channel(LoopSideEffectChannel::MediatedFile)?;
        let root = self.root.as_ref().ok_or_else(|| {
            "loop-scope: mediated writes are unavailable on this platform".to_string()
        })?;
        let path =
            relative_scope_path(&self.root_path, requested).map_err(|error| error.to_string())?;
        self.scope
            .admit_mutation(&path, self.case)
            .map_err(|rejection| format!("loop-scope: {}", rejection.message()))?;
        root.write_file(&self.scope, &path, content)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

/// CLI witnesses come from the reviewed provider catalog, keyed by the stable Agent id.
#[derive(Clone, Default)]
pub(crate) struct CatalogLoopCliCapability;

impl LoopCliCapabilityPort for CatalogLoopCliCapability {
    fn cli_witness(&self, agent_id: &str) -> Option<LoopCliWitness> {
        super::providers::definitions::DEFINITIONS
            .iter()
            .find(|definition| definition.id == agent_id)
            .map(|definition| LoopCliWitness {
                provider_id: definition.id.to_string(),
                adapter_revision: definition.adapter_revision.to_string(),
                executable: definition.executable.to_string(),
                acp: definition.acp.is_some(),
                mapped_permission_hook: definition.permissions && definition.id == "claude-code",
            })
    }
}

#[derive(Clone, Default)]
pub(crate) struct ThreadLoopBackground;

impl LoopBackgroundPort for ThreadLoopBackground {
    fn spawn(
        &self,
        name: &str,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        std::thread::Builder::new()
            .name(name.to_string())
            .spawn(task)
            .map(|_| ())
            .map_err(|error| {
                AgentRuntimeApplicationError::Loop(format!(
                    "Could not start the Loop acceptance thread: {error}"
                ))
            })
    }
}

fn scan_failure(failure: ScanFailure) -> LoopScopeFailure {
    LoopScopeFailure::new(failure.code(), failure.message())
}

fn store_failure(error: EvidenceStoreError) -> LoopScopeFailure {
    LoopScopeFailure::new(error.code(), error.message())
}

fn fs_failure(error: ScopedFsError) -> LoopScopeFailure {
    LoopScopeFailure::new(error.code(), error.to_string())
}

/// Builds a real scope guard over a temporary workspace for tests in other modules. The binding
/// is produced by the platform itself so the guard exercises the same handle-relative boundary
/// the runtime installs.
#[cfg(test)]
pub(crate) fn test_guard(
    workspace: &Path,
    storage: &Path,
    allowed: &[&str],
    protected: &[&str],
    mode: LoopRequestedMode,
    role: LoopGuardRole,
) -> Arc<dyn LoopScopeGuard> {
    let platform = NativeLoopScopePlatform::new(storage.to_path_buf());
    let bound = platform
        .bind_root(
            "test-run",
            &workspace.to_string_lossy(),
            &LoopVerificationCancellation::default(),
        )
        .expect("bind test root");
    let scope = LoopScopeConfig::parse(
        &allowed
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        &protected
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
    )
    .expect("test scope");
    let binding = LoopScopeBinding {
        schema_version: crate::contexts::agent_runtime::domain::LOOP_SCOPE_SCHEMA_VERSION,
        allowed_paths: scope.allowed_display(),
        protected_paths: scope.protected_display(),
        requested_mode: mode.as_str().to_string(),
        definition_version: 1,
        scope_digest: scope.digest(mode),
        root: bound.root,
        base_commit: None,
        baseline_manifest_id: bound.baseline_manifest_id,
        baseline_digest: bound.baseline_digest,
        witness_digest: "sha256:test-witness".to_string(),
        audit_receipt_id: None,
        bound_at: "2026-01-01T00:00:00Z".to_string(),
    };
    platform.guard(&binding, role).expect("test guard")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::LOOP_SCOPE_SCHEMA_VERSION;
    use crate::test_support::TempDirectory;

    fn make_binding(
        platform: &NativeLoopScopePlatform,
        run_id: &str,
        workspace: &Path,
        allowed: &[&str],
        protected: &[&str],
        mode: LoopRequestedMode,
    ) -> LoopScopeBinding {
        let bound = platform
            .bind_root(
                run_id,
                &workspace.to_string_lossy(),
                &LoopVerificationCancellation::default(),
            )
            .expect("bind");
        let scope = LoopScopeConfig::parse(
            &allowed
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            &protected
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("scope");
        LoopScopeBinding {
            schema_version: LOOP_SCOPE_SCHEMA_VERSION,
            allowed_paths: scope.allowed_display(),
            protected_paths: scope.protected_display(),
            requested_mode: mode.as_str().to_string(),
            definition_version: 1,
            scope_digest: scope.digest(mode),
            root: bound.root,
            base_commit: None,
            baseline_manifest_id: bound.baseline_manifest_id,
            baseline_digest: bound.baseline_digest,
            witness_digest: "w".to_string(),
            audit_receipt_id: None,
            bound_at: "t".to_string(),
        }
    }

    #[test]
    fn guard_admits_only_scoped_worker_writes_and_refuses_every_verifier_mutation() {
        let workspace = TempDirectory::new("scope-platform-guard");
        let storage = TempDirectory::new("scope-platform-store");
        workspace.write("src/generated/a.ts", "keep");
        workspace.write("docs/readme.md", "keep");
        let platform = NativeLoopScopePlatform::new(storage.path().to_path_buf());
        let binding = make_binding(
            &platform,
            "run-1",
            workspace.path(),
            &["src"],
            &["src/generated"],
            LoopRequestedMode::PreventiveRequired,
        );
        let worker = platform
            .guard(&binding, LoopGuardRole::Worker)
            .expect("worker guard");
        assert!(worker.mediated());
        worker.write("src/app.ts", b"ok").expect("allowed write");
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("src/app.ts")).expect("written"),
            "ok"
        );
        assert!(worker
            .write("docs/readme.md", b"x")
            .expect_err("outside")
            .contains("scope-outside-allowed"));
        assert!(worker
            .write("src/generated/a.ts", b"x")
            .expect_err("protected")
            .contains("scope-protected-path"));
        assert!(worker
            .write(
                &format!("{}/docs/readme.md", workspace.path().display()),
                b"x"
            )
            .is_err());
        assert!(worker.write("/etc/passwd", b"x").is_err());
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("docs/readme.md")).expect("kept"),
            "keep"
        );
        assert!(worker.admit_channel(LoopSideEffectChannel::Shell).is_err());
        assert!(worker.admit_channel(LoopSideEffectChannel::Mcp).is_err());
        assert!(worker
            .admit_channel(LoopSideEffectChannel::Terminal)
            .is_err());
        assert_eq!(worker.read("src/app.ts").expect("read"), b"ok");

        let verifier = platform
            .guard(&binding, LoopGuardRole::Verifier)
            .expect("verifier guard");
        assert!(verifier.write("src/app.ts", b"mutated").is_err());
        assert!(verifier
            .admit_channel(LoopSideEffectChannel::MediatedFile)
            .is_err());
        assert!(verifier
            .admit_channel(LoopSideEffectChannel::Terminal)
            .is_err());
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("src/app.ts")).expect("unchanged"),
            "ok"
        );

        let audited = make_binding(
            &platform,
            "run-2",
            workspace.path(),
            &["src"],
            &[],
            LoopRequestedMode::ArtifactAudited,
        );
        let audit_worker = platform
            .guard(&audited, LoopGuardRole::Worker)
            .expect("audit guard");
        assert!(audit_worker
            .admit_channel(LoopSideEffectChannel::Terminal)
            .is_ok());
        assert!(audit_worker
            .admit_channel(LoopSideEffectChannel::Shell)
            .is_err());
    }

    #[test]
    fn manifests_native_check_and_sealing_follow_the_bound_root() {
        let workspace = TempDirectory::new("scope-platform-seal");
        let storage = TempDirectory::new("scope-platform-seal-store");
        workspace.write("src/a.txt", "one\n");
        let platform = NativeLoopScopePlatform::new(storage.path().to_path_buf());
        let binding = make_binding(
            &platform,
            "run-3",
            workspace.path(),
            &["src"],
            &[],
            LoopRequestedMode::PreventiveRequired,
        );
        let path = workspace.path().to_string_lossy().to_string();
        workspace.write("src/a.txt", "one\ntwo \n");
        let phase = platform
            .capture_manifest(
                "run-3",
                &path,
                "worker-1",
                &LoopVerificationCancellation::default(),
            )
            .expect("manifest");
        let changes = platform
            .diff_manifests("run-3", &binding.baseline_manifest_id, "worker-1")
            .expect("diff");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "src/a.txt");
        let check = platform
            .native_check(
                "run-3",
                &path,
                &binding.root,
                &binding.baseline_manifest_id,
                "worker-1",
            )
            .expect("check");
        assert_eq!(check.status, "failed");
        assert_eq!(
            check.findings,
            vec!["src/a.txt:2: trailing whitespace".to_string()]
        );
        let sealed = platform
            .seal_contents(
                "run-3",
                &path,
                "sealed-1",
                &LoopVerificationCancellation::default(),
            )
            .expect("seal");
        assert_eq!(sealed.digest, phase.digest);
        assert!(storage
            .path()
            .join("run-3/manifests/sealed-1.json")
            .exists());

        let moved = workspace.path().with_extension("moved");
        std::fs::rename(workspace.path(), &moved).expect("move");
        std::fs::create_dir_all(workspace.path()).expect("replacement root");
        assert_eq!(
            platform
                .verify_root(&path, &binding.root)
                .expect_err("replaced")
                .code,
            "scope-root-changed"
        );
        std::fs::remove_dir_all(&moved).expect("cleanup");
    }
}
