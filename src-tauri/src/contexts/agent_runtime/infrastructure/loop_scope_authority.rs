//! Derives the enforcement guard for a Loop-owned session from backend ownership only.
//!
//! The session's Loop ownership is written by the sessions context when the role session is
//! created; the run's binding is written by the orchestrator after the worktree is prepared.
//! Neither can be supplied by a model, a tool call or the frontend, so a guard obtained here is
//! authoritative. A Loop-owned session without a derivable guard fails closed.

use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, AgentSessionGateway, LoopGuardRole, LoopRepository,
    LoopScopeAuthorityPort, LoopScopeGuard, LoopScopePlatformPort,
};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub(crate) struct LoopScopeAuthority {
    inner: RwLock<Option<Installed>>,
}

struct Installed {
    loops: Arc<dyn LoopRepository>,
    sessions: Arc<dyn AgentSessionGateway>,
    platform: Arc<dyn LoopScopePlatformPort>,
}

impl LoopScopeAuthority {
    pub(crate) fn install(
        &self,
        loops: Arc<dyn LoopRepository>,
        sessions: Arc<dyn AgentSessionGateway>,
        platform: Arc<dyn LoopScopePlatformPort>,
    ) {
        if let Ok(mut inner) = self.inner.write() {
            *inner = Some(Installed {
                loops,
                sessions,
                platform,
            });
        }
    }
}

impl LoopScopeAuthorityPort for LoopScopeAuthority {
    fn guard_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Arc<dyn LoopScopeGuard>>, AgentRuntimeApplicationError> {
        let inner = self.inner.read().map_err(|_| {
            AgentRuntimeApplicationError::Loop("Loop scope authority is unavailable.".to_string())
        })?;
        let Some(installed) = inner.as_ref() else {
            return Err(AgentRuntimeApplicationError::Loop(
                "Loop scope authority is not installed; Loop-owned sessions cannot execute."
                    .to_string(),
            ));
        };
        let Some(session) = installed.sessions.find_session(session_id)? else {
            return Ok(None);
        };
        let Some(ownership) = session.loop_ownership else {
            return Ok(None);
        };
        let role = LoopGuardRole::parse(&ownership.role).ok_or_else(|| {
            AgentRuntimeApplicationError::Loop(format!(
                "Loop session {session_id} has an unknown role."
            ))
        })?;
        let record = installed.loops.find_run_scope(&ownership.run_id)?;
        let Some(binding) = record.and_then(|record| record.binding) else {
            return Err(AgentRuntimeApplicationError::Loop(format!(
                "scope-binding-missing: Loop run {} has no trustworthy scope binding.",
                ownership.run_id
            )));
        };
        installed
            .platform
            .guard(&binding, role)
            .map(Some)
            .map_err(AgentRuntimeApplicationError::from)
    }
}
