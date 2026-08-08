use ignore::WalkBuilder;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use std::{fs::File, io::Read};

use super::super::domain::code_admission::{FileAdmissionPolicy, FileSkipCounts, FileSkipReason};
use super::super::domain::{CodeIndexConfigurationUpdate, CodeLanguage, RetrievalError};

const BINARY_SNIFF_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryFile {
    pub(crate) absolute_path: PathBuf,
    pub(crate) relative_path: String,
    pub(crate) language: CodeLanguage,
    pub(crate) byte_size: u64,
    pub(crate) modified_ns: i64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct InventoryResult {
    pub(crate) files: Vec<InventoryFile>,
    pub(crate) skip_counts: FileSkipCounts,
}

pub(crate) fn inventory_workspace(
    workspace_root: &Path,
    configuration: &CodeIndexConfigurationUpdate,
) -> Result<InventoryResult, RetrievalError> {
    let canonical_root = workspace_root.canonicalize().map_err(storage_error)?;
    if !canonical_root.is_dir() {
        return Err(RetrievalError::InvalidScope);
    }
    let configuration = configuration.clone().validate()?;
    let policy = FileAdmissionPolicy::compile(&configuration)?;
    let roots = inventory_roots(&canonical_root, &configuration.selected_roots)?;
    let mut result = InventoryResult::default();
    let mut seen = BTreeSet::new();

    for root in roots {
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .ignore(true)
            .parents(true)
            .require_git(false)
            .follow_links(false)
            .build();
        for entry in walker {
            let Ok(entry) = entry else {
                result.skip_counts.record(FileSkipReason::Unreadable);
                continue;
            };
            let Some(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }
            let Some(relative_path) = workspace_relative(&canonical_root, entry.path()) else {
                result
                    .skip_counts
                    .record(FileSkipReason::OutsideSelectedRoots);
                continue;
            };
            if !seen.insert(relative_path.clone()) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                result.skip_counts.record(FileSkipReason::Unreadable);
                continue;
            };
            let language = match policy.admit_metadata(&relative_path, metadata.len()) {
                Ok(language) => language,
                Err(reason) => {
                    result.skip_counts.record(reason);
                    continue;
                }
            };
            match is_binary(entry.path()) {
                Ok(true) => {
                    result.skip_counts.record(FileSkipReason::Binary);
                    continue;
                }
                Err(()) => {
                    result.skip_counts.record(FileSkipReason::Unreadable);
                    continue;
                }
                Ok(false) => {}
            }
            result.files.push(InventoryFile {
                absolute_path: entry.path().to_path_buf(),
                relative_path,
                language,
                byte_size: metadata.len(),
                modified_ns: modified_ns(&metadata),
            });
        }
    }
    result
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(result)
}

pub(crate) fn inspect_workspace_path(
    workspace_root: &Path,
    relative_path: &str,
    configuration: &CodeIndexConfigurationUpdate,
) -> Result<Option<InventoryFile>, RetrievalError> {
    let canonical_root = workspace_root.canonicalize().map_err(storage_error)?;
    if !canonical_root.is_dir() {
        return Err(RetrievalError::InvalidScope);
    }
    let normalized = normalize_explicit_path(relative_path)?;
    let candidate = canonical_root.join(&normalized);
    if !candidate.exists() {
        return Ok(None);
    }
    let link_metadata = candidate.symlink_metadata().map_err(storage_error)?;
    if link_metadata.file_type().is_symlink() {
        return Err(RetrievalError::InvalidScope);
    }
    let canonical = candidate.canonicalize().map_err(storage_error)?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        return Err(RetrievalError::InvalidScope);
    }
    let configuration = configuration.clone().validate()?;
    let policy = FileAdmissionPolicy::compile(&configuration)?;
    let metadata = canonical.metadata().map_err(storage_error)?;
    let language = match policy.admit_metadata(&normalized, metadata.len()) {
        Ok(language) => language,
        Err(_) => return Ok(None),
    };
    if is_binary(&canonical).map_err(|_| RetrievalError::Unavailable)? {
        return Ok(None);
    }
    Ok(Some(InventoryFile {
        absolute_path: canonical,
        relative_path: normalized,
        language,
        byte_size: metadata.len(),
        modified_ns: modified_ns(&metadata),
    }))
}

pub(crate) fn normalize_explicit_path(relative_path: &str) -> Result<String, RetrievalError> {
    let path = Path::new(relative_path);
    if relative_path.trim().is_empty() || path.is_absolute() {
        return Err(RetrievalError::InvalidScope);
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => {
                parts.push(part.to_string_lossy().into_owned());
            }
            std::path::Component::CurDir => {}
            _ => return Err(RetrievalError::InvalidScope),
        }
    }
    (!parts.is_empty())
        .then(|| parts.join("/"))
        .ok_or(RetrievalError::InvalidScope)
}

fn inventory_roots(
    canonical_root: &Path,
    selected_roots: &[String],
) -> Result<Vec<PathBuf>, RetrievalError> {
    let mut roots = BTreeSet::new();
    for relative in selected_roots {
        let candidate = if relative.is_empty() {
            canonical_root.to_path_buf()
        } else {
            canonical_root.join(relative)
        };
        if !candidate.exists() {
            continue;
        }
        let canonical = candidate.canonicalize().map_err(storage_error)?;
        if !canonical.is_dir() || !canonical.starts_with(canonical_root) {
            return Err(RetrievalError::InvalidScope);
        }
        roots.insert(canonical);
    }
    Ok(roots.into_iter().collect())
}

fn workspace_relative(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn is_binary(path: &Path) -> Result<bool, ()> {
    let mut file = File::open(path).map_err(|_| ())?;
    let mut buffer = [0_u8; BINARY_SNIFF_BYTES];
    let read = file.read(&mut buffer).map_err(|_| ())?;
    Ok(buffer[..read].contains(&0))
}

fn modified_ns(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

fn storage_error(error: std::io::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::domain::code_index::DEFAULT_MAX_FILE_BYTES;
    use crate::test_support::TempDirectory;

    fn configuration() -> CodeIndexConfigurationUpdate {
        CodeIndexConfigurationUpdate {
            enabled: true,
            selected_roots: vec!["src".to_string()],
            languages: vec![CodeLanguage::Rust],
            exclusion_patterns: vec!["*.generated.rs".to_string()],
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    #[test]
    fn inventory_applies_ignore_and_admission_gates_before_returning_files() {
        let workspace = TempDirectory::new("code-inventory-gates");
        std::fs::create_dir_all(workspace.path().join("src/nested")).expect("create src");
        workspace.write("outside.rs", "fn outside() {}");
        workspace.write("src/kept.rs", "fn kept() {}");
        workspace.write("src/ignored.rs", "fn ignored() {}");
        workspace.write("src/generated.generated.rs", "fn generated() {}");
        workspace.write("src/disabled.ts", "export const disabled = true;");
        workspace.write("src/.env.local", "TOKEN=SENSITIVE-SENTINEL");
        workspace.write("src/nested/.gitignore", "ignored_nested.rs\n");
        workspace.write("src/nested/ignored_nested.rs", "fn ignored() {}");
        workspace.write("src/.gitignore", "ignored.rs\n");
        workspace.write("src/binary.rs", "prefix\0suffix");
        let large =
            std::fs::File::create(workspace.path().join("src/large.rs")).expect("create large");
        large
            .set_len(DEFAULT_MAX_FILE_BYTES + 1)
            .expect("extend large");

        let result =
            inventory_workspace(workspace.path(), &configuration()).expect("inventory succeeds");

        assert_eq!(
            result
                .files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/kept.rs"]
        );
        assert_eq!(
            result.skip_counts.ordered(),
            vec![
                ("sensitive_file", 1),
                ("user_excluded", 1),
                ("language_disabled", 3),
                ("size_limit", 1),
                ("binary", 1),
            ]
        );
    }

    #[test]
    fn inventory_deduplicates_overlapping_selected_roots() {
        let workspace = TempDirectory::new("code-inventory-overlap");
        workspace.write("src/app/main.rs", "fn main() {}");
        let mut configuration = configuration();
        configuration.selected_roots = vec!["src".to_string(), "src/app".to_string()];
        let result =
            inventory_workspace(workspace.path(), &configuration).expect("inventory succeeds");
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].relative_path, "src/app/main.rs");
    }

    #[test]
    fn selected_root_symlink_cannot_escape_the_workspace_when_supported() {
        let workspace = TempDirectory::new("code-inventory-symlink-root");
        let outside = TempDirectory::new("code-inventory-symlink-outside");
        outside.write("secret.rs", "fn secret() {}");
        let link = workspace.path().join("src");
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(outside.path(), &link).is_ok();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(outside.path(), &link).is_ok();
        if linked {
            assert_eq!(
                inventory_workspace(workspace.path(), &configuration()),
                Err(RetrievalError::InvalidScope)
            );
        }
    }
}
