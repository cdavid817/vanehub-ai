use super::*;

#[test]
fn resolve_embedding_endpoint_returns_the_saved_profiles_endpoint_and_credential() {
    let world = test_world();
    let runtime = service(world.clone());
    let saved = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("deepseek-embeddings".to_string()),
            name: "DeepSeek embeddings".to_string(),
            provider_id: "deepseek".to_string(),
            endpoint_type: "openai-chat-completions".to_string(),
            model_id: "deepseek-chat".to_string(),
            api_key: Some("sk-embed-secret".to_string()),
        })
        .expect("save profile");
    let profile_id = saved.profiles[0].id.clone();

    let resolved = runtime
        .resolve_embedding_endpoint(&profile_id)
        .expect("resolve endpoint");

    assert_eq!(resolved.base_url, "https://api.deepseek.com/v1");
    assert_eq!(
        resolved.interface_format,
        INTERFACE_FORMAT_OPENAI_COMPATIBLE
    );
    assert_eq!(resolved.credential, "sk-embed-secret");
}

#[test]
fn resolve_embedding_endpoint_rejects_an_unknown_profile() {
    let result = service(test_world()).resolve_embedding_endpoint("missing-profile");

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn resolve_embedding_endpoint_rejects_a_non_openai_compatible_profile() {
    let world = test_world();
    let runtime = service(world.clone());
    let saved = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("anthropic-primary".to_string()),
            name: "Anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "claude-test".to_string(),
            api_key: Some("sk-anthropic".to_string()),
        })
        .expect("save profile");
    let profile_id = saved.profiles[0].id.clone();

    let result = runtime.resolve_embedding_endpoint(&profile_id);

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn list_embedding_models_keeps_only_embedding_models_and_prefers_the_transient_credential() {
    let world = test_world();
    let runtime = service(world.clone());
    let saved = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("deepseek-embeddings".to_string()),
            name: "DeepSeek embeddings".to_string(),
            provider_id: "deepseek".to_string(),
            endpoint_type: "openai-chat-completions".to_string(),
            model_id: "deepseek-chat".to_string(),
            api_key: Some("sk-profile-secret".to_string()),
        })
        .expect("save profile");
    let profile_id = saved.profiles[0].id.clone();

    let models = runtime
        .list_embedding_models(&profile_id, None)
        .expect("list embedding models using the stored credential");

    assert_eq!(
        models,
        vec![OnePieceProviderModelOption {
            id: "test-embedding-model".to_string(),
            display_name: "Embedding".to_string(),
            source: "api".to_string(),
        }]
    );
    assert_eq!(
        world
            .last_model_discovery_request
            .lock()
            .expect("last request")
            .as_ref()
            .expect("a request was made")
            .api_key,
        "sk-profile-secret"
    );

    runtime
        .list_embedding_models(&profile_id, Some("sk-transient-secret"))
        .expect("list embedding models using the transient credential");

    assert_eq!(
        world
            .last_model_discovery_request
            .lock()
            .expect("last request")
            .as_ref()
            .expect("a request was made")
            .api_key,
        "sk-transient-secret"
    );
}

#[test]
fn list_embedding_models_rejects_an_unknown_profile() {
    let result = service(test_world()).list_embedding_models("missing-profile", None);

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn list_embedding_models_rejects_a_non_openai_compatible_profile() {
    let world = test_world();
    let runtime = service(world.clone());
    let saved = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("anthropic-primary".to_string()),
            name: "Anthropic".to_string(),
            provider_id: "anthropic".to_string(),
            endpoint_type: "anthropic-messages".to_string(),
            model_id: "claude-test".to_string(),
            api_key: Some("sk-anthropic".to_string()),
        })
        .expect("save profile");
    let profile_id = saved.profiles[0].id.clone();

    let result = runtime.list_embedding_models(&profile_id, None);

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn list_embedding_models_requires_a_transient_or_stored_credential() {
    let world = test_world();
    world
        .onepiece_profiles
        .lock()
        .expect("onepiece profiles")
        .push(StoredOnePieceProviderProfile {
            id: "credentialless".to_string(),
            name: "Credential-less profile".to_string(),
            source_preset_id: Some("deepseek".to_string()),
            source_provider_id: Some("deepseek".to_string()),
            source_endpoint_type: Some("openai-chat-completions".to_string()),
            source_preset_version: Some(1),
            provider: "DeepSeek".to_string(),
            model_id: "deepseek-chat".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
            active: false,
        });

    let result = service(world).list_embedding_models("credentialless", None);

    assert!(matches!(
        result,
        Err(AgentRuntimeApplicationError::Validation(_))
    ));
}

#[test]
fn list_embedding_models_propagates_discovery_failures_without_leaking_the_credential() {
    let world = test_world();
    world.model_discovery_failure.store(true, Ordering::SeqCst);
    let runtime = service(world.clone());
    let saved = runtime
        .save_onepiece_provider_profile(SaveOnePieceProviderProfileInput {
            id: Some("deepseek-embeddings".to_string()),
            name: "DeepSeek embeddings".to_string(),
            provider_id: "deepseek".to_string(),
            endpoint_type: "openai-chat-completions".to_string(),
            model_id: "deepseek-chat".to_string(),
            api_key: Some("sk-never-log-this".to_string()),
        })
        .expect("save profile");
    let profile_id = saved.profiles[0].id.clone();

    let result = runtime.list_embedding_models(&profile_id, None);

    let Err(error) = result else {
        panic!("expected the simulated discovery failure to propagate");
    };
    assert!(!error.to_string().contains("sk-never-log-this"));
}
