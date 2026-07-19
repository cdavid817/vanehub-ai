mod error;
mod path;
mod project;
mod remote_workspace;
mod shell;
mod worktree;

pub(crate) use error::WorkspaceDomainError;
#[allow(unused_imports)]
pub(crate) use path::{
    normalize_windows_extended_length_path, CanonicalPathBoundary, WorkspaceRelativePath,
};
pub(crate) use project::{ensure_git_worktree_available, ProjectInspection, ProjectPath};
pub(crate) use remote_workspace::RemoteWorkspace;
pub(crate) use shell::{reset_directory_command, ShellHost, TerminalDimensions};
pub(crate) use worktree::{ensure_worktree_compatible, WorktreeName};
