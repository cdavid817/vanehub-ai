use std::path::Path;

use super::WorkspaceDomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProjectPath(String);

impl ProjectPath {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, WorkspaceDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(WorkspaceDomainError::ProjectPathRequired)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn display_name(&self) -> String {
        Path::new(&self.0)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.0.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectInspection {
    path: ProjectPath,
    display_name: String,
    git_root: Option<String>,
}

impl ProjectInspection {
    pub(crate) fn from_probe(
        canonical_path: impl Into<String>,
        git_root: Option<String>,
    ) -> Result<Self, WorkspaceDomainError> {
        let path = ProjectPath::parse(canonical_path)?;
        let display_name = path.display_name();
        let git_root = git_root.filter(|root| !root.is_empty());
        Ok(Self {
            path,
            display_name,
            git_root,
        })
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) fn is_git(&self) -> bool {
        self.git_root.is_some()
    }

    pub(crate) fn git_root(&self) -> Option<&str> {
        self.git_root.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn ensure_worktree_available(&self) -> Result<(), WorkspaceDomainError> {
        ensure_git_worktree_available(self.is_git())
    }
}

pub(crate) fn ensure_git_worktree_available(is_git: bool) -> Result<(), WorkspaceDomainError> {
    if is_git {
        Ok(())
    } else {
        Err(WorkspaceDomainError::GitWorktreeUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_paths_are_required_trimmed_and_named_from_the_last_component() {
        assert_eq!(
            ProjectPath::parse(" \t "),
            Err(WorkspaceDomainError::ProjectPathRequired)
        );
        let project = ProjectPath::parse("  work/vanehub-ai  ").expect("project path");
        assert_eq!(project.as_str(), "work/vanehub-ai");
        assert_eq!(project.display_name(), "vanehub-ai");
    }

    #[test]
    fn inspection_derives_git_state_and_display_name_from_probe_facts() {
        let repository =
            ProjectInspection::from_probe("work/vanehub-ai", Some("work/vanehub-ai".to_string()))
                .expect("git project");
        assert_eq!(repository.display_name(), "vanehub-ai");
        assert!(repository.is_git());
        assert_eq!(repository.git_root(), Some("work/vanehub-ai"));
        assert!(repository.ensure_worktree_available().is_ok());

        let plain = ProjectInspection::from_probe("work/scratch", None).expect("plain project");
        assert!(!plain.is_git());
        assert_eq!(
            plain.ensure_worktree_available(),
            Err(WorkspaceDomainError::GitWorktreeUnavailable)
        );
    }

    #[test]
    fn empty_git_probe_output_is_not_a_repository() {
        let inspection =
            ProjectInspection::from_probe("work/scratch", Some(String::new())).expect("project");

        assert!(!inspection.is_git());
        assert_eq!(inspection.git_root(), None);
    }
}
