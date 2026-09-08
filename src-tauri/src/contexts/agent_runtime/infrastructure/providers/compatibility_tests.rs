use super::compatibility::{builtin_cli_provider_registry, fixture_provider};
use super::definitions::{definition, EXPANDED_PROVIDER_IDS};
use crate::contexts::agent_runtime::application::{
    AgentProvider, AgentProviderError, ProviderAcpInvocationRequest,
    ProviderGenerationInvocationRequest, ProviderInteractiveInvocationRequest,
    ProviderOptionRequest, ProviderPermissionMode, ProviderPromptDelivery, ProviderRegistry,
};
use crate::contexts::agent_runtime::domain::{
    AgentProviderId, InteractionMode, ProviderCapability, ProviderFamily, ProviderHealth,
    ProviderSessionRef, ProviderTransport, ProviderUsageCapability,
};
use std::sync::Arc;
use std::time::Instant;

const ORIGINAL_FIVE: [&str; 5] = [
    "antigravity-cli",
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
];

/// The mandatory contract, applied per declared transport. Common concerns run for everyone;
/// each transport's concerns run only where the transport is declared, and the negative case (a
/// transport that is not declared must be refused) always runs.
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
    let capabilities = provider.capabilities();
    assert!(capabilities.supports_transport(ProviderTransport::Terminal));
    let permission_mapping = provider.map_options(ProviderOptionRequest {
        permission: Some(ProviderPermissionMode::Standard),
        model: None,
        reasoning: None,
    });
    if capabilities.permissions() {
        assert!(permission_mapping.is_ok(), "{id}");
    } else {
        assert!(
            matches!(
                permission_mapping,
                Err(AgentProviderError::UnsupportedCapability { .. })
            ),
            "{id}"
        );
    }

    // Every provider has a terminal.
    let executable = provider.readiness_prerequisites().executable_names()[0].clone();
    let interactive = provider
        .prepare_interactive(ProviderInteractiveInvocationRequest {
            executable: executable.clone(),
            provider_session: None,
            global_args: &[],
            invocation_args: &[],
        })
        .expect("interactive mapping");
    assert_eq!(interactive.executable, executable, "{id}");

    let generation = provider.prepare_generation(ProviderGenerationInvocationRequest {
        executable: executable.clone(),
        prompt: "fixture prompt",
        provider_session: None,
        global_args: &[],
        invocation_args: &[],
        role_briefing: None,
    });
    if capabilities.supports_transport(ProviderTransport::Headless) {
        let invocation = generation.expect("generation mapping");
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
    } else {
        assert!(
            matches!(
                generation,
                Err(AgentProviderError::UnsupportedCapability { ref capability, .. })
                    if capability == "headless-generation"
            ),
            "{id} must refuse headless generation"
        );
    }

    let acp = provider.prepare_acp(ProviderAcpInvocationRequest {
        executable: executable.clone(),
        global_args: &["--model".to_string(), "fixture".to_string()],
        invocation_args: &[],
        account_profile: None,
    });
    if capabilities.supports_transport(ProviderTransport::AcpStdio) {
        let spec = acp.expect("acp mapping");
        assert_eq!(spec.executable, executable);
        // The prompt never appears on argv; the profile tokens precede the ACP entry.
        assert!(!spec.args.iter().any(|arg| arg == "fixture prompt"));
        assert_eq!(&spec.args[..2], &["--model", "fixture"]);
        assert!(spec.args.len() > 2, "{id} needs an ACP entry token");
        assert!(!spec.adapter_revision.is_empty());
    } else {
        assert!(
            matches!(
                acp,
                Err(AgentProviderError::UnsupportedCapability { ref capability, .. })
                    if capability == "acp-stdio"
            ),
            "{id} must refuse ACP"
        );
    }
}

#[test]
fn builtins_have_complete_deterministic_contracts() {
    let registry = builtin_cli_provider_registry().expect("registry");
    let providers = registry.list();
    let mut expected: Vec<&str> = ORIGINAL_FIVE
        .iter()
        .chain(EXPANDED_PROVIDER_IDS.iter())
        .copied()
        .collect();
    expected.sort_unstable();
    assert_eq!(
        providers
            .iter()
            .map(|provider| provider.metadata().id().as_str())
            .collect::<Vec<_>>(),
        expected
    );
    for provider in providers {
        assert!(!provider.metadata().display_name().is_empty());
        assert_eq!(provider.metadata().family(), ProviderFamily::CodingCli);
        assert_eq!(
            provider.capabilities().interaction_modes(),
            &[InteractionMode::Cli]
        );
        assert!(provider.capabilities().terminal());
        assert!(!provider
            .readiness_prerequisites()
            .executable_names()
            .is_empty());
    }
    // The original five keep every declaration they had before the expansion.
    for id in ORIGINAL_FIVE {
        let provider = registry.get(id).expect(id);
        assert!(provider.capabilities().session_resume(), "{id}");
        assert!(provider.capabilities().structured_output(), "{id}");
        assert!(provider.capabilities().permissions(), "{id}");
        assert!(provider.capabilities().model_selection(), "{id}");
        assert_eq!(
            provider.capabilities().transports(),
            &[ProviderTransport::Terminal, ProviderTransport::Headless],
            "{id}"
        );
        assert!(provider.capabilities().usage().is_reported(), "{id}");
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
    // The additions declare usage unavailable and are never marked as reporting it.
    for id in EXPANDED_PROVIDER_IDS {
        let provider = registry.get(id).expect(id);
        assert_eq!(
            provider.capabilities().usage(),
            ProviderUsageCapability::Unavailable,
            "{id}"
        );
        assert!(matches!(
            registry.require(id, ProviderCapability::Usage),
            Err(AgentProviderError::UnsupportedCapability { .. })
        ));
    }
    let iflow = registry.get("iflow-cli").expect("iflow");
    assert_eq!(
        iflow.capabilities().transports(),
        &[ProviderTransport::Terminal]
    );
    assert_eq!(iflow.capabilities().managed_transport(), None);
    assert!(!iflow.capabilities().permissions());
    assert!(definition("iflow-cli").expect("iflow").is_legacy());
    for id in [
        "qwen-code",
        "kimi-cli",
        "qoder-cli",
        "codebuddy-code",
        "copilot-cli",
        "cursor-agent-cli",
    ] {
        assert_eq!(
            registry
                .get(id)
                .expect(id)
                .capabilities()
                .managed_transport(),
            Some(ProviderTransport::AcpStdio),
            "{id}"
        );
    }
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
fn acp_account_profiles_are_validated_and_scoped_to_the_process_environment() {
    let registry = builtin_cli_provider_registry().expect("registry");
    let codebuddy = registry.get("codebuddy-code").expect("codebuddy");
    let china = codebuddy
        .prepare_acp(ProviderAcpInvocationRequest {
            executable: "codebuddy".to_string(),
            global_args: &[],
            invocation_args: &[],
            account_profile: Some("china"),
        })
        .expect("china profile");
    assert_eq!(
        china.environment.get("CODEBUDDY_INTERNET_ENVIRONMENT"),
        Some(&"internal".to_string())
    );
    assert_eq!(china.args, vec!["--acp"]);
    let international = codebuddy
        .prepare_acp(ProviderAcpInvocationRequest {
            executable: "codebuddy".to_string(),
            global_args: &[],
            invocation_args: &[],
            account_profile: Some("international"),
        })
        .expect("international profile");
    assert!(international.environment.is_empty());
    assert!(matches!(
        codebuddy.prepare_acp(ProviderAcpInvocationRequest {
            executable: "codebuddy".to_string(),
            global_args: &[],
            invocation_args: &[],
            account_profile: Some("made-up"),
        }),
        Err(AgentProviderError::Preparation { .. })
    ));
    // A vendor with one environment refuses a profile it never documented.
    let qwen = registry.get("qwen-code").expect("qwen");
    assert!(qwen
        .prepare_acp(ProviderAcpInvocationRequest {
            executable: "qwen".to_string(),
            global_args: &[],
            invocation_args: &[],
            account_profile: Some("china"),
        })
        .is_err());
    let copilot = registry
        .get("copilot-cli")
        .expect("copilot")
        .prepare_acp(ProviderAcpInvocationRequest {
            executable: "copilot".to_string(),
            global_args: &["--effort=max".to_string()],
            invocation_args: &[],
            account_profile: None,
        })
        .expect("copilot");
    assert_eq!(copilot.args, vec!["--effort=max", "--acp", "--stdio"]);
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
        "provider_sdk registry_fixture=12 iterations={iterations} elapsed={elapsed:?} target={} arch={}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    assert_eq!(registry.list().len(), 12);
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
