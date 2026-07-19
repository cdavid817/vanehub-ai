use std::path::{Component, Path, PathBuf};

use super::WorkspaceDomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceRelativePath(PathBuf);

impl WorkspaceRelativePath {
    pub(crate) fn parse(value: &str) -> Result<Self, WorkspaceDomainError> {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Err(WorkspaceDomainError::AbsoluteWorkspacePath);
        }
        for component in path.components() {
            match component {
                Component::Normal(name) => {
                    if name.to_string_lossy().starts_with('.') {
                        return Err(WorkspaceDomainError::HiddenWorkspacePath);
                    }
                }
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspaceDomainError::WorkspacePathEscape);
                }
            }
        }
        Ok(Self(path))
    }

    #[cfg(test)]
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }

    #[cfg(test)]
    pub(crate) fn normalized(&self) -> String {
        self.0.to_string_lossy().replace('\\', "/")
    }

    pub(crate) fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

pub(crate) fn normalize_windows_extended_length_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", rest);
    }
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    path.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalPathBoundary {
    root: PathBuf,
}

impl CanonicalPathBoundary {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub(crate) fn ensure_inside(&self, candidate: &Path) -> Result<(), WorkspaceDomainError> {
        if candidate.starts_with(&self.root) {
            Ok(())
        } else {
            Err(WorkspaceDomainError::WorkspacePathOutsideRoot)
        }
    }

    pub(crate) fn relative(&self, candidate: &Path) -> Result<String, WorkspaceDomainError> {
        self.ensure_inside(candidate)?;
        candidate
            .strip_prefix(&self.root)
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .map_err(|_| WorkspaceDomainError::WorkspacePathOutsideRoot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_reject_absolute_escape_and_hidden_components() {
        let safe = WorkspaceRelativePath::parse("src/main.rs").expect("relative path");
        assert_eq!(safe.normalized(), "src/main.rs");
        assert_eq!(safe.as_path(), Path::new("src/main.rs"));

        assert_eq!(
            WorkspaceRelativePath::parse("../secret"),
            Err(WorkspaceDomainError::WorkspacePathEscape)
        );
        assert_eq!(
            WorkspaceRelativePath::parse(".git/config"),
            Err(WorkspaceDomainError::HiddenWorkspacePath)
        );
        let absolute = if cfg!(windows) {
            "C:\\secret".to_string()
        } else {
            "/secret".to_string()
        };
        assert_eq!(
            WorkspaceRelativePath::parse(&absolute),
            Err(WorkspaceDomainError::AbsoluteWorkspacePath)
        );
    }

    #[test]
    fn windows_extended_length_paths_are_normalized_for_shells_and_labels() {
        assert_eq!(
            normalize_windows_extended_length_path(r"\\?\D:\cdavid\Documents\code\claude-code"),
            r"D:\cdavid\Documents\code\claude-code"
        );
        assert_eq!(
            normalize_windows_extended_length_path(r"\\?\UNC\server\share\repo"),
            r"\\server\share\repo"
        );
        assert_eq!(
            normalize_windows_extended_length_path(r"D:\cdavid\Documents\code\claude-code"),
            r"D:\cdavid\Documents\code\claude-code"
        );
    }

    #[test]
    fn canonical_boundary_accepts_descendants_and_rejects_siblings() {
        let root = PathBuf::from("work").join("app");
        let boundary = CanonicalPathBoundary::new(&root);

        assert_eq!(
            boundary.relative(&root.join("src").join("main.rs")),
            Ok("src/main.rs".to_string())
        );
        assert_eq!(
            boundary.ensure_inside(&PathBuf::from("work").join("app-copy").join("secret.txt")),
            Err(WorkspaceDomainError::WorkspacePathOutsideRoot)
        );
    }
}
