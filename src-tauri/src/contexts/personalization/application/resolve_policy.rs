use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::models::MemoryEligibilityCriteria;
use super::policy_cache::{is_transient_read_failure, LastKnownGoodPolicyCache, PolicyCacheKey};
use super::ports::{AgentCapabilityPort, MemoryHealthPort, MemoryProjectionPort, PolicyRepository};
use crate::contexts::personalization::domain::{
    resolve, AgentId, EffectivePersonalizationSnapshot, MaintenanceState, MemoryBlockReason,
    MemoryRuntimeHealth, PersonalizationPolicyPatch, PersonalizationPolicyScope,
    PersonalizationResolutionContext, PersonalizationWarning, PersonalizationWarningCode,
    PolicyResolutionBundle, SessionId, SessionPersonalizationMode, WorkspaceIdentity,
    DEFAULT_POLICY_SET_ID,
};

/// How many memory refs one snapshot carries.
///
/// Bounded because the snapshot is taken per generation and its refs travel with it; the exact
/// eligible count is reported regardless, so a truncated page never reads as the whole set.
const ELIGIBLE_REFS_LIMIT: usize = 200;

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
    projection: Arc<dyn MemoryProjectionPort>,
    health: Arc<dyn MemoryHealthPort>,
    cache: Arc<LastKnownGoodPolicyCache>,
}

impl PolicyResolutionService {
    pub(crate) fn new(
        policies: Arc<dyn PolicyRepository>,
        agents: Arc<dyn AgentCapabilityPort>,
        projection: Arc<dyn MemoryProjectionPort>,
        health: Arc<dyn MemoryHealthPort>,
        cache: Arc<LastKnownGoodPolicyCache>,
    ) -> Self {
        Self {
            policies,
            agents,
            projection,
            health,
            cache,
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

        // Read fresh every time, cached or not: a bundle says what policy the user chose, never
        // whether the store behind it is currently safe to read.
        let maintenance = maintenance_state(self.health.health());
        let scopes = Self::scopes_for(&request);
        let (bundle, used_last_known_good) =
            match self.read_bundle(&request, &scopes, maintenance.migration_generation) {
                Some(result) => result,
                None => {
                    return Ok(EffectivePersonalizationSnapshot::fail_closed(
                        context,
                        PersonalizationWarningCode::NoValidatedPolicy,
                    ))
                }
            };

        let mut layers = bundle.into_layers();
        if layers.global.is_none() {
            return Ok(EffectivePersonalizationSnapshot::fail_closed(
                context,
                PersonalizationWarningCode::NoValidatedPolicy,
            ));
        }
        layers.session_override = request.session_override;

        let mut snapshot = resolve(context, layers, capabilities, maintenance);
        if used_last_known_good {
            snapshot.warnings.push(PersonalizationWarning::new(
                PersonalizationWarningCode::UsingLastKnownGoodPolicy,
            ));
        }
        Ok(self.attach_eligibility(snapshot))
    }

    /// The policy bundle for this request, and whether it came from cache.
    ///
    /// `None` means fail closed. Three outcomes, and the middle one is the whole reason this is not
    /// a plain read:
    ///
    /// - a successful read is remembered and returned;
    /// - a *transient* failure falls back to a bundle that was validated for this exact context, if
    ///   there is one, because a locked database is not a reason to forget the user's settings;
    /// - anything else — a schema mismatch, a corrupted value, an enum this build does not know —
    ///   fails closed, because answering from an older copy would assert a fact about data that has
    ///   just failed to validate.
    fn read_bundle(
        &self,
        request: &ResolutionRequest,
        scopes: &[PersonalizationPolicyScope],
        generation: u64,
    ) -> Option<(PolicyResolutionBundle, bool)> {
        let key = PolicyCacheKey::new(
            DEFAULT_POLICY_SET_ID,
            request.agent_id.clone(),
            request
                .workspace
                .as_ref()
                .map(|workspace| workspace.key().clone()),
            scopes,
            generation,
        );
        match self.policies.load_resolution_bundle(scopes) {
            Ok(bundle) => {
                self.cache.remember(key, bundle.clone());
                Some((bundle, false))
            }
            Err(error) if is_transient_read_failure(&error) => {
                self.cache.recall(&key).map(|bundle| (bundle, true))
            }
            Err(_) => None,
        }
    }

    /// Queries what is eligible under the access this snapshot resolved, and freezes it in.
    ///
    /// Runs before token budgeting and relevance selection, which is why it returns refs and counts
    /// rather than bodies: loading every body to decide what fits would defeat the budgeting it
    /// feeds. A snapshot whose memory is blocked skips the query entirely — reporting per-record
    /// exclusion counts would imply an enumeration that never happened.
    fn attach_eligibility(
        &self,
        snapshot: EffectivePersonalizationSnapshot,
    ) -> EffectivePersonalizationSnapshot {
        if !snapshot.memory_access.read {
            return snapshot;
        }
        let allowance = snapshot.memory_access.readable_scopes();
        let criteria = MemoryEligibilityCriteria {
            agent_id: snapshot.context.agent_id.clone(),
            allow_global: allowance.global,
            workspace: allowance.workspace,
            project_only: matches!(
                snapshot.context.session_mode,
                SessionPersonalizationMode::ProjectOnly
            ),
            limit: ELIGIBLE_REFS_LIMIT,
        };
        match self.projection.eligible_page(&criteria) {
            Ok(summary) => snapshot.with_memory(summary),
            // A projection that cannot answer is not a reason to inject a guess. The snapshot keeps
            // its resolved instructions and reports no eligible memory.
            Err(_) => {
                let mut blocked = snapshot;
                blocked
                    .memory_access
                    .block(MemoryBlockReason::MaintenanceState);
                blocked.warnings.push(PersonalizationWarning::new(
                    PersonalizationWarningCode::RepairRequired,
                ));
                blocked
            }
        }
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
