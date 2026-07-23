use super::{
    CreateShellRequest, ResizeShellRequest, ShellEvent, ShellLaunch, ShellLog, ShellSession,
    WorkspaceApplicationError, WorkspaceLogLevel, WorkspaceShellContextPort,
    WorkspaceShellEventPort, WorkspaceShellIdPort, WorkspaceShellLogPort,
    WorkspaceShellRuntimePort,
};
use crate::contexts::workspaces::domain::TerminalDimensions;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceShellApplicationService {
    contexts: Arc<dyn WorkspaceShellContextPort>,
    runtime: Arc<dyn WorkspaceShellRuntimePort>,
    ids: Arc<dyn WorkspaceShellIdPort>,
    events: Arc<dyn WorkspaceShellEventPort>,
    logging: Arc<dyn WorkspaceShellLogPort>,
}

impl WorkspaceShellApplicationService {
    pub(crate) fn new(
        contexts: Arc<dyn WorkspaceShellContextPort>,
        runtime: Arc<dyn WorkspaceShellRuntimePort>,
        ids: Arc<dyn WorkspaceShellIdPort>,
        events: Arc<dyn WorkspaceShellEventPort>,
        logging: Arc<dyn WorkspaceShellLogPort>,
    ) -> Self {
        Self {
            contexts,
            runtime,
            ids,
            events,
            logging,
        }
    }

    pub(crate) fn create_shell(
        &self,
        request: &CreateShellRequest,
    ) -> Result<ShellSession, WorkspaceApplicationError> {
        let workspace = self.contexts.load_shell_workspace(&request.session_id)?;
        if workspace.read_only {
            self.logging.write(ShellLog {
                level: WorkspaceLogLevel::Warn,
                session_id: request.session_id.clone(),
                shell_id: "policy".to_string(),
                message: "Verifier shell creation denied by read-only policy.".to_string(),
            });
            return Err(WorkspaceApplicationError::PolicyDenied {
                session_id: request.session_id.clone(),
                action: "create-shell".to_string(),
            });
        }
        if workspace.remote {
            return Err(WorkspaceApplicationError::Validation(
                "Remote workspace shell is unsupported.".to_string(),
            ));
        }
        let root = workspace.root.ok_or_else(|| {
            WorkspaceApplicationError::Validation("Session workspace is unavailable.".to_string())
        })?;
        let shell_id = self.ids.next_shell_id();
        self.runtime.open_shell(&ShellLaunch {
            shell_id: shell_id.clone(),
            session_id: request.session_id.clone(),
            root,
            dimensions: TerminalDimensions::bounded(request.rows, request.cols),
        })?;
        self.logging.write(ShellLog {
            level: WorkspaceLogLevel::Info,
            session_id: request.session_id.clone(),
            shell_id: shell_id.clone(),
            message: format!("Shell connected for agent {}.", workspace.agent_id),
        });
        Ok(ShellSession {
            shell_id,
            session_id: request.session_id.clone(),
            state: "connected",
            capability: "native",
        })
    }

    pub(crate) fn write_input(
        &self,
        shell_id: &str,
        content: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.runtime.write_input(shell_id, content)
    }

    pub(crate) fn reset_directory(&self, shell_id: &str) -> Result<(), WorkspaceApplicationError> {
        self.runtime.reset_directory(shell_id)
    }

    pub(crate) fn resize_shell(
        &self,
        request: &ResizeShellRequest,
    ) -> Result<(), WorkspaceApplicationError> {
        self.runtime.resize(
            &request.shell_id,
            TerminalDimensions::bounded(request.rows, request.cols),
        )
    }

    pub(crate) fn kill_shell(&self, shell_id: &str) -> Result<(), WorkspaceApplicationError> {
        let Some(session_id) = self.runtime.stop(shell_id)? else {
            return Ok(());
        };
        self.publish_disconnected(shell_id.to_string(), session_id);
        Ok(())
    }

    pub(crate) fn kill_for_session(
        &self,
        session_id: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        for (shell_id, owning_session_id) in self.runtime.stop_for_session(session_id)? {
            self.publish_disconnected(shell_id, owning_session_id);
        }
        Ok(())
    }

    fn publish_disconnected(&self, shell_id: String, session_id: String) {
        self.logging.write(ShellLog {
            level: WorkspaceLogLevel::Info,
            session_id: session_id.clone(),
            shell_id: shell_id.clone(),
            message: "Shell disconnected.".to_string(),
        });
        self.events.publish(ShellEvent::State {
            shell_id,
            session_id,
            state: "disconnected",
            error: None,
        });
    }
}
