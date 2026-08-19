use super::*;

#[test]
fn update_api_agent_trims_fields_and_forwards_the_normalized_input_to_the_gateway() {
    let world = test_world();
    world
        .provider_config
        .lock()
        .expect("provider config")
        .replace(ApiProviderConfig {
            source_provider_id: None,
            model_id: "old-model".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
            auto_approve_tools: false,
        });

    let updated = service(world.clone())
        .update_api_agent(
            "my-api-agent",
            UpdateApiAgentInput {
                display_name: "  Renamed Agent  ".to_string(),
                model_id: "  new-model  ".to_string(),
                base_url: None,
                new_api_key: None,
            },
        )
        .expect("update");

    assert_eq!(updated.display_name, "Renamed Agent");
    let calls = world.updated_agents.lock().expect("updated agents");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "my-api-agent");
    assert_eq!(calls[0].1.display_name, "Renamed Agent");
    assert_eq!(calls[0].1.model_id, "new-model");
    assert_eq!(calls[0].1.base_url, None);
    assert_eq!(calls[0].1.new_api_key, None);
}

#[test]
fn update_api_agent_rejects_a_missing_base_url_when_the_stored_format_is_openai_compatible() {
    let world = test_world();
    world
        .provider_config
        .lock()
        .expect("provider config")
        .replace(ApiProviderConfig {
            source_provider_id: None,
            model_id: "old-model".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some("https://old.example.test".to_string()),
            auto_approve_tools: false,
        });

    let result = service(world.clone()).update_api_agent(
        "my-api-agent",
        UpdateApiAgentInput {
            display_name: "Renamed".to_string(),
            model_id: "new-model".to_string(),
            base_url: None,
            new_api_key: None,
        },
    );

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    assert!(world
        .updated_agents
        .lock()
        .expect("updated agents")
        .is_empty());
}

#[test]
fn update_api_agent_rotating_the_key_stores_it_without_touching_other_fields() {
    let world = test_world();
    world
        .provider_config
        .lock()
        .expect("provider config")
        .replace(ApiProviderConfig {
            source_provider_id: None,
            model_id: "gpt-test".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
            auto_approve_tools: false,
        });

    service(world.clone())
        .update_api_agent(
            "my-api-agent",
            UpdateApiAgentInput {
                display_name: "My Agent".to_string(),
                model_id: "gpt-test".to_string(),
                base_url: None,
                new_api_key: Some("sk-new-key".to_string()),
            },
        )
        .expect("update");

    let stored = world.stored_credentials.lock().expect("stored credentials");
    assert_eq!(
        stored.as_slice(),
        [("my-api-agent".to_string(), "sk-new-key".to_string())]
    );
    // The rotated key never reaches the gateway — it's an OS-keychain concern, not a DB column.
    let calls = world.updated_agents.lock().expect("updated agents");
    assert_eq!(calls[0].1.new_api_key, None);
}

#[test]
fn delete_api_agent_removes_the_stored_credential_after_a_successful_delete() {
    let world = test_world();

    service(world.clone())
        .delete_api_agent("my-api-agent")
        .expect("delete");

    assert_eq!(
        *world.deleted_agent_ids.lock().expect("deleted agent ids"),
        vec!["my-api-agent".to_string()]
    );
    assert_eq!(
        *world
            .removed_credentials
            .lock()
            .expect("removed credentials"),
        vec!["my-api-agent".to_string()]
    );
}

#[test]
fn delete_api_agent_does_not_touch_the_credential_when_the_gateway_rejects_the_delete() {
    let world = test_world();
    world.delete_api_agent_failure.store(true, Ordering::SeqCst);

    let result = service(world.clone()).delete_api_agent("my-api-agent");

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    assert!(world
        .deleted_agent_ids
        .lock()
        .expect("deleted agent ids")
        .is_empty());
    assert!(world
        .removed_credentials
        .lock()
        .expect("removed credentials")
        .is_empty());
}
