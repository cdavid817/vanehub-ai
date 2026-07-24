use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandRunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandRun {
    pub(crate) id: String,
    pub(crate) template_id: Option<String>,
    pub(crate) session_id: String,
    pub(crate) connection_id: Option<String>,
    pub(crate) command_snapshot: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) status: CommandRunStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum CommandRunError {
    #[error("command run requires a non-empty immutable command snapshot")]
    InvalidCommand,
    #[error("command run session is required")]
    InvalidSession,
    #[error("command run cannot transition from its current status")]
    InvalidTransition,
}

impl CommandRun {
    pub(crate) fn validate(&self) -> Result<(), CommandRunError> {
        if self.command_snapshot.trim().is_empty() || self.command_snapshot.len() > 16_384 {
            return Err(CommandRunError::InvalidCommand);
        }
        if self.session_id.trim().is_empty() {
            return Err(CommandRunError::InvalidSession);
        }
        Ok(())
    }

    pub(crate) fn finish(
        &mut self,
        status: CommandRunStatus,
        exit_code: Option<i32>,
        finished_at: String,
    ) -> Result<(), CommandRunError> {
        if !matches!(
            self.status,
            CommandRunStatus::Queued | CommandRunStatus::Running
        ) {
            return Err(CommandRunError::InvalidTransition);
        }
        self.status = status;
        self.exit_code = exit_code;
        self.finished_at = Some(finished_at);
        Ok(())
    }
}
