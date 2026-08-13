use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, AgentSkillPort, AgentSkillReadRequest, AgentToolCallOutcome,
    BoundSkillPrompt, UtilityDelegationResolutionPort,
};
use crate::contexts::agent_runtime::domain::UtilityDelegationSnapshot;
use crate::contexts::skill_evolution_evidence::application::{
    RuntimeEvidenceProjector, SkillLifecycleFact,
};
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, ObservedSkillRevision, SkillAssociationKind,
    SkillLoadOutcome as EvidenceSkillLoadOutcome, SourceFidelity,
};
use crate::contexts::tooling::skills::api::{
    SkillAccessRefusal, SkillApi, SkillAvailability, SkillDelivery, SkillDiscoveryRequest,
    SkillLoadOutcome, SkillLoadRequest, SkillResourceEntry, SkillResourceIndex,
    SkillResourceReadOutcome, SkillResourceReadRequest, SkillType, UtilitySkillResolutionOutcome,
};
use chrono::Utc;
use serde_json::{json, Value};

/// Wraps `tooling::skills`' public facade to satisfy `agent_runtime`'s own `AgentSkillPort` —
/// mirrors `RuntimeEffectivePromptAdapter`'s existing pattern for depending on another context's
/// API through an `agent_runtime`-owned port rather than that context's types directly.
#[derive(Clone)]
pub(crate) struct RuntimeAgentSkillAdapter {
    skills: SkillApi,
    evidence: RuntimeEvidenceProjector,
}

impl RuntimeAgentSkillAdapter {
    pub(crate) fn new(skills: SkillApi, evidence: RuntimeEvidenceProjector) -> Self {
        Self { skills, evidence }
    }
}

impl AgentSkillPort for RuntimeAgentSkillAdapter {
    fn bound_skill_prompts(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError> {
        self.skills
            .bound_skill_prompts_for_api_agent(agent_id, workspace_path)
            .map(|prompts| {
                let occurred_at = Utc::now().to_rfc3339();
                let observations = prompts
                    .iter()
                    .map(|prompt| ObservedSkillRevision {
                        skill_id: prompt.id.clone(),
                        revision: prompt.revision.clone(),
                        association_kind: SkillAssociationKind::Injected,
                        observed_at: occurred_at.clone(),
                    })
                    .collect::<Vec<_>>();
                prompts
                    .into_iter()
                    .map(|prompt| {
                        self.project_loaded_skill(
                            prompt.id.clone(),
                            prompt.revision.clone(),
                            Some(agent_id),
                            workspace_path,
                            occurred_at.clone(),
                            observations.clone(),
                        );
                        BoundSkillPrompt {
                            id: prompt.id,
                            name: prompt.name,
                            body: prompt.body,
                            revision: prompt.revision,
                        }
                    })
                    .collect()
            })
            .map_err(|error| AgentRuntimeApplicationError::Skill(error.to_string()))
    }

    fn execute_read(&self, request: AgentSkillReadRequest) -> AgentToolCallOutcome {
        match request {
            AgentSkillReadRequest::List {
                workspace_path,
                query,
                skill_type,
                delivery,
                availability,
                limit,
            } => {
                let request = SkillDiscoveryRequest {
                    workspace_path,
                    query,
                    skill_type: match skill_type {
                        Some(value) => match SkillType::parse(&value) {
                            Ok(value) => Some(value),
                            Err(_) => return invalid_filter("type"),
                        },
                        None => None,
                    },
                    delivery: match delivery {
                        Some(value) => match SkillDelivery::parse(&value) {
                            Ok(value) => Some(value),
                            Err(_) => return invalid_filter("delivery"),
                        },
                        None => None,
                    },
                    availability: match availability {
                        Some(value) => match SkillAvailability::parse(&value) {
                            Some(value) => Some(value),
                            None => return invalid_filter("availability"),
                        },
                        None => None,
                    },
                    limit,
                };
                match self.skills.list_for_agent(request) {
                    Ok(result) => success(json!({
                        "status": "listed",
                        "skills": result.skills.into_iter().map(|skill| json!({
                            "id": skill.id,
                            "name": skill.name,
                            "description": skill.description,
                            "aliases": skill.aliases,
                            "type": skill.skill_type.as_str(),
                            "delivery": skill.delivery.as_str(),
                            "layer": skill.layer.as_str(),
                            "availability": skill.availability.as_str(),
                            "version": skill.version,
                        })).collect::<Vec<_>>(),
                        "truncated": result.truncated,
                    })),
                    Err(_) => runtime_error(),
                }
            }
            AgentSkillReadRequest::Load {
                workspace_path,
                id_or_alias,
            } => match self.skills.load_for_agent(SkillLoadRequest {
                id_or_alias,
                workspace_path: workspace_path.clone(),
            }) {
                Ok(SkillLoadOutcome::Loaded(skill)) => {
                    self.project_loaded_skill(
                        skill.id.clone(),
                        skill.revision.clone(),
                        None,
                        workspace_path.as_deref(),
                        Utc::now().to_rfc3339(),
                        Vec::new(),
                    );
                    success(json!({
                        "status": "loaded",
                        "skill": {
                            "id": skill.id,
                            "name": skill.name,
                            "content": skill.content,
                            "truncated": skill.truncated,
                            "revision": skill.revision,
                            "baseUri": skill.base_uri,
                            "resources": resource_index_json(skill.resources),
                        }
                    }))
                }
                Ok(SkillLoadOutcome::Refused(refusal)) => refused(refusal),
                Err(_) => runtime_error(),
            },
            AgentSkillReadRequest::ReadResource {
                workspace_path,
                uri,
                revision,
            } => match self
                .skills
                .read_resource_for_agent(SkillResourceReadRequest {
                    uri,
                    revision,
                    workspace_path,
                }) {
                Ok(SkillResourceReadOutcome::Read(resource)) => success(json!({
                    "status": "read",
                    "resource": {
                        "id": resource.id,
                        "uri": resource.uri,
                        "revision": resource.revision,
                        "content": resource.content,
                        "sizeBytes": resource.size_bytes,
                    }
                })),
                Ok(SkillResourceReadOutcome::Refused(refusal)) => refused(refusal),
                Err(_) => runtime_error(),
            },
        }
    }
}

impl UtilityDelegationResolutionPort for RuntimeAgentSkillAdapter {
    fn resolve(
        &self,
        skill_id: &str,
        canonical_workspace: Option<&str>,
    ) -> Result<UtilityDelegationSnapshot, AgentRuntimeApplicationError> {
        match self
            .skills
            .resolve_utility_for_execution(skill_id, canonical_workspace)
            .map_err(|error| AgentRuntimeApplicationError::Skill(error.to_string()))?
        {
            UtilitySkillResolutionOutcome::Resolved(snapshot) => Ok(UtilityDelegationSnapshot {
                skill_id: snapshot.id,
                revision: snapshot.revision,
                instructions: snapshot.instructions,
                canonical_workspace: snapshot.workspace_path,
            }),
            UtilitySkillResolutionOutcome::Refused(refusal) => {
                Err(AgentRuntimeApplicationError::Skill(format!(
                    "Utility delegation refused: {}",
                    refusal.reason.as_str()
                )))
            }
        }
    }

    fn current_revision(
        &self,
        skill_id: &str,
        canonical_workspace: Option<&str>,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.resolve(skill_id, canonical_workspace)
            .map(|snapshot| snapshot.revision)
    }
}

impl RuntimeAgentSkillAdapter {
    fn project_loaded_skill(
        &self,
        skill_id: String,
        revision: String,
        agent_id: Option<&str>,
        workspace: Option<&str>,
        occurred_at: String,
        observed_skill_revisions: Vec<ObservedSkillRevision>,
    ) {
        let _ = self.evidence.skill_lifecycle(SkillLifecycleFact {
            common: EnvelopeCommon {
                source_event_id: format!("skill-load:{}", uuid::Uuid::new_v4()),
                occurred_at,
                stable_agent_id: agent_id.map(str::to_string),
                session_id: None,
                message_id: None,
                run_id: None,
                attempt_id: None,
                workspace: self.evidence.workspace_scope(workspace),
                fidelity: SourceFidelity::Native,
                observed_skill_revisions,
            },
            skill_id,
            revision,
            outcome: EvidenceSkillLoadOutcome::Loaded,
            anomaly: None,
            observation_count: 1,
        });
    }
}

fn resource_index_json(index: SkillResourceIndex) -> Value {
    fn entries(entries: Vec<SkillResourceEntry>) -> Vec<Value> {
        entries
            .into_iter()
            .map(|entry| {
                json!({
                    "uri": entry.uri,
                    "relativePath": entry.relative_path,
                    "sizeBytes": entry.size_bytes,
                })
            })
            .collect()
    }
    json!({
        "scripts": entries(index.scripts),
        "references": entries(index.references),
        "templates": entries(index.templates),
        "assets": entries(index.assets),
        "truncated": index.truncated,
    })
}

fn refused(refusal: SkillAccessRefusal) -> AgentToolCallOutcome {
    failure(json!({
        "status": "refused",
        "refusal": {
            "requested": refusal.requested,
            "canonicalId": refusal.canonical_id,
            "reason": refusal.reason.as_str(),
            "conflictingIds": refusal.conflicting_ids,
        }
    }))
}

fn invalid_filter(field: &str) -> AgentToolCallOutcome {
    failure(json!({
        "status": "error",
        "error": { "code": "invalid-filter", "field": field }
    }))
}

fn runtime_error() -> AgentToolCallOutcome {
    failure(json!({
        "status": "error",
        "error": {
            "code": "skill-runtime-unavailable",
            "message": "The effective Skill runtime could not complete this read."
        }
    }))
}

fn success(value: Value) -> AgentToolCallOutcome {
    AgentToolCallOutcome {
        output: value.to_string(),
        is_error: false,
    }
}

fn failure(value: Value) -> AgentToolCallOutcome {
    AgentToolCallOutcome {
        output: value.to_string(),
        is_error: true,
    }
}
