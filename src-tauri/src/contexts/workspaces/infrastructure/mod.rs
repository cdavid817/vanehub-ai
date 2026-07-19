mod filesystem;
mod git;
mod portable_pty;
mod runtime_support;
mod selection;
mod session_queries;
mod session_shell_workspace;
mod shell_support;
mod sqlite_repository;

pub(crate) use filesystem::WorkspaceFilesystemAdapter;
pub(crate) use git::WorkspaceGitAdapter;
pub(crate) use portable_pty::PortablePtyShellRuntime;
pub(crate) use runtime_support::SystemWorkspaceClock;
pub(crate) use selection::TauriProjectDirectorySelection;
pub(crate) use session_queries::SessionWorkspaceQueryAdapter;
pub(crate) use session_shell_workspace::SqliteShellWorkspaceAdapter;
pub(crate) use shell_support::{
    TauriWorkspaceShellEventPublisher, UuidWorkspaceShellId, WorkspaceShellLoggingAdapter,
};
pub(crate) use sqlite_repository::SqliteWorkspaceHistoryRepository;
