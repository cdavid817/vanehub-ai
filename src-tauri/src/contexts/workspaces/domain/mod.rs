mod command_run;
mod command_template;
mod error;
mod output_chunk;
mod path;
mod project;
mod remote_terminal_limits;
mod remote_workspace;
mod shell;
mod worktree;

pub(crate) use command_run::{CommandRun, CommandRunStatus};
pub(crate) use command_template::{CommandTemplate, CommandTemplateError, CommandTemplateScope};
pub(crate) use error::WorkspaceDomainError;
pub(crate) use output_chunk::{TerminalOutputChunk, TerminalOutputSource};
#[allow(unused_imports)]
pub(crate) use path::{
    normalize_windows_extended_length_path, CanonicalPathBoundary, WorkspaceRelativePath,
};
pub(crate) use project::{ensure_git_worktree_available, ProjectInspection, ProjectPath};
#[allow(unused_imports)]
pub(crate) use remote_terminal_limits::{
    REMOTE_TERMINAL_CONNECT_TIMEOUT_SECONDS, REMOTE_TERMINAL_DRAIN_TIMEOUT_SECONDS,
    REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS, REMOTE_TERMINAL_KEEPALIVE_SECONDS,
    REMOTE_TERMINAL_POOL_CAPACITY, REMOTE_TERMINAL_TRANSCRIPT_BYTES, TERMINAL_CAPTURE_BATCH_CHUNKS,
    TERMINAL_CAPTURE_CAPACITY_BYTES, TERMINAL_CAPTURE_CHUNK_BYTES, TERMINAL_CAPTURE_QUEUE_CHUNKS,
    TERMINAL_CAPTURE_RETENTION_DAYS, TERMINAL_SEARCH_DEFAULT_PAGE_SIZE,
    TERMINAL_SEARCH_MAX_CURSOR_BYTES, TERMINAL_SEARCH_MAX_PAGE_SIZE,
    TERMINAL_SEARCH_MAX_QUERY_BYTES,
};
pub(crate) use remote_workspace::RemoteWorkspace;
pub(crate) use shell::{reset_directory_command, ShellHost, TerminalDimensions};
pub(crate) use worktree::{ensure_worktree_compatible, GitReference, WorktreeName};
