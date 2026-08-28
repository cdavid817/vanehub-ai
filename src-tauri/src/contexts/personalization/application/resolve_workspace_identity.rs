use super::error::PersonalizationApplicationError;
use super::models::WorkspaceIdentityRequest;
use super::ports::WorkspaceIdentityPort;
use crate::contexts::personalization::domain::{
    LocalPathRules, WorkspaceIdentity, WorkspaceIdentitySource,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Turns whatever a caller knows about a workspace into a stable local key.
///
/// Preference order, and it matters: a stable id the workspace subsystem already assigns beats one
/// derived here, because two subsystems deriving their own answer is how "the same workspace" ends
/// up meaning two different things. Derivation is the fallback, not the default.
///
/// A worktree resolves to its own identity rather than its parent project's. Two worktrees of one
/// repository are different working directories with different state, and merging their memories
/// would surface one branch's notes while working on another.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkspaceIdentityResolver {
    /// Which spellings this filesystem treats as one directory. Injected rather than read from
    /// `cfg!` at the point of use so each rule is exercised in both directions on every platform.
    local_rules: LocalPathRules,
}

impl WorkspaceIdentityResolver {
    pub(crate) fn for_this_platform() -> Self {
        Self {
            local_rules: LocalPathRules::for_this_platform(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_local_rules(local_rules: LocalPathRules) -> Self {
        Self { local_rules }
    }

    /// Chooses the identity source a request describes, or `None` when it describes no workspace.
    fn source(request: &WorkspaceIdentityRequest) -> Option<WorkspaceIdentitySource> {
        if let Some(stable_id) = request
            .stable_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(WorkspaceIdentitySource::StableId(stable_id.to_string()));
        }
        if let Some(remote_uri) = request
            .remote_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return parse_remote_uri(remote_uri);
        }
        // A worktree is its own workspace, so it wins over the project it was cut from.
        let local = request
            .worktree_path
            .as_deref()
            .or(request.project_path.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        Some(WorkspaceIdentitySource::LocalRoot {
            path: local.to_string(),
        })
    }
}

/// Parses `ssh://user@host:port/path` into its connection identity.
///
/// Only the connection identity is extracted — host, port, user, path. Anything a URI might carry
/// that authenticates (a password component) is deliberately not read: an identity derived from a
/// secret changes when the secret rotates, and would put recoverable material into a value that
/// appears in diagnostics.
fn parse_remote_uri(uri: &str) -> Option<WorkspaceIdentitySource> {
    let rest = uri.split_once("://").map(|(_, rest)| rest).unwrap_or(uri);
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (user, host_port) = match authority.rsplit_once('@') {
        Some((user, host_port)) => (Some(user), host_port),
        None => (None, authority),
    };
    // A password component (`user:password@host`) is discarded rather than hashed.
    let user = user.and_then(|value| value.split_once(':').map(|(name, _)| name).or(Some(value)));

    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().unwrap_or(22)),
        None => (host_port, 22),
    };
    if host.trim().is_empty() {
        return None;
    }
    Some(WorkspaceIdentitySource::Remote {
        host: host.to_string(),
        port,
        user: user
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        path: format!("/{path}"),
    })
}

impl WorkspaceIdentityPort for WorkspaceIdentityResolver {
    fn resolve(&self, request: &WorkspaceIdentityRequest) -> Result<Option<WorkspaceIdentity>> {
        let Some(source) = Self::source(request) else {
            return Ok(None);
        };
        Ok(Some(source.resolve(self.local_rules)?))
    }
}
