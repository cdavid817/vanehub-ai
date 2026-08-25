use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::ports::{AgentCapabilityPort, MemoryHealthPort, PolicyRepository};
use crate::contexts::personalization::domain::{
    resolve, AgentId, EffectivePersonalizationSnapshot, MaintenanceState, MemoryRuntimeHealth,
    PersonalizationPolicyPatch, PersonalizationPolicyScope, PersonalizationResolutionContext,
    PersonalizationWarningCode, SessionId, SessionPersonalizationMode, WorkspaceIdentity,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Everything a caller knows before resolution runs.
///
/// Deliberately not a snapshot of policy: the caller supplies who is asking and in what session,
/// and the resolver reads the policy itself. A caller that could pass its own policy in could pass
/// a stale one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolutionRequest {
    pub(crate) agent_id: AgentId,
    pub(crate) session_id: SessionId,
    pub(crate) workspace: Option<WorkspaceIdentity>,
    pub(crate) session_mode: SessionPersonalizationMode,
    /// Lives with the session rather than as a durable row, so it disappears with the session.
    pub(crate) session_override: Option<PersonalizationPolicyPatch>,
}

/// Resolves one immutable snapshot from stored policy, registry capabilities, and memory health.
///
/// The precedence order is the domain's and is not restated here. What this owns is *reading* —
/// which scope keys to ask for, in one consistent read, and what to do when the answer is missing:
///
/// - no global row: fail closed, because an installation with no validated policy has nothing to
///   resolve against and defaults would silently grant whatever they happen to allow;
/// - Agent not in the registry: fail closed, because capabilities that were never declared cannot
///   be assumed, and assuming them would hand an unknown runtime a surface it never claimed;
/// - project-only session with no workspace: fail closed, because "read everything global" is the
///   single interpretation a project-isolated session must never degrade to.
///
/// Every one of those still returns a snapshot rather than an error. Generation continues without
/// personalization; it never stops because personalization could not be established.
pub(crate) struct PolicyResolutionService {
    policies: Arc<dyn PolicyRepository>,
    agents: Arc<dyn AgentCapabilityPort>,
    health: Arc<dyn MemoryHealthPort>,
}

impl PolicyResolutionService {
    pub(crate) fn new(
        policies: Arc<dyn PolicyRepository>,
        agents: Arc<dyn AgentCapabilityPort>,
        health: Arc<dyn MemoryHealthPort>,
    ) -> Self {
        Self {
            policies,
            agents,
            health,
        }
    }

    /// The scope keys that apply to one request, in precedence order.
    ///
    /// Workspace keys are omitted entirely when there is no workspace rather than asked for and
    /// found absent: a key that cannot exist is not a finding about the store.
    pub(crate) fn scopes_for(request: &ResolutionRequest) -> Vec<PersonalizationPolicyScope> {
        let mut scopes = vec![
            PersonalizationPolicyScope::Global,
            PersonalizationPolicyScope::Agent {
                agent_id: request.agent_id.clone(),
            },
        ];
        if let Some(workspace) = request.workspace.as_ref() {
            scopes.push(PersonalizationPolicyScope::Workspace {
                workspace_key: workspace.key().clone(),
            });
            scopes.push(PersonalizationPolicyScope::WorkspaceAgent {
                workspace_key: workspace.key().clone(),
                agent_id: request.agent_id.clone(),
            });
        }
        scopes
    }

    pub(crate) fn resolve(
        &self,
        request: ResolutionRequest,
    ) -> Result<EffectivePersonalizationSnapshot> {
        let context = PersonalizationResolutionContext {
            agent_id: request.agent_id.clone(),
            session_id: request.session_id.clone(),
            workspace: request.workspace.clone(),
            runtime_kind: self.runtime_kind(&request.agent_id)?,
            session_mode: request.session_mode,
        };

        // Asked of the registry, never assumed. An Agent registered while the application is
        // running reaches this by the same path as one that shipped with it.
        let Some(capabilities) = self.agents.capabilities(&request.agent_id)? else {
            return Ok(EffectivePersonalizationSnapshot::fail_closed(
                context,
                PersonalizationWarningCode::UnknownAgent,
            ));
        };

        // Creation is supposed to reject this. Reaching resolution without a workspace means
        // something upstream failed, and the resolver refuses rather than widening the scope.
        if matches!(
            request.session_mode,
            SessionPersonalizationMode::ProjectOnly
        ) && request.workspace.is_none()
        {
            return Ok(EffectivePersonalizationSnapshot::fail_closed(
                context,
                PersonalizationWarningCode::WorkspaceRequired,
            ));
        }

        let bundle = self
            .policies
            .load_resolution_bundle(&Self::scopes_for(&request))?;
        let mut layers = bundle.into_layers();
        if layers.global.is_none() {
            return Ok(EffectivePersonalizationSnapshot::fail_closed(
                context,
                PersonalizationWarningCode::NoValidatedPolicy,
            ));
        }
        layers.session_override = request.session_override;

        Ok(resolve(
            context,
            layers,
            capabilities,
            maintenance_state(self.health.health()),
        ))
    }

    /// The runtime shape this Agent is driven by.
    ///
    /// Read from the registry rather than inferred from the id. This decides whether VaneHub owns
    /// the compaction around a generation — never which Agent may read which memory, which is
    /// capability- and policy-driven.
    fn runtime_kind(
        &self,
        agent_id: &AgentId,
    ) -> Result<crate::contexts::personalization::domain::AgentRuntimeKind> {
        let _ = agent_id;
        // The registry's runtime shape arrives with the runtime adapters; until then every caller
        // resolves as an API Agent, which is the shape that owns no external context.
        Ok(crate::contexts::personalization::domain::AgentRuntimeKind::Api)
    }
}

/// Health, in the shape the domain resolver consumes.
///
/// Only `Ready` is a complete migration. Every other state — including one this build does not
/// recognize — reads as incomplete, which denies memory without touching instructions.
fn maintenance_state(health: MemoryRuntimeHealth) -> MaintenanceState {
    match health {
        MemoryRuntimeHealth::Ready { generation } => MaintenanceState {
            migration_generation: generation,
            migration_complete: true,
            repair_required: false,
        },
        MemoryRuntimeHealth::RepairRequired => MaintenanceState {
            migration_generation: 0,
            migration_complete: false,
            repair_required: true,
        },
        _ => MaintenanceState {
            migration_generation: 0,
            migration_complete: false,
            repair_required: false,
        },
    }
}
