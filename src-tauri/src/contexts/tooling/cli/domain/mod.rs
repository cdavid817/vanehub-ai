// The source-aware environment model is being built alongside the flat `CliToolStatus` model it
// replaces. Task group 3 moves the application layer onto these modules and task group 13 deletes
// what is left below; until then they have no production caller.
//
// Consumers name the module (`domain::ids::CliToolId`) rather than a re-export, so one attribute
// per module covers everything inside it. A re-export list would need a second attribute for
// `unused_imports` and an edit for every type added during the migration.
//
// `expect` rather than `allow`, so it errors once task group 3 lands instead of silently outliving
// the migration -- and `not(test)`, because the domain tests below *do* use these items. That
// makes the attribute double as a coverage gate: an item no test touches fails the test build.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod action;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod bulk;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod catalog;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod definition;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod ids;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod installation;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod plan;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod probe;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod registry;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod source;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod status;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod trust;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the CLI application service in task group 3; remove with that group"
    )
)]
pub(crate) mod version;

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolDefinition {
    pub(crate) agent_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) provider: &'static str,
    pub(crate) executable_name: &'static str,
    /// `None` for CLIs distributed only by installer script, which have no npm package to
    /// install, query for versions, or name in guidance.
    pub(crate) package_name: Option<&'static str>,
    pub(crate) script_install_url: Option<&'static str>,
    pub(crate) powershell_install_url: Option<&'static str>,
    pub(crate) winget_package_id: Option<&'static str>,
    /// How this CLI's shell installer accepts a version, when it accepts one at all.
    ///
    /// The npm path pins by construction -- `package@version` cannot be misread. A script is an
    /// opaque program whose calling convention is the vendor's, so pinning through one is only
    /// possible where that convention is known. `None` means a pinned install is refused rather
    /// than run: the script would otherwise install whatever it defaults to, which is latest, and
    /// report success for a version it did not install.
    pub(crate) script_version_argument: Option<ScriptVersionArgument>,
}

/// The shape a shell installer expects its version in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptVersionArgument {
    /// First positional argument, as in `install.sh 1.2.3`.
    Positional,
    /// Named flag followed by the version, as in `install --version 1.2.3`.
    Flag(&'static str),
}

/// Which interpreter an installer URL must be fed to. The URL alone does not say: a `.sh`
/// installer piped into PowerShell would execute as nonsense, so the interpreter travels with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptInstaller {
    Shell(&'static str),
    PowerShell(&'static str),
}

impl ToolDefinition {
    /// Windows has no POSIX shell to run a `.sh` installer through, so a CLI that ships only a
    /// shell installer relies on its npm or winget package there. Publishing a PowerShell
    /// installer is what makes script installation reachable on Windows.
    pub(crate) fn platform_installer(self) -> Option<ScriptInstaller> {
        if cfg!(target_os = "windows") {
            if let Some(url) = self.powershell_install_url {
                return Some(ScriptInstaller::PowerShell(url));
            }
        }
        self.script_install_url.map(ScriptInstaller::Shell)
    }
}

pub(crate) const CLI_TOOL_DEFINITIONS: [ToolDefinition; 5] = [
    ToolDefinition {
        agent_id: "claude-code",
        display_name: "Anthropic Claude Code CLI",
        provider: "Anthropic",
        executable_name: "claude",
        package_name: Some("@anthropic-ai/claude-code"),
        script_install_url: Some("https://claude.ai/install.sh"),
        powershell_install_url: None,
        winget_package_id: Some("Anthropic.ClaudeCode"),
        // `Usage: $0 [stable|latest|VERSION]`.
        script_version_argument: Some(ScriptVersionArgument::Positional),
    },
    ToolDefinition {
        agent_id: "codex-cli",
        display_name: "OpenAI Codex CLI",
        provider: "OpenAI",
        executable_name: "codex",
        package_name: Some("@openai/codex"),
        script_install_url: None,
        powershell_install_url: None,
        winget_package_id: None,
        script_version_argument: None,
    },
    ToolDefinition {
        agent_id: "gemini-cli",
        display_name: "Google Gemini CLI",
        provider: "Google",
        executable_name: "gemini",
        package_name: Some("@google/gemini-cli"),
        script_install_url: None,
        powershell_install_url: None,
        winget_package_id: None,
        script_version_argument: None,
    },
    ToolDefinition {
        agent_id: "opencode",
        display_name: "OpenCode CLI",
        provider: "OpenCode",
        executable_name: "opencode",
        package_name: Some("opencode-ai"),
        script_install_url: Some("https://opencode.ai/install"),
        powershell_install_url: None,
        winget_package_id: None,
        // `-v, --version <version> Install a specific version`.
        script_version_argument: Some(ScriptVersionArgument::Flag("--version")),
    },
    ToolDefinition {
        agent_id: "antigravity-cli",
        display_name: "Google Antigravity CLI",
        provider: "Google",
        executable_name: "agy",
        package_name: None,
        script_install_url: Some("https://antigravity.google/cli/install.sh"),
        powershell_install_url: Some("https://antigravity.google/cli/install.ps1"),
        winget_package_id: None,
        // Left unset deliberately: its installers' version convention has not been verified, and
        // guessing one would reintroduce exactly the silent wrong-version install this field
        // exists to prevent.
        script_version_argument: None,
    },
];

pub(crate) fn definition(agent_id: &str) -> Option<ToolDefinition> {
    CLI_TOOL_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| definition.agent_id == agent_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvironmentType {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersionCheckStatus {
    Unsupported,
    NotDetected,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallSource {
    Npm,
    Winget,
    Desktop,
    Homebrew,
    Volta,
    Bun,
    Vendor,
    System,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConflictState {
    None,
    Multiple,
    VersionMismatch,
    RunnableMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleEligibility {
    Npm,
    Wget,
    Winget,
    Manual,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Installation {
    pub(crate) path: String,
    pub(crate) version: Option<String>,
    pub(crate) runnable: bool,
    pub(crate) error: Option<String>,
    pub(crate) source: InstallSource,
    pub(crate) environment_type: EnvironmentType,
    pub(crate) is_active: bool,
}

pub(crate) fn derive_conflict_state(installations: &[Installation]) -> ConflictState {
    if installations.len() <= 1 {
        return ConflictState::None;
    }
    let has_runnable = installations
        .iter()
        .any(|installation| installation.runnable);
    let has_broken = installations
        .iter()
        .any(|installation| !installation.runnable);
    if has_runnable && has_broken {
        return ConflictState::RunnableMismatch;
    }
    let versions = installations
        .iter()
        .filter_map(|installation| installation.version.as_deref())
        .collect::<HashSet<_>>();
    if versions.len() > 1 {
        ConflictState::VersionMismatch
    } else {
        ConflictState::Multiple
    }
}

pub(crate) fn derive_lifecycle_eligibility(
    definition: ToolDefinition,
    installed: bool,
    active: Option<&Installation>,
) -> LifecycleEligibility {
    if !installed {
        return if definition.platform_installer().is_some() {
            LifecycleEligibility::Wget
        } else if definition.package_name.is_some() {
            LifecycleEligibility::Npm
        } else {
            LifecycleEligibility::Manual
        };
    }
    match active {
        Some(installation)
            if installation.runnable
                && installation.source == InstallSource::Npm
                && definition.package_name.is_some() =>
        {
            LifecycleEligibility::Npm
        }
        Some(installation)
            if installation.runnable
                && installation.source == InstallSource::Vendor
                && definition.platform_installer().is_some() =>
        {
            LifecycleEligibility::Wget
        }
        Some(installation)
            if installation.runnable
                && installation.source == InstallSource::Winget
                && (definition.winget_package_id.is_some()
                    || winget_package_id(&installation.path).is_some()) =>
        {
            LifecycleEligibility::Winget
        }
        Some(_) => LifecycleEligibility::Manual,
        None => LifecycleEligibility::Unavailable,
    }
}

pub(crate) fn classify_install_source(path: &str, has_npm_sibling: bool) -> InstallSource {
    let value = path.replace('\\', "/").to_ascii_lowercase();
    if value.contains("/microsoft/winget/packages/") || value.contains("/microsoft/winget/links/") {
        InstallSource::Winget
    } else if value.contains("/programs/openai/codex/") {
        InstallSource::Desktop
    } else if value.contains("/appdata/roaming/npm/")
        || value.contains("/.npm/")
        // `npm config set prefix ~/.npm-global` is what the install guide recommends instead of
        // `sudo npm install -g`, and `~/.npm-global/bin` is already a search location here. Without
        // this arm the source came back Unknown, which makes the lifecycle Manual, so a CLI this
        // application installed there through npm was then refused an upgrade with "must be
        // updated by its source-native installer" -- npm being exactly the source-native
        // installer. `/.npm/` does not cover it: the next character is a dash, not a slash.
        || value.contains("/.npm-global/")
        || value.contains("/node_modules/")
        || has_npm_sibling
    {
        InstallSource::Npm
    } else if value.contains("/homebrew/") || value.contains("/cellar/") {
        InstallSource::Homebrew
    } else if value.contains("/.volta/") {
        InstallSource::Volta
    } else if value.contains("/.bun/") {
        InstallSource::Bun
    } else if value.contains("/.local/bin/")
        || value.contains("/.claude/")
        || value.contains("/.opencode/")
    {
        InstallSource::Vendor
    } else if value.starts_with("/usr/bin/") || value.starts_with("/usr/local/bin/") {
        InstallSource::System
    } else {
        InstallSource::Unknown
    }
}

pub(crate) fn winget_package_id(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let normalized_lower = normalized.to_ascii_lowercase();
    let marker = "/microsoft/winget/packages/";
    let start = normalized_lower.find(marker)? + marker.len();
    let package_dir = normalized[start..].split('/').next()?;
    let package_id = package_dir
        .split("_Microsoft.Winget.")
        .next()
        .unwrap_or(package_dir)
        .trim();
    (!package_id.is_empty()).then(|| package_id.to_string())
}

pub(crate) fn is_stable_version(version: &str) -> bool {
    !version.contains('-')
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

pub(crate) fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let mut left_parts = version_parts(left)?;
    let mut right_parts = version_parts(right)?;
    let max_len = left_parts.len().max(right_parts.len());
    left_parts.resize(max_len, 0);
    right_parts.resize(max_len, 0);
    Some(left_parts.cmp(&right_parts))
}

fn version_parts(version: &str) -> Option<Vec<u64>> {
    let trimmed = version.trim().trim_start_matches('v');
    if trimmed.contains('-') {
        return None;
    }
    trimmed
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

#[derive(Debug, Default)]
pub(crate) struct MutationClaims {
    active_agent_ids: BTreeSet<String>,
}

impl MutationClaims {
    pub(crate) fn try_acquire(&mut self, agent_id: &str) -> bool {
        self.active_agent_ids.insert(agent_id.to_string())
    }

    pub(crate) fn release(&mut self, agent_id: &str) {
        self.active_agent_ids.remove(agent_id);
    }

    pub(crate) fn try_acquire_many<'a>(
        &mut self,
        agent_ids: impl IntoIterator<Item = &'a str>,
    ) -> Vec<String> {
        agent_ids
            .into_iter()
            .filter(|agent_id| self.active_agent_ids.insert((*agent_id).to_string()))
            .map(str::to_string)
            .collect()
    }

    pub(crate) fn release_many<'a>(&mut self, agent_ids: impl IntoIterator<Item = &'a str>) {
        for agent_id in agent_ids {
            self.active_agent_ids.remove(agent_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installation(
        path: &str,
        version: Option<&str>,
        runnable: bool,
        source: InstallSource,
    ) -> Installation {
        Installation {
            path: path.to_string(),
            version: version.map(str::to_string),
            runnable,
            error: None,
            source,
            environment_type: EnvironmentType::Linux,
            is_active: true,
        }
    }

    #[test]
    fn catalog_has_stable_order_ids_and_verified_sources() {
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
                "antigravity-cli"
            ]
        );
        assert_eq!(
            definition("claude-code").and_then(|definition| definition.winget_package_id),
            Some("Anthropic.ClaudeCode")
        );
        assert_eq!(
            definition("opencode").and_then(|definition| definition.script_install_url),
            Some("https://opencode.ai/install")
        );
        assert!(definition("unknown").is_none());
    }

    #[test]
    fn adding_a_script_only_cli_leaves_the_package_managed_ones_intact() {
        for agent_id in ["claude-code", "codex-cli", "gemini-cli", "opencode"] {
            let definition = definition(agent_id).expect("definition");
            assert!(
                definition.package_name.is_some(),
                "{agent_id} must keep its npm package"
            );
            assert_eq!(
                definition.powershell_install_url, None,
                "{agent_id} must not gain a PowerShell installer"
            );
        }
    }

    #[test]
    fn antigravity_is_script_only_on_both_installer_families() {
        let antigravity = definition("antigravity-cli").expect("antigravity");
        assert_eq!(antigravity.executable_name, "agy");
        assert_eq!(antigravity.package_name, None);
        assert_eq!(antigravity.winget_package_id, None);
        assert_eq!(
            antigravity.script_install_url,
            Some("https://antigravity.google/cli/install.sh")
        );
        assert_eq!(
            antigravity.powershell_install_url,
            Some("https://antigravity.google/cli/install.ps1")
        );
        // Whichever platform the suite runs on, one of the two installers must be reachable.
        assert!(antigravity.platform_installer().is_some());
    }

    #[test]
    fn platform_installer_pairs_each_url_with_its_own_interpreter() {
        let antigravity = definition("antigravity-cli").expect("antigravity");
        let expected = if cfg!(target_os = "windows") {
            ScriptInstaller::PowerShell("https://antigravity.google/cli/install.ps1")
        } else {
            ScriptInstaller::Shell("https://antigravity.google/cli/install.sh")
        };
        assert_eq!(antigravity.platform_installer(), Some(expected));

        // A CLI with only a shell installer never gets fed to PowerShell, even on Windows.
        let claude = definition("claude-code").expect("claude");
        assert_eq!(
            claude.platform_installer(),
            Some(ScriptInstaller::Shell("https://claude.ai/install.sh"))
        );
    }

    #[test]
    fn conflict_state_distinguishes_count_version_and_runnability() {
        let one = installation("/one", Some("1.0.0"), true, InstallSource::Npm);
        let same = installation("/two", Some("1.0.0"), true, InstallSource::Npm);
        let newer = installation("/three", Some("2.0.0"), true, InstallSource::Npm);
        let broken = installation("/four", None, false, InstallSource::Npm);

        assert_eq!(
            derive_conflict_state(std::slice::from_ref(&one)),
            ConflictState::None
        );
        assert_eq!(
            derive_conflict_state(&[one.clone(), same]),
            ConflictState::Multiple
        );
        assert_eq!(
            derive_conflict_state(&[one.clone(), newer]),
            ConflictState::VersionMismatch
        );
        assert_eq!(
            derive_conflict_state(&[one, broken]),
            ConflictState::RunnableMismatch
        );
    }

    #[test]
    fn lifecycle_eligibility_follows_install_state_and_active_source() {
        let claude = definition("claude-code").expect("claude");
        let codex = definition("codex-cli").expect("codex");
        assert_eq!(
            derive_lifecycle_eligibility(claude, false, None),
            LifecycleEligibility::Wget
        );
        assert_eq!(
            derive_lifecycle_eligibility(codex, false, None),
            LifecycleEligibility::Npm
        );
        let npm = installation("/npm/codex", Some("1.0.0"), true, InstallSource::Npm);
        assert_eq!(
            derive_lifecycle_eligibility(codex, true, Some(&npm)),
            LifecycleEligibility::Npm
        );
        let system = installation("/usr/bin/codex", Some("1.0.0"), true, InstallSource::System);
        assert_eq!(
            derive_lifecycle_eligibility(codex, true, Some(&system)),
            LifecycleEligibility::Manual
        );
        assert_eq!(
            derive_lifecycle_eligibility(codex, true, None),
            LifecycleEligibility::Unavailable
        );
    }

    #[test]
    fn script_only_cli_is_installer_eligible_and_never_npm_eligible() {
        let antigravity = definition("antigravity-cli").expect("antigravity");
        assert_eq!(
            derive_lifecycle_eligibility(antigravity, false, None),
            LifecycleEligibility::Wget
        );

        // Even if a path somehow classifies as npm-managed, there is no package to mutate, so the
        // page must fall through to source-native guidance instead of offering an npm upgrade.
        let npm = installation(r"\npm\agy", Some("1.0.0"), true, InstallSource::Npm);
        assert_eq!(
            derive_lifecycle_eligibility(antigravity, true, Some(&npm)),
            LifecycleEligibility::Manual
        );

        let vendor = installation(
            r"\home\.local\bin\agy",
            Some("1.0.0"),
            true,
            InstallSource::Vendor,
        );
        assert_eq!(
            derive_lifecycle_eligibility(antigravity, true, Some(&vendor)),
            LifecycleEligibility::Wget
        );
    }

    #[test]
    fn a_cli_with_no_package_and_no_installer_falls_back_to_manual() {
        let orphan = ToolDefinition {
            agent_id: "fixture-cli",
            display_name: "Fixture CLI",
            provider: "Fixture",
            executable_name: "fixture",
            package_name: None,
            script_install_url: None,
            powershell_install_url: None,
            winget_package_id: None,
            script_version_argument: None,
        };
        assert_eq!(
            derive_lifecycle_eligibility(orphan, false, None),
            LifecycleEligibility::Manual
        );
    }

    #[test]
    fn source_and_winget_identity_rules_are_path_string_pure() {
        assert_eq!(
            classify_install_source("C:\\Users\\a\\AppData\\Roaming\\npm\\codex.cmd", false),
            InstallSource::Npm
        );
        assert_eq!(
            classify_install_source("/opt/homebrew/bin/claude", false),
            InstallSource::Homebrew
        );
        // A custom npm prefix, which is what the install guide recommends over `sudo npm
        // install -g` and what this crate already searches. Classified Unknown before, which made
        // the lifecycle Manual and had the app refuse to upgrade a CLI it had itself installed
        // there through npm. `has_npm_sibling` is false on purpose: npm lives at its own prefix,
        // not beside the packages it installs.
        assert_eq!(
            classify_install_source("/home/someone/.npm-global/bin/gemini", false),
            InstallSource::Npm
        );
        // Still distinct from npm's cache directory, which the neighbouring `/.npm/` arm covers.
        assert_eq!(
            classify_install_source("/home/someone/.npm/_cacache/bin/gemini", false),
            InstallSource::Npm
        );
        assert_eq!(
            winget_package_id(
                "C:\\Users\\a\\Microsoft\\WinGet\\Packages\\Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe\\claude.exe"
            )
            .as_deref(),
            Some("Anthropic.ClaudeCode")
        );
    }

    #[test]
    fn stable_versions_reject_prereleases_and_compare_numeric_segments() {
        assert!(is_stable_version("1.2.3"));
        assert!(!is_stable_version("1.2.3-beta.1"));
        assert!(!is_stable_version("v1.2.3"));
        assert_eq!(
            compare_versions("v1.10.0", "1.9.9"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_versions("1.2", "1.2.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.2-beta", "1.2"), None);
    }

    #[test]
    fn mutation_claims_serialize_per_agent_without_blocking_other_agents() {
        let mut claims = MutationClaims::default();
        assert!(claims.try_acquire("codex-cli"));
        assert!(!claims.try_acquire("codex-cli"));
        assert_eq!(
            claims.try_acquire_many(["codex-cli", "gemini-cli", "opencode"]),
            vec!["gemini-cli", "opencode"]
        );
        claims.release("codex-cli");
        claims.release_many(["gemini-cli", "opencode"]);
        assert!(claims.try_acquire("codex-cli"));
        assert_eq!(
            claims.try_acquire_many(["gemini-cli", "opencode"]),
            vec!["gemini-cli", "opencode"]
        );
    }
}
