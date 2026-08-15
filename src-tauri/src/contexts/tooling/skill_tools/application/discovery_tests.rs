use super::{
    project_inventory_summary, EffectiveSkillToolPackage, SkillToolApplicationError,
    SkillToolDiscoveryService, SkillToolFileEntry, SkillToolPackageRef, SkillToolPackageSource,
    SkillToolRevisionState,
};
use crate::contexts::tooling::skill_tools::domain::{
    content_hash_of, SkillToolDiagnosticSummary, SkillToolLifecycle, SkillToolOwnerId,
    SkillToolRevision, SkillToolScope, SkillToolSourceScope, SkillToolValidationState,
    DEFAULT_MANIFEST_LIMITS, MANIFEST_PATH,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

const DECLARATIVE_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-declarative.json");
const MODULE_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/valid-module.json");
const HASH_MISMATCH_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/hash-mismatch.json");
const UNKNOWN_VERSION_MANIFEST: &str =
    include_str!("../../../../../tests/fixtures/skill-tools/adversarial/unknown-version.json");

/// Records which package roots were read so "the shadowed revision is ignored" can be asserted as
/// "its bytes were never requested", not merely as "its tools did not appear".
#[derive(Default)]
struct SpyPackageSource {
    manifests: BTreeMap<String, Vec<u8>>,
    files: BTreeMap<String, Vec<SkillToolFileEntry>>,
    implementations: BTreeMap<(String, String), Vec<u8>>,
    reads: Mutex<BTreeSet<String>>,
}

impl SpyPackageSource {
    fn with_manifest(mut self, root: &str, manifest: &str) -> Self {
        self.manifests
            .insert(root.to_string(), manifest.as_bytes().to_vec());
        self
    }

    fn with_file(mut self, root: &str, path: &str, bytes: Vec<u8>) -> Self {
        self.files
            .entry(root.to_string())
            .or_default()
            .push(SkillToolFileEntry {
                relative_path: path.to_string(),
                size_bytes: bytes.len() as u64,
            });
        self.implementations
            .insert((root.to_string(), path.to_string()), bytes);
        self
    }

    fn with_listing_only(mut self, root: &str, path: &str, size_bytes: u64) -> Self {
        self.files
            .entry(root.to_string())
            .or_default()
            .push(SkillToolFileEntry {
                relative_path: path.to_string(),
                size_bytes,
            });
        self
    }

    fn touched(&self, root: &str) -> bool {
        self.reads.lock().expect("spy reads").contains(root)
    }

    fn note(&self, root: &str) {
        self.reads
            .lock()
            .expect("spy reads")
            .insert(root.to_string());
    }
}

impl SkillToolPackageSource for SpyPackageSource {
    fn read_manifest(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Option<Vec<u8>>, SkillToolApplicationError> {
        self.note(&package.root_path);
        Ok(self.manifests.get(&package.root_path).cloned())
    }

    fn list_tool_files(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolFileEntry>, SkillToolApplicationError> {
        self.note(&package.root_path);
        let mut entries = self
            .files
            .get(&package.root_path)
            .cloned()
            .unwrap_or_default();
        if self.manifests.contains_key(&package.root_path) {
            entries.push(SkillToolFileEntry {
                relative_path: MANIFEST_PATH.to_string(),
                size_bytes: 0,
            });
        }
        Ok(entries)
    }

    fn read_implementation(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        self.note(&package.root_path);
        self.implementations
            .get(&(package.root_path.clone(), relative_path.to_string()))
            .cloned()
            .ok_or_else(|| {
                SkillToolApplicationError::MissingImplementationFile(relative_path.to_string())
            })
    }
}

fn package(root: &str) -> SkillToolPackageRef {
    SkillToolPackageRef {
        owner: SkillToolOwnerId::parse("code-review").expect("owner"),
        source: SkillToolSourceScope::global(),
        base_revision: format!("base-{root}"),
        root_path: root.to_string(),
    }
}

fn effective(root: &str, shadowed: &[&str]) -> EffectiveSkillToolPackage {
    EffectiveSkillToolPackage {
        effective: package(root),
        shadowed: shadowed.iter().map(|root| package(root)).collect(),
    }
}

fn module_bytes(export: &str) -> Vec<u8> {
    let mut bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut body = vec![1_u8, export.len() as u8];
    body.extend_from_slice(export.as_bytes());
    body.extend([0, 0]);
    bytes.extend([7_u8, body.len() as u8]);
    bytes.extend(body);
    bytes
}

fn discover(
    source: &SpyPackageSource,
    package: &EffectiveSkillToolPackage,
) -> super::SkillToolDiscoveryOutcome {
    SkillToolDiscoveryService::new(source, DEFAULT_MANIFEST_LIMITS)
        .discover(package)
        .expect("discovery")
}

#[test]
fn only_the_winning_revision_is_read_and_shadowed_revisions_are_reported_untouched() {
    let source = SpyPackageSource::default()
        .with_manifest("project", DECLARATIVE_MANIFEST)
        .with_manifest("user", MODULE_MANIFEST)
        .with_manifest("system", MODULE_MANIFEST);
    let outcome = discover(&source, &effective("project", &["user", "system"]));

    assert!(source.touched("project"));
    assert!(!source.touched("user"));
    assert!(!source.touched("system"));
    assert_eq!(
        outcome.ignored_shadowed_revisions,
        vec!["base-user".to_string(), "base-system".to_string()]
    );
    assert_eq!(
        outcome
            .discovered
            .iter()
            .map(|tool| tool.key.tool.as_str())
            .collect::<Vec<_>>(),
        vec!["diff-summary", "checklist"]
    );
    assert!(outcome
        .discovered
        .iter()
        .all(|tool| !tool.requires_module_runtime));
}

#[test]
fn a_skill_without_a_manifest_discovers_nothing_and_is_not_an_error() {
    let source = SpyPackageSource::default();
    let outcome = discover(&source, &effective("project", &[]));
    assert!(!outcome.manifest_present);
    assert!(outcome.is_empty());
    assert!(outcome.undeclared_files.is_empty());
}

#[test]
fn the_revision_witness_binds_the_base_revision_manifest_and_capabilities() {
    let source = SpyPackageSource::default().with_manifest("project", DECLARATIVE_MANIFEST);
    let first = discover(&source, &effective("project", &[]));

    let changed_base = SpyPackageSource::default().with_manifest("other", DECLARATIVE_MANIFEST);
    let second = discover(&changed_base, &effective("other", &[]));
    assert_ne!(
        first.discovered[0].key.revision,
        second.discovered[0].key.revision
    );

    let retargeted = SpyPackageSource::default().with_manifest(
        "project",
        &DECLARATIVE_MANIFEST.replace("\"$const\": \"utf-8\"", "\"$const\": \"utf-16\""),
    );
    let third = discover(&retargeted, &effective("project", &[]));
    assert_ne!(
        first.discovered[0].key.revision,
        third.discovered[0].key.revision
    );
    assert_eq!(
        first.discovered[0].integrity.base_revision,
        "base-project".to_string()
    );
    assert_eq!(
        first.discovered[0].canonical_name,
        format!(
            "skill__code-review__diff-summary__{}",
            first.discovered[0].key.revision.short_fragment()
        )
    );
}

#[test]
fn a_module_whose_bytes_do_not_match_its_bound_hash_is_rejected_without_the_others() {
    let bytes = module_bytes("run");
    let manifest = HASH_MISMATCH_MANIFEST.replace(
        "sha256:5555555555555555555555555555555555555555555555555555555555555555",
        content_hash_of(&bytes).as_str(),
    );
    let matching = SpyPackageSource::default()
        .with_manifest("project", &manifest)
        .with_file("project", "scripts/modules/swapped.wasm", bytes);
    let accepted = discover(&matching, &effective("project", &[]));
    assert_eq!(accepted.discovered.len(), 1);
    assert!(accepted.discovered[0].requires_module_runtime);

    let mismatched = SpyPackageSource::default()
        .with_manifest("project", HASH_MISMATCH_MANIFEST)
        .with_file(
            "project",
            "scripts/modules/swapped.wasm",
            module_bytes("run"),
        );
    let refused = discover(&mismatched, &effective("project", &[]));
    assert!(refused.discovered.is_empty());
    assert_eq!(refused.rejected[0].tool_id.as_deref(), Some("swapped"));
    assert_eq!(refused.rejected[0].diagnostic.code, "integrity-mismatch");
}

#[test]
fn a_module_importing_a_forbidden_host_capability_is_refused_at_discovery() {
    let mut bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut imports = vec![1_u8, 21];
    imports.extend_from_slice(b"wasi_snapshot_preview1");
    imports.extend([8]);
    imports.extend_from_slice(b"fd_write");
    imports.extend([0, 0]);
    imports[1] = 22;
    bytes.extend([2_u8, imports.len() as u8]);
    bytes.extend(imports);
    bytes.extend(module_bytes("run")[8..].to_vec());

    let manifest = HASH_MISMATCH_MANIFEST.replace(
        "sha256:5555555555555555555555555555555555555555555555555555555555555555",
        content_hash_of(&bytes).as_str(),
    );
    let source = SpyPackageSource::default()
        .with_manifest("project", &manifest)
        .with_file("project", "scripts/modules/swapped.wasm", bytes);
    let outcome = discover(&source, &effective("project", &[]));
    assert!(outcome.discovered.is_empty());
    assert_eq!(
        outcome.rejected[0].diagnostic.code,
        "module-forbidden-import"
    );
}

#[test]
fn an_undeclared_file_is_reported_and_never_read() {
    let bytes = module_bytes("score");
    let manifest = MODULE_MANIFEST.replace(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        content_hash_of(&bytes).as_str(),
    );
    let source = SpyPackageSource::default()
        .with_manifest("project", &manifest)
        .with_file("project", "scripts/modules/complexity.wasm", bytes)
        .with_listing_only("project", "scripts/modules/stowaway.wasm", 32)
        .with_listing_only("project", "scripts/notes.md", 16);

    let outcome = discover(&source, &effective("project", &[]));
    assert_eq!(outcome.discovered.len(), 1);
    // Only content under the reserved module directory is reported; unrelated Skill files are not.
    assert_eq!(
        outcome.undeclared_files,
        vec!["scripts/modules/stowaway.wasm".to_string()]
    );
    // Reading it would have failed, because the spy holds no bytes for it.
    assert!(!source.implementations.contains_key(&(
        "project".to_string(),
        "scripts/modules/stowaway.wasm".to_string()
    )));
}

#[test]
fn a_manifest_level_failure_rejects_the_whole_package_without_a_tool_id() {
    let source = SpyPackageSource::default().with_manifest("project", UNKNOWN_VERSION_MANIFEST);
    let outcome = discover(&source, &effective("project", &[]));
    assert!(outcome.manifest_present);
    assert!(outcome.discovered.is_empty());
    assert_eq!(outcome.rejected.len(), 1);
    assert_eq!(outcome.rejected[0].tool_id, None);
    assert_eq!(
        outcome.rejected[0].diagnostic.code,
        "unsupported-manifest-version"
    );
}

#[test]
fn a_manifest_declaring_a_different_skill_than_its_package_is_refused() {
    let source = SpyPackageSource::default().with_manifest(
        "project",
        &DECLARATIVE_MANIFEST.replace(
            "\"skillId\": \"code-review\"",
            "\"skillId\": \"other-skill\"",
        ),
    );
    let outcome = discover(&source, &effective("project", &[]));
    assert!(outcome.discovered.is_empty());
    assert_eq!(
        outcome.rejected[0].diagnostic.code,
        "manifest-owner-mismatch"
    );
}

#[test]
fn the_aggregate_tool_directory_budget_is_enforced_over_everything_present() {
    let source = SpyPackageSource::default()
        .with_manifest("project", DECLARATIVE_MANIFEST)
        .with_listing_only(
            "project",
            "scripts/modules/huge.wasm",
            DEFAULT_MANIFEST_LIMITS.maximum_aggregate_module_bytes + 1,
        );
    let error = SkillToolDiscoveryService::new(&source, DEFAULT_MANIFEST_LIMITS)
        .discover(&effective("project", &[]))
        .expect_err("aggregate budget");
    assert!(matches!(
        error,
        SkillToolApplicationError::OversizedPackage { .. }
    ));
}

#[test]
fn an_oversized_module_is_rejected_by_the_per_file_budget() {
    let bytes = vec![0_u8; DEFAULT_MANIFEST_LIMITS.maximum_module_bytes as usize + 1];
    let manifest = HASH_MISMATCH_MANIFEST.replace(
        "sha256:5555555555555555555555555555555555555555555555555555555555555555",
        content_hash_of(&bytes).as_str(),
    );
    let source = SpyPackageSource::default()
        .with_manifest("project", &manifest)
        .with_file("project", "scripts/modules/swapped.wasm", bytes);
    let outcome = discover(&source, &effective("project", &[]));
    assert_eq!(outcome.rejected[0].diagnostic.code, "oversized-file");
}

#[test]
fn the_bounded_inventory_projection_reports_state_without_executable_bytes() {
    let source = SpyPackageSource::default().with_manifest("project", DECLARATIVE_MANIFEST);
    let outcome = discover(&source, &effective("project", &[]));

    let mut state = BTreeMap::new();
    let trusted = &outcome.discovered[0];
    state.insert(
        trusted.key.revision.clone(),
        SkillToolRevisionState {
            key: trusted.key.clone(),
            integrity: trusted.integrity.clone(),
            implementation_kind: "declarative".to_string(),
            lifecycle: SkillToolLifecycle {
                validation: SkillToolValidationState::Valid,
                trusted: true,
                enabled: true,
                ..SkillToolLifecycle::default()
            },
            validation_code: None,
            diagnostics: SkillToolDiagnosticSummary::default(),
            created_at: "2026-08-16T00:00:00Z".to_string(),
            updated_at: "2026-08-16T00:00:00Z".to_string(),
        },
    );

    let summary = project_inventory_summary(&outcome, &state, true);
    assert_eq!(summary.declared_count, 2);
    assert_eq!(summary.available_count, 1);
    assert!(!summary.truncated);
    assert_eq!(summary.entries[0].availability, "available");
    assert_eq!(summary.entries[0].capabilities, vec!["tool:read_file"]);
    // The second tool has no persisted row, so it projects to the fail-closed default.
    assert_eq!(summary.entries[1].availability, "invalid");
    assert!(!summary.entries[1].trusted);
    assert!(summary
        .entries
        .iter()
        .all(|entry| entry.implementation_hash.starts_with("sha256:")));
}

#[test]
fn declarative_tools_stay_usable_with_the_module_runtime_disabled() {
    use crate::contexts::tooling::skill_tools::MODULE_RUNTIME_ENABLED;

    let bytes = module_bytes("score");
    let module_manifest = MODULE_MANIFEST.replace(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        content_hash_of(&bytes).as_str(),
    );
    let source = SpyPackageSource::default()
        .with_manifest("declarative", DECLARATIVE_MANIFEST)
        .with_manifest("module", &module_manifest)
        .with_file("module", "scripts/modules/complexity.wasm", bytes);

    let runnable = SkillToolLifecycle {
        validation: SkillToolValidationState::Valid,
        trusted: true,
        enabled: true,
        ..SkillToolLifecycle::default()
    };
    let summarize = |root: &str, module_runtime_available: bool| {
        let outcome = discover(&source, &effective(root, &[]));
        let state = outcome
            .discovered
            .iter()
            .map(|tool| {
                (
                    tool.key.revision.clone(),
                    SkillToolRevisionState {
                        key: tool.key.clone(),
                        integrity: tool.integrity.clone(),
                        implementation_kind: tool.declaration.implementation.kind().to_string(),
                        lifecycle: runnable.clone(),
                        validation_code: None,
                        diagnostics: SkillToolDiagnosticSummary::default(),
                        created_at: "2026-08-16T00:00:00Z".to_string(),
                        updated_at: "2026-08-16T00:00:00Z".to_string(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        project_inventory_summary(&outcome, &state, module_runtime_available)
    };

    // Declarative tools are unaffected by the module runtime in either build configuration.
    let declarative = summarize("declarative", MODULE_RUNTIME_ENABLED);
    assert_eq!(declarative.available_count, 2);
    assert_eq!(summarize("declarative", false).available_count, 2);

    // A module tool is visibly unavailable rather than silently absent when the runtime is off,
    // and the same revision becomes available when it is on. Nothing else about it changes.
    let module = summarize("module", MODULE_RUNTIME_ENABLED);
    assert_eq!(module.declared_count, 1);
    assert_eq!(module.entries[0].implementation_kind, "wasm");
    assert_eq!(
        module.available_count,
        usize::from(MODULE_RUNTIME_ENABLED),
        "module availability must follow the compiled-in runtime"
    );

    let disabled = summarize("module", false);
    assert_eq!(disabled.available_count, 0);
    assert_eq!(
        disabled.entries[0].availability,
        "module-runtime-unavailable"
    );
    let enabled = summarize("module", true);
    assert_eq!(enabled.available_count, 1);
    assert_eq!(enabled.entries[0].availability, "available");
    assert_eq!(enabled.entries[0].revision, disabled.entries[0].revision);
}

#[test]
fn workspace_scoped_packages_key_separately_from_global_ones() {
    let source = SpyPackageSource::default().with_manifest("project", DECLARATIVE_MANIFEST);
    let global = discover(&source, &effective("project", &[]));

    let mut scoped = effective("project", &[]);
    scoped.effective.source =
        SkillToolSourceScope::new(SkillToolScope::Workspace, Some("D:/work")).expect("workspace");
    let workspace = discover(&source, &scoped);

    assert_ne!(
        global.discovered[0].key.revision,
        workspace.discovered[0].key.revision
    );
    assert_eq!(
        global.discovered[0].key.lineage_key(),
        "code-review:global::diff-summary"
    );
    assert_eq!(
        workspace.discovered[0].key.lineage_key(),
        "code-review:workspace:D:/work:diff-summary"
    );
    assert!(SkillToolRevision::parse(global.discovered[0].key.revision.as_str()).is_ok());
}
