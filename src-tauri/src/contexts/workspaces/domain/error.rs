use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceDomainError {
    ProjectPathRequired,
    RemoteWorkspaceIncomplete,
    InvalidRemoteWorkspace,
    RemoteWorktreeUnsupported,
    GitWorktreeUnavailable,
    InvalidWorktreeName,
    AbsoluteWorkspacePath,
    HiddenWorkspacePath,
    WorkspacePathEscape,
    WorkspacePathOutsideRoot,
}

impl fmt::Display for WorkspaceDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectPathRequired => formatter.write_str("Project path is required."),
            Self::RemoteWorkspaceIncomplete => {
                formatter.write_str("Remote workspace requires host and path")
            }
            Self::InvalidRemoteWorkspace => formatter.write_str("Invalid remote workspace"),
            Self::RemoteWorktreeUnsupported => {
                formatter.write_str("Remote workspace cannot use Git worktree")
            }
            Self::GitWorktreeUnavailable => formatter.write_str("Git worktree unavailable"),
            Self::InvalidWorktreeName => formatter.write_str("Invalid worktree name"),
            Self::AbsoluteWorkspacePath => {
                formatter.write_str("Session workspace paths must be relative.")
            }
            Self::HiddenWorkspacePath => {
                formatter.write_str("Hidden workspace paths are unavailable.")
            }
            Self::WorkspacePathEscape => {
                formatter.write_str("Session workspace path escapes are not allowed.")
            }
            Self::WorkspacePathOutsideRoot => {
                formatter.write_str("Path resolves outside the session root.")
            }
        }
    }
}

impl std::error::Error for WorkspaceDomainError {}
