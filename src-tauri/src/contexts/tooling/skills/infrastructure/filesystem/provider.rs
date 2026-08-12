use super::{
    document,
    paths::{default_home_root, SKILL_FILE},
};
use crate::contexts::tooling::skills::application::{
    normalize_utility_availability, SkillApplicationError, SkillLayerProvider,
    SkillPackageDescriptor,
};
use crate::contexts::tooling::skills::domain::{
    SkillAvailability, SkillDelivery, SkillLayer, SkillMetadata, SkillOrigin, SkillTrust, SkillType,
};
use std::path::{Path, PathBuf};

const MAX_DISCOVERY_DEPTH: usize = 3;
const EXCLUDED_DIRECTORIES: [&str; 7] = [
    ".git",
    ".venv",
    "node_modules",
    "__pycache__",
    "build",
    "dist",
    "target",
];

#[derive(Clone)]
pub(crate) struct FilesystemSkillLayerProvider {
    layer: SkillLayer,
    user_home: PathBuf,
}

impl FilesystemSkillLayerProvider {
    pub(crate) fn user() -> Self {
        Self {
            layer: SkillLayer::User,
            user_home: default_home_root(),
        }
    }

    pub(crate) fn project() -> Self {
        Self {
            layer: SkillLayer::Project,
            user_home: default_home_root(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_root(layer: SkillLayer, root: PathBuf) -> Self {
        Self {
            layer,
            user_home: root,
        }
    }

    fn discovery_root(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Option<(PathBuf, Option<String>)>, SkillApplicationError> {
        let (boundary, workspace) = match self.layer {
            SkillLayer::User => (
                self.user_home.canonicalize().map_err(filesystem_error)?,
                None,
            ),
            SkillLayer::Project => {
                let Some(workspace) = workspace_path else {
                    return Ok(None);
                };
                let boundary = PathBuf::from(workspace)
                    .canonicalize()
                    .map_err(filesystem_error)?;
                if !boundary.is_dir() {
                    return Err(SkillApplicationError::Filesystem(
                        "Skill workspace boundary is not a directory".to_string(),
                    ));
                }
                (boundary.clone(), Some(normalize_path(&boundary)))
            }
            _ => {
                return Err(SkillApplicationError::Validation(
                    "Filesystem Skill provider supports only project and user layers".to_string(),
                ));
            }
        };
        let root = boundary.join(".vanehub").join("skills");
        if !root.exists() {
            return Ok(None);
        }
        let root = root.canonicalize().map_err(filesystem_error)?;
        if !root.is_dir() || !root.starts_with(&boundary) {
            return Err(SkillApplicationError::Filesystem(
                "Skill discovery root escapes its canonical boundary".to_string(),
            ));
        }
        Ok(Some((root, workspace)))
    }
}

impl SkillLayerProvider for FilesystemSkillLayerProvider {
    fn inventory(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
        let Some((root, workspace)) = self.discovery_root(workspace_path)? else {
            return Ok(Vec::new());
        };
        let mut packages = Vec::new();
        discover(
            &root,
            &root,
            0,
            self.layer,
            workspace.as_deref(),
            &mut packages,
        )?;
        packages.sort_by(|left, right| left.package_key.cmp(&right.package_key));
        Ok(packages)
    }
}

#[derive(Clone, Default)]
pub(crate) struct EmptyRegistrySkillProvider;

impl SkillLayerProvider for EmptyRegistrySkillProvider {
    fn inventory(
        &self,
        _workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError> {
        Ok(Vec::new())
    }
}

fn discover(
    root: &Path,
    directory: &Path,
    depth: usize,
    layer: SkillLayer,
    workspace_path: Option<&str>,
    packages: &mut Vec<SkillPackageDescriptor>,
) -> Result<(), SkillApplicationError> {
    if depth > MAX_DISCOVERY_DEPTH {
        return Ok(());
    }
    let document_path = directory.join(SKILL_FILE);
    if document_path.is_file() {
        let canonical = directory.canonicalize().map_err(filesystem_error)?;
        if !canonical.starts_with(root) {
            return Err(SkillApplicationError::Filesystem(
                "Discovered Skill package escapes its layer root".to_string(),
            ));
        }
        let content = match document::read_import_document(&document_path) {
            Ok(content) => content,
            Err(_) => {
                if let Some(descriptor) =
                    invalid_descriptor(&canonical, layer, workspace_path, "unreadable")
                {
                    packages.push(descriptor);
                }
                return Ok(());
            }
        };
        let metadata = match document::parse(&content) {
            Ok(metadata) => metadata,
            Err(_) => {
                if let Some(descriptor) = invalid_descriptor(
                    &canonical,
                    layer,
                    workspace_path,
                    &document::content_hash(&content),
                ) {
                    packages.push(descriptor);
                }
                return Ok(());
            }
        };
        let mut descriptor = SkillPackageDescriptor {
            package_key: normalize_path(&canonical),
            workspace_path: workspace_path.map(str::to_string),
            metadata,
            layer,
            origin: SkillOrigin::Created,
            trust: SkillTrust::Trusted,
            availability: SkillAvailability::Available,
            revision: document::content_hash(&content),
            source_path: Some(normalize_path(&canonical)),
        };
        normalize_utility_availability(&mut descriptor, false);
        packages.push(descriptor);
        return Ok(());
    }
    if depth == MAX_DISCOVERY_DEPTH {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(directory)
        .map_err(filesystem_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(filesystem_error)?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry.file_type().map_err(filesystem_error)?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if EXCLUDED_DIRECTORIES.contains(&name.as_str()) || name.starts_with('.') {
            continue;
        }
        discover(
            root,
            &entry.path(),
            depth + 1,
            layer,
            workspace_path,
            packages,
        )?;
    }
    Ok(())
}

fn invalid_descriptor(
    directory: &Path,
    layer: SkillLayer,
    workspace_path: Option<&str>,
    revision: &str,
) -> Option<SkillPackageDescriptor> {
    let id = directory.file_name()?.to_str()?;
    let metadata = SkillMetadata::with_classification(
        id,
        id,
        "Invalid Skill package",
        "invalid",
        "unknown",
        Vec::new(),
        Vec::new(),
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    )
    .ok()?;
    Some(SkillPackageDescriptor {
        package_key: normalize_path(directory),
        workspace_path: workspace_path.map(str::to_string),
        metadata,
        layer,
        origin: SkillOrigin::Created,
        trust: SkillTrust::Trusted,
        availability: SkillAvailability::Invalid,
        revision: revision.to_string(),
        source_path: Some(normalize_path(directory)),
    })
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
    use crate::test_support::TempDirectory;

    const SKILL: &str = "---\nid: fixture-skill\nname: Fixture\ndescription: Description\ncategory: test\nversion: 1.0.0\ntype: role\ndelivery: on-demand\n---\n\nBody";

    #[test]
    fn project_provider_is_workspace_bound_and_omitted_without_workspace() {
        let workspace = TempDirectory::new("effective-project-provider");
        workspace.write(".vanehub/skills/fixture/SKILL.md", SKILL);
        let provider = FilesystemSkillLayerProvider::project();
        assert!(provider.inventory(None).expect("no workspace").is_empty());
        let workspace_path = workspace.path().canonicalize().expect("workspace");
        let packages = provider
            .inventory(Some(&normalize_path(&workspace_path)))
            .expect("packages");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].layer, SkillLayer::Project);
    }

    #[test]
    fn user_provider_is_bounded_and_skips_excluded_and_deep_directories() {
        let home = TempDirectory::new("effective-user-provider");
        home.write(".vanehub/skills/valid/SKILL.md", SKILL);
        home.write(".vanehub/skills/node_modules/ignored/SKILL.md", SKILL);
        home.write(".vanehub/skills/a/b/c/d/SKILL.md", SKILL);
        let provider =
            FilesystemSkillLayerProvider::with_root(SkillLayer::User, home.path().to_path_buf());
        let packages = provider.inventory(None).expect("packages");
        assert_eq!(packages.len(), 1);
        assert!(packages[0].package_key.ends_with("valid"));
    }

    #[test]
    fn registry_provider_is_explicitly_empty_and_network_free() {
        let provider = EmptyRegistrySkillProvider;
        assert!(provider
            .inventory(Some("ignored"))
            .expect("inventory")
            .is_empty());
    }
}
