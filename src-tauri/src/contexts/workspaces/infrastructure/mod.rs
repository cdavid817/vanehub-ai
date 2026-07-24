mod capture_maintenance;
mod capture_queue;
mod command_runs;
mod command_templates;
mod filesystem;
mod git;
mod output_search;
mod portable_pty;
mod remote_terminal_logging;
mod remote_terminal_schema;
mod runtime_support;
mod selection;
mod session_queries;
mod session_shell_workspace;
mod shell_support;
mod sqlite_repository;

pub(crate) use capture_maintenance::TerminalCaptureMaintenance;
pub(crate) use capture_queue::BoundedCaptureQueue;
pub(crate) use command_runs::SqliteCommandRunRepository;
pub(crate) use command_templates::SqliteCommandTemplateRepository;
pub(crate) use filesystem::WorkspaceFilesystemAdapter;
pub(crate) use git::WorkspaceGitAdapter;
pub(crate) use output_search::{SqliteTerminalOutputSearch, TerminalSearchHit};
pub(crate) use portable_pty::PortablePtyShellRuntime;
pub(crate) use remote_terminal_logging::{
    log_remote_terminal_event, redact_remote_terminal_message,
};
pub(crate) use remote_terminal_schema::apply_remote_terminal_schema;
pub(crate) use runtime_support::SystemWorkspaceClock;
pub(crate) use selection::TauriProjectDirectorySelection;
pub(crate) use session_queries::SessionWorkspaceQueryAdapter;
pub(crate) use session_shell_workspace::SqliteShellWorkspaceAdapter;
pub(crate) use shell_support::{
    TauriWorkspaceShellEventPublisher, UuidWorkspaceShellId, WorkspaceShellLoggingAdapter,
};
pub(crate) use sqlite_repository::SqliteWorkspaceHistoryRepository;
