use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolFileEntry, SkillToolIntegrityPort, SkillToolPackageRef,
    SkillToolPackageSource,
};
use crate::contexts::tooling::skill_tools::domain::{
    content_hash_of, ContentHash, SkillToolManifestLimits, DEFAULT_MANIFEST_LIMITS, MANIFEST_PATH,
    MODULE_DIRECTORY,
};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

/// Reads Skill tool content from a package directory on disk.
///
/// Containment is enforced twice: the requested relative path must be textually safe, and the
/// canonicalized result must still live under the canonicalized package root. The second check is
/// what catches a symlink or junction that the first cannot see.
pub(crate) struct FilesystemSkillToolSource {
    limits: SkillToolManifestLimits,
}

impl FilesystemSkillToolSource {
    pub(crate) fn new() -> Self {
        Self {
            limits: DEFAULT_MANIFEST_LIMITS,
        }
    }

    pub(crate) fn shared() -> &'static Self {
        static SOURCE: OnceLock<FilesystemSkillToolSource> = OnceLock::new();
        SOURCE.get_or_init(Self::new)
    }

    fn contained_path(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
    ) -> Result<PathBuf, SkillToolApplicationError> {
        validate_relative_path(relative_path)?;
        let root = PathBuf::from(&package.root_path)
            .canonicalize()
            .map_err(filesystem)?;
        let candidate = root.join(relative_path);
        let metadata = std::fs::symlink_metadata(&candidate).map_err(filesystem)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SkillToolApplicationError::PathEscape(
                relative_path.to_string(),
            ));
        }
        let canonical = candidate.canonicalize().map_err(filesystem)?;
        if !canonical.starts_with(&root) {
            return Err(SkillToolApplicationError::PathEscape(
                relative_path.to_string(),
            ));
        }
        Ok(canonical)
    }

    fn read_bounded(
        &self,
        path: &Path,
        relative_path: &str,
        maximum: u64,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        let metadata = std::fs::metadata(path).map_err(filesystem)?;
        // Size is checked before the read so an oversized file is refused rather than buffered.
        if metadata.len() > maximum {
            return Err(SkillToolApplicationError::OversizedFile {
                path: relative_path.to_string(),
                maximum,
                actual: metadata.len(),
            });
        }
        std::fs::read(path).map_err(filesystem)
    }
}

impl SkillToolPackageSource for FilesystemSkillToolSource {
    fn read_manifest(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Option<Vec<u8>>, SkillToolApplicationError> {
        match self.contained_path(package, MANIFEST_PATH) {
            Ok(path) => self
                .read_bounded(&path, MANIFEST_PATH, self.limits.maximum_manifest_bytes)
                .map(Some),
            // A package without a manifest is the normal case, not a failure.
            Err(SkillToolApplicationError::Filesystem(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn list_tool_files(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolFileEntry>, SkillToolApplicationError> {
        const MAX_SCANNED_ENTRIES: usize = 256;

        let Ok(root) = PathBuf::from(&package.root_path).canonicalize() else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        if let Ok(metadata) = std::fs::symlink_metadata(root.join(MANIFEST_PATH)) {
            if metadata.is_file() && !metadata.file_type().is_symlink() {
                entries.push(SkillToolFileEntry {
                    relative_path: MANIFEST_PATH.to_string(),
                    size_bytes: metadata.len(),
                });
            }
        }

        let modules = root.join(MODULE_DIRECTORY.trim_end_matches('/'));
        let Ok(directory) = std::fs::read_dir(&modules) else {
            return Ok(entries);
        };
        // Flat, non-recursive: `validate_module_path` already refuses nested module paths, so a
        // subdirectory can hold nothing a manifest could bind and is not worth walking.
        for entry in directory.flatten() {
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            entries.push(SkillToolFileEntry {
                relative_path: format!("{MODULE_DIRECTORY}{name}"),
                size_bytes: metadata.len(),
            });
            if entries.len() >= MAX_SCANNED_ENTRIES {
                break;
            }
        }
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(entries)
    }

    fn read_implementation(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillToolApplicationError> {
        let path = self.contained_path(package, relative_path)?;
        self.read_bounded(&path, relative_path, self.limits.maximum_module_bytes)
    }
}

impl SkillToolIntegrityPort for FilesystemSkillToolSource {
    fn verify_implementation(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
        expected: &ContentHash,
    ) -> Result<(), SkillToolApplicationError> {
        let bytes = self.read_implementation(package, relative_path)?;
        if &content_hash_of(&bytes) == expected {
            Ok(())
        } else {
            Err(SkillToolApplicationError::IntegrityMismatch {
                path: relative_path.to_string(),
            })
        }
    }
}

/// Refuses a relative path before it ever reaches the filesystem.
///
/// `Component::Normal` is the only accepted component kind, which rejects `..`, `.`, absolute
/// roots, and Windows drive prefixes in one predicate rather than as separate string checks.
fn validate_relative_path(value: &str) -> Result<(), SkillToolApplicationError> {
    let escape = || SkillToolApplicationError::PathEscape(value.chars().take(80).collect());
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        return Err(escape());
    }
    if !value.starts_with(MODULE_DIRECTORY) && value != MANIFEST_PATH {
        return Err(escape());
    }
    for component in Path::new(value).components() {
        let Component::Normal(component) = component else {
            return Err(escape());
        };
        let component = component.to_str().ok_or_else(escape)?;
        if component.is_empty() || component.starts_with('.') {
            return Err(escape());
        }
    }
    Ok(())
}

fn filesystem(error: std::io::Error) -> SkillToolApplicationError {
    SkillToolApplicationError::Filesystem(error.to_string())
}
