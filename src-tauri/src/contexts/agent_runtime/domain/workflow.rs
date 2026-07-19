use super::{AgentDefinition, AgentId, AgentRuntimeDomainError, InteractionMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLifecycle {
    Idle,
    Starting,
    Running,
    Failed,
    Stopped,
}

impl AgentLifecycle {
    pub(crate) fn from_storage_lossy(value: &str) -> Self {
        match value {
            "starting" => Self::Starting,
            "running" => Self::Running,
            "failed" => Self::Failed,
            "stopped" => Self::Stopped,
            _ => Self::Idle,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReadiness {
    ready: bool,
    reason: Option<String>,
    requires_authentication: bool,
}

impl AgentReadiness {
    pub(crate) fn for_browser(agent: &AgentDefinition) -> Self {
        let ready = agent.supports(InteractionMode::Browser);
        Self {
            ready,
            reason: (!ready).then(|| {
                format!(
                    "{} does not support browser interaction mode.",
                    agent.display_name()
                )
            }),
            requires_authentication: ready,
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    pub(crate) fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub(crate) fn requires_authentication(&self) -> bool {
        self.requires_authentication
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentWorkflow {
    active_agent_id: Option<AgentId>,
    active_interaction_mode: Option<InteractionMode>,
    lifecycle: AgentLifecycle,
    intent: String,
}

impl AgentWorkflow {
    #[cfg(test)]
    pub(crate) fn new(intent: impl Into<String>) -> Self {
        Self {
            active_agent_id: None,
            active_interaction_mode: None,
            lifecycle: AgentLifecycle::Idle,
            intent: intent.into(),
        }
    }

    pub(crate) fn rehydrate(
        active_agent_id: Option<String>,
        active_interaction_mode: Option<InteractionMode>,
        lifecycle: AgentLifecycle,
        intent: String,
    ) -> Result<Self, AgentRuntimeDomainError> {
        if active_agent_id.is_some() != active_interaction_mode.is_some() {
            return Err(AgentRuntimeDomainError::IncompleteWorkflowSelection);
        }
        Ok(Self {
            active_agent_id: active_agent_id.map(AgentId::parse).transpose()?,
            active_interaction_mode,
            lifecycle,
            intent,
        })
    }

    pub(crate) fn select(
        &mut self,
        agent: &AgentDefinition,
        mode: InteractionMode,
    ) -> Result<(), AgentRuntimeDomainError> {
        agent.ensure_selectable(mode)?;
        self.active_agent_id = Some(agent.id().clone());
        self.active_interaction_mode = Some(mode);
        self.lifecycle = AgentLifecycle::Idle;
        Ok(())
    }

    pub(crate) fn begin_launch(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.ensure_selected()?;
        self.transition_to(AgentLifecycle::Starting)
    }

    pub(crate) fn mark_running(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.transition_to(AgentLifecycle::Running)
    }

    pub(crate) fn mark_failed(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.transition_to(AgentLifecycle::Failed)
    }

    #[cfg(test)]
    pub(crate) fn mark_stopped(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.transition_to(AgentLifecycle::Stopped)
    }

    fn ensure_selected(&self) -> Result<(), AgentRuntimeDomainError> {
        if self.active_agent_id.is_some() && self.active_interaction_mode.is_some() {
            Ok(())
        } else {
            Err(AgentRuntimeDomainError::NoActiveAgent)
        }
    }

    fn transition_to(&mut self, next: AgentLifecycle) -> Result<(), AgentRuntimeDomainError> {
        let allowed = match (self.lifecycle, next) {
            (current, target) if current == target => true,
            (_, AgentLifecycle::Idle) => true,
            (_, AgentLifecycle::Starting) => true,
            (AgentLifecycle::Starting, AgentLifecycle::Running) => true,
            (AgentLifecycle::Starting | AgentLifecycle::Running, AgentLifecycle::Failed) => true,
            (AgentLifecycle::Starting | AgentLifecycle::Running, AgentLifecycle::Stopped) => true,
            _ => false,
        };
        if !allowed {
            return Err(AgentRuntimeDomainError::InvalidLifecycleTransition {
                from: self.lifecycle.as_str().to_string(),
                to: next.as_str().to_string(),
            });
        }
        self.lifecycle = next;
        Ok(())
    }

    pub(crate) fn active_agent_id(&self) -> Option<&AgentId> {
        self.active_agent_id.as_ref()
    }

    pub(crate) fn active_interaction_mode(&self) -> Option<InteractionMode> {
        self.active_interaction_mode
    }

    pub(crate) fn lifecycle(&self) -> AgentLifecycle {
        self.lifecycle
    }

    pub(crate) fn intent(&self) -> &str {
        &self.intent
    }
}
