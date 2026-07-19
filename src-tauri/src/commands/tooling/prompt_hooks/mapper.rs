use super::dto;
use crate::contexts::tooling::prompt_hooks::api as prompt;

pub(super) fn create_request(
    input: dto::PromptHookMutationInput,
) -> Result<prompt::PromptHookCreateRequest, prompt::PromptHookError> {
    Ok(prompt::PromptHookCreateRequest {
        manifest: manifest(
            input.id,
            input.name,
            input.category,
            input.stage,
            input.order,
            input.template_body,
            input.cli_bindings,
        )?,
        description: input.description,
        enabled: input.enabled,
        governance: governance_to_application(input.governance),
    })
}

pub(super) fn update_request(
    hook_id: String,
    input: dto::PromptHookUpdateInput,
) -> Result<prompt::PromptHookUpdateRequest, prompt::PromptHookError> {
    Ok(prompt::PromptHookUpdateRequest {
        hook_id: prompt::PromptHookId::parse(hook_id)?,
        manifest: manifest(
            input.id,
            input.name,
            input.category,
            input.stage,
            input.order,
            input.template_body,
            input.cli_bindings,
        )?,
        description: input.description,
        version: input.version,
        enabled: input.enabled,
        governance: governance_to_application(input.governance),
    })
}

pub(super) fn hook_id(value: String) -> Result<prompt::PromptHookId, prompt::PromptHookError> {
    prompt::PromptHookId::parse(value).map_err(Into::into)
}

pub(super) fn bindings(
    values: Vec<String>,
) -> Result<prompt::PromptHookBindings, prompt::PromptHookError> {
    prompt::PromptHookBindings::new(&values).map_err(Into::into)
}

pub(super) fn preview_request(
    input: dto::PromptHookPreviewInput,
) -> Result<prompt::PromptHookPreviewRequest, prompt::PromptHookError> {
    Ok(prompt::PromptHookPreviewRequest {
        hook_id: prompt::PromptHookId::parse(input.hook_id)?,
        agent_id: prompt::ManagedCliAgentId::parse(&input.agent_id)?,
        sample_input: input.sample_input,
    })
}

pub(super) fn list_to_dto(result: prompt::PromptHookListResult) -> dto::PromptHookListResult {
    dto::PromptHookListResult {
        hooks: result.hooks.into_iter().map(record_to_dto).collect(),
        stats: dto::PromptHookStats {
            total: result.stats.total,
            enabled: result.stats.enabled,
            builtin: result.stats.builtin,
            user: result.stats.user,
        },
    }
}

pub(super) fn record_to_dto(record: prompt::PromptHookRecord) -> dto::PromptHook {
    dto::PromptHook {
        id: record.id().as_str().to_string(),
        name: record.manifest.name().as_str().to_string(),
        description: record.description,
        category: category_to_dto(record.manifest.category()),
        stage: stage_to_dto(record.manifest.stage()),
        order: record.manifest.order().value(),
        version: record.version,
        source: source_to_dto(record.source),
        enabled: record.enabled,
        disableable: record.disableable,
        cli_bindings: record.manifest.bindings().to_strings(),
        governance: governance_to_dto(record.governance),
        template_body: Some(record.manifest.template().as_str().to_string()),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(super) fn preview_to_dto(preview: prompt::PromptHookPreview) -> dto::PromptHookPreview {
    dto::PromptHookPreview {
        hook_id: Some(preview.hook_id.as_str().to_string()),
        agent_id: preview.agent_id.as_str().to_string(),
        rendered_content: preview.rendered_content,
        trace: preview.trace.into_iter().map(trace_to_dto).collect(),
    }
}

pub(super) fn assembly_to_dto(
    agent_id: String,
    result: prompt::PromptAssemblyResult,
) -> dto::PromptHookPreview {
    dto::PromptHookPreview {
        hook_id: None,
        agent_id,
        rendered_content: result.effective_prompt,
        trace: result.trace.into_iter().map(trace_to_dto).collect(),
    }
}

pub(super) fn traces_to_dto(
    traces: Vec<prompt::PromptHookTrace>,
) -> Vec<dto::PromptHookTraceSummary> {
    traces.into_iter().map(trace_to_dto).collect()
}

fn trace_to_dto(trace: prompt::PromptHookTrace) -> dto::PromptHookTraceSummary {
    dto::PromptHookTraceSummary {
        id: trace.id,
        hook_id: trace.hook_id.as_str().to_string(),
        category: category_to_dto(trace.category),
        stage: stage_to_dto(trace.stage),
        status: trace.status.as_str().to_string(),
        version: trace.version,
        content_hash: trace.content_hash,
        token_estimate: trace.token_estimate,
        reason: trace.reason,
        agent_id: trace.agent_id.map(|agent_id| agent_id.as_str().to_string()),
        session_id: trace.session_id,
        created_at: trace.created_at,
    }
}

#[allow(clippy::too_many_arguments)]
fn manifest(
    id: String,
    name: String,
    category: dto::PromptHookCategory,
    stage: dto::PromptHookStage,
    order: i64,
    template_body: String,
    cli_bindings: Vec<String>,
) -> Result<prompt::PromptHookManifest, prompt::PromptHookError> {
    prompt::PromptHookManifest::new(
        id,
        name,
        category_to_domain(category),
        stage_to_domain(stage),
        order,
        template_body,
        &cli_bindings,
    )
    .map_err(Into::into)
}

fn governance_to_application(value: dto::PromptHookGovernance) -> prompt::PromptHookGovernance {
    prompt::PromptHookGovernance {
        safety_tier: value.safety_tier,
        transparency_tier: value.transparency_tier,
        governance_tier: value.governance_tier,
    }
}

fn governance_to_dto(value: prompt::PromptHookGovernance) -> dto::PromptHookGovernance {
    dto::PromptHookGovernance {
        safety_tier: value.safety_tier,
        transparency_tier: value.transparency_tier,
        governance_tier: value.governance_tier,
    }
}

fn category_to_domain(value: dto::PromptHookCategory) -> prompt::PromptHookCategory {
    match value {
        dto::PromptHookCategory::Bootstrap => prompt::PromptHookCategory::Bootstrap,
        dto::PromptHookCategory::Callback => prompt::PromptHookCategory::Callback,
        dto::PromptHookCategory::Dynamic => prompt::PromptHookCategory::Dynamic,
        dto::PromptHookCategory::Law => prompt::PromptHookCategory::Law,
        dto::PromptHookCategory::Navigation => prompt::PromptHookCategory::Navigation,
        dto::PromptHookCategory::Routing => prompt::PromptHookCategory::Routing,
        dto::PromptHookCategory::Static => prompt::PromptHookCategory::Static,
    }
}

fn category_to_dto(value: prompt::PromptHookCategory) -> dto::PromptHookCategory {
    match value {
        prompt::PromptHookCategory::Bootstrap => dto::PromptHookCategory::Bootstrap,
        prompt::PromptHookCategory::Callback => dto::PromptHookCategory::Callback,
        prompt::PromptHookCategory::Dynamic => dto::PromptHookCategory::Dynamic,
        prompt::PromptHookCategory::Law => dto::PromptHookCategory::Law,
        prompt::PromptHookCategory::Navigation => dto::PromptHookCategory::Navigation,
        prompt::PromptHookCategory::Routing => dto::PromptHookCategory::Routing,
        prompt::PromptHookCategory::Static => dto::PromptHookCategory::Static,
    }
}

fn stage_to_domain(value: dto::PromptHookStage) -> prompt::PromptHookStage {
    match value {
        dto::PromptHookStage::SessionInit => prompt::PromptHookStage::SessionInit,
        dto::PromptHookStage::PerTurn => prompt::PromptHookStage::PerTurn,
    }
}

fn stage_to_dto(value: prompt::PromptHookStage) -> dto::PromptHookStage {
    match value {
        prompt::PromptHookStage::SessionInit => dto::PromptHookStage::SessionInit,
        prompt::PromptHookStage::PerTurn => dto::PromptHookStage::PerTurn,
    }
}

fn source_to_dto(value: prompt::PromptHookSource) -> dto::PromptHookSource {
    match value {
        prompt::PromptHookSource::Builtin => dto::PromptHookSource::Builtin,
        prompt::PromptHookSource::User => dto::PromptHookSource::User,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mutation_request_accepts_existing_camel_case_and_kebab_case_values() {
        let input: dto::PromptHookMutationInput = serde_json::from_value(json!({
            "id": "fixture-hook",
            "name": "Fixture Hook",
            "description": "Fixture description",
            "category": "dynamic",
            "stage": "per-turn",
            "order": 450,
            "templateBody": "Fixture {{agentId}}",
            "enabled": true,
            "cliBindings": ["codex-cli"],
            "governance": {
                "safetyTier": "editable",
                "transparencyTier": "visible-by-default",
                "governanceTier": "human-gated"
            }
        }))
        .expect("mutation DTO");

        let request = create_request(input).expect("create request");

        assert_eq!(request.manifest.id().as_str(), "fixture-hook");
        assert_eq!(
            request.manifest.category(),
            prompt::PromptHookCategory::Dynamic
        );
        assert_eq!(request.manifest.stage(), prompt::PromptHookStage::PerTurn);
        assert_eq!(request.manifest.bindings().to_strings(), ["codex-cli"]);
    }

    #[test]
    fn hook_response_preserves_the_complete_existing_transport_contract() {
        let record = prompt::PromptHookRecord {
            manifest: prompt::PromptHookManifest::new(
                "fixture-hook",
                "Fixture Hook",
                prompt::PromptHookCategory::Dynamic,
                prompt::PromptHookStage::PerTurn,
                450,
                "Fixture {{agentId}} {{sampleInput}}",
                &["codex-cli".to_string(), "opencode".to_string()],
            )
            .expect("manifest"),
            description: "Fixture description".to_string(),
            version: 2,
            source: prompt::PromptHookSource::User,
            enabled: true,
            disableable: true,
            governance: prompt::PromptHookGovernance {
                safety_tier: "editable".to_string(),
                transparency_tier: "visible-by-default".to_string(),
                governance_tier: "human-gated".to_string(),
            },
            created_at: "2026-07-17T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(record_to_dto(record)).expect("hook DTO");

        assert_eq!(
            value,
            json!({
                "id": "fixture-hook",
                "name": "Fixture Hook",
                "description": "Fixture description",
                "category": "dynamic",
                "stage": "per-turn",
                "order": 450,
                "version": 2,
                "source": "user",
                "enabled": true,
                "disableable": true,
                "cliBindings": ["codex-cli", "opencode"],
                "governance": {
                    "safetyTier": "editable",
                    "transparencyTier": "visible-by-default",
                    "governanceTier": "human-gated"
                },
                "templateBody": "Fixture {{agentId}} {{sampleInput}}",
                "createdAt": "2026-07-17T00:00:00Z",
                "updatedAt": "2026-07-18T00:00:00Z"
            })
        );
    }

    #[test]
    fn trace_response_preserves_camel_case_and_nullable_fields() {
        let value = serde_json::to_value(dto::PromptHookTraceSummary {
            id: "trace-1".to_string(),
            hook_id: "fixture-hook".to_string(),
            category: dto::PromptHookCategory::Dynamic,
            stage: dto::PromptHookStage::PerTurn,
            status: "fired".to_string(),
            version: Some(2),
            content_hash: Some("hash".to_string()),
            token_estimate: Some(4),
            reason: None,
            agent_id: Some("codex-cli".to_string()),
            session_id: None,
            created_at: "2026-07-18T00:00:00Z".to_string(),
        })
        .expect("trace DTO");

        assert_eq!(value["hookId"], "fixture-hook");
        assert_eq!(value["category"], "dynamic");
        assert_eq!(value["stage"], "per-turn");
        assert_eq!(value["tokenEstimate"], 4);
        assert!(value["reason"].is_null());
        assert!(value.get("hook_id").is_none());
    }
}
