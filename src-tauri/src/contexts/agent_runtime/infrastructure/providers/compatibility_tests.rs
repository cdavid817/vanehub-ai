use super::compatibility::builtin_cli_provider_registry;
use crate::contexts::agent_runtime::application::{
    AgentProviderError, ProviderGenerationInvocationRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentProviderId, InteractionMode, ProviderFamily, ProviderSessionRef, ProviderUsageCapability,
};

#[test]
fn builtins_have_complete_deterministic_contracts() {
    let registry = builtin_cli_provider_registry().expect("registry");
    let providers = registry.list();
    assert_eq!(
        providers
            .iter()
            .map(|provider| provider.metadata().id().as_str())
            .collect::<Vec<_>>(),
        vec![
            "antigravity-cli",
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode"
        ]
    );
    for provider in providers {
        assert!(!provider.metadata().display_name().is_empty());
        assert_eq!(provider.metadata().family(), ProviderFamily::CodingCli);
        assert_eq!(
            provider.capabilities().interaction_modes(),
            &[InteractionMode::Cli]
        );
        assert!(provider.capabilities().session_resume());
        assert!(provider.capabilities().structured_output());
        assert!(provider.capabilities().terminal());
        assert!(provider.capabilities().permissions());
        assert!(provider.capabilities().model_selection());
        assert!(!provider
            .readiness_prerequisites()
            .executable_names()
            .is_empty());
    }
}

#[test]
fn capabilities_are_declared_instead_of_inferred_from_ids() {
    let registry = builtin_cli_provider_registry().expect("registry");
    assert!(registry
        .get("codex-cli")
        .expect("codex")
        .capabilities()
        .sandbox());
    assert!(!registry
        .get("opencode")
        .expect("opencode")
        .capabilities()
        .sandbox());
    assert!(registry
        .get("claude-code")
        .expect("claude")
        .capabilities()
        .reasoning());
    assert!(!registry
        .get("gemini-cli")
        .expect("gemini")
        .capabilities()
        .reasoning());
    assert_eq!(
        registry
            .get("antigravity-cli")
            .expect("agy")
            .capabilities()
            .usage(),
        ProviderUsageCapability::HeadlessReported
    );
}

#[test]
fn compatibility_provider_preserves_invocation_and_session_ownership() {
    let registry = builtin_cli_provider_registry().expect("registry");
    let codex = registry.get("codex-cli").expect("codex");
    let provider_session = ProviderSessionRef::new(
        AgentProviderId::parse("codex-cli").expect("id"),
        "session-1",
    )
    .expect("session");
    let invocation = codex
        .prepare_generation(ProviderGenerationInvocationRequest {
            executable: "codex".to_string(),
            prompt: "hello",
            provider_session: Some(&provider_session),
            managed_args: &[],
            role_briefing: None,
        })
        .expect("invocation");
    assert_eq!(
        invocation.args,
        vec!["exec", "resume", "session-1", "--json", "-"]
    );

    let foreign_session =
        ProviderSessionRef::new(AgentProviderId::parse("gemini-cli").expect("id"), "foreign")
            .expect("session");
    assert!(matches!(
        codex.prepare_generation(ProviderGenerationInvocationRequest {
            executable: "codex".to_string(),
            prompt: "hello",
            provider_session: Some(&foreign_session),
            managed_args: &[],
            role_briefing: None,
        }),
        Err(AgentProviderError::Preparation { .. })
    ));
}
