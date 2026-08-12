use super::filesystem::{compose_document, default_home_root};
use crate::contexts::tooling::skills::application::{
    preview_package, ManagedSkillSource, SkillApplicationError, SkillPackageDescriptor,
    SkillPackageMaterializer, SkillPackageReader,
};
use crate::contexts::tooling::skills::domain::SkillLayer;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) struct SystemSkillDerivedCache {
    root: PathBuf,
    reader: Arc<dyn SkillPackageReader>,
}

impl SystemSkillDerivedCache {
    pub(crate) fn new(reader: Arc<dyn SkillPackageReader>) -> Self {
        Self {
            root: default_home_root()
                .join(".vanehub")
                .join("cache")
                .join("skills")
                .join("system"),
            reader,
        }
    }

    #[cfg(test)]
    pub(super) fn with_root(root: PathBuf, reader: Arc<dyn SkillPackageReader>) -> Self {
        Self { root, reader }
    }

    fn materialize_work(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        if package.layer != SkillLayer::System || !valid_revision(&package.revision) {
            return Err(SkillApplicationError::Validation(
                "Only a revision-pinned System package can be materialized".to_string(),
            ));
        }
        let preview = preview_package(package, self.reader.as_ref())?;
        let content = compose_document(&preview.document);
        verify_revision(&content, &package.revision)?;

        std::fs::create_dir_all(&self.root).map_err(filesystem_error)?;
        let canonical_root = self.root.canonicalize().map_err(filesystem_error)?;
        let package_root = canonical_root
            .join(package.metadata.id.as_str())
            .join(&package.revision);
        let skill_file = package_root.join("SKILL.md");
        if skill_file.is_file() {
            if verify_existing(&skill_file, &package.revision).is_ok() {
                make_read_only(&skill_file)?;
                return Ok(managed_source(package_root, skill_file, package));
            }
            make_writable(&skill_file)?;
            std::fs::remove_file(&skill_file).map_err(filesystem_error)?;
            std::fs::remove_dir(&package_root).map_err(filesystem_error)?;
        }

        let parent = package_root.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Invalid System cache target".to_string())
        })?;
        std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        let temporary = parent.join(format!(".{}.tmp", package.revision));
        if temporary.exists() {
            remove_temporary(&temporary)?;
        }
        std::fs::create_dir(&temporary).map_err(filesystem_error)?;
        let temporary_file = temporary.join("SKILL.md");
        std::fs::write(&temporary_file, content.as_bytes()).map_err(filesystem_error)?;
        verify_existing(&temporary_file, &package.revision)?;
        make_read_only(&temporary_file)?;
        match std::fs::rename(&temporary, &package_root) {
            Ok(()) => {}
            Err(_) if skill_file.is_file() => remove_temporary(&temporary)?,
            Err(error) => {
                remove_temporary(&temporary)?;
                return Err(filesystem_error(error));
            }
        }
        verify_existing(&skill_file, &package.revision)?;
        Ok(managed_source(package_root, skill_file, package))
    }
}

impl SkillPackageMaterializer for SystemSkillDerivedCache {
    fn materialize(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.materialize_work(package)
    }
}

fn managed_source(
    package_root: PathBuf,
    skill_file: PathBuf,
    package: &SkillPackageDescriptor,
) -> ManagedSkillSource {
    ManagedSkillSource {
        skill_dir: normalize_path(&package_root),
        skill_md_path: normalize_path(&skill_file),
        content_hash: package.revision.clone(),
    }
}

fn verify_existing(path: &Path, expected: &str) -> Result<(), SkillApplicationError> {
    let content = std::fs::read(path).map_err(filesystem_error)?;
    let actual = sha256(&content);
    if actual == expected {
        Ok(())
    } else {
        Err(SkillApplicationError::ConcurrentModification(
            "system-derived-cache".to_string(),
        ))
    }
}

fn verify_revision(content: &str, expected: &str) -> Result<(), SkillApplicationError> {
    if sha256(content.as_bytes()) == expected {
        Ok(())
    } else {
        Err(SkillApplicationError::Validation(
            "System package revision does not match its document".to_string(),
        ))
    }
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_revision(revision: &str) -> bool {
    revision.len() == 64 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn make_read_only(path: &Path) -> Result<(), SkillApplicationError> {
    let mut permissions = std::fs::metadata(path)
        .map_err(filesystem_error)?
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions).map_err(filesystem_error)
}

fn remove_temporary(path: &Path) -> Result<(), SkillApplicationError> {
    let file = path.join("SKILL.md");
    if file.exists() {
        make_writable(&file)?;
        std::fs::remove_file(&file).map_err(filesystem_error)?;
    }
    if path.exists() {
        std::fs::remove_dir(path).map_err(filesystem_error)?;
    }
    Ok(())
}

#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "the Windows read-only file attribute must be cleared before cache cleanup"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::{SkillLayerProvider, SkillPackageReader};
    use crate::contexts::tooling::skills::infrastructure::SystemSkillPackages;
    use crate::test_support::TempDirectory;

    #[test]
    fn materialization_is_read_only_revision_pinned_and_idempotent() {
        let temporary = TempDirectory::new("system-skill-derived-cache");
        let packages = Arc::new(SystemSkillPackages);
        let descriptor = packages
            .inventory(None)
            .expect("inventory")
            .into_iter()
            .find(|package| package.metadata.id.as_str() == "code-review")
            .expect("code-review");
        let reader: Arc<dyn SkillPackageReader> = packages;
        let cache = SystemSkillDerivedCache::with_root(temporary.path().join("derived"), reader);

        let first = cache.materialize(&descriptor).expect("materialized");
        let second = cache.materialize(&descriptor).expect("idempotent");
        assert_eq!(first, second);
        assert!(Path::new(&first.skill_md_path)
            .metadata()
            .expect("metadata")
            .permissions()
            .readonly());
        assert!(first.skill_dir.contains(&descriptor.revision));
        assert_eq!(
            sha256(&std::fs::read(&first.skill_md_path).expect("content")),
            descriptor.revision
        );
    }

    #[test]
    fn materialization_rejects_non_system_and_unpinned_packages() {
        let temporary = TempDirectory::new("system-skill-derived-cache-rejection");
        let packages = Arc::new(SystemSkillPackages);
        let mut descriptor = packages.inventory(None).expect("inventory").remove(0);
        let reader: Arc<dyn SkillPackageReader> = packages;
        let cache = SystemSkillDerivedCache::with_root(temporary.path().join("derived"), reader);
        descriptor.layer = SkillLayer::User;
        assert!(cache.materialize(&descriptor).is_err());
        descriptor.layer = SkillLayer::System;
        descriptor.revision = "../escape".to_string();
        assert!(cache.materialize(&descriptor).is_err());
        assert!(!temporary.path().join("derived").exists());
    }
}
