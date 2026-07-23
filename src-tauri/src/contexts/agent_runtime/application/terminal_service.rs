use super::{
    AgentCliProfileGateway, AgentClockPort, AgentEventPort, AgentLog, AgentLogLevel,
    AgentLoggingPort, AgentRegistryRepository, AgentRuntimeApplicationError, AgentSessionGateway,
    AgentTerminalEvent, AgentTerminalEventPort, AgentTerminalGateway, AgentTerminalInputRequest,
    AgentTerminalProcessRequest, AgentTerminalSession, AgentTerminalState, AgentView,
    OpenAgentTerminalRequest, ResizeAgentTerminalRequest, StopAgentTerminalRequest,
};
use crate::contexts::agent_runtime::domain::{AgentLifecycle, InteractionMode};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AgentTerminalApplicationPorts {
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) sessions: Arc<dyn AgentSessionGateway>,
    pub(crate) cli_profiles: Arc<dyn AgentCliProfileGateway>,
    pub(crate) terminals: Arc<dyn AgentTerminalGateway>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) events: Arc<dyn AgentEventPort>,
    pub(crate) terminal_events: Arc<dyn AgentTerminalEventPort>,
}

#[derive(Clone)]
pub(crate) struct AgentTerminalApplicationService {
    ports: AgentTerminalApplicationPorts,
}

impl AgentTerminalApplicationService {
    pub(crate) fn new(ports: AgentTerminalApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn open_or_attach(
        &self,
        request: OpenAgentTerminalRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        let session = match self.ports.sessions.find_session(&request.session_id) {
            Ok(Some(session)) => session,
            Ok(None) => {
                let error = AgentRuntimeApplicationError::SessionNotFound(request.session_id);
                self.record_terminal_start_failure(&error, None, None);
                return Err(error);
            }
            Err(error) => {
                self.record_terminal_start_failure(&error, None, Some(&request.session_id));
                return Err(error);
            }
        };
        if session.archived {
            let error = AgentRuntimeApplicationError::Validation(
                "Archived sessions cannot start Agent terminals.".to_string(),
            );
            self.record_terminal_start_failure(&error, Some(&session.agent_id), Some(&session.id));
            return Err(error);
        }
        if session.read_only {
            let error = AgentRuntimeApplicationError::PolicyDenied {
                session_id: session.id.clone(),
                action: "open-terminal".to_string(),
            };
            self.record_terminal_start_failure(&error, Some(&session.agent_id), Some(&session.id));
            return Err(error);
        }
        if session.interaction_mode != InteractionMode::Cli {
            let error = AgentRuntimeApplicationError::UnsupportedInteractionMode(
                session.interaction_mode.as_str().to_string(),
            );
            self.record_terminal_start_failure(&error, Some(&session.agent_id), Some(&session.id));
            return Err(error);
        }
        if let Some(terminal) = self.ports.terminals.attach_retained(&session.id)? {
            let _ = self
                .ports
                .terminal_events
                .publish_terminal(AgentTerminalEvent::State {
                    terminal_id: terminal.terminal_id.clone(),
                    session_id: session.id.clone(),
                    state: terminal.state,
                    error: None,
                });
            self.publish_running_workflow(&session);
            self.record_log(
                AgentLogLevel::Info,
                "agent.terminal",
                "Agent terminal attached".to_string(),
                Some(&session.agent_id),
                Some(&session.id),
            );
            return Ok(terminal);
        }
        let agent = match self.ports.registry.find(&session.agent_id) {
            Ok(Some(agent)) => agent,
            Ok(None) => {
                let error = AgentRuntimeApplicationError::AgentNotFound(session.agent_id.clone());
                self.record_terminal_start_failure(
                    &error,
                    Some(&session.agent_id),
                    Some(&session.id),
                );
                return Err(error);
            }
            Err(error) => {
                self.record_terminal_start_failure(
                    &error,
                    Some(&session.agent_id),
                    Some(&session.id),
                );
                return Err(error);
            }
        };
        if !agent.supports(InteractionMode::Cli) {
            let error = AgentRuntimeApplicationError::UnsupportedInteractionMode(
                InteractionMode::Cli.as_str().to_string(),
            );
            self.record_terminal_start_failure(&error, Some(&session.agent_id), Some(&session.id));
            return Err(error);
        }
        let profile = match self
            .ports
            .cli_profiles
            .load_interactive(agent.id().as_str())
        {
            Ok(profile) => profile,
            Err(error) => {
                self.record_terminal_start_failure(
                    &error,
                    Some(&session.agent_id),
                    Some(&session.id),
                );
                return Err(error);
            }
        };
        if let Err(error) = self
            .ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Starting)
        {
            self.record_terminal_start_failure(&error, Some(&session.agent_id), Some(&session.id));
            return Err(error);
        }
        let terminal = match self
            .ports
            .terminals
            .open_or_attach(AgentTerminalProcessRequest {
                session: session.clone(),
                agent: AgentView::from(&agent),
                cli_profile: profile,
                size: request.size,
            }) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = self
                    .ports
                    .sessions
                    .update_lifecycle(&session.id, AgentLifecycle::Failed);
                self.record_terminal_start_failure(
                    &error,
                    Some(&session.agent_id),
                    Some(&session.id),
                );
                return Err(error);
            }
        };
        if let Some(runtime_session_id) = terminal
            .runtime_session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            self.ports
                .sessions
                .update_runtime_session_id(&session.id, runtime_session_id)?;
        }
        self.ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Running)?;
        let _ = self
            .ports
            .terminal_events
            .publish_terminal(AgentTerminalEvent::State {
                terminal_id: terminal.terminal_id.clone(),
                session_id: session.id.clone(),
                state: terminal.state,
                error: None,
            });
        let _ = self.ports.events.publish(self.running_workflow(&session));
        self.record_log(
            AgentLogLevel::Info,
            "agent.terminal",
            "Agent terminal opened or attached".to_string(),
            Some(&session.agent_id),
            Some(&session.id),
        );
        Ok(terminal)
    }

    pub(crate) fn input(
        &self,
        request: AgentTerminalInputRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports.terminals.input(request)
    }

    pub(crate) fn resize(
        &self,
        request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports.terminals.resize(request)
    }

    pub(crate) fn stop(
        &self,
        request: StopAgentTerminalRequest,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        self.ports.terminals.stop(request)
    }

    pub(crate) fn cleanup_idle(
        &self,
        idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let stopped = self.ports.terminals.cleanup_idle(idle_after_seconds)?;
        for session_id in &stopped {
            self.record_log(
                AgentLogLevel::Info,
                "agent.terminal",
                "Agent terminal stopped after inactivity".to_string(),
                None,
                Some(session_id),
            );
        }
        Ok(stopped)
    }

    pub(crate) fn shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let stopped = self.ports.terminals.shutdown()?;
        for session_id in &stopped {
            self.record_log(
                AgentLogLevel::Info,
                "agent.terminal",
                "Agent terminal stopped during shutdown".to_string(),
                None,
                Some(session_id),
            );
            let _ = self
                .ports
                .terminal_events
                .publish_terminal(AgentTerminalEvent::State {
                    terminal_id: String::new(),
                    session_id: session_id.clone(),
                    state: AgentTerminalState::Stopped,
                    error: None,
                });
        }
        Ok(stopped)
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        category: &str,
        message: String,
        agent_id: Option<&str>,
        session_id: Option<&str>,
    ) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            operation_id: None,
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.ports.clock.now(),
        });
    }

    fn publish_running_workflow(&self, session: &super::AgentSession) {
        let _ = self.ports.events.publish(self.running_workflow(session));
    }

    fn running_workflow(&self, session: &super::AgentSession) -> super::AgentEvent {
        super::AgentEvent::WorkflowChanged(super::WorkflowView {
            active_agent_id: Some(session.agent_id.clone()),
            active_interaction_mode: Some(InteractionMode::Cli),
            lifecycle: AgentLifecycle::Running,
            intent: "agent-terminal".to_string(),
        })
    }

    fn record_terminal_start_failure(
        &self,
        error: &AgentRuntimeApplicationError,
        agent_id: Option<&str>,
        session_id: Option<&str>,
    ) {
        self.record_log(
            AgentLogLevel::Error,
            "session.agent_terminal",
            format!("Agent terminal startup failed before process launch: {error}"),
            agent_id,
            session_id,
        );
    }
}
