//! The static built-in provider catalog: the original five headless CLIs, the six ACP-capable
//! additions, and the iFlow legacy terminal entry.
//!
//! Everything vendor-specific about how a provider is *started* lives here as reviewed data:
//! the ACP entry flag, the account-environment variable a vendor documents, the manifest the SDK
//! validates. Nothing here is accepted over the wire, and no entry may name an install URL, a
//! credential, or a script -- those belong to `tooling::cli` and to the user's own CLI config.
//!
//! Reviewed against the official sources listed in
//! `openspec/changes/extend-cli-providers-with-acp/references/official-sources.md` on 2026-09-06.

use crate::contexts::agent_runtime::application::ProviderOutputFormat;
use crate::contexts::agent_runtime::domain::{ProviderTransport, ProviderUsageCapability};

/// Whether a provider is an actively supported product or a legacy local-compatibility entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderLifecycle {
    Active,
    /// The vendor's official service is gone; only a locally installed CLI with user-managed
    /// configuration is supported, and only through the terminal.
    Legacy {
        /// ISO date the official service shut down, shown verbatim in the UI.
        service_shutdown: &'static str,
    },
}

/// One reviewed account/environment profile an ACP vendor distinguishes at process start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AcpAccountProfile {
    pub(crate) id: &'static str,
    /// The environment variable that selects the profile, or `None` for the vendor default.
    /// Values are documented vendor switches, never credentials.
    pub(crate) environment: Option<(&'static str, &'static str)>,
}

/// How a provider is started as an ACP stdio agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AcpLaunchGrammar {
    /// Reviewed tokens that put the executable into ACP-over-stdio mode.
    pub(crate) args: &'static [&'static str],
    /// Reviewed account profiles. Empty means the vendor has one environment.
    pub(crate) account_profiles: &'static [AcpAccountProfile],
    /// Provider extension methods the host answers. Anything else with an id is refused with a
    /// protocol error rather than awaited.
    pub(crate) blocking_extensions: &'static [&'static str],
    /// Provider extension notifications the host projects and never replies to.
    pub(crate) notification_extensions: &'static [&'static str],
}

impl AcpLaunchGrammar {
    const fn plain(args: &'static [&'static str]) -> Self {
        Self {
            args,
            account_profiles: &[],
            blocking_extensions: &[],
            notification_extensions: &[],
        }
    }
}

pub(crate) struct CompatibilityProviderDefinition {
    pub(crate) id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) executable: &'static str,
    pub(crate) managed_sdk_dependency_id: Option<&'static str>,
    pub(crate) output_format: ProviderOutputFormat,
    pub(crate) usage: ProviderUsageCapability,
    pub(crate) reasoning: bool,
    pub(crate) sandbox: bool,
    pub(crate) transports: &'static [ProviderTransport],
    pub(crate) resume: bool,
    pub(crate) model_selection: bool,
    pub(crate) permissions: bool,
    pub(crate) structured_output: bool,
    pub(crate) acp: Option<AcpLaunchGrammar>,
    pub(crate) lifecycle: ProviderLifecycle,
    /// Bumped whenever this provider's reviewed grammar or fixture expectations change. Recorded
    /// on every session binding so a stored session can be judged against the adapter that made
    /// it.
    pub(crate) adapter_revision: &'static str,
}

const HEADLESS: &[ProviderTransport] = &[ProviderTransport::Terminal, ProviderTransport::Headless];
const ACP: &[ProviderTransport] = &[ProviderTransport::Terminal, ProviderTransport::AcpStdio];
const TERMINAL_ONLY: &[ProviderTransport] = &[ProviderTransport::Terminal];

/// CodeBuddy's account environment is a process-level switch, documented as
/// `CODEBUDDY_INTERNET_ENVIRONMENT`: unset for the international site, `internal` for the China
/// site, `ioa` for Tencent iOA. It is independent of the UI language on purpose.
const CODEBUDDY_ACCOUNT_PROFILES: &[AcpAccountProfile] = &[
    AcpAccountProfile {
        id: "international",
        environment: None,
    },
    AcpAccountProfile {
        id: "china",
        environment: Some(("CODEBUDDY_INTERNET_ENVIRONMENT", "internal")),
    },
    AcpAccountProfile {
        id: "ioa",
        environment: Some(("CODEBUDDY_INTERNET_ENVIRONMENT", "ioa")),
    },
];

pub(crate) const DEFINITIONS: [CompatibilityProviderDefinition; 12] = [
    CompatibilityProviderDefinition {
        id: "claude-code",
        display_name: "Claude Code",
        executable: "claude",
        managed_sdk_dependency_id: Some("claude-sdk"),
        output_format: ProviderOutputFormat::ClaudeStreamJson,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: true,
        sandbox: false,
        transports: HEADLESS,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "headless-v1",
    },
    CompatibilityProviderDefinition {
        id: "codex-cli",
        display_name: "Codex CLI",
        executable: "codex",
        managed_sdk_dependency_id: Some("codex-sdk"),
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: true,
        sandbox: true,
        transports: HEADLESS,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "headless-v1",
    },
    CompatibilityProviderDefinition {
        id: "gemini-cli",
        display_name: "Gemini CLI",
        executable: "gemini",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: false,
        sandbox: true,
        transports: HEADLESS,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "headless-v1",
    },
    CompatibilityProviderDefinition {
        id: "opencode",
        display_name: "OpenCode",
        executable: "opencode",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: false,
        sandbox: false,
        transports: HEADLESS,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "headless-v1",
    },
    CompatibilityProviderDefinition {
        id: "antigravity-cli",
        display_name: "Antigravity CLI",
        executable: "agy",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::AntigravityStreamJson,
        usage: ProviderUsageCapability::HeadlessReported,
        reasoning: true,
        sandbox: true,
        transports: HEADLESS,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "headless-v1",
    },
    // Qwen Code: `qwen --acp` (packages/cli/src/config/config.ts, `--experimental-acp` is the
    // deprecated hidden alias). Its grammar is its own: nothing below is inherited from Gemini.
    CompatibilityProviderDefinition {
        id: "qwen-code",
        display_name: "Qwen Code",
        executable: "qwen",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar::plain(&["--acp"])),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // Kimi Code CLI: `kimi acp` prints no banner and waits for `initialize`; documents
    // `session/load`. `-p` print mode performs no human approval and is deliberately not used.
    CompatibilityProviderDefinition {
        id: "kimi-cli",
        display_name: "Kimi Code CLI",
        executable: "kimi",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar::plain(&["acp"])),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // Qoder CLI: `qoder --acp`, same login state as the interactive CLI. Windows arm64 has no
    // supported distribution; that limit is enforced by the tooling catalog, not here.
    CompatibilityProviderDefinition {
        id: "qoder-cli",
        display_name: "Qoder CLI",
        executable: "qoder",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: false,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar::plain(&["--acp"])),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // CodeBuddy Code: `codebuddy --acp`; account environment is a process-level variable; team
    // member events arrive as `_meta` on ordinary updates and stay attributed to the owning seat.
    CompatibilityProviderDefinition {
        id: "codebuddy-code",
        display_name: "CodeBuddy Code",
        executable: "codebuddy",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: false,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar {
            args: &["--acp"],
            account_profiles: CODEBUDDY_ACCOUNT_PROFILES,
            blocking_extensions: &[],
            notification_extensions: &[],
        }),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // GitHub Copilot CLI: the standalone `copilot` executable, `--acp --stdio`. Never the
    // historical `gh copilot` extension. Tool filters and effort are server-start options, so one
    // process serves exactly one binding.
    CompatibilityProviderDefinition {
        id: "copilot-cli",
        display_name: "GitHub Copilot CLI",
        executable: "copilot",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: true,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: false,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar::plain(&["--acp", "--stdio"])),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // Cursor Agent CLI: the executable is the generic `agent`, so identity is verified by the
    // tooling catalog before launch. `cursor/ask_question` and `cursor/create_plan` block until
    // answered; the todo/task/image methods are notifications.
    CompatibilityProviderDefinition {
        id: "cursor-agent-cli",
        display_name: "Cursor Agent CLI",
        executable: "agent",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: ACP,
        resume: true,
        model_selection: false,
        permissions: true,
        structured_output: true,
        acp: Some(AcpLaunchGrammar {
            args: &["acp"],
            account_profiles: &[],
            blocking_extensions: &["cursor/ask_question", "cursor/create_plan"],
            notification_extensions: &[
                "cursor/update_todos",
                "cursor/task",
                "cursor/generate_image",
            ],
        }),
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "acp-v1",
    },
    // iFlow CLI: official service and API shut down on 2026-04-17. Only a locally installed
    // program with user-managed custom API configuration is supported, terminal only.
    CompatibilityProviderDefinition {
        id: "iflow-cli",
        display_name: "iFlow CLI",
        executable: "iflow",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::Unavailable,
        reasoning: false,
        sandbox: false,
        transports: TERMINAL_ONLY,
        resume: true,
        model_selection: false,
        permissions: false,
        structured_output: false,
        acp: None,
        lifecycle: ProviderLifecycle::Legacy {
            service_shutdown: "2026-04-17",
        },
        adapter_revision: "legacy-terminal-v1",
    },
];

/// The ids added by `extend-cli-providers-with-acp`, in the order the UI ranks them after the
/// original five. Domestic CLIs first, then the two international ones, then the legacy entry.
#[cfg(test)]
pub(crate) const EXPANDED_PROVIDER_IDS: [&str; 7] = [
    "qwen-code",
    "kimi-cli",
    "qoder-cli",
    "codebuddy-code",
    "copilot-cli",
    "cursor-agent-cli",
    "iflow-cli",
];

pub(crate) fn definition(id: &str) -> Option<&'static CompatibilityProviderDefinition> {
    DEFINITIONS.iter().find(|definition| definition.id == id)
}

impl CompatibilityProviderDefinition {
    pub(crate) fn is_legacy(&self) -> bool {
        matches!(self.lifecycle, ProviderLifecycle::Legacy { .. })
    }

    #[cfg(test)]
    pub(crate) fn managed_transport(&self) -> Option<ProviderTransport> {
        if self.transports.contains(&ProviderTransport::AcpStdio) {
            Some(ProviderTransport::AcpStdio)
        } else if self.transports.contains(&ProviderTransport::Headless) {
            Some(ProviderTransport::Headless)
        } else {
            None
        }
    }

    /// The manifest this definition is validated through. The original five keep their exact
    /// version 1 document; the additions declare version 2 with explicit transports and usage.
    pub(crate) fn manifest_json(&self) -> String {
        let terminal = self.transports.contains(&ProviderTransport::Terminal);
        if self.acp.is_none() && !self.is_legacy() {
            return format!(
                r#"{{"schemaVersion":1,"id":"{}","name":"{}","runtime":"cli","executables":["{}"],"capabilities":{{"terminal":true,"resume":true,"structuredOutput":true,"images":false,"usage":true,"permissions":true,"modelSelection":true,"reasoning":{},"sandbox":{}}}}}"#,
                self.id, self.display_name, self.executable, self.reasoning, self.sandbox
            );
        }
        let transports = self
            .transports
            .iter()
            .map(|transport| format!("\"{}\"", transport.as_str()))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schemaVersion":2,"id":"{}","name":"{}","runtime":"cli","executables":["{}"],"transports":[{}],"capabilities":{{"terminal":{},"resume":{},"structuredOutput":{},"images":false,"usage":"{}","permissions":{},"modelSelection":{},"reasoning":{},"sandbox":{}}}}}"#,
            self.id,
            self.display_name,
            self.executable,
            transports,
            terminal,
            self.resume,
            self.structured_output,
            self.usage.as_str(),
            self.permissions,
            self.model_selection,
            self.reasoning,
            self.sandbox
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_keeps_the_original_five_first_and_adds_seven_stable_ids() {
        let ids: Vec<&str> = DEFINITIONS.iter().map(|definition| definition.id).collect();
        assert_eq!(
            &ids[..5],
            &[
                "claude-code",
                "codex-cli",
                "gemini-cli",
                "opencode",
                "antigravity-cli"
            ]
        );
        assert_eq!(&ids[5..], &EXPANDED_PROVIDER_IDS);
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn original_providers_keep_their_version_one_manifest_verbatim() {
        let codex = definition("codex-cli").expect("codex");
        assert_eq!(
            codex.manifest_json(),
            r#"{"schemaVersion":1,"id":"codex-cli","name":"Codex CLI","runtime":"cli","executables":["codex"],"capabilities":{"terminal":true,"resume":true,"structuredOutput":true,"images":false,"usage":true,"permissions":true,"modelSelection":true,"reasoning":true,"sandbox":true}}"#
        );
        for id in [
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli",
        ] {
            let entry = definition(id).expect(id);
            assert!(entry.acp.is_none());
            assert_eq!(entry.managed_transport(), Some(ProviderTransport::Headless));
            assert!(entry.manifest_json().contains("\"schemaVersion\":1"));
        }
    }

    #[test]
    fn acp_providers_use_their_reviewed_entry_flags_and_never_a_borrowed_grammar() {
        let expectations: [(&str, &[&str]); 6] = [
            ("qwen-code", &["--acp"]),
            ("kimi-cli", &["acp"]),
            ("qoder-cli", &["--acp"]),
            ("codebuddy-code", &["--acp"]),
            ("copilot-cli", &["--acp", "--stdio"]),
            ("cursor-agent-cli", &["acp"]),
        ];
        for (id, args) in expectations {
            let entry = definition(id).expect(id);
            let grammar = entry.acp.expect("acp grammar");
            assert_eq!(grammar.args, args, "{id}");
            assert_eq!(entry.managed_transport(), Some(ProviderTransport::AcpStdio));
            assert_eq!(entry.usage, ProviderUsageCapability::Unavailable);
            assert!(entry.manifest_json().contains("\"schemaVersion\":2"));
            assert!(!entry.is_legacy());
        }
    }

    #[test]
    fn codebuddy_account_profiles_are_environment_switches_not_credentials() {
        let grammar = definition("codebuddy-code")
            .and_then(|entry| entry.acp)
            .expect("codebuddy grammar");
        assert_eq!(
            grammar
                .account_profiles
                .iter()
                .map(|profile| profile.id)
                .collect::<Vec<_>>(),
            vec!["international", "china", "ioa"]
        );
        assert_eq!(grammar.account_profiles[0].environment, None);
        assert_eq!(
            grammar.account_profiles[1].environment,
            Some(("CODEBUDDY_INTERNET_ENVIRONMENT", "internal"))
        );
        assert_eq!(
            grammar.account_profiles[2].environment,
            Some(("CODEBUDDY_INTERNET_ENVIRONMENT", "ioa"))
        );
        for profile in grammar.account_profiles {
            if let Some((key, _)) = profile.environment {
                assert!(!key.to_ascii_lowercase().contains("key"));
                assert!(!key.to_ascii_lowercase().contains("token"));
            }
        }
    }

    #[test]
    fn cursor_declares_its_blocking_and_notification_extensions() {
        let grammar = definition("cursor-agent-cli")
            .and_then(|entry| entry.acp)
            .expect("cursor grammar");
        assert_eq!(
            grammar.blocking_extensions,
            &["cursor/ask_question", "cursor/create_plan"]
        );
        assert_eq!(
            grammar.notification_extensions,
            &[
                "cursor/update_todos",
                "cursor/task",
                "cursor/generate_image"
            ]
        );
    }

    #[test]
    fn iflow_is_legacy_terminal_only_with_no_managed_conversation() {
        let iflow = definition("iflow-cli").expect("iflow");
        assert!(iflow.is_legacy());
        assert_eq!(
            iflow.lifecycle,
            ProviderLifecycle::Legacy {
                service_shutdown: "2026-04-17"
            }
        );
        assert_eq!(iflow.transports, TERMINAL_ONLY);
        assert_eq!(iflow.managed_transport(), None);
        assert!(iflow.acp.is_none());
        assert!(!iflow.structured_output);
        assert!(!iflow.permissions);
        let manifest = iflow.manifest_json();
        assert!(manifest.contains("\"transports\":[\"terminal\"]"));
        assert!(manifest.contains("\"usage\":\"unavailable\""));
    }

    #[test]
    fn the_tooling_catalog_and_the_runtime_agree_on_transport_and_lifecycle() {
        use crate::contexts::tooling::cli::domain::definition::{
            CliManagedTransport, CliToolLifecycle,
        };
        use crate::contexts::tooling::cli::domain::registry::CLI_TOOL_DEFINITIONS;

        // The CLI Management page reads transport/lifecycle from the tooling catalog; the runtime
        // decides from its own definitions. Both must describe the same program.
        assert_eq!(CLI_TOOL_DEFINITIONS.len(), DEFINITIONS.len());
        for tool in CLI_TOOL_DEFINITIONS {
            let runtime = definition(tool.agent_id)
                .unwrap_or_else(|| panic!("{} has no runtime definition", tool.agent_id));
            let expected_transport = match runtime.managed_transport() {
                Some(ProviderTransport::AcpStdio) => CliManagedTransport::AcpStdio,
                Some(ProviderTransport::Headless) => CliManagedTransport::Headless,
                Some(ProviderTransport::Terminal) | None => CliManagedTransport::TerminalOnly,
            };
            assert_eq!(
                tool.managed_transport, expected_transport,
                "{} transport disagrees between catalog and runtime",
                tool.agent_id
            );
            match (&tool.lifecycle, &runtime.lifecycle) {
                (CliToolLifecycle::Active, ProviderLifecycle::Active) => {}
                (
                    CliToolLifecycle::Legacy {
                        service_shutdown: catalog_date,
                    },
                    ProviderLifecycle::Legacy {
                        service_shutdown: runtime_date,
                    },
                ) => assert_eq!(
                    catalog_date, runtime_date,
                    "{} shutdown date",
                    tool.agent_id
                ),
                (catalog, runtime) => panic!(
                    "{} lifecycle disagrees: catalog {catalog:?} vs runtime {runtime:?}",
                    tool.agent_id
                ),
            }
            assert_eq!(
                tool.executable_names[0], runtime.executable,
                "{} primary executable",
                tool.agent_id
            );
        }
    }
}
