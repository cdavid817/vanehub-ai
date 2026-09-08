//! The audited catalog: every CLI VaneHub manages and every source that may distribute it.
//!
//! This is the only place a package identifier, installer URL, or probe command is allowed to come
//! from. Nothing here is ever accepted over the wire.

use super::definition::{
    CliArchitectureExclusion, CliCompatibilityPolicy, CliDistributionDefinition, CliIdentityRule,
    CliManagedTransport, CliPackageReference, CliToolDefinition, CliToolLifecycle,
    NPM_CAPABILITIES, STABLE_CHANNEL_ONLY, VENDOR_CAPABILITIES, WINGET_CAPABILITIES,
};
use super::probe::{CliProbeAvailability, CliProbeCommand, CliProbeDefinition};
use super::probe_interpretation::{CliAuthParser, CliDoctorParser};
use super::source::CliSourceCapabilities;
use super::source::{CliPlatform, CliSourceKind, CliTargetVersionMode, PlatformSet};
use super::trust::{
    CliInstallerRuntime, CliInstallerTemplate, CliInstallerTrust, CliSourceTrustPolicy,
};
use crate::contexts::tooling::managed_install::api::{ArtifactIntegrity, RetrievalPolicy};

/// Stable source ids. They appear in persisted plans and snapshots, so renaming one is a
/// migration, not a refactor.
pub(crate) const SOURCE_NPM: &str = "npm";
pub(crate) const SOURCE_WINGET: &str = "winget";
pub(crate) const SOURCE_VENDOR: &str = "vendor";
/// A locally installed program VaneHub detects but never installs, upgrades, or removes.
pub(crate) const SOURCE_LOCAL: &str = "local";

/// Detect-only: the only source a legacy entry gets. No package, no installer, no action.
const fn local_distribution() -> CliDistributionDefinition {
    CliDistributionDefinition {
        source_id: SOURCE_LOCAL,
        kind: CliSourceKind::Manual,
        package_reference: None,
        platforms: PlatformSet::ALL,
        capabilities: CliSourceCapabilities::DETECT_ONLY,
        channels: &[],
        trust: CliSourceTrustPolicy::DetectOnly,
    }
}

const fn npm_distribution(package: &'static str) -> CliDistributionDefinition {
    CliDistributionDefinition {
        source_id: SOURCE_NPM,
        kind: CliSourceKind::Npm,
        package_reference: Some(CliPackageReference {
            identifier: package,
        }),
        platforms: PlatformSet::ALL,
        capabilities: NPM_CAPABILITIES,
        channels: STABLE_CHANNEL_ONLY,
        trust: CliSourceTrustPolicy::PackageManager,
    }
}

const fn winget_distribution(package_id: &'static str) -> CliDistributionDefinition {
    CliDistributionDefinition {
        source_id: SOURCE_WINGET,
        kind: CliSourceKind::Winget,
        package_reference: Some(CliPackageReference {
            identifier: package_id,
        }),
        platforms: PlatformSet::WINDOWS_ONLY,
        capabilities: WINGET_CAPABILITIES,
        channels: STABLE_CHANNEL_ONLY,
        trust: CliSourceTrustPolicy::PackageManager,
    }
}

const fn vendor_distribution(trust: CliInstallerTrust) -> CliDistributionDefinition {
    CliDistributionDefinition {
        source_id: SOURCE_VENDOR,
        kind: CliSourceKind::VendorInstaller,
        package_reference: None,
        // Platform reach is decided by the templates below, not by this field. A platform with no
        // template is not actionable however wide this set is.
        platforms: PlatformSet::ALL,
        capabilities: VENDOR_CAPABILITIES,
        channels: STABLE_CHANNEL_ONLY,
        trust: CliSourceTrustPolicy::AuditedInstaller(trust),
    }
}

const fn shell_template(platform: CliPlatform, url: &'static str) -> CliInstallerTemplate {
    CliInstallerTemplate {
        platform,
        runtime: CliInstallerRuntime::ShellFile {
            interpreter: "bash",
        },
        url,
        // No vendor's version convention has been verified against a published contract, so none
        // of these may be aimed at an exact version. The alternative is installing latest and
        // reporting success under the requested version's name.
        target_version: CliTargetVersionMode::LatestOnly,
        version_argument: None,
        integrity: ArtifactIntegrity::Unverified,
    }
}

/// Claude Code publishes a shell installer only. Windows is therefore absent from this list, and
/// `template_for(Windows)` returns `None` -- which is the whole fix: no bash plan is generated on
/// Windows, and no npm substitution happens behind the user's back.
const CLAUDE_CODE_INSTALLER: CliInstallerTrust = CliInstallerTrust {
    policy: RetrievalPolicy {
        allowed_hosts: &["claude.ai"],
        max_download_bytes: 8 * 1024 * 1024,
        download_timeout_seconds: 90,
    },
    templates: &[
        shell_template(CliPlatform::Macos, "https://claude.ai/install.sh"),
        shell_template(CliPlatform::Linux, "https://claude.ai/install.sh"),
    ],
};

const OPENCODE_INSTALLER: CliInstallerTrust = CliInstallerTrust {
    policy: RetrievalPolicy {
        allowed_hosts: &["opencode.ai"],
        max_download_bytes: 8 * 1024 * 1024,
        download_timeout_seconds: 90,
    },
    templates: &[
        shell_template(CliPlatform::Macos, "https://opencode.ai/install"),
        shell_template(CliPlatform::Linux, "https://opencode.ai/install"),
    ],
};

/// Antigravity is the one vendor that publishes a Windows-native installer, so it is the one
/// vendor distribution actionable on Windows.
const ANTIGRAVITY_INSTALLER: CliInstallerTrust = CliInstallerTrust {
    policy: RetrievalPolicy {
        allowed_hosts: &["antigravity.google"],
        max_download_bytes: 8 * 1024 * 1024,
        download_timeout_seconds: 90,
    },
    templates: &[
        CliInstallerTemplate {
            platform: CliPlatform::Windows,
            runtime: CliInstallerRuntime::PowerShellFile,
            url: "https://antigravity.google/cli/install.ps1",
            target_version: CliTargetVersionMode::LatestOnly,
            version_argument: None,
            integrity: ArtifactIntegrity::Unverified,
        },
        shell_template(
            CliPlatform::Macos,
            "https://antigravity.google/cli/install.sh",
        ),
        shell_template(
            CliPlatform::Linux,
            "https://antigravity.google/cli/install.sh",
        ),
    ],
};

/// Cursor publishes `https://cursor.com/install` for macOS/Linux and `https://cursor.com/install?win32=true`
/// for native Windows (cursor.com/docs/cli/installation, reviewed 2026-09-06). There is no npm
/// package to fall back to, so the audited installer is its only managed source; the script is
/// downloaded to a bounded file and reviewed before it runs, never piped into a shell.
const CURSOR_INSTALLER: CliInstallerTrust = CliInstallerTrust {
    policy: RetrievalPolicy {
        allowed_hosts: &["cursor.com"],
        max_download_bytes: 8 * 1024 * 1024,
        download_timeout_seconds: 90,
    },
    templates: &[
        CliInstallerTemplate {
            platform: CliPlatform::Windows,
            runtime: CliInstallerRuntime::PowerShellFile,
            url: "https://cursor.com/install?win32=true",
            target_version: CliTargetVersionMode::LatestOnly,
            version_argument: None,
            integrity: ArtifactIntegrity::Unverified,
        },
        shell_template(CliPlatform::Macos, "https://cursor.com/install"),
        shell_template(CliPlatform::Linux, "https://cursor.com/install"),
    ],
};

/// Qoder documents Windows arm64 as temporarily unsupported (docs.qoder.com/cli/installation).
const QODER_COMPATIBILITY: CliCompatibilityPolicy = CliCompatibilityPolicy {
    minimum_supported: None,
    platforms: PlatformSet::ALL,
    excluded_targets: &[CliArchitectureExclusion {
        platform: CliPlatform::Windows,
        architecture: "aarch64",
    }],
};

/// `agent` is a generic basename. Cursor's installer places the real program under a
/// `cursor-agent` directory and its version banner names Cursor; a candidate with neither is some
/// other program and is never launched as this provider.
const CURSOR_IDENTITY: CliIdentityRule = CliIdentityRule::Reviewed {
    canonical_path_markers: &["cursor-agent", "cursor"],
    version_output_markers: &["cursor"],
};

/// Claude Code is the only registered CLI with a documented read-only `doctor`. Authentication is
/// left undocumented: the Doctor output has no signal stable enough to parse into a login state,
/// and `unknown` beats a guess that says "authenticated".
const CLAUDE_CODE_PROBES: CliProbeDefinition = CliProbeDefinition {
    version: CliProbeCommand::version(&["--version"]),
    doctor: CliProbeAvailability::Supported(CliProbeCommand::diagnostic(&["doctor"], 30)),
    authentication: CliProbeAvailability::Undocumented,
    authentication_parser: CliAuthParser::Undocumented,
    doctor_parser: CliDoctorParser::ClaudeCodeDoctor,
};

const CODEX_PROBES: CliProbeDefinition = CliProbeDefinition {
    version: CliProbeCommand::version(&["--version"]),
    doctor: CliProbeAvailability::Undocumented,
    doctor_parser: CliDoctorParser::Undocumented,
    authentication_parser: CliAuthParser::CodexLoginStatus,
    authentication: CliProbeAvailability::Supported(CliProbeCommand::diagnostic(
        &["login", "status"],
        20,
    )),
};

const OPENCODE_PROBES: CliProbeDefinition = CliProbeDefinition {
    version: CliProbeCommand::version(&["--version"]),
    doctor: CliProbeAvailability::Undocumented,
    doctor_parser: CliDoctorParser::Undocumented,
    authentication_parser: CliAuthParser::OpenCodeAuthList,
    // Parsed only into a normalized summary; the raw account list never reaches storage or the UI.
    authentication: CliProbeAvailability::Supported(CliProbeCommand::diagnostic(
        &["auth", "list"],
        20,
    )),
};

/// Order is stable and matches the existing catalog. It reaches the UI through the shared display
/// ordering, so a reordering here is a visible change.
pub(crate) const CLI_TOOL_DEFINITIONS: &[CliToolDefinition] = &[
    CliToolDefinition {
        agent_id: "claude-code",
        display_name: "Anthropic Claude Code CLI",
        provider: "Anthropic",
        executable_names: &["claude"],
        distributions: &[
            npm_distribution("@anthropic-ai/claude-code"),
            winget_distribution("Anthropic.ClaudeCode"),
            vendor_distribution(CLAUDE_CODE_INSTALLER),
        ],
        probes: CLAUDE_CODE_PROBES,
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::Headless,
        login_docs_url: None,
    },
    CliToolDefinition {
        agent_id: "codex-cli",
        display_name: "OpenAI Codex CLI",
        provider: "OpenAI",
        executable_names: &["codex"],
        distributions: &[npm_distribution("@openai/codex")],
        probes: CODEX_PROBES,
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::Headless,
        login_docs_url: None,
    },
    CliToolDefinition {
        agent_id: "gemini-cli",
        display_name: "Google Gemini CLI",
        provider: "Google",
        executable_names: &["gemini"],
        distributions: &[npm_distribution("@google/gemini-cli")],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::Headless,
        login_docs_url: None,
    },
    CliToolDefinition {
        agent_id: "opencode",
        display_name: "OpenCode CLI",
        provider: "OpenCode",
        executable_names: &["opencode"],
        distributions: &[
            npm_distribution("opencode-ai"),
            vendor_distribution(OPENCODE_INSTALLER),
        ],
        probes: OPENCODE_PROBES,
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::Headless,
        login_docs_url: None,
    },
    CliToolDefinition {
        agent_id: "antigravity-cli",
        display_name: "Google Antigravity CLI",
        provider: "Google",
        executable_names: &["agy"],
        // No npm package exists for this CLI, so the vendor installer is the only source.
        distributions: &[vendor_distribution(ANTIGRAVITY_INSTALLER)],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::Headless,
        login_docs_url: None,
    },
    // --- extend-cli-providers-with-acp: reviewed against the official sources on 2026-09-06.
    // Every vendor script that is documented but unverified for source-locking (Qwen standalone,
    // Kimi native, Qoder, CodeBuddy native) stays guidance-only: only the npm package is managed.
    CliToolDefinition {
        agent_id: "qwen-code",
        display_name: "Qwen Code",
        provider: "Alibaba",
        executable_names: &["qwen"],
        distributions: &[npm_distribution("@qwen-code/qwen-code")],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://qwenlm.github.io/qwen-code-docs/en/index"),
    },
    CliToolDefinition {
        agent_id: "kimi-cli",
        display_name: "Kimi Code CLI",
        provider: "Moonshot AI",
        executable_names: &["kimi"],
        // The retired Python/uv distribution shares the basename; discovery tells the two apart
        // by source kind (`uv`), and migration stays a user-run `kimi migrate`.
        distributions: &[npm_distribution("@moonshot-ai/kimi-code")],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started.html"),
    },
    CliToolDefinition {
        agent_id: "qoder-cli",
        display_name: "Qoder CLI",
        provider: "Qoder",
        // `qodercli` is the package's historical bin name; it is accepted only as a launcher
        // alias of the same npm install, never as a different grammar.
        executable_names: &["qoder", "qodercli"],
        distributions: &[npm_distribution("@qoder-ai/qodercli")],
        probes: CliProbeDefinition::version_only(),
        compatibility: QODER_COMPATIBILITY,
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://docs.qoder.com/cli/installation"),
    },
    CliToolDefinition {
        agent_id: "codebuddy-code",
        display_name: "CodeBuddy Code",
        provider: "Tencent",
        executable_names: &["codebuddy"],
        distributions: &[npm_distribution("@tencent-ai/codebuddy-code")],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://www.codebuddy.ai/docs/cli/quickstart"),
    },
    CliToolDefinition {
        agent_id: "copilot-cli",
        display_name: "GitHub Copilot CLI",
        provider: "GitHub",
        // The standalone `copilot` program, never the historical `gh copilot` extension.
        executable_names: &["copilot"],
        distributions: &[
            npm_distribution("@github/copilot"),
            winget_distribution("GitHub.Copilot"),
        ],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli"),
    },
    CliToolDefinition {
        agent_id: "cursor-agent-cli",
        display_name: "Cursor Agent CLI",
        provider: "Cursor",
        executable_names: &["agent"],
        distributions: &[vendor_distribution(CURSOR_INSTALLER)],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Active,
        identity: CURSOR_IDENTITY,
        managed_transport: CliManagedTransport::AcpStdio,
        login_docs_url: Some("https://cursor.com/docs/cli/installation"),
    },
    // iFlow's official service and API shut down on 2026-04-17. Only a program the user already
    // has, with the user's own custom API configuration, is supported -- and only detected.
    CliToolDefinition {
        agent_id: "iflow-cli",
        display_name: "iFlow CLI",
        provider: "iFlow",
        executable_names: &["iflow"],
        distributions: &[local_distribution()],
        probes: CliProbeDefinition::version_only(),
        compatibility: CliCompatibilityPolicy::any_desktop(),
        lifecycle: CliToolLifecycle::Legacy {
            service_shutdown: "2026-04-17",
        },
        identity: CliIdentityRule::Basename,
        managed_transport: CliManagedTransport::TerminalOnly,
        login_docs_url: Some("https://vibex.iflow.cn/t/topic/4819"),
    },
];

pub(crate) fn definition(agent_id: &str) -> Option<&'static CliToolDefinition> {
    CLI_TOOL_DEFINITIONS
        .iter()
        .find(|definition| definition.agent_id == agent_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::definition::CliDistributionAction;

    #[test]
    fn the_catalog_keeps_its_stable_ids_and_order() {
        assert_eq!(
            CLI_TOOL_DEFINITIONS
                .iter()
                .map(|definition| definition.agent_id)
                .collect::<Vec<_>>(),
            vec![
                "claude-code",
                "codex-cli",
                "gemini-cli",
                "opencode",
                "antigravity-cli",
                "qwen-code",
                "kimi-cli",
                "qoder-cli",
                "codebuddy-code",
                "copilot-cli",
                "cursor-agent-cli",
                "iflow-cli",
            ]
        );
        assert!(definition("unknown").is_none());
        assert_eq!(
            definition("opencode").map(|tool| tool.display_name),
            Some("OpenCode CLI")
        );
    }

    #[test]
    fn every_identifier_in_the_catalog_is_a_valid_value_object() {
        for tool in CLI_TOOL_DEFINITIONS {
            assert!(
                tool.tool_id().is_ok(),
                "{} has an invalid id",
                tool.agent_id
            );
            assert!(!tool.executable_names.is_empty());
            assert!(!tool.distributions.is_empty());
            for distribution in tool.distributions {
                assert!(distribution.source_id().is_ok());
            }
        }
    }

    #[test]
    fn claude_code_reaches_three_sources_that_do_not_share_a_catalog() {
        let claude = definition("claude-code").expect("claude-code");
        assert_eq!(claude.distributions.len(), 3);

        let npm = claude.distribution(SOURCE_NPM).expect("npm");
        let winget = claude.distribution(SOURCE_WINGET).expect("winget");
        assert_eq!(
            npm.package_reference.map(|reference| reference.identifier),
            Some("@anthropic-ai/claude-code")
        );
        // The WinGet package identifier is a WinGet id, not an npm name. Sharing one identifier
        // across sources is how npm registry data ended up describing a WinGet install.
        assert_eq!(
            winget
                .package_reference
                .map(|reference| reference.identifier),
            Some("Anthropic.ClaudeCode")
        );
        assert_ne!(npm.package_reference, winget.package_reference);
    }

    #[test]
    fn a_shell_only_vendor_is_unactionable_on_windows_while_npm_still_works() {
        let claude = definition("claude-code").expect("claude-code");
        let vendor = claude.distribution(SOURCE_VENDOR).expect("vendor");
        let npm = claude.distribution(SOURCE_NPM).expect("npm");

        assert!(!vendor.is_actionable_on(CliPlatform::Windows));
        assert!(vendor.is_actionable_on(CliPlatform::Macos));
        assert!(vendor.is_actionable_on(CliPlatform::Linux));
        // npm remains available on Windows, so the tool is still manageable there -- through a
        // source the user is shown, not through a silent substitution.
        assert!(npm.is_actionable_on(CliPlatform::Windows));
    }

    #[test]
    fn expanded_catalog_entries_are_source_safe_and_truthful() {
        // npm only, where a vendor script is documented but not verified to be source-locked.
        for (id, package) in [
            ("qwen-code", "@qwen-code/qwen-code"),
            ("kimi-cli", "@moonshot-ai/kimi-code"),
            ("qoder-cli", "@qoder-ai/qodercli"),
            ("codebuddy-code", "@tencent-ai/codebuddy-code"),
            ("copilot-cli", "@github/copilot"),
        ] {
            let tool = definition(id).expect(id);
            let npm = tool.distribution(SOURCE_NPM).expect("npm");
            assert_eq!(
                npm.package_reference.map(|reference| reference.identifier),
                Some(package),
                "{id}"
            );
            assert!(tool.distribution(SOURCE_VENDOR).is_none(), "{id} vendor");
            assert!(!tool.is_legacy(), "{id}");
            assert!(!tool.probes.authentication.is_supported(), "{id} auth");
        }
        let copilot = definition("copilot-cli").expect("copilot");
        assert_eq!(
            copilot
                .distribution(SOURCE_WINGET)
                .and_then(|winget| winget.package_reference)
                .map(|reference| reference.identifier),
            Some("GitHub.Copilot")
        );
        // Cursor: no npm package exists, so the reviewed installer is its only managed source,
        // and its generic basename needs an identity rule.
        let cursor = definition("cursor-agent-cli").expect("cursor");
        assert!(cursor.distribution(SOURCE_NPM).is_none());
        let vendor = cursor.distribution(SOURCE_VENDOR).expect("vendor");
        assert!(vendor.is_actionable_on(CliPlatform::Windows));
        assert!(vendor.is_actionable_on(CliPlatform::Macos));
        assert!(matches!(cursor.identity, CliIdentityRule::Reviewed { .. }));
        assert_eq!(cursor.executable_names, &["agent"]);
        // Qoder: documented Windows arm64 exclusion.
        let qoder = definition("qoder-cli").expect("qoder");
        assert!(!qoder
            .compatibility
            .supports_target(CliPlatform::Windows, "aarch64"));
        assert!(qoder
            .compatibility
            .supports_target(CliPlatform::Windows, "x86_64"));
        assert_eq!(qoder.executable_names, &["qoder", "qodercli"]);
        // Transport is catalog data: the original five stay headless, the six ACP additions are
        // ACP, and the legacy entry has no managed conversation at all.
        for id in [
            "claude-code",
            "codex-cli",
            "gemini-cli",
            "opencode",
            "antigravity-cli",
        ] {
            assert_eq!(
                definition(id).expect(id).managed_transport,
                CliManagedTransport::Headless,
                "{id}"
            );
        }
        for id in [
            "qwen-code",
            "kimi-cli",
            "qoder-cli",
            "codebuddy-code",
            "copilot-cli",
            "cursor-agent-cli",
        ] {
            assert_eq!(
                definition(id).expect(id).managed_transport,
                CliManagedTransport::AcpStdio,
                "{id}"
            );
        }
        // iFlow: legacy, detect-only, no install action of any kind.
        let iflow = definition("iflow-cli").expect("iflow");
        assert!(iflow.is_legacy());
        assert_eq!(iflow.managed_transport, CliManagedTransport::TerminalOnly);
        assert_eq!(iflow.lifecycle.service_shutdown(), Some("2026-04-17"));
        assert_eq!(iflow.distributions.len(), 1);
        let local = &iflow.distributions[0];
        assert_eq!(local.source_id, SOURCE_LOCAL);
        assert!(local.kind.is_detect_only());
        assert!(!local.capabilities.manages_anything());
        assert!(local.trust.installer().is_none());
        assert!(local.package_reference.is_none());
        assert!(local.default_channel().is_none());
    }

    #[test]
    fn antigravity_is_the_only_vendor_actionable_on_windows() {
        for tool in CLI_TOOL_DEFINITIONS {
            let Some(vendor) = tool.distribution(SOURCE_VENDOR) else {
                continue;
            };
            let expected = matches!(tool.agent_id, "antigravity-cli" | "cursor-agent-cli");
            assert_eq!(
                vendor.is_actionable_on(CliPlatform::Windows),
                expected,
                "{} vendor windows actionability",
                tool.agent_id
            );
        }
    }

    #[test]
    fn antigravity_has_no_npm_source_to_fall_back_to() {
        let antigravity = definition("antigravity-cli").expect("antigravity-cli");
        assert!(antigravity.distribution(SOURCE_NPM).is_none());
        assert!(antigravity
            .distribution_of_kind(CliSourceKind::Npm)
            .is_none());
        assert_eq!(antigravity.distributions.len(), 1);
    }

    #[test]
    fn winget_is_declared_for_windows_only() {
        for tool in CLI_TOOL_DEFINITIONS {
            let Some(winget) = tool.distribution(SOURCE_WINGET) else {
                continue;
            };
            assert!(winget.is_actionable_on(CliPlatform::Windows));
            assert!(!winget.is_actionable_on(CliPlatform::Macos));
            assert!(!winget.is_actionable_on(CliPlatform::Linux));
        }
    }

    #[test]
    fn no_vendor_distribution_claims_an_exact_target_version() {
        for tool in CLI_TOOL_DEFINITIONS {
            let Some(vendor) = tool.distribution(SOURCE_VENDOR) else {
                continue;
            };
            for platform in [CliPlatform::Windows, CliPlatform::Macos, CliPlatform::Linux] {
                for action in [
                    CliDistributionAction::Install,
                    CliDistributionAction::Upgrade,
                    CliDistributionAction::Downgrade,
                    CliDistributionAction::Reinstall,
                ] {
                    assert!(
                        !vendor
                            .target_mode_on(action, platform)
                            .accepts_exact_target(),
                        "{} vendor must not accept an exact target on {}",
                        tool.agent_id,
                        platform.as_str()
                    );
                }
            }
        }
    }

    #[test]
    fn every_installer_url_is_https_and_on_its_own_allowlist() {
        for tool in CLI_TOOL_DEFINITIONS {
            for distribution in tool.distributions {
                let Some(trust) = distribution.trust.installer() else {
                    continue;
                };
                // The shared capability refuses an unbounded declaration; this is where that
                // refusal is observable, because the catalog is `static` data the build fixes and
                // a fallible constructor would trade a compile-time fact for a runtime one.
                assert!(
                    trust.policy.is_bounded(),
                    "{} declares a distribution without an allowlist or a ceiling",
                    tool.agent_id
                );
                for template in trust.templates {
                    assert!(
                        trust.permits_url(template.url),
                        "{} template url {} is not admissible",
                        tool.agent_id,
                        template.url
                    );
                }
            }
        }
    }

    #[test]
    fn a_powershell_template_is_never_paired_with_a_unix_platform() {
        for tool in CLI_TOOL_DEFINITIONS {
            for distribution in tool.distributions {
                let Some(trust) = distribution.trust.installer() else {
                    continue;
                };
                for template in trust.templates {
                    match (template.platform, template.runtime) {
                        (CliPlatform::Windows, CliInstallerRuntime::PowerShellFile) => {}
                        (
                            CliPlatform::Macos | CliPlatform::Linux,
                            CliInstallerRuntime::ShellFile { .. },
                        ) => {}
                        (platform, runtime) => panic!(
                            "{} pairs {} with {runtime:?}",
                            tool.agent_id,
                            platform.as_str()
                        ),
                    }
                }
            }
        }
    }

    #[test]
    fn only_documented_probes_are_declared() {
        let claude = definition("claude-code").expect("claude-code");
        assert!(claude.probes.doctor.is_supported());
        // Claude Code's Doctor output has no stable login signal, so authentication stays unknown.
        assert!(!claude.probes.authentication.is_supported());

        let codex = definition("codex-cli").expect("codex-cli");
        assert!(!codex.probes.doctor.is_supported());
        assert_eq!(
            codex.probes.authentication.command().map(|c| c.args),
            Some(&["login", "status"][..])
        );

        let opencode = definition("opencode").expect("opencode");
        assert_eq!(
            opencode.probes.authentication.command().map(|c| c.args),
            Some(&["auth", "list"][..])
        );

        // Gemini, Antigravity, and every expanded CLI have no verified non-interactive probe of
        // either kind: their login state stays unknown until a reviewed probe exists.
        for agent_id in [
            "gemini-cli",
            "antigravity-cli",
            "qwen-code",
            "kimi-cli",
            "qoder-cli",
            "codebuddy-code",
            "copilot-cli",
            "cursor-agent-cli",
            "iflow-cli",
        ] {
            let tool = definition(agent_id).expect(agent_id);
            assert!(!tool.probes.doctor.is_supported(), "{agent_id} doctor");
            assert!(
                !tool.probes.authentication.is_supported(),
                "{agent_id} authentication"
            );
        }
    }

    #[test]
    fn every_tool_can_report_its_version() {
        for tool in CLI_TOOL_DEFINITIONS {
            assert_eq!(tool.probes.version.args, &["--version"]);
            assert!(tool.probes.version.timeout_seconds > 0);
        }
    }

    #[test]
    fn npm_distributions_carry_a_package_and_vendor_ones_do_not() {
        for tool in CLI_TOOL_DEFINITIONS {
            for distribution in tool.distributions {
                match distribution.kind {
                    CliSourceKind::Npm | CliSourceKind::Winget => assert!(
                        distribution.package_reference.is_some(),
                        "{} {} needs a package reference",
                        tool.agent_id,
                        distribution.source_id
                    ),
                    // An installer is identified by its audited URL, not by a package name.
                    _ => assert!(distribution.package_reference.is_none()),
                }
            }
        }
    }
}
