use super::*;

#[test]
fn resolve_tool_approval_returns_false_without_an_active_generation() {
    let world = test_world();
    let resolved = service(world.clone())
        .resolve_tool_approval("session-1", "call-1", ToolApprovalDecision::Approved)
        .expect("resolve");
    assert!(!resolved);
    assert!(world
        .resolved_approvals
        .lock()
        .expect("resolved approvals")
        .is_empty());
}

#[test]
fn resolve_tool_approval_delegates_to_the_active_generations_process_id() {
    let world = test_world();
    let service_instance = service(world.clone());
    service_instance
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");

    let resolved = service_instance
        .resolve_tool_approval("session-1", "call-1", ToolApprovalDecision::Approved)
        .expect("resolve");
    assert!(resolved);
    assert_eq!(
        *world.resolved_approvals.lock().expect("resolved approvals"),
        vec![(
            "process-1".to_string(),
            "call-1".to_string(),
            ToolApprovalDecision::Approved
        )]
    );
}

#[test]
fn prompt_execution_without_fired_versions_records_no_observation() {
    let world = test_world();
    world.no_prompt_versions.store(true, Ordering::SeqCst);
    service(world.clone())
        .send_message(SendMessageRequest {
            source: AgentMessageSource::Desktop,
            session_id: "session-1".to_string(),
            content: "hello".to_string(),
            configuration: chat_configuration(),
            file_references: Vec::new(),
        })
        .expect("send");
    world
        .generation_sinks
        .lock()
        .expect("generation sinks")
        .get("process-1")
        .cloned()
        .expect("sink")
        .handle(GenerationProcessEvent::Completed(None))
        .expect("complete");

    assert!(world
        .prompt_reports
        .lock()
        .expect("prompt reports")
        .is_empty());
}

#[test]
fn custom_local_profile_preserves_metadata_and_needs_no_credential() {
    let world = test_world();
    let runtime = service(world.clone());
    let overview = runtime
        .save_custom_onepiece_provider_profile(SaveCustomOnePieceProviderProfileInput {
            id: None,
            name: "Local Qwen".to_string(),
            base_url: "http://127.0.0.1:11434/v1/".to_string(),
            model_id: "qwen-local".to_string(),
            runtime_kind: "local".to_string(),
            authentication_mode: "none".to_string(),
            api_key: None,
            timeout_ms: 30_000,
            privacy_classification: "local".to_string(),
            tool_calling_capability: "unsupported".to_string(),
            image_input_capability: "unknown".to_string(),
            structured_output_capability: "unknown".to_string(),
            reasoning_field_capability: "unknown".to_string(),
            context_window_tokens: Some(32_768),
            reserved_output_tokens: 4_096,
        })
        .expect("save custom local profile");
    assert_eq!(world.credential_reads.load(Ordering::SeqCst), 0);
    assert!(world
        .removed_credentials
        .lock()
        .expect("removed credentials")
        .is_empty());
    let profile = &overview.profiles[0];
    assert!(profile.active);
    assert!(!profile.credential_present);
    assert_eq!(profile.provider, "Local endpoint");
    let metadata = runtime
        .endpoint_profile_metadata(&profile.id)
        .expect("metadata")
        .expect("stored metadata");
    assert_eq!(metadata.runtime_kind, "local");
    assert_eq!(metadata.capability_provenance, "configured");
    assert_eq!(metadata.context_capacity_provenance, "configured-estimate");
    runtime
        .replace_hybrid_routing_rules(vec![StoredHybridRoutingRule {
            id: "summary-local".to_string(),
            enabled: true,
            position: 0,
            task_class: "summarization".to_string(),
            preferred_profile_id: profile.id.clone(),
            fallback_profile_id: None,
            data_policy: "local-only".to_string(),
        }])
        .expect("save route");
    let frozen = runtime
        .freeze_endpoint_profile("onepiece", "Summarize this safely")
        .expect("route")
        .expect("frozen Profile");
    assert_eq!(frozen.profile_id, profile.id);
    assert_eq!(frozen.routing_rule_id.as_deref(), Some("summary-local"));
    assert_eq!(frozen.routing_reason, "rule-preferred");
    assert_eq!(frozen.context_window_tokens, Some(32_768));
    runtime
        .activate_onepiece_provider_profile(&profile.id)
        .expect("credential-free activation");
    assert_eq!(world.credential_reads.load(Ordering::SeqCst), 0);
    assert!(world
        .removed_credentials
        .lock()
        .expect("removed credentials")
        .is_empty());
}

#[test]
fn custom_local_profile_rejects_non_loopback_location() {
    let runtime = service(test_world());
    let invalid =
        runtime.save_custom_onepiece_provider_profile(SaveCustomOnePieceProviderProfileInput {
            id: None,
            name: "Unsafe local".to_string(),
            base_url: "http://192.168.1.7:11434".to_string(),
            model_id: "model".to_string(),
            runtime_kind: "local".to_string(),
            authentication_mode: "none".to_string(),
            api_key: None,
            timeout_ms: 30_000,
            privacy_classification: "local".to_string(),
            tool_calling_capability: "unknown".to_string(),
            image_input_capability: "unknown".to_string(),
            structured_output_capability: "unknown".to_string(),
            reasoning_field_capability: "unknown".to_string(),
            context_window_tokens: None,
            reserved_output_tokens: 0,
        });
    assert!(matches!(
        invalid,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn local_api_agent_accepts_explicit_no_auth_while_cloud_stays_authenticated() {
    let runtime = service(test_world());
    let local = runtime
        .register_api_agent(RegisterApiAgentInput {
            display_name: "Local API Agent".to_string(),
            provider: "OpenAI-compatible".to_string(),
            api_key: String::new(),
            model_id: "local-model".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some("http://127.0.0.1:8000/v1".to_string()),
            runtime_kind: "local".to_string(),
            authentication_mode: "none".to_string(),
            timeout_ms: 5_000,
            privacy_classification: "local".to_string(),
        })
        .expect("register unauthenticated local API Agent");
    assert_eq!(local.display_name, "Local API Agent");

    let cloud = runtime.register_api_agent(RegisterApiAgentInput {
        display_name: "Unsafe cloud Agent".to_string(),
        provider: "OpenAI-compatible".to_string(),
        api_key: String::new(),
        model_id: "cloud-model".to_string(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        base_url: Some("https://api.example.test/v1".to_string()),
        runtime_kind: "cloud".to_string(),
        authentication_mode: "none".to_string(),
        timeout_ms: 5_000,
        privacy_classification: "cloud".to_string(),
    });
    assert!(matches!(
        cloud,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}
