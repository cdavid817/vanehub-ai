use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::{IdentityRejection, PersonalizationDomainError};
use super::scope::{AgentId, SessionId, WorkspaceKey};
use super::snapshot::{EffectiveMemoryAccess, PersonalizationExclusionReason};

pub(crate) const MEMORY_NAME_MAX_CHARS: usize = 120;
pub(crate) const MEMORY_DESCRIPTION_MAX_CHARS: usize = 500;
pub(crate) const MEMORY_CONTENT_MAX_CHARS: usize = 32_000;
pub(crate) const MEMORY_AUDIENCE_MAX_AGENTS: usize = 100;

/// Long enough that a generated UUID or ULID fits, short enough that nothing hand-written and
/// path-shaped can. Both bounds exist to keep this from becoming a place user text can arrive.
const MEMORY_ID_MIN_CHARS: usize = 8;
const MEMORY_ID_MAX_CHARS: usize = 64;

/// An immutable memory identity, and the only thing a filename is derived from.
///
/// The charset is deliberately narrower than the other identities: `.` is excluded so `..` can
/// never be constructed, and case-folding collisions on Windows and macOS are avoided because a
/// generated id never differs from another only by case.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct MemoryId(String);

impl MemoryId {
    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        Self::validate(value)
            .map(|value| Self(value.to_string()))
            .map_err(PersonalizationDomainError::InvalidMemoryId)
    }

    fn validate(value: &str) -> Result<&str, IdentityRejection> {
        if value.is_empty() {
            return Err(IdentityRejection::Empty);
        }
        let length = value.chars().count();
        if length < MEMORY_ID_MIN_CHARS {
            return Err(IdentityRejection::TooShort {
                limit: MEMORY_ID_MIN_CHARS,
            });
        }
        if length > MEMORY_ID_MAX_CHARS {
            return Err(IdentityRejection::TooLong {
                limit: MEMORY_ID_MAX_CHARS,
            });
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return Err(IdentityRejection::UnsupportedCharacter);
        }
        Ok(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Where a memory may be read, as opposed to where it happened to be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryScope {
    Global,
    Workspace { workspace_key: WorkspaceKey },
}

impl MemoryScope {
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace { .. } => "workspace",
        }
    }

    pub(crate) fn workspace_key(&self) -> Option<&WorkspaceKey> {
        match self {
            Self::Global => None,
            Self::Workspace { workspace_key } => Some(workspace_key),
        }
    }

    pub(crate) fn from_parts(
        kind: &str,
        workspace_key: Option<&WorkspaceKey>,
    ) -> Result<Self, PersonalizationDomainError> {
        match (kind, workspace_key) {
            ("global", None) => Ok(Self::Global),
            ("workspace", Some(workspace_key)) => Ok(Self::Workspace {
                workspace_key: workspace_key.clone(),
            }),
            ("global", Some(_)) => Err(PersonalizationDomainError::InconsistentScopeColumns {
                kind: "global",
                reason: "a global memory must not carry a workspace key",
            }),
            ("workspace", None) => Err(PersonalizationDomainError::InconsistentScopeColumns {
                kind: "workspace",
                reason: "a workspace memory requires a workspace key",
            }),
            (other, _) => Err(PersonalizationDomainError::UnknownMemoryScopeKind(
                other.to_string(),
            )),
        }
    }
}

/// An additional restriction applied *after* scope, never a substitute for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryAudience {
    AllAgents,
    SelectedAgents { agent_ids: Vec<AgentId> },
}

impl MemoryAudience {
    pub(crate) fn admits(&self, agent_id: &AgentId) -> bool {
        match self {
            Self::AllAgents => true,
            Self::SelectedAgents { agent_ids } => agent_ids.contains(agent_id),
        }
    }
}

macro_rules! string_enum {
    ($name:ident, $error:ident, $doc:literal, $(($variant:ident, $text:literal)),+ $(,)?) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name {
            $($variant),+
        }

        impl $name {
            pub(crate) fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(PersonalizationDomainError::$error(other.to_string())),
                }
            }
        }
    };
}

string_enum!(
    MemoryStatus,
    UnknownMemoryStatus,
    "Lifecycle position. Only `Active` ever reaches a prompt.",
    (Candidate, "candidate"),
    (Active, "active"),
    (Archived, "archived"),
);

string_enum!(
    MemorySource,
    UnknownMemorySource,
    "How the record came to exist. Provenance for the user's judgement, never authorization.",
    (ExplicitUser, "explicit_user"),
    (OnePieceAutomatic, "onepiece_automatic"),
    (CliAutomatic, "cli_automatic"),
    (ModelMemoryTool, "model_memory_tool"),
    (LegacyMigration, "legacy_migration"),
    (ExternalFileEdit, "external_file_edit"),
);

string_enum!(
    MemoryType,
    UnknownMemoryType,
    "The closed taxonomy, plus an explicit compatibility value for migrated records whose type\ncould not be established. `Untyped` is a migration outcome, not something a user may choose.",
    (User, "user"),
    (Feedback, "feedback"),
    (Project, "project"),
    (Reference, "reference"),
    (Untyped, "untyped"),
);

string_enum!(
    MemorySensitivity,
    UnknownMemorySensitivity,
    "Whether the user has marked this record as sensitive.",
    (Normal, "normal"),
    (Sensitive, "sensitive"),
);

/// Where the record came from. Kept separate from scope and audience so that changing who may read
/// a memory never rewrites the history of who produced it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MemoryProvenance {
    pub(crate) source_agent_id: Option<AgentId>,
    pub(crate) source_session_id: Option<SessionId>,
    pub(crate) source_message_id: Option<String>,
    pub(crate) source_workspace_key: Option<WorkspaceKey>,
}

/// One governed memory. The Markdown file remains authoritative for this content; SQLite and the
/// retrieval index are projections of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryRecord {
    pub(crate) id: MemoryId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) scope: MemoryScope,
    pub(crate) audience: MemoryAudience,
    pub(crate) status: MemoryStatus,
    pub(crate) source: MemorySource,
    pub(crate) provenance: MemoryProvenance,
    pub(crate) sensitivity: MemorySensitivity,
    pub(crate) revision: u64,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) verified_at: Option<DateTime<Utc>>,
    pub(crate) last_used_at: Option<DateTime<Utc>>,
    pub(crate) use_count: u64,
}

impl MemoryRecord {
    /// The file this record lives in. Derived from the id alone, which is what makes a rename a
    /// metadata edit rather than a move, and what makes two same-named memories independent.
    pub(crate) fn file_name(&self) -> String {
        format!("{}.md", self.id)
    }

    pub(crate) fn validate(&self) -> Result<(), PersonalizationDomainError> {
        validate_bounded("name", &self.name, 1, MEMORY_NAME_MAX_CHARS)?;
        validate_bounded(
            "description",
            &self.description,
            0,
            MEMORY_DESCRIPTION_MAX_CHARS,
        )?;
        validate_bounded("content", &self.content, 1, MEMORY_CONTENT_MAX_CHARS)?;

        if let MemoryAudience::SelectedAgents { agent_ids } = &self.audience {
            if agent_ids.is_empty() {
                return Err(PersonalizationDomainError::EmptyMemoryAudience);
            }
            if agent_ids.len() > MEMORY_AUDIENCE_MAX_AGENTS {
                return Err(PersonalizationDomainError::MemoryAudienceTooLarge {
                    limit: MEMORY_AUDIENCE_MAX_AGENTS,
                    actual: agent_ids.len(),
                });
            }
        }

        if matches!(self.memory_type, MemoryType::Untyped)
            && !matches!(self.source, MemorySource::LegacyMigration)
        {
            return Err(PersonalizationDomainError::UntypedMemoryRequiresLegacySource);
        }
        Ok(())
    }
}

fn validate_bounded(
    field: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), PersonalizationDomainError> {
    let actual = value.chars().count();
    if actual < minimum {
        return Err(PersonalizationDomainError::MemoryFieldEmpty { field });
    }
    if actual > maximum {
        return Err(PersonalizationDomainError::MemoryFieldTooLong {
            field,
            limit: maximum,
            actual,
        });
    }
    Ok(())
}

/// Whether one record may be surfaced to one Agent under one resolved snapshot.
///
/// Runs before token budgeting and relevance selection, so an ineligible record can never consume
/// budget or be offered to a selector that might widen its own scope. The order is not incidental:
/// lifecycle first (a candidate is excluded even when everything else is permissive), then the
/// policy switch, then scope, then audience.
pub(crate) fn eligibility(
    record: &MemoryRecord,
    access: &EffectiveMemoryAccess,
    agent_id: &AgentId,
) -> Result<(), PersonalizationExclusionReason> {
    match record.status {
        MemoryStatus::Candidate => return Err(PersonalizationExclusionReason::PendingCandidate),
        MemoryStatus::Archived => return Err(PersonalizationExclusionReason::Archived),
        MemoryStatus::Active => {}
    }
    if !access.read {
        return Err(PersonalizationExclusionReason::MemoryReadDisabled);
    }
    match &record.scope {
        MemoryScope::Global => {
            if !access.global_memory {
                return Err(PersonalizationExclusionReason::GlobalMemoryDisabled);
            }
        }
        MemoryScope::Workspace { workspace_key } => {
            if access.workspace.as_ref() != Some(workspace_key) {
                return Err(PersonalizationExclusionReason::OtherWorkspace);
            }
        }
    }
    if !record.audience.admits(agent_id) {
        return Err(PersonalizationExclusionReason::AgentAudience);
    }
    Ok(())
}
