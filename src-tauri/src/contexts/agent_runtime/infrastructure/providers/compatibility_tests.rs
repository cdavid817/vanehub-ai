use super::compatibility::{builtin_cli_provider_registry, fixture_provider};
use crate::contexts::agent_runtime::application::{
    AgentProvider, AgentProviderError, ProviderGenerationInvocationRequest, ProviderOptionRequest,
    ProviderPermissionMode, ProviderPromptDelivery, ProviderRegistry,
};
use crate::contexts::agent_runtime::domain::{
    AgentProviderId, InteractionMode, ProviderCapability, ProviderFamily, ProviderHealth,
    ProviderSessionRef, ProviderUsageCapability,
};
use std::sync::Arc;
use std::time::Instant;

fn assert_provider_conformance(provider: &Arc<dyn AgentProvider>) {
    let id = provider.metadata().id().as_str();
    assert!(!id.is_empty());
    assert_eq!(provider.metadata().family(), ProviderFamily::CodingCli);
    assert_eq!(
        provider.capabilities().interaction_modes(),
        &[InteractionMode::Cli]
    );
    assert!(!provider
        .readiness_prerequisites()
        .executable_names()
        .is_empty());
    assert!(!provider.version_probe().args().is_empty());
    assert!(provider.version_probe().timeout_ms() <= 15_000);
    assert!(provider.parser_policy().max_buffer_bytes() <= 1_048_576);
    assert!(provider.cancellation_policy().grace_period_ms() <= 30_000);
    assert!(provider.cancellation_policy().uses_process_tree());
    assert_eq!(provider.classify_health(true, true), ProviderHealth::Ready);
    assert_eq!(
        provider.classify_health(false, false),
        ProviderHealth::Unavailable
    );
    assert!(provider
        .map_options(ProviderOptionRequest {
            permission: Some(ProviderPermissionMode::Standard),
            model: None,
            reasoning: None,
        })
        .is_ok());

    let invocation = provider
        .prepare_generation(ProviderGenerationInvocationRequest {
            executable: provider.readiness_prerequisites().executable_names()[0].clone(),
            prompt: "fixture prompt",
            provider_session: None,
            global_args: &[],
            invocation_args: &[],
            role_briefing: None,
        })
        .expect("generation mapping");
    assert!(!invocation.executable.is_empty());
    let prompt_arguments = invocation
        .args
        .iter()
        .filter(|arg| arg.as_str() == "fixture prompt")
        .count();
    match invocation.prompt_delivery {
        ProviderPromptDelivery::Stdin => assert_eq!(prompt_arguments, 0),
        ProviderPromptDelivery::Argument => assert_eq!(prompt_arguments, 1),
    }
}

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
            global_args: &[],
            invocation_args: &[],
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
            global_args: &[],
            invocation_args: &[],
            role_briefing: None,
        }),
        Err(AgentProviderError::Preparation { .. })
    ));
}

#[test]
fn mandatory_conformance_harness_covers_builtins_and_test_fixture() {
    let registry = builtin_cli_provider_registry().expect("registry");
    for provider in registry.list() {
        assert_provider_conformance(&provider);
    }

    let fixture = fixture_provider();
    assert_provider_conformance(&fixture);
    let fixture_registry = ProviderRegistry::new(vec![fixture]).expect("fixture registry");
    assert!(fixture_registry.get("fixture-cli").is_ok());
    assert!(builtin_cli_provider_registry()
        .expect("production registry")
        .get("fixture-cli")
        .is_err());
    assert!(matches!(
        fixture_registry.require("fixture-cli", ProviderCapability::Reasoning),
        Err(AgentProviderError::UnsupportedCapability { .. })
    ));
}

#[test]
fn provider_sdk_fixed_fixture_benchmark() {
    let registry = builtin_cli_provider_registry().expect("registry");
    let iterations = 10_000;
    let started = Instant::now();
    for index in 0..iterations {
        let id = if index % 2 == 0 {
            "codex-cli"
        } else {
            "claude-code"
        };
        assert!(registry
            .require(id, ProviderCapability::StructuredOutput)
            .is_ok());
    }
    let elapsed = started.elapsed();
    eprintln!(
        "provider_sdk registry_fixture=5 iterations={iterations} elapsed={elapsed:?} target={} arch={}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    assert_eq!(registry.list().len(), 5);
}

#[cfg(unix)]
#[test]
fn fake_cli_desktop_integration_streams_session_usage_failure_and_cancels() {
    use super::{output_parser_for_format, BoundedProviderLines, ProviderOutputEvent};
    use crate::contexts::agent_runtime::application::ProviderOutputFormat;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let script = concat!(
        "printf '%s\\n' '",
        r#"{"type":"session_init","session_id":"fixture-session"}"#,
        "' '",
        r#"{"type":"message","text":"fixture token"}"#,
        "' '",
        r#"{"type":"turn.completed","usage":{"input_tokens":5,"output_tokens":3,"total_tokens":8}}"#,
        "'"
    );
    let output = Command::new("sh")
        .args(["-c", script])
        .output()
        .expect("launch fake CLI");
    assert!(output.status.success());
    let parser = output_parser_for_format(ProviderOutputFormat::StructuredJsonLines);
    let events = BoundedProviderLines::new(output.stdout.as_slice(), 4096)
        .map(|line| parser.parse_line(&line.expect("bounded fake CLI output")))
        .collect::<Vec<_>>();
    assert!(events.contains(&ProviderOutputEvent::SessionId(
        "fixture-session".to_string()
    )));
    assert!(events.contains(&ProviderOutputEvent::Token("fixture token".to_string())));
    assert!(matches!(
        events.last(),
        Some(ProviderOutputEvent::Completed(Some(usage)))
            if usage.input_tokens == 5 && usage.output_tokens == 3
    ));

    let mut child = Command::new("sh")
        .args(["-c", "sleep 30"])
        .stdout(Stdio::null())
        .spawn()
        .expect("launch cancellable fake CLI");
    thread::sleep(Duration::from_millis(20));
    child.kill().expect("bounded cancellation");
    assert!(!child.wait().expect("wait after cancellation").success());

    let failure = parser.parse_line(
        r#"{"type":"error","error":{"code":"permission_denied","message":"safe fixture failure"}}"#,
    );
    assert!(matches!(failure, ProviderOutputEvent::Failed(_)));
}
