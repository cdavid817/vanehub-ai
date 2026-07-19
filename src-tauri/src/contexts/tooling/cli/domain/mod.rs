use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolDefinition {
    pub(crate) agent_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) provider: &'static str,
    pub(crate) executable_name: &'static str,
    pub(crate) package_name: &'static str,
    pub(crate) script_install_url: Option<&'static str>,
    pub(crate) winget_package_id: Option<&'static str>,
}

pub(crate) const CLI_TOOL_DEFINITIONS: [ToolDefinition; 4] = [
    ToolDefinition {
        agent_id: "claude-code",
        display_name: "Anthropic Claude Code CLI",
        provider: "Anthropic",
        executable_name: "claude",
        package_name: "@anthropic-ai/claude-code",
        script_install_url: Some("https://claude.ai/install.sh"),
        winget_package_id: Some("Anthropic.ClaudeCode"),
    },
    ToolDefinition {
        agent_id: "codex-cli",
        display_name: "OpenAI Codex CLI",
        provider: "OpenAI",
        executable_name: "codex",
        package_name: "@openai/codex",
        script_install_url: None,
        winget_package_id: None,
    },
    ToolDefinition {
        agent_id: "gemini-cli",
        display_name: "Google Gemini CLI",
        provider: "Google",
        executable_name: "gemini",
        package_name: "@google/gemini-cli",
        script_install_url: None,
        winget_package_id: None,
    },
    ToolDefinition {
        agent_id: "opencode",
        display_name: "OpenCode CLI",
        provider: "OpenCode",
        executable_name: "opencode",
        package_name: "opencode-ai",
        script_install_url: Some("https://opencode.ai/install"),
        winget_package_id: None,
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
        return if definition.script_install_url.is_some() {
            LifecycleEligibility::Wget
        } else {
            LifecycleEligibility::Npm
        };
    }
    match active {
        Some(installation)
            if installation.runnable && installation.source == InstallSource::Npm =>
        {
            LifecycleEligibility::Npm
        }
        Some(installation)
            if installation.runnable
                && installation.source == InstallSource::Vendor
                && definition.script_install_url.is_some() =>
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
            vec!["claude-code", "codex-cli", "gemini-cli", "opencode"]
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
    fn source_and_winget_identity_rules_are_path_string_pure() {
        assert_eq!(
            classify_install_source("C:\\Users\\a\\AppData\\Roaming\\npm\\codex.cmd", false),
            InstallSource::Npm
        );
        assert_eq!(
            classify_install_source("/opt/homebrew/bin/claude", false),
            InstallSource::Homebrew
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
