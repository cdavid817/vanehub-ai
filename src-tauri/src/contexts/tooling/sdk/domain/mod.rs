use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SdkId {
    ClaudeSdk,
    CodexSdk,
}

impl SdkId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeSdk => "claude-sdk",
            Self::CodexSdk => "codex-sdk",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "claude-sdk" => Some(Self::ClaudeSdk),
            "codex-sdk" => Some(Self::CodexSdk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SdkDefinition {
    pub(crate) id: SdkId,
    pub(crate) display_name: &'static str,
    pub(crate) npm_package: &'static str,
    pub(crate) default_version: &'static str,
    pub(crate) companion_packages: &'static [&'static str],
    pub(crate) fallback_versions: &'static [&'static str],
    pub(crate) description: &'static str,
    pub(crate) related_providers: &'static [&'static str],
}

pub(crate) const SDK_DEFINITIONS: [SdkDefinition; 2] = [
    SdkDefinition {
        id: SdkId::ClaudeSdk,
        display_name: "Claude Code SDK",
        npm_package: "@anthropic-ai/claude-agent-sdk",
        default_version: "0.2.88",
        companion_packages: &["@anthropic-ai/sdk", "@anthropic-ai/bedrock-sdk"],
        fallback_versions: &["0.2.88", "0.2.81", "0.2.58"],
        description: "Claude AI 功能所需，包含 Agent SDK 和 Bedrock 支持。",
        related_providers: &["anthropic", "bedrock"],
    },
    SdkDefinition {
        id: SdkId::CodexSdk,
        display_name: "Codex SDK",
        npm_package: "@openai/codex-sdk",
        default_version: "0.117.0",
        companion_packages: &[],
        fallback_versions: &["0.117.0", "0.116.0", "0.115.0"],
        description: "Codex AI 功能所需。",
        related_providers: &["openai"],
    },
];

pub(crate) fn definition(id: SdkId) -> SdkDefinition {
    match id {
        SdkId::ClaudeSdk => SDK_DEFINITIONS[0],
        SdkId::CodexSdk => SDK_DEFINITIONS[1],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdkInstallStatus {
    Installed,
    NotInstalled,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkStatus {
    pub(crate) id: SdkId,
    pub(crate) status: SdkInstallStatus,
    pub(crate) installed_version: Option<String>,
    pub(crate) latest_version: Option<String>,
    pub(crate) has_update: bool,
    pub(crate) install_path: Option<String>,
    pub(crate) last_checked: Option<String>,
    pub(crate) error_message: Option<String>,
}

impl SdkStatus {
    pub(crate) fn observed(
        id: SdkId,
        installed_version: Option<String>,
        latest_version: Option<String>,
        install_path: Option<String>,
        last_checked: Option<String>,
        error_message: Option<String>,
    ) -> Self {
        let has_update = update_available(installed_version.as_deref(), latest_version.as_deref());
        let status = if error_message.is_some() {
            SdkInstallStatus::Error
        } else if installed_version.is_some() {
            SdkInstallStatus::Installed
        } else {
            SdkInstallStatus::NotInstalled
        };
        Self {
            id,
            status,
            installed_version,
            latest_version,
            has_update,
            install_path,
            last_checked,
            error_message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdkVersionSource {
    Remote,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkVersionInfo {
    pub(crate) sdk_id: SdkId,
    pub(crate) versions: Vec<String>,
    pub(crate) fallback_versions: Vec<String>,
    pub(crate) source: SdkVersionSource,
    pub(crate) latest_version: Option<String>,
    pub(crate) error: Option<String>,
}

impl SdkVersionInfo {
    pub(crate) fn from_remote(
        definition: SdkDefinition,
        remote: Result<Vec<String>, String>,
    ) -> Self {
        let fallback_versions = definition
            .fallback_versions
            .iter()
            .map(|version| (*version).to_string())
            .collect::<Vec<_>>();
        match remote {
            Ok(versions) => {
                let versions = stable_versions(versions);
                if versions.is_empty() {
                    return Self::fallback(
                        definition,
                        fallback_versions,
                        "No remote versions returned".to_string(),
                    );
                }
                Self {
                    sdk_id: definition.id,
                    latest_version: versions.first().cloned(),
                    versions,
                    fallback_versions,
                    source: SdkVersionSource::Remote,
                    error: None,
                }
            }
            Err(error) => Self::fallback(definition, fallback_versions, error),
        }
    }

    fn fallback(definition: SdkDefinition, fallback_versions: Vec<String>, error: String) -> Self {
        Self {
            sdk_id: definition.id,
            latest_version: fallback_versions.first().cloned(),
            versions: fallback_versions.clone(),
            fallback_versions,
            source: SdkVersionSource::Fallback,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkUpdateInfo {
    pub(crate) id: SdkId,
    pub(crate) latest_version: Option<String>,
    pub(crate) has_update: bool,
    pub(crate) error_message: Option<String>,
}

impl SdkUpdateInfo {
    pub(crate) fn observed(
        id: SdkId,
        installed_version: Option<&str>,
        latest_version: Option<String>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id,
            has_update: update_available(installed_version, latest_version.as_deref()),
            latest_version,
            error_message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdkOperationType {
    Install,
    Update,
    Rollback,
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SdkLifecycleAction {
    InstallPackages,
    RemoveInstallation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkLifecyclePlan {
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) action: SdkLifecycleAction,
    pub(crate) requested_version: Option<String>,
    pub(crate) package_specs: Vec<String>,
}

pub(crate) fn lifecycle_plan(
    sdk_id: SdkId,
    operation: SdkOperationType,
    requested_version: Option<&str>,
) -> SdkLifecyclePlan {
    let definition = definition(sdk_id);
    if operation == SdkOperationType::Uninstall {
        return SdkLifecyclePlan {
            sdk_id,
            operation,
            action: SdkLifecycleAction::RemoveInstallation,
            requested_version: None,
            package_specs: Vec::new(),
        };
    }
    let requested_version = requested_version
        .and_then(normalize_requested_version)
        .unwrap_or_else(|| definition.default_version.to_string());
    SdkLifecyclePlan {
        sdk_id,
        operation,
        action: SdkLifecycleAction::InstallPackages,
        package_specs: package_specs(definition, &requested_version),
        requested_version: Some(requested_version),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkOperationOutcome {
    pub(crate) success: bool,
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) installed_version: Option<String>,
    pub(crate) requested_version: Option<String>,
    pub(crate) error: Option<String>,
}

impl SdkOperationOutcome {
    pub(crate) fn succeeded(plan: &SdkLifecyclePlan, installed_version: Option<String>) -> Self {
        Self {
            success: true,
            sdk_id: plan.sdk_id,
            operation: plan.operation,
            installed_version,
            requested_version: plan.requested_version.clone(),
            error: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn failed(plan: &SdkLifecyclePlan, error: impl Into<String>) -> Self {
        Self {
            success: false,
            sdk_id: plan.sdk_id,
            operation: plan.operation,
            installed_version: None,
            requested_version: plan.requested_version.clone(),
            error: Some(error.into()),
        }
    }
}

pub(crate) fn normalize_requested_version(version: &str) -> Option<String> {
    let trimmed = version.trim();
    let trimmed = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    is_semver_like(trimmed).then(|| trimmed.to_string())
}

pub(crate) fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left = normalize_requested_version(left)?;
    let right = normalize_requested_version(right)?;
    Some(version_parts(&left).cmp(&version_parts(&right)))
}

fn is_semver_like(version: &str) -> bool {
    let mut parts = version.splitn(2, '-');
    let core = parts.next().unwrap_or_default();
    let core_parts = core.split('.').collect::<Vec<_>>();
    if core_parts.len() != 3
        || !core_parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return false;
    }
    parts
        .next()
        .map(|suffix| {
            !suffix.is_empty()
                && suffix
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        })
        .unwrap_or(true)
}

fn version_parts(version: &str) -> (u64, u64, u64, Option<&str>) {
    let mut parts = version.splitn(2, '-');
    let core = parts.next().unwrap_or_default();
    let mut core = core.split('.').map(|part| part.parse::<u64>().unwrap_or(0));
    (
        core.next().unwrap_or(0),
        core.next().unwrap_or(0),
        core.next().unwrap_or(0),
        parts.next(),
    )
}

fn stable_versions(versions: Vec<String>) -> Vec<String> {
    let mut versions = versions
        .into_iter()
        .filter_map(|version| normalize_requested_version(&version))
        .filter(|version| !version.contains('-'))
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| compare_versions(right, left).unwrap_or(Ordering::Equal));
    versions.dedup();
    versions
}

fn update_available(installed: Option<&str>, latest: Option<&str>) -> bool {
    installed
        .zip(latest)
        .and_then(|(installed, latest)| compare_versions(installed, latest))
        == Some(Ordering::Less)
}

fn package_specs(definition: SdkDefinition, version: &str) -> Vec<String> {
    let mut packages = vec![format!("{}@{version}", definition.npm_package)];
    packages.extend(
        definition
            .companion_packages
            .iter()
            .map(|package| (*package).to_string()),
    );
    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_and_ids_preserve_stable_contract_values() {
        assert_eq!(SdkId::parse("claude-sdk"), Some(SdkId::ClaudeSdk));
        assert_eq!(SdkId::parse("codex-sdk"), Some(SdkId::CodexSdk));
        assert!(SdkId::parse("unknown-sdk").is_none());
        assert_eq!(
            SDK_DEFINITIONS
                .iter()
                .map(|definition| definition.id.as_str())
                .collect::<Vec<_>>(),
            vec!["claude-sdk", "codex-sdk"]
        );
        assert_eq!(
            definition(SdkId::ClaudeSdk).companion_packages,
            &["@anthropic-ai/sdk", "@anthropic-ai/bedrock-sdk"]
        );
    }

    #[test]
    fn status_and_update_rules_use_numeric_versions_and_errors() {
        let installed = SdkStatus::observed(
            SdkId::ClaudeSdk,
            Some("0.2.81".to_string()),
            Some("0.2.88".to_string()),
            Some("/dependencies/claude-sdk".to_string()),
            Some("checked".to_string()),
            None,
        );
        assert_eq!(installed.status, SdkInstallStatus::Installed);
        assert!(installed.has_update);

        let failed = SdkStatus::observed(
            SdkId::CodexSdk,
            None,
            Some("0.117.0".to_string()),
            None,
            None,
            Some("manifest unreadable".to_string()),
        );
        assert_eq!(failed.status, SdkInstallStatus::Error);
        assert!(!failed.has_update);
    }

    #[test]
    fn versions_normalize_sort_and_fall_back_without_runtime_dependencies() {
        assert_eq!(
            normalize_requested_version(" v0.2.81 ").as_deref(),
            Some("0.2.81")
        );
        assert!(normalize_requested_version("latest").is_none());
        assert!(normalize_requested_version("1.0.0 && rm -rf /").is_none());
        assert_eq!(
            compare_versions("0.2.90", "v0.2.88"),
            Some(Ordering::Greater)
        );

        let remote = SdkVersionInfo::from_remote(
            definition(SdkId::CodexSdk),
            Ok(vec![
                "0.115.0".to_string(),
                "v0.117.0".to_string(),
                "0.116.0-beta.1".to_string(),
                "0.116.0".to_string(),
            ]),
        );
        assert_eq!(remote.source, SdkVersionSource::Remote);
        assert_eq!(remote.versions, vec!["0.117.0", "0.116.0", "0.115.0"]);

        let fallback = SdkVersionInfo::from_remote(
            definition(SdkId::ClaudeSdk),
            Err("registry unavailable".to_string()),
        );
        assert_eq!(fallback.source, SdkVersionSource::Fallback);
        assert_eq!(fallback.latest_version.as_deref(), Some("0.2.88"));
        assert_eq!(fallback.error.as_deref(), Some("registry unavailable"));
    }

    #[test]
    fn lifecycle_plans_keep_default_rollback_and_uninstall_rules() {
        let install = lifecycle_plan(SdkId::ClaudeSdk, SdkOperationType::Install, None);
        assert_eq!(install.action, SdkLifecycleAction::InstallPackages);
        assert_eq!(install.requested_version.as_deref(), Some("0.2.88"));
        assert_eq!(
            install.package_specs,
            vec![
                "@anthropic-ai/claude-agent-sdk@0.2.88",
                "@anthropic-ai/sdk",
                "@anthropic-ai/bedrock-sdk"
            ]
        );

        let rollback = lifecycle_plan(
            SdkId::CodexSdk,
            SdkOperationType::Rollback,
            Some(" v0.115.0 "),
        );
        assert_eq!(rollback.requested_version.as_deref(), Some("0.115.0"));

        let invalid = lifecycle_plan(SdkId::CodexSdk, SdkOperationType::Update, Some("latest"));
        assert_eq!(invalid.requested_version.as_deref(), Some("0.117.0"));

        let uninstall = lifecycle_plan(
            SdkId::ClaudeSdk,
            SdkOperationType::Uninstall,
            Some("0.2.58"),
        );
        assert_eq!(uninstall.action, SdkLifecycleAction::RemoveInstallation);
        assert!(uninstall.requested_version.is_none());
        assert!(uninstall.package_specs.is_empty());
    }

    #[test]
    fn operation_outcomes_preserve_requested_and_terminal_state() {
        let plan = lifecycle_plan(SdkId::ClaudeSdk, SdkOperationType::Rollback, Some("0.2.58"));
        let success = SdkOperationOutcome::succeeded(&plan, Some("0.2.58".to_string()));
        assert!(success.success);
        assert_eq!(success.installed_version.as_deref(), Some("0.2.58"));
        assert!(success.error.is_none());

        let failure = SdkOperationOutcome::failed(&plan, "npm failed");
        assert!(!failure.success);
        assert_eq!(failure.requested_version.as_deref(), Some("0.2.58"));
        assert_eq!(failure.error.as_deref(), Some("npm failed"));
    }
}
