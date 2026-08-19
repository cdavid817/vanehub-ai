use super::*;

#[test]
fn onepiece_first_configuration_normalizes_fields_and_never_returns_the_secret() {
    let world = test_world();

    let configured = service(world.clone())
        .save_onepiece_provider_config(SaveOnePieceProviderConfigInput {
            provider: "  Anthropic  ".to_string(),
            model_id: "  claude-sonnet-test  ".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
            api_key: Some("  sk-secret  ".to_string()),
        })
        .expect("configure OnePiece");

    assert_eq!(configured.provider, "Anthropic");
    assert_eq!(configured.model_id.as_deref(), Some("claude-sonnet-test"));
    assert_eq!(
        configured.interface_format.as_deref(),
        Some(INTERFACE_FORMAT_ANTHROPIC)
    );
    assert_eq!(configured.base_url, None);
    assert!(configured.credential_present);
    assert_eq!(
        world
            .current_onepiece_credential
            .lock()
            .expect("onepiece credential")
            .as_deref(),
        Some("sk-secret")
    );
}

#[test]
fn onepiece_provider_and_interface_can_be_replaced_on_the_stable_identity() {
    let world = test_world();
    *world
        .current_onepiece_credential
        .lock()
        .expect("onepiece credential") = Some("sk-existing".to_string());

    let configured = service(world.clone())
        .save_onepiece_provider_config(SaveOnePieceProviderConfigInput {
            provider: "OpenAI Proxy".to_string(),
            model_id: "gpt-test".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some(" https://gateway.example.test/v1/ ".to_string()),
            api_key: None,
        })
        .expect("replace provider");

    assert_eq!(configured.provider, "OpenAI Proxy");
    assert_eq!(configured.model_id.as_deref(), Some("gpt-test"));
    assert_eq!(
        configured.interface_format.as_deref(),
        Some(INTERFACE_FORMAT_OPENAI_COMPATIBLE)
    );
    assert_eq!(
        configured.base_url.as_deref(),
        Some("https://gateway.example.test/v1/")
    );
    assert!(configured.credential_present);
    assert!(world
        .stored_credentials
        .lock()
        .expect("stored credentials")
        .is_empty());
}

#[test]
fn onepiece_configuration_rejects_invalid_or_credentialless_first_setup() {
    let world = test_world();
    let application = service(world.clone());

    let missing_key = application.save_onepiece_provider_config(SaveOnePieceProviderConfigInput {
        provider: "Anthropic".to_string(),
        model_id: "claude-test".to_string(),
        interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
        base_url: None,
        api_key: None,
    });
    let missing_url = application.save_onepiece_provider_config(SaveOnePieceProviderConfigInput {
        provider: "OpenAI".to_string(),
        model_id: "gpt-test".to_string(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        base_url: None,
        api_key: Some("sk-secret".to_string()),
    });

    assert!(matches!(
        missing_key,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    assert!(matches!(
        missing_url,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    assert!(world
        .stored_credentials
        .lock()
        .expect("stored credentials")
        .is_empty());
}

#[test]
fn onepiece_configuration_restores_the_previous_credential_on_persistence_failure() {
    let world = test_world();
    *world
        .current_onepiece_credential
        .lock()
        .expect("onepiece credential") = Some("sk-old".to_string());
    world.save_onepiece_failure.store(true, Ordering::SeqCst);

    let result =
        service(world.clone()).save_onepiece_provider_config(SaveOnePieceProviderConfigInput {
            provider: "Anthropic".to_string(),
            model_id: "claude-test".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
            api_key: Some("sk-new".to_string()),
        });

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Registry(_))
    ));
    assert_eq!(
        world
            .current_onepiece_credential
            .lock()
            .expect("onepiece credential")
            .as_deref(),
        Some("sk-old")
    );
    assert_eq!(
        world
            .stored_credentials
            .lock()
            .expect("stored credentials")
            .as_slice(),
        [
            ("onepiece".to_string(), "sk-new".to_string()),
            ("onepiece".to_string(), "sk-old".to_string())
        ]
    );
}

#[test]
fn onepiece_reset_clears_provider_state_trust_and_credential() {
    let world = test_world();
    *world.onepiece_config.lock().expect("onepiece config") = StoredOnePieceProviderConfig {
        provider: "OpenAI".to_string(),
        model_id: Some("gpt-test".to_string()),
        interface_format: Some(INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string()),
        base_url: Some("https://gateway.example.test/v1".to_string()),
        auto_approve_tools: true,
    };
    *world
        .current_onepiece_credential
        .lock()
        .expect("onepiece credential") = Some("sk-existing".to_string());

    let reset = service(world.clone())
        .reset_onepiece_provider_config()
        .expect("reset OnePiece");

    assert_eq!(reset.provider, "VaneHub");
    assert_eq!(reset.model_id, None);
    assert_eq!(reset.interface_format, None);
    assert_eq!(reset.base_url, None);
    assert!(!reset.auto_approve_tools);
    assert!(!reset.credential_present);
    assert_eq!(
        world
            .removed_credentials
            .lock()
            .expect("removed credentials")
            .as_slice(),
        ["onepiece".to_string()]
    );
}

#[test]
fn onepiece_profiles_keep_independent_credentials_and_delete_active_without_fallback() {
    let world = test_world();
    let runtime = service(world.clone());
    let first = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("anthropic-primary".to_string()),
            name: "  Anthropic primary  ".to_string(),
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "claude-test".to_string(),
            api_key: Some("sk-anthropic".to_string()),
        })
        .expect("save first profile");
    assert_eq!(
        first.active_profile_id.as_deref(),
        Some("anthropic-primary")
    );
    assert_eq!(first.profiles[0].name, "Anthropic primary");
    assert!(first.profiles[0].credential_present);

    let second = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("deepseek-anthropic".to_string()),
            name: "DeepSeek Anthropic".to_string(),
            provider_id: "deepseek".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "deepseek-chat".to_string(),
            api_key: Some("sk-deepseek".to_string()),
        })
        .expect("save second profile");
    assert_eq!(second.profiles.len(), 2);
    assert_eq!(
        second.active_profile_id.as_deref(),
        Some("anthropic-primary")
    );
    assert!(
        !second
            .profiles
            .iter()
            .find(|profile| profile.id == "deepseek-anthropic")
            .expect("second profile")
            .active
    );
    let deepseek = second
        .profiles
        .iter()
        .find(|profile| profile.id == "deepseek-anthropic")
        .expect("DeepSeek profile");
    assert_eq!(deepseek.source_provider_id.as_deref(), Some("deepseek"));
    assert_eq!(
        deepseek.source_endpoint_type.as_deref(),
        Some("anthropic-messages")
    );
    assert_eq!(deepseek.interface_format, "anthropic");
    assert_eq!(
        deepseek.base_url.as_deref(),
        Some("https://api.deepseek.com/anthropic")
    );

    let activated = runtime
        .activate_onepiece_provider_profile("deepseek-anthropic")
        .expect("activate second profile");
    assert_eq!(
        activated.active_profile_id.as_deref(),
        Some("deepseek-anthropic")
    );
    assert_eq!(
        world
            .current_onepiece_credential
            .lock()
            .expect("runtime credential")
            .as_deref(),
        Some("sk-deepseek")
    );

    let deleted = runtime
        .delete_onepiece_provider_profile("deepseek-anthropic")
        .expect("delete active profile");
    assert_eq!(deleted.active_profile_id, None);
    assert_eq!(deleted.profiles.len(), 1);
    assert!(!deleted.profiles[0].active);
    assert_eq!(
        world
            .current_onepiece_credential
            .lock()
            .expect("runtime credential")
            .as_deref(),
        None
    );
    assert_eq!(
        world
            .profile_credentials
            .lock()
            .expect("profile credentials")
            .get("onepiece-profile:anthropic-primary")
            .map(String::as_str),
        Some("sk-anthropic")
    );
}

#[test]
fn onepiece_profile_rejects_unknown_presets_before_storing_credentials() {
    let world = test_world();

    let result =
        service(world.clone()).save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: None,
            name: "Unknown provider".to_string(),
            provider_id: "custom-provider".to_string(),
            endpoint_type: "openai-chat-completions".to_string(),
            model_id: "custom-model".to_string(),
            api_key: Some("sk-must-not-be-stored".to_string()),
        });

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    assert!(world
        .stored_credentials
        .lock()
        .expect("stored credentials")
        .is_empty());
}

#[test]
fn onepiece_profile_edit_keeps_its_catalog_provider() {
    let world = test_world();
    let runtime = service(world.clone());
    runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("stable-profile".to_string()),
            name: "Anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "claude-test".to_string(),
            api_key: Some("sk-existing".to_string()),
        })
        .expect("save initial profile");

    let result = runtime.save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
        id: Some("stable-profile".to_string()),
        name: "OpenRouter".to_string(),
        provider_id: "openrouter".to_string(),
        endpoint_type: "openai-chat-completions".to_string(),
        model_id: "gpt-test".to_string(),
        api_key: None,
    });

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
    let stored = world.onepiece_profiles.lock().expect("onepiece profiles");
    assert_eq!(stored[0].source_provider_id.as_deref(), Some("anthropic"));
    assert_eq!(
        stored[0].source_endpoint_type.as_deref(),
        Some("anthropic-messages")
    );
    assert_eq!(stored[0].provider, "Anthropic");
}

#[test]
fn onepiece_model_discovery_merges_catalog_and_api_models() {
    let world = test_world();
    let result = service(world)
        .discover_onepiece_provider_models(DiscoverOnePieceProviderModelsInput {
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            profile_id: None,
            api_key: Some("sk-transient".to_string()),
        })
        .expect("discover models");

    assert_eq!(result.provider_id, "anthropic");
    assert_eq!(result.endpoint_type, "anthropic-messages");
    assert_eq!(result.source, "merged");
    assert!(result.warning.is_none());
    assert!(result
        .models
        .iter()
        .any(|model| { model.id == "test-chat-model" && model.source == "api" }));
    assert_eq!(
        result
            .models
            .iter()
            .filter(|model| model.id == "test-chat-model")
            .count(),
        1
    );
    assert!(!result
        .models
        .iter()
        .any(|model| model.id.contains("embedding")));
    assert!(result.models.iter().any(|model| model.source == "catalog"));
}

#[test]
fn onepiece_model_discovery_requires_a_transient_or_profile_credential() {
    let result = service(test_world()).discover_onepiece_provider_models(
        DiscoverOnePieceProviderModelsInput {
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            profile_id: None,
            api_key: None,
        },
    );

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn onepiece_credential_validation_rejects_a_profile_from_another_catalog_target() {
    let world = test_world();
    let created = service(world.clone())
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: None,
            name: "Anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "claude-test".to_string(),
            api_key: Some("sk-never-send".to_string()),
        })
        .expect("save profile");
    let profile_id = created.profiles[0].id.clone();

    let result = service(world).validate_onepiece_provider_credential(
        ValidateOnePieceProviderCredentialInput {
            provider_id: "deepseek".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "deepseek-chat".to_string(),
            profile_id: Some(profile_id),
            api_key: None,
        },
    );

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn onepiece_model_discovery_falls_back_and_logs_without_the_secret() {
    let world = test_world();
    world.model_discovery_failure.store(true, Ordering::SeqCst);
    let result = service(world.clone())
        .discover_onepiece_provider_models(DiscoverOnePieceProviderModelsInput {
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            profile_id: None,
            api_key: Some("sk-never-log-this".to_string()),
        })
        .expect("catalog fallback");

    assert_eq!(result.source, "catalog");
    assert_eq!(result.warning.as_deref(), Some("live-unavailable"));
    let logs = world.logs.lock().expect("logs");
    let log = logs.last().expect("model discovery log");
    assert_eq!(log.level, AgentLogLevel::Warn);
    assert_eq!(log.category, "onepiece.model-discovery");
    assert!(!log.message.contains("sk-never-log-this"));
}
