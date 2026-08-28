use serde::{Deserialize, Serialize};

use super::error::{IdentityRejection, PersonalizationDomainError};

/// Upper bound shared by every identity newtype. Generous enough for any registry id or hashed
/// workspace key, small enough that an identity can never be the reason a row or a filename grows
/// unbounded.
const IDENTITY_MAX_CHARS: usize = 120;

/// Shared validation for every identity this context stores or joins into a key.
///
/// Separators are rejected here rather than escaped later because both consumers — the scope key
/// and the memory filename — are safer if the value simply cannot contain one.
fn validate_identity(value: &str) -> Result<String, IdentityRejection> {
    if value.is_empty() {
        return Err(IdentityRejection::Empty);
    }
    if value != value.trim() {
        return Err(IdentityRejection::NotTrimmed);
    }
    if value.chars().count() > IDENTITY_MAX_CHARS {
        return Err(IdentityRejection::TooLong {
            limit: IDENTITY_MAX_CHARS,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(IdentityRejection::ContainsSeparator);
    }
    if value.chars().any(char::is_control) {
        return Err(IdentityRejection::ContainsControlCharacter);
    }
    Ok(value.to_string())
}

macro_rules! identity_newtype {
    ($name:ident, $error:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
                validate_identity(value)
                    .map(Self)
                    .map_err(PersonalizationDomainError::$error)
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

identity_newtype!(
    AgentId,
    InvalidAgentId,
    "A stable Agent id taken from the Agent registry.\n\nThis type deliberately knows nothing about *which* Agents exist. Personalization coverage is\ndriven by registry membership and declared capabilities, so a newly registered Agent must reach\npolicy resolution without any change here."
);
identity_newtype!(SessionId, InvalidSessionId, "A durable session id.");
identity_newtype!(
    WorkspaceKey,
    InvalidWorkspaceKey,
    "A stable, local workspace identity.\n\nDerived from an existing stable project id when one exists, otherwise from a platform-normalized\ncanonical root plus the remote connection identity. Never the display path: two remote workspaces\ncan share a path on different hosts and must not share a scope."
);

/// Whether a workspace lives on this machine or behind a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkspaceKind {
    Local,
    Remote,
}

/// A resolved workspace: the stable key used for authorization, plus the path used for display.
///
/// The two are separate on purpose. The key is what scope comparisons and memory filtering use; the
/// path is what a user reads. Showing the key as the primary label would be unreadable, and
/// comparing the path would merge unrelated remote workspaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceIdentity {
    key: WorkspaceKey,
    display_path: String,
    kind: WorkspaceKind,
}

impl WorkspaceIdentity {
    pub(crate) fn new(key: WorkspaceKey, display_path: String, kind: WorkspaceKind) -> Self {
        Self {
            key,
            display_path,
            kind,
        }
    }

    pub(crate) fn key(&self) -> &WorkspaceKey {
        &self.key
    }

    pub(crate) fn display_path(&self) -> &str {
        &self.display_path
    }

    pub(crate) fn kind(&self) -> WorkspaceKind {
        self.kind
    }
}

/// Which runtime shape is assembling the prompt.
///
/// This is a *runtime* distinction, not an Agent list: it decides whether VaneHub owns the
/// compaction around this generation, never which Agent may read which memory. Per-Agent behavior
/// comes from registry capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentRuntimeKind {
    /// VaneHub-native generation; VaneHub owns its context compaction.
    OnePiece,
    /// A CLI process wrapped by a VaneHub adapter; the CLI owns its own internal compaction.
    Cli,
    /// A provider API Agent driven through the standard generation service.
    Api,
}

impl AgentRuntimeKind {
    /// A stable code. Part of the snapshot revision token, so it must not change meaning without
    /// the token version moving with it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::OnePiece => "onepiece",
            Self::Cli => "cli",
            Self::Api => "api",
        }
    }
}

/// The durable scopes a policy row can occupy.
///
/// Session overrides are intentionally absent: they live with the session record rather than as a
/// durable policy row, because they must disappear with the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersonalizationPolicyScope {
    Global,
    Agent {
        agent_id: AgentId,
    },
    Workspace {
        workspace_key: WorkspaceKey,
    },
    WorkspaceAgent {
        workspace_key: WorkspaceKey,
        agent_id: AgentId,
    },
}

impl PersonalizationPolicyScope {
    pub(crate) fn scope_kind(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Agent { .. } => "agent",
            Self::Workspace { .. } => "workspace",
            Self::WorkspaceAgent { .. } => "workspace-agent",
        }
    }

    /// The unique key this scope is stored and looked up under.
    ///
    /// Built from typed values joined with `/`, which is safe precisely because every identity
    /// newtype rejects `/`. Assembling this from display text would let a workspace name forge
    /// another scope's key.
    pub(crate) fn scope_key(&self) -> String {
        match self {
            Self::Global => "global".to_string(),
            Self::Agent { agent_id } => format!("agent/{agent_id}"),
            Self::Workspace { workspace_key } => format!("workspace/{workspace_key}"),
            Self::WorkspaceAgent {
                workspace_key,
                agent_id,
            } => format!("workspace-agent/{workspace_key}/{agent_id}"),
        }
    }

    /// Later layers override earlier ones. Workspace outranks a generic Agent override so that
    /// project guidance wins by default; a workspace-Agent row is the explicit exception.
    pub(crate) fn precedence_rank(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Agent { .. } => 1,
            Self::Workspace { .. } => 2,
            Self::WorkspaceAgent { .. } => 3,
        }
    }

    pub(crate) fn workspace_key(&self) -> Option<&WorkspaceKey> {
        match self {
            Self::Global | Self::Agent { .. } => None,
            Self::Workspace { workspace_key } | Self::WorkspaceAgent { workspace_key, .. } => {
                Some(workspace_key)
            }
        }
    }

    pub(crate) fn agent_id(&self) -> Option<&AgentId> {
        match self {
            Self::Global | Self::Workspace { .. } => None,
            Self::Agent { agent_id } | Self::WorkspaceAgent { agent_id, .. } => Some(agent_id),
        }
    }

    /// Rebuilds a scope from the persisted `scope_kind` plus its two nullable columns.
    ///
    /// The table cannot express "these columns are required for exactly this kind", so this is the
    /// only place that contradiction is caught. A row whose columns disagree with its kind is a
    /// corruption signal, not something to interpret generously.
    pub(crate) fn from_parts(
        kind: &str,
        workspace_key: Option<&WorkspaceKey>,
        agent_id: Option<&AgentId>,
    ) -> Result<Self, PersonalizationDomainError> {
        let inconsistent = |reason: &'static str| {
            Err(PersonalizationDomainError::InconsistentScopeColumns {
                kind: match kind {
                    "global" => "global",
                    "agent" => "agent",
                    "workspace" => "workspace",
                    _ => "workspace-agent",
                },
                reason,
            })
        };
        match kind {
            "global" => {
                if workspace_key.is_some() || agent_id.is_some() {
                    return inconsistent("a global row must not carry a workspace key or Agent id");
                }
                Ok(Self::Global)
            }
            "agent" => match (workspace_key, agent_id) {
                (None, Some(agent_id)) => Ok(Self::Agent {
                    agent_id: agent_id.clone(),
                }),
                (Some(_), _) => inconsistent("an Agent row must not carry a workspace key"),
                (None, None) => inconsistent("an Agent row requires an Agent id"),
            },
            "workspace" => match (workspace_key, agent_id) {
                (Some(workspace_key), None) => Ok(Self::Workspace {
                    workspace_key: workspace_key.clone(),
                }),
                (_, Some(_)) => inconsistent("a workspace row must not carry an Agent id"),
                (None, None) => inconsistent("a workspace row requires a workspace key"),
            },
            "workspace-agent" => match (workspace_key, agent_id) {
                (Some(workspace_key), Some(agent_id)) => Ok(Self::WorkspaceAgent {
                    workspace_key: workspace_key.clone(),
                    agent_id: agent_id.clone(),
                }),
                _ => inconsistent(
                    "a workspace-Agent row requires both a workspace key and an Agent id",
                ),
            },
            other => Err(PersonalizationDomainError::UnknownScopeKind(
                other.to_string(),
            )),
        }
    }
}
