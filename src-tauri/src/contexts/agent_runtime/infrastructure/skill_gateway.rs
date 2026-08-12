use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, AgentSkillPort, AgentSkillReadRequest, AgentToolCallOutcome,
    BoundSkillPrompt,
};
use crate::contexts::tooling::skills::api::{
    SkillAccessRefusal, SkillApi, SkillAvailability, SkillDelivery, SkillDiscoveryRequest,
    SkillLoadOutcome, SkillLoadRequest, SkillResourceEntry, SkillResourceIndex,
    SkillResourceReadOutcome, SkillResourceReadRequest, SkillType,
};
use serde_json::{json, Value};

/// Wraps `tooling::skills`' public facade to satisfy `agent_runtime`'s own `AgentSkillPort` —
/// mirrors `RuntimeEffectivePromptAdapter`'s existing pattern for depending on another context's
/// API through an `agent_runtime`-owned port rather than that context's types directly.
#[derive(Clone)]
pub(crate) struct RuntimeAgentSkillAdapter {
    skills: SkillApi,
}

impl RuntimeAgentSkillAdapter {
    pub(crate) fn new(skills: SkillApi) -> Self {
        Self { skills }
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
                prompts
                    .into_iter()
                    .map(|prompt| BoundSkillPrompt {
                        id: prompt.id,
                        name: prompt.name,
                        body: prompt.body,
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
                workspace_path,
            }) {
                Ok(SkillLoadOutcome::Loaded(skill)) => success(json!({
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
                })),
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
