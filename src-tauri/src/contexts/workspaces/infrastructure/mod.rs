mod bounded_selection;
mod capture_maintenance;
mod capture_queue;
mod command_runs;
mod command_templates;
mod content_search;
#[cfg(test)]
mod content_search_tests;
mod evaluation_fixture;
mod filesystem;
mod git;
mod ignore_matcher;
mod managed_worktree_repository;
mod output_search;
mod path_search;
#[cfg(test)]
mod path_search_tests;
#[cfg(test)]
mod provider_contract_tests;
mod remote_helper;
mod remote_terminal_logging;
mod remote_terminal_schema;
mod retained_remote_shell;
#[cfg(test)]
mod retained_remote_shell_tests;
mod retained_shell_process;
mod retained_shell_runtime;
#[cfg(test)]
mod retained_shell_runtime_tests;
mod retained_shell_support;
mod runtime_support;
mod selection;
#[cfg(test)]
mod session_log_export_tests;
mod session_queries;
mod session_search;
mod session_shell_workspace;
mod shell_lifecycle_diagnostics;
mod sqlite_repository;
mod ssh_shell_transport;
#[cfg(test)]
mod structural_performance_tests;
mod workspace_inspection;
#[cfg(test)]
mod workspace_inspection_tests;
mod workspace_invalidation;
#[cfg(test)]
mod workspace_invalidation_tests;
#[cfg(test)]
mod worktree_cleanup_tests;
mod worktree_git_parsing;
mod worktree_ignored_scan;
mod worktree_probe;

pub(crate) use evaluation_fixture::{
    changed_evaluation_paths, cleanup_evaluation_fixture, prepare_evaluation_fixture,
    PreparedEvaluationFixture,
};
pub(crate) use filesystem::WorkspaceFilesystemAdapter;
pub(crate) use git::WorkspaceGitAdapter;
pub(crate) use managed_worktree_repository::{
    apply_managed_worktree_schema, SqliteManagedWorktreeRepository, SqliteWorkspaceUseGate,
    SystemWorktreeCleanupClock, UuidWorktreeIds,
};
pub(crate) use remote_helper::{
    RemoteWorkspaceInspectionProvider, SshRemoteHelperSession, SshRemoteProfileSource,
};
pub(crate) use remote_terminal_schema::apply_remote_terminal_schema;
pub(crate) use retained_remote_shell::{RetainedRemoteShellRuntime, RoutedShellRuntime};
pub(crate) use retained_shell_runtime::RetainedLocalShellRuntime;
pub(crate) use retained_shell_support::{
    SqliteSessionShellWorkspace, SystemShellClock, TauriSessionShellNotices, UuidShellIds,
};
pub(crate) use runtime_support::SystemWorkspaceClock;
pub(crate) use selection::TauriProjectDirectorySelection;
pub(crate) use session_queries::SessionWorkspaceQueryAdapter;
pub(crate) use session_shell_workspace::SqliteShellWorkspaceAdapter;
pub(crate) use shell_lifecycle_diagnostics::UnifiedLogShellDiagnostics;
pub(crate) use sqlite_repository::SqliteWorkspaceHistoryRepository;
pub(crate) use ssh_shell_transport::SshShellTransport;
pub(crate) use workspace_inspection::{
    LocalWorkspaceInspectionProvider, SessionWorkspaceTargetResolver,
};
pub(crate) use workspace_invalidation::{
    SystemWorkspaceChangeObserver, TauriWorkspaceInvalidationNotices, WorkspaceInvalidationPoller,
};
pub(crate) use worktree_probe::GitWorktreeProbe;
