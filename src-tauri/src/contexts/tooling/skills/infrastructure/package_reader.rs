use super::filesystem::{
    default_home_root, document_content_hash, parse_document, read_bounded_document,
};
use crate::contexts::tooling::skills::application::{
    validate_relative_resource_path, SkillApplicationError, SkillDocument, SkillPackageDescriptor,
    SkillPackageReader, SkillPackageResource, SkillResourceDocument, MAX_RESOURCE_CHARACTERS,
};
use crate::contexts::tooling::skills::domain::{SkillLayer, DEFAULT_OVERLAY_LIMITS};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) struct LayeredSkillPackageReader {
    system: Arc<dyn SkillPackageReader>,
    user_home: PathBuf,
}

impl LayeredSkillPackageReader {
    pub(crate) fn new(system: Arc<dyn SkillPackageReader>) -> Self {
        Self {
            system,
            user_home: default_home_root(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_home_root(system: Arc<dyn SkillPackageReader>, user_home: PathBuf) -> Self {
        Self { system, user_home }
    }

    fn read_filesystem_package(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<SkillDocument, SkillApplicationError> {
        let source = self.filesystem_source(package)?;
        let skill_file = source
            .join("SKILL.md")
            .canonicalize()
            .map_err(filesystem_error)?;
        if skill_file.parent() != Some(source.as_path()) {
            return Err(SkillApplicationError::Filesystem(
                "Effective Skill document escapes its package".to_string(),
            ));
        }
        let raw = read_bounded_document(&skill_file)?;
        if document_content_hash(&raw) != package.revision {
            return Err(SkillApplicationError::ConcurrentModification(
                package.metadata.id.as_str().to_string(),
            ));
        }
        let document = parse_document(&raw)?;
        if document.metadata.id != package.metadata.id {
            return Err(SkillApplicationError::Validation(
                "Effective Skill identity changed during read".to_string(),
            ));
        }
        Ok(document)
    }

    fn filesystem_source(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<PathBuf, SkillApplicationError> {
        let source = package.source_path.as_deref().ok_or_else(|| {
            SkillApplicationError::Filesystem("Filesystem Skill has no managed source".to_string())
        })?;
        let source = PathBuf::from(source)
            .canonicalize()
            .map_err(filesystem_error)?;
        let root = self
            .package_root(package)?
            .canonicalize()
            .map_err(filesystem_error)?;
        if !source.starts_with(&root) {
            return Err(SkillApplicationError::ResourceEscape);
        }
        Ok(source)
    }

    fn verify_filesystem_revision(
        &self,
        package: &SkillPackageDescriptor,
        source: &Path,
    ) -> Result<(), SkillApplicationError> {
        let skill_file = source
            .join("SKILL.md")
            .canonicalize()
            .map_err(filesystem_error)?;
        if skill_file.parent() != Some(source) {
            return Err(SkillApplicationError::ResourceEscape);
        }
        let raw = read_bounded_document(&skill_file)?;
        if document_content_hash(&raw) != package.revision {
            return Err(SkillApplicationError::ConcurrentModification(
                package.metadata.id.as_str().to_string(),
            ));
        }
        Ok(())
    }

    fn list_filesystem_resources(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
        const RESOURCE_DIRECTORIES: [&str; 4] = ["scripts", "references", "templates", "assets"];
        const MAX_SCANNED_RESOURCES: usize = 256;
        const MAX_RESOURCE_DEPTH: usize = 5;
        let source = self.filesystem_source(package)?;
        self.verify_filesystem_revision(package, &source)?;
        let mut resources = Vec::new();
        let mut pending = RESOURCE_DIRECTORIES
            .iter()
            .rev()
            .map(|directory| (source.join(directory), 1_usize))
            .collect::<Vec<_>>();
        while let Some((directory, depth)) = pending.pop() {
            let metadata = match std::fs::symlink_metadata(&directory) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(filesystem_error(error)),
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            let mut entries = std::fs::read_dir(&directory)
                .map_err(filesystem_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(filesystem_error)?;
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries.into_iter().rev() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }
                let metadata = std::fs::symlink_metadata(entry.path()).map_err(filesystem_error)?;
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    if depth < MAX_RESOURCE_DEPTH {
                        pending.push((entry.path(), depth + 1));
                    }
                    continue;
                }
                if !metadata.is_file() {
                    continue;
                }
                let canonical = entry.path().canonicalize().map_err(filesystem_error)?;
                if !canonical.starts_with(&source) {
                    return Err(SkillApplicationError::ResourceEscape);
                }
                let relative_path = canonical
                    .strip_prefix(&source)
                    .map_err(|_| SkillApplicationError::ResourceEscape)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if validate_relative_resource_path(&relative_path).is_err() {
                    continue;
                }
                resources.push(SkillPackageResource {
                    media_type: resource_media_type(&relative_path).to_string(),
                    content_hash: sha256_file(&canonical)?,
                    relative_path,
                    size_bytes: metadata.len(),
                });
                if resources.len() >= MAX_SCANNED_RESOURCES {
                    resources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
                    return Ok(resources);
                }
            }
        }
        resources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(resources)
    }

    fn read_filesystem_resource(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let bytes = self.read_filesystem_resource_bytes(package, relative_path)?;
        let size_bytes = bytes.len() as u64;
        let content =
            String::from_utf8(bytes).map_err(|_| SkillApplicationError::BinaryResource)?;
        if content.chars().count() > MAX_RESOURCE_CHARACTERS {
            return Err(SkillApplicationError::OversizedResource);
        }
        Ok(SkillResourceDocument {
            content,
            size_bytes,
        })
    }

    fn read_filesystem_resource_bytes(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        validate_relative_resource_path(relative_path)?;
        let source = self.filesystem_source(package)?;
        self.verify_filesystem_revision(package, &source)?;
        let candidate = source.join(relative_path);
        let metadata = std::fs::symlink_metadata(&candidate).map_err(filesystem_error)?;
        if metadata.file_type().is_symlink() {
            return Err(SkillApplicationError::ResourceEscape);
        }
        if !metadata.is_file()
            || metadata.len() > DEFAULT_OVERLAY_LIMITS.maximum_supporting_file_bytes
        {
            return Err(SkillApplicationError::OversizedResource);
        }
        let canonical = candidate.canonicalize().map_err(filesystem_error)?;
        if !canonical.starts_with(&source) {
            return Err(SkillApplicationError::ResourceEscape);
        }
        std::fs::read(&canonical).map_err(filesystem_error)
    }

    fn package_root(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<PathBuf, SkillApplicationError> {
        match package.layer {
            SkillLayer::User => Ok(self.user_home.join(".vanehub").join("skills")),
            SkillLayer::Project => package
                .workspace_path
                .as_deref()
                .map(Path::new)
                .map(|workspace| workspace.join(".vanehub").join("skills"))
                .ok_or_else(|| {
                    SkillApplicationError::Validation(
                        "Project Skill requires a canonical workspace".to_string(),
                    )
                }),
            _ => Err(SkillApplicationError::Validation(
                "Package does not use a filesystem layer".to_string(),
            )),
        }
    }
}

impl SkillPackageReader for LayeredSkillPackageReader {
    fn read_document(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<SkillDocument, SkillApplicationError> {
        match package.layer {
            SkillLayer::System => self.system.read_document(package),
            SkillLayer::Project | SkillLayer::User => self.read_filesystem_package(package),
            SkillLayer::Registry => Err(SkillApplicationError::Validation(
                "Registry Skill reading is unavailable".to_string(),
            )),
        }
    }

    fn list_resources(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
        match package.layer {
            SkillLayer::System => self.system.list_resources(package),
            SkillLayer::Project | SkillLayer::User => self.list_filesystem_resources(package),
            SkillLayer::Registry => Err(SkillApplicationError::Validation(
                "Registry Skill reading is unavailable".to_string(),
            )),
        }
    }

    fn read_resource(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        match package.layer {
            SkillLayer::System => self.system.read_resource(package, relative_path),
            SkillLayer::Project | SkillLayer::User => {
                self.read_filesystem_resource(package, relative_path)
            }
            SkillLayer::Registry => Err(SkillApplicationError::Validation(
                "Registry Skill reading is unavailable".to_string(),
            )),
        }
    }

    fn read_resource_bytes(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        match package.layer {
            SkillLayer::System => self.system.read_resource_bytes(package, relative_path),
            SkillLayer::Project | SkillLayer::User => {
                self.read_filesystem_resource_bytes(package, relative_path)
            }
            SkillLayer::Registry => Err(SkillApplicationError::Validation(
                "Registry Skill reading is unavailable".to_string(),
            )),
        }
    }
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

fn sha256_file(path: &Path) -> Result<String, SkillApplicationError> {
    let content = std::fs::read(path).map_err(filesystem_error)?;
    Ok(Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn resource_media_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("md" | "markdown") => "text/markdown",
        Some("json") => "application/json",
        Some("yaml" | "yml") => "application/yaml",
        Some("txt") => "text/plain",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::{
        SkillLayerProvider, MAX_RESOURCE_PATH_CHARACTERS,
    };
    use crate::contexts::tooling::skills::infrastructure::{
        FilesystemSkillLayerProvider, SystemSkillPackages,
    };
    use crate::test_support::TempDirectory;

    fn fixture() -> (
        TempDirectory,
        LayeredSkillPackageReader,
        SkillPackageDescriptor,
    ) {
        let home = TempDirectory::new("progressive-reader-home");
        let package_dir = home.path().join(".vanehub/skills/resource-skill");
        std::fs::create_dir_all(package_dir.join("references")).expect("resource directory");
        std::fs::create_dir_all(package_dir.join("assets")).expect("asset directory");
        std::fs::write(
            package_dir.join("SKILL.md"),
            "---\nid: resource-skill\nname: Resource Skill\ndescription: Reader fixture\ncategory: testing\nversion: 1.0.0\ntype: role\ndelivery: on-demand\n---\nInstructions",
        )
        .expect("Skill document");
        let provider =
            FilesystemSkillLayerProvider::with_root(SkillLayer::User, home.path().to_path_buf());
        let package = provider
            .inventory(None)
            .expect("inventory")
            .into_iter()
            .next()
            .expect("package");
        let reader = LayeredSkillPackageReader::with_home_root(
            Arc::new(SystemSkillPackages),
            home.path().to_path_buf(),
        );
        (home, reader, package)
    }

    #[test]
    fn filesystem_resources_are_indexed_deterministically_without_hidden_or_linked_entries() {
        let (home, reader, package) = fixture();
        let package_dir = home.path().join(".vanehub/skills/resource-skill");
        std::fs::write(package_dir.join("references/z.md"), "Z").expect("z");
        std::fs::write(package_dir.join("references/a.md"), "A").expect("a");
        std::fs::write(package_dir.join("references/.hidden.md"), "hidden").expect("hidden");
        let outside = home.path().join("outside.txt");
        std::fs::write(&outside, "secret").expect("outside");
        let linked = create_file_symlink(&outside, &package_dir.join("references/link.md"));

        let resources = reader.list_resources(&package).expect("resources");
        assert_eq!(
            resources
                .iter()
                .map(|resource| resource.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["references/a.md", "references/z.md"]
        );
        if linked {
            assert_eq!(
                reader.read_resource(&package, "references/link.md"),
                Err(SkillApplicationError::ResourceEscape)
            );
        }
    }

    #[test]
    fn filesystem_resource_reads_reject_paths_binary_and_oversized_content() {
        let (home, reader, package) = fixture();
        let package_dir = home.path().join(".vanehub/skills/resource-skill");
        std::fs::write(package_dir.join("references/guide.md"), "Guide").expect("guide");
        std::fs::write(package_dir.join("assets/binary.bin"), [0xff, 0xfe, 0xfd]).expect("binary");
        std::fs::write(
            package_dir.join("assets/large.txt"),
            vec![
                b'x';
                crate::contexts::tooling::skills::application::MAX_RESOURCE_BYTES as usize + 1
            ],
        )
        .expect("large");

        assert_eq!(
            reader
                .read_resource(&package, "references/guide.md")
                .expect("read")
                .content,
            "Guide"
        );
        for invalid in [
            "C:/secret.txt".to_string(),
            "references/../secret.md".to_string(),
            "references/.hidden.md".to_string(),
            format!("references/{}.md", "x".repeat(MAX_RESOURCE_PATH_CHARACTERS)),
        ] {
            assert_eq!(
                reader.read_resource(&package, &invalid),
                Err(SkillApplicationError::InvalidResourceUri)
            );
        }
        assert_eq!(
            reader.read_resource(&package, "assets/binary.bin"),
            Err(SkillApplicationError::BinaryResource)
        );
        assert_eq!(
            reader.read_resource(&package, "assets/large.txt"),
            Err(SkillApplicationError::OversizedResource)
        );
    }

    #[test]
    fn filesystem_resource_reads_reject_a_stale_package_revision() {
        let (home, reader, package) = fixture();
        let package_dir = home.path().join(".vanehub/skills/resource-skill");
        std::fs::write(package_dir.join("references/guide.md"), "Guide").expect("guide");
        std::fs::write(
            package_dir.join("SKILL.md"),
            "---\nid: resource-skill\nname: Changed\ndescription: Reader fixture\ncategory: testing\nversion: 1.0.0\ntype: role\ndelivery: on-demand\n---\nChanged",
        )
        .expect("changed Skill");
        assert!(matches!(
            reader.read_resource(&package, "references/guide.md"),
            Err(SkillApplicationError::ConcurrentModification(_))
        ));
    }

    #[cfg(unix)]
    fn create_file_symlink(source: &Path, target: &Path) -> bool {
        std::os::unix::fs::symlink(source, target).is_ok()
    }

    #[cfg(windows)]
    fn create_file_symlink(source: &Path, target: &Path) -> bool {
        std::os::windows::fs::symlink_file(source, target).is_ok()
    }
}
