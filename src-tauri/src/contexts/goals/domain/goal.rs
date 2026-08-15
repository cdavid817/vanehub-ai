use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoalDomainError {
    MissingTitle,
    InvalidStatus(String),
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    AcceptanceNotReady,
}

impl std::fmt::Display for GoalDomainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTitle => write!(formatter, "Goal title is required."),
            Self::InvalidStatus(value) => write!(formatter, "Unknown goal status \"{value}\"."),
            Self::InvalidTransition { from, to } => {
                write!(formatter, "A goal cannot move from \"{from}\" to \"{to}\".")
            }
            Self::AcceptanceNotReady => write!(
                formatter,
                "A goal can only be accepted while it is awaiting acceptance."
            ),
        }
    }
}

/// The statuses a goal actually stores.
///
/// `AwaitingAcceptance` is deliberately absent: progress is pulled on read, so a
/// persisted awaiting-acceptance row would survive a child reopening and strand
/// the goal in a state no user action can correct. See `application::progress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GoalStatus {
    Draft,
    Active,
    Achieved,
    Abandoned,
}

impl GoalStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Achieved => "achieved",
            Self::Abandoned => "abandoned",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, GoalDomainError> {
        match value {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "achieved" => Ok(Self::Achieved),
            "abandoned" => Ok(Self::Abandoned),
            other => Err(GoalDomainError::InvalidStatus(other.to_string())),
        }
    }

    pub(crate) fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Active)
                | (Self::Active, Self::Achieved)
                | (Self::Achieved, Self::Active)
                | (Self::Abandoned, Self::Active)
                | (Self::Draft | Self::Active | Self::Achieved, Self::Abandoned)
        )
    }

    pub(crate) fn transition(self, next: Self) -> Result<Self, GoalDomainError> {
        self.can_transition_to(next)
            .then_some(next)
            .ok_or(GoalDomainError::InvalidTransition {
                from: self.as_str(),
                to: next.as_str(),
            })
    }

    /// Acceptance needs the derived readiness the caller computed from the
    /// goal's children; the aggregate cannot see them itself.
    pub(crate) fn accept(self, awaiting_acceptance: bool) -> Result<Self, GoalDomainError> {
        if !awaiting_acceptance {
            return Err(GoalDomainError::AcceptanceNotReady);
        }
        self.transition(Self::Achieved)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalInput {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) acceptance_notes: String,
    pub(crate) project_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Goal {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) acceptance_notes: String,
    pub(crate) status: GoalStatus,
    pub(crate) project_path: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl Goal {
    pub(crate) fn new(input: GoalInput, now: &str) -> Result<Self, GoalDomainError> {
        let normalized = normalize(input)?;
        Ok(Self {
            id: normalized.id,
            title: normalized.title,
            description: normalized.description,
            acceptance_notes: normalized.acceptance_notes,
            status: GoalStatus::Draft,
            project_path: normalized.project_path,
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
    }

    pub(crate) fn apply_edits(
        &mut self,
        input: GoalInput,
        now: &str,
    ) -> Result<(), GoalDomainError> {
        let normalized = normalize(input)?;
        self.title = normalized.title;
        self.description = normalized.description;
        self.acceptance_notes = normalized.acceptance_notes;
        self.project_path = normalized.project_path;
        self.updated_at = now.to_string();
        Ok(())
    }

    pub(crate) fn move_to(&mut self, next: GoalStatus, now: &str) -> Result<(), GoalDomainError> {
        self.status = self.status.transition(next)?;
        self.updated_at = now.to_string();
        Ok(())
    }

    pub(crate) fn accept(
        &mut self,
        awaiting_acceptance: bool,
        now: &str,
    ) -> Result<(), GoalDomainError> {
        self.status = self.status.accept(awaiting_acceptance)?;
        self.updated_at = now.to_string();
        Ok(())
    }
}

fn normalize(input: GoalInput) -> Result<GoalInput, GoalDomainError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(GoalDomainError::MissingTitle);
    }
    Ok(GoalInput {
        id: input.id.trim().to_string(),
        title: title.to_string(),
        description: input.description.trim().to_string(),
        acceptance_notes: input.acceptance_notes.trim().to_string(),
        project_path: input
            .project_path
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}
