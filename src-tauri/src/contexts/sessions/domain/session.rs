use super::{ArchivedSessionAction, CategoryId, SessionId, SessionsDomainError};

const DEFAULT_SESSION_TITLE: &str = "新会话";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionTitle(String);

impl SessionTitle {
    pub(crate) fn for_creation(value: Option<&str>) -> Self {
        let value = value.map(str::trim).filter(|value| !value.is_empty());
        Self(value.unwrap_or(DEFAULT_SESSION_TITLE).to_string())
    }

    pub(crate) fn for_rename(value: impl Into<String>) -> Result<Self, SessionsDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(SessionsDomainError::SessionTitleRequired)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionLifecycle {
    Idle,
    Starting,
    Running,
    Failed,
    Stopped,
}

impl SessionLifecycle {
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

    pub(crate) fn has_active_generation(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionActivation {
    Activate,
    PreserveActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionOwner {
    Desktop,
    Connector { connector_id: String },
}

impl SessionOwner {
    pub(crate) fn desktop() -> Self {
        Self::Desktop
    }

    pub(crate) fn connector(connector_id: impl Into<String>) -> Result<Self, SessionsDomainError> {
        let connector_id = connector_id.into();
        if connector_id.trim().is_empty() {
            Err(SessionsDomainError::ConnectorRequired)
        } else {
            Ok(Self::Connector { connector_id })
        }
    }

    pub(crate) fn from_parts(
        kind: &str,
        connector_id: Option<&str>,
    ) -> Result<Self, SessionsDomainError> {
        if kind == "im" {
            Self::connector(connector_id.unwrap_or_default())
        } else {
            Ok(Self::desktop())
        }
    }

    pub(crate) fn validate_activation(
        &self,
        activation: SessionActivation,
    ) -> Result<SessionActivation, SessionsDomainError> {
        if matches!(self, Self::Connector { .. }) && activation == SessionActivation::Activate {
            Err(SessionsDomainError::ConnectorCannotActivate)
        } else {
            Ok(activation)
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Connector { .. } => "im",
        }
    }

    pub(crate) fn connector_id(&self) -> Option<&str> {
        match self {
            Self::Desktop => None,
            Self::Connector { connector_id } => Some(connector_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionAggregate {
    id: SessionId,
    title: SessionTitle,
    lifecycle: SessionLifecycle,
    owner: SessionOwner,
    category_id: Option<CategoryId>,
    pinned: bool,
    archived: bool,
}

impl SessionAggregate {
    pub(crate) fn create(id: SessionId, title: SessionTitle, owner: SessionOwner) -> Self {
        Self::rehydrate(id, title, SessionLifecycle::Idle, owner, None, false, false)
    }

    pub(crate) fn rehydrate(
        id: SessionId,
        title: SessionTitle,
        lifecycle: SessionLifecycle,
        owner: SessionOwner,
        category_id: Option<CategoryId>,
        pinned: bool,
        archived: bool,
    ) -> Self {
        Self {
            id,
            title,
            lifecycle,
            owner,
            category_id,
            pinned,
            archived,
        }
    }

    pub(crate) fn rename(&mut self, title: SessionTitle) {
        self.title = title;
    }

    pub(crate) fn activation(
        &self,
        requested: SessionActivation,
    ) -> Result<SessionActivation, SessionsDomainError> {
        self.owner.validate_activation(requested)?;
        if self.archived && requested == SessionActivation::Activate {
            return Err(SessionsDomainError::ArchivedSession {
                session_id: self.id.as_str().to_string(),
                action: ArchivedSessionAction::Activate,
            });
        }
        Ok(requested)
    }

    pub(crate) fn ensure_accepts_messages(&self) -> Result<(), SessionsDomainError> {
        if self.archived {
            Err(SessionsDomainError::ArchivedSession {
                session_id: self.id.as_str().to_string(),
                action: ArchivedSessionAction::SendMessage,
            })
        } else {
            Ok(())
        }
    }

    pub(crate) fn transition_to(
        &mut self,
        next: SessionLifecycle,
    ) -> Result<(), SessionsDomainError> {
        if self.archived && next.has_active_generation() {
            return Err(SessionsDomainError::ArchivedSession {
                session_id: self.id.as_str().to_string(),
                action: ArchivedSessionAction::StartGeneration,
            });
        }
        self.lifecycle = next;
        Ok(())
    }

    pub(crate) fn assign_category(&mut self, category_id: Option<CategoryId>) {
        self.category_id = category_id;
    }

    pub(crate) fn set_pinned(&mut self, pinned: bool) {
        self.pinned = pinned;
    }

    pub(crate) fn archive(&mut self) {
        self.archived = true;
    }

    pub(crate) fn unarchive(&mut self) {
        self.archived = false;
    }

    pub(crate) fn can_archive_automatically(&self) -> bool {
        !self.archived && !self.pinned && !self.lifecycle.has_active_generation()
    }

    pub(crate) fn id(&self) -> &SessionId {
        &self.id
    }

    pub(crate) fn title(&self) -> &SessionTitle {
        &self.title
    }

    pub(crate) fn lifecycle(&self) -> SessionLifecycle {
        self.lifecycle
    }

    pub(crate) fn owner(&self) -> &SessionOwner {
        &self.owner
    }

    pub(crate) fn category_id(&self) -> Option<&CategoryId> {
        self.category_id.as_ref()
    }

    pub(crate) fn is_pinned(&self) -> bool {
        self.pinned
    }

    pub(crate) fn is_archived(&self) -> bool {
        self.archived
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(owner: SessionOwner, archived: bool) -> SessionAggregate {
        SessionAggregate::rehydrate(
            SessionId::parse("session-1").expect("session id"),
            SessionTitle::for_creation(Some("Work")),
            SessionLifecycle::Idle,
            owner,
            None,
            false,
            archived,
        )
    }

    #[test]
    fn titles_preserve_creation_default_and_reject_empty_renames() {
        assert_eq!(
            SessionTitle::for_creation(None).as_str(),
            DEFAULT_SESSION_TITLE
        );
        assert_eq!(
            SessionTitle::for_creation(Some("  Named  ")).as_str(),
            "Named"
        );
        assert_eq!(
            SessionTitle::for_rename("  "),
            Err(SessionsDomainError::SessionTitleRequired)
        );
    }

    #[test]
    fn connector_ownership_preserves_the_active_desktop_session() {
        let owner = SessionOwner::connector("telegram").expect("connector owner");
        assert_eq!(owner.kind(), "im");
        assert_eq!(owner.connector_id(), Some("telegram"));
        assert_eq!(
            owner.validate_activation(SessionActivation::Activate),
            Err(SessionsDomainError::ConnectorCannotActivate)
        );
        assert_eq!(
            owner.validate_activation(SessionActivation::PreserveActive),
            Ok(SessionActivation::PreserveActive)
        );
    }

    #[test]
    fn aggregate_controls_activation_lifecycle_pin_archive_and_category_state() {
        let mut session = session(SessionOwner::desktop(), false);
        session.rename(SessionTitle::for_rename("Renamed").expect("title"));
        session
            .transition_to(SessionLifecycle::Starting)
            .expect("starting");
        session
            .transition_to(SessionLifecycle::Running)
            .expect("running");
        session.set_pinned(true);
        session.assign_category(Some(CategoryId::parse("category-1").expect("category id")));

        assert_eq!(session.id().as_str(), "session-1");
        assert_eq!(session.title().as_str(), "Renamed");
        assert_eq!(session.lifecycle(), SessionLifecycle::Running);
        assert_eq!(session.owner(), &SessionOwner::Desktop);
        assert_eq!(
            session.category_id().map(CategoryId::as_str),
            Some("category-1")
        );
        assert!(session.is_pinned());
        assert!(!session.can_archive_automatically());

        session.set_pinned(false);
        session.transition_to(SessionLifecycle::Idle).expect("idle");
        assert!(session.can_archive_automatically());
        session.archive();
        assert!(session.is_archived());
        assert!(session.ensure_accepts_messages().is_err());
        assert!(session.activation(SessionActivation::Activate).is_err());
        assert!(session.transition_to(SessionLifecycle::Starting).is_err());
        session.unarchive();
        assert!(!session.is_archived());
    }
}
