use super::WorkspaceDomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WorktreeName(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitReference(String);

impl GitReference {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, WorkspaceDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('-')
            || trimmed.ends_with(['/', '.'])
            || trimmed.contains("..")
            || trimmed.contains("@{")
            || trimmed
                .chars()
                .any(|character| character.is_control() || " ~^:?*[\\".contains(character))
        {
            Err(WorkspaceDomainError::InvalidWorktreeName)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorktreeName {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, WorkspaceDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
            || trimmed.chars().any(char::is_control)
        {
            Err(WorkspaceDomainError::InvalidWorktreeName)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn branch_name(&self) -> String {
        format!("vanehub/{}", self.0)
    }
}

pub(crate) fn ensure_worktree_compatible(
    remote_workspace_selected: bool,
    worktree_enabled: bool,
) -> Result<(), WorkspaceDomainError> {
    if remote_workspace_selected && worktree_enabled {
        Err(WorkspaceDomainError::RemoteWorktreeUnsupported)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_name_is_trimmed_and_derives_stable_git_names() {
        let name = WorktreeName::parse("  feature-a  ").expect("worktree name");

        assert_eq!(name.as_str(), "feature-a");
        assert_eq!(name.branch_name(), "vanehub/feature-a");
    }

    #[test]
    fn worktree_name_rejects_empty_traversal_separators_and_controls() {
        for value in ["", "   ", "../bad", "bad/name", "bad\\name", "bad\nname"] {
            assert_eq!(
                WorktreeName::parse(value),
                Err(WorkspaceDomainError::InvalidWorktreeName)
            );
        }
    }

    #[test]
    fn git_reference_allows_remote_branches_and_rejects_option_like_values() {
        assert_eq!(
            GitReference::parse(" origin/main ")
                .expect("reference")
                .as_str(),
            "origin/main"
        );
        for value in ["", "-main", "feature..main", "main~1", "bad branch"] {
            assert!(GitReference::parse(value).is_err());
        }
    }

    #[test]
    fn remote_workspace_and_worktree_are_mutually_exclusive() {
        assert_eq!(ensure_worktree_compatible(false, true), Ok(()));
        assert_eq!(ensure_worktree_compatible(true, false), Ok(()));
        assert_eq!(
            ensure_worktree_compatible(true, true),
            Err(WorkspaceDomainError::RemoteWorktreeUnsupported)
        );
    }
}
