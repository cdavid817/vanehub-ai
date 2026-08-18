use super::*;
use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRegistryRepository, AgentRuntimeApplicationError,
    AgentWorkflowRepository, ApiAgentGateway, ApiProviderConfig, RegisterApiAgentInput,
    StoredEndpointProfileMetadata, StoredHybridRoutingRule, StoredOnePieceProviderProfile,
    INTERFACE_FORMAT_ANTHROPIC, INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentLifecycle, AgentWorkflow, AvailabilityAssessment, InteractionMode,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::sync::Arc;

#[derive(Clone)]
struct FakeAvailability {
    assessment: AvailabilityAssessment,
}

impl AgentAvailabilityGateway for FakeAvailability {
    fn assess(
        &self,
        _managed_sdk_dependency_id: Option<&str>,
        _executable_name: Option<&str>,
    ) -> Result<AvailabilityAssessment, AgentRuntimeApplicationError> {
        Ok(self.assessment.clone())
    }
}

fn repository(
    assessment: AvailabilityAssessment,
) -> (TempDirectory, NativeDatabase, SqliteAgentRuntimeRepository) {
    let directory = TempDirectory::new("agent-runtime-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteAgentRuntimeRepository::new(
        database.clone(),
        Arc::new(FakeAvailability { assessment }),
    );
    (directory, database, repository)
}

#[test]
fn seeded_registry_rows_map_to_stable_domain_catalog_values() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));

    let agents = repository.list().expect("agents");
    assert_eq!(
        agents
            .iter()
            .map(|agent| agent.id().as_str())
            .collect::<Vec<_>>(),
        vec![
            "antigravity-cli",
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "onepiece",
            "opencode"
        ]
    );
    let antigravity = agents
        .iter()
        .find(|agent| agent.id().as_str() == "antigravity-cli")
        .expect("antigravity");
    assert_eq!(antigravity.display_name(), "Antigravity CLI");
    assert_eq!(antigravity.provider(), "Google");
    assert_eq!(antigravity.managed_sdk_dependency_id(), None);
    assert_eq!(antigravity.launch().executable_name(), Some("agy"));
    let codex = agents
        .iter()
        .find(|agent| agent.id().as_str() == "codex-cli")
        .expect("codex");
    assert_eq!(codex.display_name(), "Codex CLI");
    assert_eq!(codex.provider(), "OpenAI");
    assert_eq!(codex.managed_sdk_dependency_id(), Some("codex-sdk"));
    assert_eq!(codex.launch().kind_str(), "cli");
    assert_eq!(codex.launch().command(), Some("codex"));
    assert_eq!(
        codex.supported_interaction_modes(),
        &[InteractionMode::Cli, InteractionMode::NativeDesktop]
    );
    assert_eq!(codex.capability_tags(), &["agent", "cli", "coding"]);
}

#[test]
fn onepiece_provider_profiles_switch_runtime_projection_and_delete_without_fallback() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let anthropic = StoredOnePieceProviderProfile {
        id: "anthropic-primary".to_string(),
        name: "Anthropic primary".to_string(),
        source_preset_id: Some("anthropic".to_string()),
        source_provider_id: Some("anthropic".to_string()),
        source_endpoint_type: Some("anthropic-messages".to_string()),
        source_preset_version: Some(1),
        provider: "Anthropic".to_string(),
        model_id: "claude-test".to_string(),
        interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
        base_url: None,
        active: true,
    };
    let proxy = StoredOnePieceProviderProfile {
        id: "openai-proxy".to_string(),
        name: "OpenAI proxy".to_string(),
        source_preset_id: Some("openrouter".to_string()),
        source_provider_id: Some("openrouter".to_string()),
        source_endpoint_type: Some("openai-chat-completions".to_string()),
        source_preset_version: Some(1),
        provider: "OpenAI".to_string(),
        model_id: "gpt-test".to_string(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        base_url: Some("https://gateway.example.test/v1".to_string()),
        active: false,
    };

    ApiAgentGateway::save_onepiece_provider_profile(&repository, &anthropic)
        .expect("save first profile");
    ApiAgentGateway::save_onepiece_provider_profile(&repository, &proxy)
        .expect("save second profile");
    let profiles =
        ApiAgentGateway::list_onepiece_provider_profiles(&repository).expect("list profiles");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles.iter().filter(|profile| profile.active).count(), 1);
    assert!(profiles
        .iter()
        .any(|profile| profile.id == anthropic.id && profile.active));

    ApiAgentGateway::activate_onepiece_provider_profile(&repository, &proxy.id)
        .expect("activate proxy");
    let projected =
        ApiAgentGateway::onepiece_provider_config(&repository).expect("project active profile");
    assert_eq!(projected.provider, "OpenAI");
    assert_eq!(projected.model_id.as_deref(), Some("gpt-test"));
    assert_eq!(
        ApiAgentGateway::provider_config(&repository, "onepiece")
            .expect("runtime config")
            .expect("active provider")
            .source_provider_id
            .as_deref(),
        Some("openrouter")
    );

    ApiAgentGateway::activate_onepiece_provider_profile(&repository, &anthropic.id)
        .expect("reactivate first-inserted profile without violating the active index");
    ApiAgentGateway::activate_onepiece_provider_profile(&repository, &proxy.id)
        .expect("reactivate proxy");

    assert!(
        ApiAgentGateway::delete_onepiece_provider_profile(&repository, &proxy.id)
            .expect("delete active profile")
    );
    let remaining = ApiAgentGateway::list_onepiece_provider_profiles(&repository)
        .expect("list remaining profiles");
    assert_eq!(
        remaining,
        vec![StoredOnePieceProviderProfile {
            active: false,
            ..anthropic
        }]
    );
    let cleared = ApiAgentGateway::onepiece_provider_config(&repository)
        .expect("read cleared runtime projection");
    assert_eq!(cleared.provider, "VaneHub");
    assert_eq!(cleared.model_id, None);
    assert_eq!(cleared.interface_format, None);
}

#[test]
fn profile_metadata_and_routing_rules_round_trip_atomically_and_cleanup_dangling_rules() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let profile = StoredOnePieceProviderProfile {
        id: "local-qwen".to_string(),
        name: "Local Qwen".to_string(),
        source_preset_id: None,
        source_provider_id: None,
        source_endpoint_type: None,
        source_preset_version: None,
        provider: "Custom local".to_string(),
        model_id: "qwen".to_string(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        base_url: Some("http://127.0.0.1:11434/v1".to_string()),
        active: true,
    };
    ApiAgentGateway::save_onepiece_provider_profile(&repository, &profile).expect("save profile");
    let metadata = StoredEndpointProfileMetadata {
        profile_id: profile.id.clone(),
        runtime_kind: "local".to_string(),
        endpoint_source: "configured".to_string(),
        authentication_mode: "none".to_string(),
        timeout_ms: 12_000,
        privacy_classification: "local".to_string(),
        text_generation_capability: "supported".to_string(),
        tool_calling_capability: "unsupported".to_string(),
        image_input_capability: "unknown".to_string(),
        structured_output_capability: "unknown".to_string(),
        reasoning_field_capability: "unknown".to_string(),
        capability_provenance: "configured".to_string(),
        context_window_tokens: Some(32_768),
        reserved_output_tokens: 4_096,
        context_capacity_provenance: "configured-estimate".to_string(),
    };
    ApiAgentGateway::save_endpoint_profile_metadata(&repository, &metadata).expect("save metadata");
    assert_eq!(
        ApiAgentGateway::endpoint_profile_metadata(&repository, &profile.id)
            .expect("read metadata"),
        Some(metadata)
    );

    let rules = vec![StoredHybridRoutingRule {
        id: "summarize-local".to_string(),
        enabled: true,
        position: 0,
        task_class: "summarization".to_string(),
        preferred_profile_id: profile.id.clone(),
        fallback_profile_id: None,
        data_policy: "local-only".to_string(),
    }];
    ApiAgentGateway::replace_hybrid_routing_rules(&repository, &rules).expect("replace rules");
    assert_eq!(
        ApiAgentGateway::list_hybrid_routing_rules(&repository).expect("list rules"),
        rules
    );

    ApiAgentGateway::delete_onepiece_provider_profile(&repository, &profile.id)
        .expect("delete preferred profile");
    assert!(ApiAgentGateway::list_hybrid_routing_rules(&repository)
        .expect("list cleaned rules")
        .is_empty());
}

#[test]
fn availability_is_injected_from_runtime_facts_not_persisted_rows() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Unavailable,
        Some("dependency missing".to_string()),
    ));

    let agent = repository.find("codex-cli").expect("query").expect("agent");
    assert_eq!(agent.availability().state(), AgentAvailability::Unavailable);
    assert_eq!(agent.availability().reason(), Some("dependency missing"));
}

#[test]
fn api_agent_registration_round_trips_and_reports_available() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));

    let input = RegisterApiAgentInput {
        display_name: "My Claude Agent".to_string(),
        provider: "Anthropic".to_string(),
        api_key: "sk-ant-unused-in-this-layer".to_string(),
        model_id: "claude-opus-4-8".to_string(),
        interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
        base_url: None,
        runtime_kind: "cloud".to_string(),
        authentication_mode: "required".to_string(),
        timeout_ms: 30_000,
        privacy_classification: "cloud".to_string(),
    };
    let registered = ApiAgentGateway::register(&repository, "my-claude-agent", &input)
        .expect("register api agent");
    assert_eq!(registered.id().as_str(), "my-claude-agent");
    assert_eq!(registered.launch().kind_str(), "api");
    assert_eq!(registered.provider(), "Anthropic");
    assert_eq!(
        registered.availability().state(),
        AgentAvailability::Available
    );
    assert_eq!(
        registered.supported_interaction_modes(),
        &[InteractionMode::Api]
    );
    assert_eq!(
        registered.origin(),
        crate::contexts::agent_runtime::domain::AgentOrigin::User
    );

    let found = repository
        .find("my-claude-agent")
        .expect("query")
        .expect("agent present");
    assert_eq!(found.display_name(), "My Claude Agent");
    assert_eq!(found.availability().state(), AgentAvailability::Available);

    assert_eq!(
        ApiAgentGateway::provider_config(&repository, "my-claude-agent").expect("provider config"),
        Some(ApiProviderConfig {
            source_provider_id: None,
            model_id: "claude-opus-4-8".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
            auto_approve_tools: false,
        })
    );

    // The seeded built-in agents are untouched by registering another API-launch-kind agent.
    let all_agents = repository.list().expect("list");
    let ids: Vec<&str> = all_agents
        .iter()
        .map(|agent| agent.id().as_str())
        .filter(|id| *id != "my-claude-agent")
        .collect();
    assert_eq!(
        ids,
        vec![
            "antigravity-cli",
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "onepiece",
            "opencode"
        ]
    );
}

#[test]
fn openai_compatible_agent_registration_persists_base_url_and_reports_available() {
    let (_directory, _database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));

    let input = RegisterApiAgentInput {
        display_name: "My DeepSeek Agent".to_string(),
        provider: "DeepSeek".to_string(),
        api_key: "sk-unused-in-this-layer".to_string(),
        model_id: "deepseek-chat".to_string(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        base_url: Some("https://api.deepseek.com/v1".to_string()),
        runtime_kind: "cloud".to_string(),
        authentication_mode: "required".to_string(),
        timeout_ms: 30_000,
        privacy_classification: "cloud".to_string(),
    };
    let registered = ApiAgentGateway::register(&repository, "my-deepseek-agent", &input)
        .expect("register api agent");
    assert_eq!(
        registered.availability().state(),
        AgentAvailability::Available
    );

    assert_eq!(
        ApiAgentGateway::provider_config(&repository, "my-deepseek-agent")
            .expect("provider config"),
        Some(ApiProviderConfig {
            source_provider_id: None,
            model_id: "deepseek-chat".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
            auto_approve_tools: false,
        })
    );
}

#[test]
fn openai_compatible_agent_missing_base_url_is_reported_unavailable() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    database
        .connection()
        .expect("connection")
        .execute(
            "INSERT INTO agents (id, display_name, provider, launch_kind, model_id, interface_format, base_url) \
             VALUES ('incomplete-openai-agent', 'Incomplete', 'DeepSeek', 'api', 'deepseek-chat', 'openai-compatible', NULL)",
            [],
        )
        .expect("insert incomplete openai-compatible agent row");

    let agent = repository
        .find("incomplete-openai-agent")
        .expect("query")
        .expect("agent present");
    assert_eq!(agent.availability().state(), AgentAvailability::Unavailable);
    assert_eq!(
        agent.availability().reason(),
        Some("API agent is missing a configured base URL.")
    );
}

#[test]
fn api_agent_missing_model_id_is_reported_unavailable_without_touching_cli_availability_gateway() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        // If the CLI availability gateway were consulted for an API agent, this would make it
        // "available" instead — proving the branch in `into_domain` is actually taken.
        AgentAvailability::Available,
        None,
    ));
    database
        .connection()
        .expect("connection")
        .execute(
            "INSERT INTO agents (id, display_name, provider, launch_kind, model_id) \
             VALUES ('incomplete-api-agent', 'Incomplete', 'Anthropic', 'api', NULL)",
            [],
        )
        .expect("insert incomplete api agent row");

    let agent = repository
        .find("incomplete-api-agent")
        .expect("query")
        .expect("agent present");
    assert_eq!(agent.availability().state(), AgentAvailability::Unavailable);
    assert_eq!(
        agent.availability().reason(),
        Some("API agent is missing a configured model.")
    );
}

#[test]
fn workflow_and_session_details_round_trip_through_singleton_rows() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let agent = repository.find("codex-cli").expect("query").expect("agent");
    let mut workflow = AgentWorkflow::new("ship refactor");
    workflow
        .select(&agent, InteractionMode::Cli)
        .expect("select");
    workflow.begin_launch().expect("start");
    workflow.mark_running().expect("running");

    repository.save(&workflow).expect("save workflow");
    let loaded = repository.load().expect("load workflow");
    assert_eq!(
        loaded.active_agent_id().map(|id| id.as_str()),
        Some("codex-cli")
    );
    assert_eq!(loaded.active_interaction_mode(), Some(InteractionMode::Cli));
    assert_eq!(loaded.lifecycle(), AgentLifecycle::Running);
    assert_eq!(loaded.intent(), "ship refactor");
    let stored = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT active_agent_id, active_interaction_mode, lifecycle_state, intent FROM workflow_state WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .expect("stored workflow");
    assert_eq!(
        stored,
        (
            Some("codex-cli".to_string()),
            Some("cli".to_string()),
            "running".to_string(),
            "ship refactor".to_string(),
        )
    );

    repository
        .save_details("cli", "CLI workflow launch routed through Tauri adapter.")
        .expect("save details");
    let (adapter, details) = repository.load_details().expect("details");
    assert_eq!(adapter, "cli");
    assert_eq!(
        details.get("message").map(String::as_str),
        Some("CLI workflow launch routed through Tauri adapter.")
    );
    assert_eq!(details.get("runtime").map(String::as_str), Some("tauri"));
}

#[test]
fn invalid_registry_modes_and_incomplete_workflows_fail_explicitly() {
    let (_directory, database, repository) = repository(AvailabilityAssessment::new(
        AgentAvailability::Available,
        None,
    ));
    let connection = database.connection().expect("connection");
    connection
        .execute("DELETE FROM agent_modes WHERE agent_id = ?1", ["codex-cli"])
        .expect("delete modes");
    connection
        .execute(
            "INSERT INTO agent_modes (agent_id, mode) VALUES (?1, ?2)",
            params!["codex-cli", "terminal"],
        )
        .expect("invalid mode");
    assert!(matches!(
        repository.find("codex-cli"),
        Err(AgentRuntimeApplicationError::Domain(_))
    ));

    connection
        .execute(
            "UPDATE workflow_state SET active_agent_id = ?1, active_interaction_mode = NULL WHERE id = 1",
            ["codex-cli"],
        )
        .expect("write incomplete workflow");
    assert!(matches!(
        repository.load(),
        Err(AgentRuntimeApplicationError::Domain(_))
    ));
}
