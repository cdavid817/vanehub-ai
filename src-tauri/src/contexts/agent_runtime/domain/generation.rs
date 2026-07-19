use super::AgentRuntimeDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationState {
    Reserved,
    Active,
    Completed,
    Failed,
    Cancelled,
}

impl GenerationState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationCancellation {
    pub(crate) message_id: Option<String>,
    pub(crate) process_attached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationAttempt {
    session_id: String,
    message_id: Option<String>,
    process_attached: bool,
    state: GenerationState,
}

impl GenerationAttempt {
    pub(crate) fn reserve(session_id: impl Into<String>) -> Result<Self, AgentRuntimeDomainError> {
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(AgentRuntimeDomainError::RequiredValue("Session id"));
        }
        if session_id.chars().any(char::is_control) {
            return Err(AgentRuntimeDomainError::ControlCharacters("Session id"));
        }
        Ok(Self {
            session_id,
            message_id: None,
            process_attached: false,
            state: GenerationState::Reserved,
        })
    }

    pub(crate) fn attach(
        &mut self,
        message_id: impl Into<String>,
        process_attached: bool,
    ) -> Result<(), AgentRuntimeDomainError> {
        self.require_state(GenerationState::Reserved, GenerationState::Active)?;
        let message_id = message_id.into();
        if message_id.trim().is_empty() {
            return Err(AgentRuntimeDomainError::GenerationMessageRequired);
        }
        self.message_id = Some(message_id);
        self.process_attached = process_attached;
        self.state = GenerationState::Active;
        Ok(())
    }

    pub(crate) fn complete(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.finish(GenerationState::Completed)
    }

    pub(crate) fn fail(&mut self) -> Result<(), AgentRuntimeDomainError> {
        self.finish(GenerationState::Failed)
    }

    pub(crate) fn cancel(&mut self) -> Result<GenerationCancellation, AgentRuntimeDomainError> {
        if self.state.is_terminal() {
            return Err(AgentRuntimeDomainError::InvalidGenerationTransition {
                from: self.state.as_str().to_string(),
                to: GenerationState::Cancelled.as_str().to_string(),
            });
        }
        self.state = GenerationState::Cancelled;
        Ok(GenerationCancellation {
            message_id: self.message_id.clone(),
            process_attached: self.process_attached,
        })
    }

    fn finish(&mut self, next: GenerationState) -> Result<(), AgentRuntimeDomainError> {
        self.require_state(GenerationState::Active, next)?;
        self.state = next;
        Ok(())
    }

    fn require_state(
        &self,
        required: GenerationState,
        next: GenerationState,
    ) -> Result<(), AgentRuntimeDomainError> {
        if self.state == required {
            Ok(())
        } else {
            Err(AgentRuntimeDomainError::InvalidGenerationTransition {
                from: self.state.as_str().to_string(),
                to: next.as_str().to_string(),
            })
        }
    }

    #[cfg(test)]
    pub(crate) fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> GenerationState {
        self.state
    }
}
