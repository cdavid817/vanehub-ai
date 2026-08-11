use super::filesystem::{compose_document, default_home_root};
use crate::contexts::tooling::skills::application::{
    validate_relative_resource_path, EffectiveSkillCatalogPort, ManagedSkillSource,
    OverlayAppliedSkillSnapshotPort, OverlayKey, OverlayRuntimeCacheInvalidationPort,
    SkillApplicationError, SkillDocument, SkillPackageDescriptor, SkillPackageMaterializer,
};
use crate::contexts::tooling::skills::domain::OverlayScope;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct DerivedFile {
    relative_path: String,
    content: Vec<u8>,
}

pub(crate) struct EffectiveSkillDerivedCache {
    root: PathBuf,
    snapshots: Arc<dyn OverlayAppliedSkillSnapshotPort>,
    invalidations: Mutex<DerivedCacheInvalidations>,
}

#[derive(Default)]
struct DerivedCacheInvalidations {
    global: BTreeMap<String, u64>,
    project: BTreeMap<(String, String), u64>,
}

impl EffectiveSkillDerivedCache {
    pub(crate) fn new(snapshots: Arc<dyn OverlayAppliedSkillSnapshotPort>) -> Self {
        Self::with_root(
            default_home_root()
                .join(".vanehub")
                .join("cache")
                .join("skills")
                .join("effective"),
            snapshots,
        )
    }

    pub(crate) fn with_root(
        root: PathBuf,
        snapshots: Arc<dyn OverlayAppliedSkillSnapshotPort>,
    ) -> Self {
        Self {
            root,
            snapshots,
            invalidations: Mutex::new(DerivedCacheInvalidations::default()),
        }
    }

    pub(crate) fn invalidate(&self, key: &OverlayKey) {
        let mut invalidations = self
            .invalidations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match (key.scope, key.workspace_identity.as_deref()) {
            (OverlayScope::Project, Some(workspace)) => {
                let generation = invalidations
                    .project
                    .entry((
                        key.canonical_skill_id.as_str().to_string(),
                        workspace.to_string(),
                    ))
                    .or_default();
                *generation = generation.saturating_add(1);
            }
            _ => {
                let generation = invalidations
                    .global
                    .entry(key.canonical_skill_id.as_str().to_string())
                    .or_default();
                *generation = generation.saturating_add(1);
            }
        }
    }

    fn generation(&self, package: &SkillPackageDescriptor) -> (u64, u64) {
        let invalidations = self
            .invalidations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let skill_id = package.metadata.id.as_str();
        let global = invalidations
            .global
            .get(skill_id)
            .copied()
            .unwrap_or_default();
        let project = package
            .workspace_path
            .as_deref()
            .and_then(|workspace| {
                invalidations
                    .project
                    .get(&(skill_id.to_string(), workspace.to_string()))
            })
            .copied()
            .unwrap_or_default();
        (global, project)
    }

    #[cfg(test)]
    pub(crate) fn invalidation_generation(
        &self,
        canonical_skill_id: &str,
        workspace: Option<&str>,
    ) -> (u64, u64) {
        let invalidations = self
            .invalidations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let global = invalidations
            .global
            .get(canonical_skill_id)
            .copied()
            .unwrap_or_default();
        let project = workspace
            .and_then(|workspace| {
                invalidations
                    .project
                    .get(&(canonical_skill_id.to_string(), workspace.to_string()))
            })
            .copied()
            .unwrap_or_default();
        (global, project)
    }

    fn materialize_work(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let workspace = package.workspace_path.as_deref();
        let snapshot = self
            .snapshots
            .read_overlay_applied_package(&package.metadata.id, workspace)?;
        let effective = snapshot.replay.effective();
        let revision = effective.effective_hash();
        if !valid_hash(revision) {
            return Err(SkillApplicationError::Validation(
                "Effective Skill revision is not a SHA-256 witness".to_string(),
            ));
        }
        let mut files = vec![DerivedFile {
            relative_path: "SKILL.md".to_string(),
            content: compose_document(&SkillDocument {
                metadata: package.metadata.clone(),
                body: effective.instructions().to_string(),
            })
            .into_bytes(),
        }];
        for resource in effective.resources() {
            validate_relative_resource_path(&resource.logical_path)?;
            let content = self.snapshots.read_overlay_applied_resource_bytes(
                &package.metadata.id,
                workspace,
                revision,
                &resource.logical_path,
            )?;
            if content.len() as u64 != resource.size_bytes
                || sha256(&content) != resource.content_hash
            {
                return Err(SkillApplicationError::ConcurrentModification(
                    package.metadata.id.as_str().to_string(),
                ));
            }
            files.push(DerivedFile {
                relative_path: resource.logical_path.clone(),
                content,
            });
        }

        std::fs::create_dir_all(&self.root).map_err(filesystem_error)?;
        let canonical_root = self.root.canonicalize().map_err(filesystem_error)?;
        let requested_parent = canonical_root.join(package.metadata.id.as_str());
        std::fs::create_dir_all(&requested_parent).map_err(filesystem_error)?;
        let package_parent = requested_parent.canonicalize().map_err(filesystem_error)?;
        if package_parent.parent() != Some(canonical_root.as_path())
            || package_parent.file_name().and_then(|value| value.to_str())
                != Some(package.metadata.id.as_str())
        {
            return Err(cache_changed());
        }
        let package_root = package_parent.join(revision);
        if let Ok(metadata) = std::fs::symlink_metadata(&package_root) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(cache_changed());
            }
            let canonical_package_root = package_root.canonicalize().map_err(filesystem_error)?;
            if canonical_package_root.parent() != Some(package_parent.as_path()) {
                return Err(cache_changed());
            }
            verify_cache(&canonical_package_root, &files)?;
            return Ok(managed_source(canonical_package_root, revision));
        }

        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = package_parent.join(format!(
            ".{revision}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        std::fs::create_dir(&temporary).map_err(filesystem_error)?;
        let result = write_cache(&temporary, &files).and_then(|()| {
            match std::fs::rename(&temporary, &package_root) {
                Ok(()) => Ok(()),
                Err(_) if package_root.is_dir() => {
                    remove_cache_tree(&temporary, &package_parent)?;
                    verify_cache(&package_root, &files)
                }
                Err(error) => Err(filesystem_error(error)),
            }
        });
        if let Err(error) = result {
            let _ = remove_cache_tree(&temporary, &package_parent);
            return Err(error);
        }
        verify_cache(&package_root, &files)?;
        Ok(managed_source(package_root, revision))
    }
}

impl SkillPackageMaterializer for EffectiveSkillDerivedCache {
    fn materialize(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let generation = self.generation(package);
        let source = self.materialize_work(package)?;
        if self.generation(package) != generation {
            return Err(SkillApplicationError::ConcurrentModification(
                package.metadata.id.as_str().to_string(),
            ));
        }
        Ok(source)
    }
}

pub(crate) struct EffectiveSkillRuntimeCacheInvalidator {
    catalog: Arc<dyn EffectiveSkillCatalogPort>,
    derived: Arc<EffectiveSkillDerivedCache>,
}

impl EffectiveSkillRuntimeCacheInvalidator {
    pub(crate) fn new(
        catalog: Arc<dyn EffectiveSkillCatalogPort>,
        derived: Arc<EffectiveSkillDerivedCache>,
    ) -> Self {
        Self { catalog, derived }
    }
}

impl OverlayRuntimeCacheInvalidationPort for EffectiveSkillRuntimeCacheInvalidator {
    fn invalidate(&self, key: &OverlayKey) {
        let workspace = (key.scope == OverlayScope::Project)
            .then_some(key.workspace_identity.as_deref())
            .flatten();
        self.catalog.invalidate(workspace);
        self.derived.invalidate(key);
    }
}

fn write_cache(root: &Path, files: &[DerivedFile]) -> Result<(), SkillApplicationError> {
    for file in files {
        let target = root.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        }
        std::fs::write(&target, &file.content).map_err(filesystem_error)?;
        make_read_only(&target)?;
    }
    verify_cache(root, files)
}

fn verify_cache(root: &Path, files: &[DerivedFile]) -> Result<(), SkillApplicationError> {
    let expected = files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<BTreeSet<_>>();
    let actual = collect_cache_files(root, expected.len())?;
    if actual != expected {
        return Err(cache_changed());
    }
    for file in files {
        let path = root.join(&file.relative_path);
        let content = std::fs::read(&path).map_err(filesystem_error)?;
        if content != file.content {
            return Err(cache_changed());
        }
        make_read_only(&path)?;
    }
    Ok(())
}

fn collect_cache_files(
    root: &Path,
    maximum_files: usize,
) -> Result<BTreeSet<String>, SkillApplicationError> {
    let mut relative = BTreeSet::new();
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    while let Some((directory, depth)) = pending.pop() {
        for entry in std::fs::read_dir(&directory).map_err(filesystem_error)? {
            let entry = entry.map_err(filesystem_error)?;
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(filesystem_error)?;
            if metadata.file_type().is_symlink() {
                return Err(cache_changed());
            }
            if metadata.is_dir() {
                if depth >= 8 {
                    return Err(cache_changed());
                }
                pending.push((entry.path(), depth + 1));
                continue;
            }
            if !metadata.is_file() {
                return Err(cache_changed());
            }
            let path = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| cache_changed())?
                .to_string_lossy()
                .replace('\\', "/");
            relative.insert(path);
            if relative.len() > maximum_files {
                return Err(cache_changed());
            }
        }
    }
    Ok(relative)
}

fn managed_source(package_root: PathBuf, revision: &str) -> ManagedSkillSource {
    ManagedSkillSource {
        skill_md_path: normalize_path(&package_root.join("SKILL.md")),
        skill_dir: normalize_path(&package_root),
        content_hash: revision.to_string(),
    }
}

fn remove_cache_tree(path: &Path, expected_parent: &Path) -> Result<(), SkillApplicationError> {
    if !path.exists() {
        return Ok(());
    }
    if path.parent() != Some(expected_parent)
        || !path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.starts_with('.') && value.ends_with(".tmp"))
    {
        return Err(SkillApplicationError::Filesystem(
            "Refused unsafe effective cache cleanup".to_string(),
        ));
    }
    let mut entries = std::fs::read_dir(path)
        .map_err(filesystem_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(filesystem_error)?;
    for entry in &entries {
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(filesystem_error)?;
        if metadata.file_type().is_symlink() {
            return Err(cache_changed());
        }
        if metadata.is_dir() {
            remove_cache_directory(&entry.path())?;
        } else {
            make_writable(&entry.path())?;
            std::fs::remove_file(entry.path()).map_err(filesystem_error)?;
        }
    }
    entries.clear();
    std::fs::remove_dir(path).map_err(filesystem_error)
}

fn remove_cache_directory(path: &Path) -> Result<(), SkillApplicationError> {
    for entry in std::fs::read_dir(path).map_err(filesystem_error)? {
        let entry = entry.map_err(filesystem_error)?;
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(filesystem_error)?;
        if metadata.file_type().is_symlink() {
            return Err(cache_changed());
        }
        if metadata.is_dir() {
            remove_cache_directory(&entry.path())?;
        } else {
            make_writable(&entry.path())?;
            std::fs::remove_file(entry.path()).map_err(filesystem_error)?;
        }
    }
    std::fs::remove_dir(path).map_err(filesystem_error)
}

fn cache_changed() -> SkillApplicationError {
    SkillApplicationError::ConcurrentModification("effective-derived-cache".to_string())
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn make_read_only(path: &Path) -> Result<(), SkillApplicationError> {
    let mut permissions = std::fs::metadata(path)
        .map_err(filesystem_error)?
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions).map_err(filesystem_error)
}

#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "the Windows read-only file attribute must be cleared for cache cleanup"
)]
fn make_writable(path: &Path) -> Result<(), SkillApplicationError> {
    let mut permissions = std::fs::metadata(path)
        .map_err(filesystem_error)?
        .permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(path, permissions).map_err(filesystem_error)
}

#[cfg(unix)]
fn make_writable(path: &Path) -> Result<(), SkillApplicationError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .map_err(filesystem_error)?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    std::fs::set_permissions(path, permissions).map_err(filesystem_error)
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
