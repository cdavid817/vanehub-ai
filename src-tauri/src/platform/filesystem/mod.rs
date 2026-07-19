//! Canonical path boundaries for workspace-owned filesystem access.

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum BoundaryError {
    #[error("path must be relative")]
    Absolute,
    #[error("hidden path components are unavailable")]
    Hidden,
    #[error("path escape is not allowed")]
    Escape,
    #[error("path has no valid parent")]
    MissingParent,
    #[error("path has no valid file name")]
    MissingFileName,
    #[error("path resolves outside the configured root")]
    OutsideRoot,
    #[error("path is not a directory")]
    NotDirectory,
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct BoundedFilesystem {
    root: PathBuf,
}

impl BoundedFilesystem {
    pub(crate) fn new(root: &Path) -> Result<Self, BoundaryError> {
        let canonical = root.canonicalize()?;
        if !canonical.is_dir() {
            return Err(BoundaryError::NotDirectory);
        }
        Ok(Self { root: canonical })
    }

    pub(crate) fn validate_relative(&self, relative: &str) -> Result<PathBuf, BoundaryError> {
        validate_relative(relative)
    }

    pub(crate) fn resolve_existing(&self, relative: &str) -> Result<PathBuf, BoundaryError> {
        let relative = self.validate_relative(relative)?;
        let canonical = self.root.join(relative).canonicalize()?;
        self.ensure_inside(&canonical)?;
        Ok(canonical)
    }

    pub(crate) fn resolve_with_existing_parent(
        &self,
        relative: &str,
    ) -> Result<(PathBuf, String), BoundaryError> {
        let relative = self.validate_relative(relative)?;
        let candidate = self.root.join(&relative);
        let parent = candidate.parent().ok_or(BoundaryError::MissingParent)?;
        let canonical_parent = parent.canonicalize()?;
        self.ensure_inside(&canonical_parent)?;
        Ok((candidate, normalized(&relative)))
    }

    fn ensure_inside(&self, path: &Path) -> Result<(), BoundaryError> {
        if path.starts_with(&self.root) {
            Ok(())
        } else {
            Err(BoundaryError::OutsideRoot)
        }
    }
}

pub(crate) fn canonical_directory_if_available(
    candidate: Option<&Path>,
) -> Result<Option<PathBuf>, BoundaryError> {
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    if !candidate.exists() || !candidate.is_dir() {
        return Ok(None);
    }
    Ok(Some(candidate.canonicalize()?))
}

pub(crate) fn current_directory() -> Result<PathBuf, BoundaryError> {
    let path = std::env::current_dir()?;
    Ok(path.canonicalize().unwrap_or(path))
}

pub(crate) fn sibling_worktree_target(
    project_path: &Path,
    worktree_name: &str,
) -> Result<PathBuf, BoundaryError> {
    let parent = project_path.parent().ok_or(BoundaryError::MissingParent)?;
    let project_name = project_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(BoundaryError::MissingFileName)?;
    let target = parent.join(format!("{project_name}-{worktree_name}"));
    if target.exists() {
        return Err(BoundaryError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "worktree target exists",
        )));
    }
    if target.starts_with(project_path) && target != project_path {
        return Err(BoundaryError::OutsideRoot);
    }
    Ok(target)
}

pub(crate) fn open_directory(path: &Path) -> Result<(), String> {
    let executable = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut command =
        crate::platform::process::std_command(executable).map_err(|error| error.to_string())?;
    command
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn validate_relative(relative: &str) -> Result<PathBuf, BoundaryError> {
    let relative = PathBuf::from(relative);
    if relative.is_absolute() {
        return Err(BoundaryError::Absolute);
    }
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                if name.to_string_lossy().starts_with('.') {
                    return Err(BoundaryError::Hidden);
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(BoundaryError::Escape)
            }
        }
    }
    Ok(relative)
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn canonical_boundary_rejects_traversal_absolute_and_hidden_paths() {
        let directory = TempDirectory::new("bounded-filesystem");
        std::fs::create_dir_all(directory.path().join("src")).expect("src");
        std::fs::write(directory.path().join("src/main.rs"), "fn main() {}").expect("file");
        let boundary = BoundedFilesystem::new(directory.path()).expect("boundary");

        assert!(boundary.resolve_existing("src/main.rs").is_ok());
        assert!(matches!(
            boundary.resolve_existing("../secret"),
            Err(BoundaryError::Escape)
        ));
        assert!(matches!(
            boundary.resolve_existing(".git/config"),
            Err(BoundaryError::Hidden)
        ));
        assert!(matches!(
            boundary.resolve_existing(&directory.path().join("src").to_string_lossy()),
            Err(BoundaryError::Absolute)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_boundary_rejects_symlinks_outside_the_root() {
        use std::os::unix::fs::symlink;
        let root = TempDirectory::new("bounded-symlink-root");
        let outside = TempDirectory::new("bounded-symlink-outside");
        let secret = outside.write("secret.txt", "private");
        symlink(secret, root.path().join("escape.txt")).expect("symlink");
        let boundary = BoundedFilesystem::new(root.path()).expect("boundary");

        assert!(matches!(
            boundary.resolve_existing("escape.txt"),
            Err(BoundaryError::OutsideRoot)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn canonical_boundary_rejects_symlinks_outside_the_root_when_supported() {
        use std::os::windows::fs::symlink_file;
        let root = TempDirectory::new("bounded-symlink-root");
        let outside = TempDirectory::new("bounded-symlink-outside");
        let secret = outside.write("secret.txt", "private");
        if symlink_file(secret, root.path().join("escape.txt")).is_ok() {
            let boundary = BoundedFilesystem::new(root.path()).expect("boundary");
            assert!(matches!(
                boundary.resolve_existing("escape.txt"),
                Err(BoundaryError::OutsideRoot)
            ));
        }
    }

    #[test]
    fn worktree_target_is_a_non_existing_sibling() {
        let directory = TempDirectory::new("bounded-worktree");
        let project = directory.path().join("app");
        std::fs::create_dir_all(&project).expect("project");

        let target = sibling_worktree_target(&project, "feature-a").expect("target");
        assert_eq!(target, directory.path().join("app-feature-a"));
        std::fs::create_dir_all(&target).expect("existing target");
        assert!(matches!(
            sibling_worktree_target(&project, "feature-a"),
            Err(BoundaryError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
    }
}
