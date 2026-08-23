use super::{
    CreateShellRequest, NoWorkspaceEvidence, ResizeShellRequest, ShellEvent, ShellLaunch, ShellLog,
    ShellSession, WorkspaceApplicationError, WorkspaceEvidencePort, WorkspaceEvidenceSignal,
    WorkspaceLogLevel, WorkspaceShellCloseReason, WorkspaceShellContextPort,
    WorkspaceShellEventPort, WorkspaceShellIdPort, WorkspaceShellLogPort,
    WorkspaceShellRuntimeKind, WorkspaceShellRuntimePort,
};
use crate::contexts::workspaces::domain::{ShellRuntimeDescriptor, TerminalDimensions};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceShellApplicationService {
    contexts: Arc<dyn WorkspaceShellContextPort>,
    runtime: Arc<dyn WorkspaceShellRuntimePort>,
    ids: Arc<dyn WorkspaceShellIdPort>,
    events: Arc<dyn WorkspaceShellEventPort>,
    logging: Arc<dyn WorkspaceShellLogPort>,
    evidence: Arc<dyn WorkspaceEvidencePort>,
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
            // A build with no bridge assembled still opens shells. Bootstrap swaps this for the
            // real publisher; nothing else may.
            evidence: Arc::new(NoWorkspaceEvidence),
        }
    }

    pub(crate) fn with_evidence(mut self, evidence: Arc<dyn WorkspaceEvidencePort>) -> Self {
        self.evidence = evidence;
        self
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
                seat_id: request.seat_id.clone(),
                message: "Verifier shell creation denied by read-only policy.".to_string(),
            });
            return Err(WorkspaceApplicationError::PolicyDenied {
                session_id: request.session_id.clone(),
                action: "create-shell".to_string(),
            });
        }
        let root = workspace
            .root
            .or_else(|| {
                workspace
                    .remote_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.path.clone())
            })
            .ok_or_else(|| {
                WorkspaceApplicationError::Validation(
                    "Session workspace is unavailable.".to_string(),
                )
            })?;
        // Describe the runtime before opening it. A remote workspace whose SSH binding is missing
        // cannot be named honestly — and the local PTY path would otherwise open a shell at a
        // remote path and label it `remote`.
        let runtime = if workspace.remote {
            let binding = workspace.ssh_binding.as_ref().ok_or_else(|| {
                WorkspaceApplicationError::Validation(
                    "Remote session workspace has no current SSH binding.".to_string(),
                )
            })?;
            ShellRuntimeDescriptor::Remote {
                connection_id: binding.connection_id.clone(),
                profile_revision: binding.revision,
                supports_reconnect: false,
            }
        } else {
            ShellRuntimeDescriptor::Native
        };
        let shell_id = self.ids.next_shell_id();
        self.runtime.open_shell(&ShellLaunch {
            shell_id: shell_id.clone(),
            session_id: request.session_id.clone(),
            root,
            dimensions: TerminalDimensions::bounded(request.rows, request.cols),
            remote_endpoint: workspace.remote_endpoint.clone(),
            ssh_binding: workspace.ssh_binding.clone(),
        })?;
        self.logging.write(ShellLog {
            level: WorkspaceLogLevel::Info,
            session_id: request.session_id.clone(),
            shell_id: shell_id.clone(),
            seat_id: request.seat_id.clone(),
            message: if workspace.remote {
                format!("Remote shell connected for agent {}.", workspace.agent_id)
            } else {
                format!("Shell connected for agent {}.", workspace.agent_id)
            },
        });
        // After the shell is open and logged, never before: this reports what happened, and a
        // report that could change the outcome would be a precondition rather than an observation.
        // `try_publish` cannot fail, so there is nothing here to handle.
        self.evidence
            .try_publish(WorkspaceEvidenceSignal::ShellOpened {
                session_id: request.session_id.clone(),
                shell_id: shell_id.clone(),
                seat_id: request.seat_id.clone(),
                runtime: if workspace.remote {
                    WorkspaceShellRuntimeKind::Remote
                } else {
                    WorkspaceShellRuntimeKind::Local
                },
                occurred_at: chrono::Utc::now().to_rfc3339(),
            });
        Ok(ShellSession {
            shell_id,
            session_id: request.session_id.clone(),
            state: "connected",
            runtime,
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
        // `stop` answers `None` for a shell that is already gone, which is what makes a repeated
        // close idempotent: nothing is published for a shell that had already ended.
        let Some(session_id) = self.runtime.stop(shell_id)? else {
            return Ok(());
        };
        self.publish_disconnected(
            shell_id.to_string(),
            session_id,
            WorkspaceShellCloseReason::ExplicitClose,
        );
        Ok(())
    }

    pub(crate) fn kill_for_session(
        &self,
        session_id: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        for (shell_id, owning_session_id) in self.runtime.stop_for_session(session_id)? {
            self.publish_disconnected(
                shell_id,
                owning_session_id,
                WorkspaceShellCloseReason::Shutdown,
            );
        }
        Ok(())
    }

    fn publish_disconnected(
        &self,
        shell_id: String,
        session_id: String,
        reason: WorkspaceShellCloseReason,
    ) {
        self.logging.write(ShellLog {
            level: WorkspaceLogLevel::Info,
            session_id: session_id.clone(),
            shell_id: shell_id.clone(),
            seat_id: None,
            message: "Shell disconnected.".to_string(),
        });
        self.evidence
            .try_publish(WorkspaceEvidenceSignal::ShellClosed {
                session_id: session_id.clone(),
                shell_id: shell_id.clone(),
                seat_id: None,
                reason,
                occurred_at: chrono::Utc::now().to_rfc3339(),
            });
        self.events.publish(ShellEvent::State {
            shell_id,
            session_id,
            state: "disconnected",
            error: None,
        });
    }
}
