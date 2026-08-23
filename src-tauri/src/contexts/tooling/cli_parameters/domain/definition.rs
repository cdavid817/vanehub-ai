use super::rendering::CliParameterRenderer;
use super::selection::CliParameterSelection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterCategory {
    Model,
    Experience,
    Context,
    Runtime,
    Diagnostics,
}

/// Only `UserEditable` definitions are returned to the settings page. The other two are kept in
/// the registry because the launch path still has to render them from their owning source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterOwnership {
    UserEditable,
    PolicyGoverned,
    RuntimeReserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterMaturity {
    Stable,
    Preview,
    Experimental,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterControl {
    Enum,
    BooleanFlag,
    TriState,
    MultiEnum,
    CustomText,
    OrderedStringList,
    PathList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterRisk {
    Normal,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliLaunchScope {
    Interactive,
    Chat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterPlatform {
    Windows,
    Macos,
    Linux,
}

impl CliParameterPlatform {
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }
}

fn all_platforms() -> Vec<CliParameterPlatform> {
    vec![
        CliParameterPlatform::Windows,
        CliParameterPlatform::Macos,
        CliParameterPlatform::Linux,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterCompatibility {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) min_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_version: Option<String>,
    #[serde(default = "all_platforms")]
    pub(crate) platforms: Vec<CliParameterPlatform>,
}

impl Default for CliParameterCompatibility {
    fn default() -> Self {
        Self {
            min_version: None,
            max_version: None,
            platforms: all_platforms(),
        }
    }
}

impl CliParameterCompatibility {
    pub(crate) fn is_version_gated(&self) -> bool {
        self.min_version.is_some() || self.max_version.is_some()
    }

    pub(crate) fn required_range(&self) -> Option<String> {
        match (&self.min_version, &self.max_version) {
            (None, None) => None,
            (Some(min), None) => Some(format!(">= {min}")),
            (None, Some(max)) => Some(format!("<= {max}")),
            (Some(min), Some(max)) => Some(format!(">= {min}, <= {max}")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_items: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) item_max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) item_pattern: Option<String>,
    #[serde(default)]
    pub(crate) dedupe: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) exclusive_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) ordering: Option<CliParameterOrdering>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterOrdering {
    Catalog,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConditionOperator {
    Equals,
    NotInherit,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CliParameterCondition {
    pub(crate) parameter_id: String,
    pub(crate) operator: CliConditionOperator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) value: Option<CliConditionValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum CliConditionValue {
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterDependencies {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) requires_all: Vec<CliParameterCondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterAudit {
    pub(crate) source_id: String,
    pub(crate) source_url: String,
    pub(crate) reviewed_at: String,
    /// Which artefact the review actually read, and in what state. A date alone cannot say whether
    /// the reviewer saw the published page, the installed binary's own help, or both.
    pub(crate) reviewed_state: String,
    pub(crate) verification: CliParameterVerification,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterVerification {
    /// Confirmed against a source the vendor publishes: its documentation, or its own binary's
    /// help and argument rejection behaviour.
    Verified,
    /// Confirmed only against something in this repository. Never sufficient on its own.
    RepositoryVerified,
    /// Carried forward without a source that settles it. It must not be presented as audited.
    PendingReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterOption {
    pub(crate) value: String,
    pub(crate) label_key: String,
    pub(crate) description_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) compatibility: Option<CliParameterCompatibility>,
}

fn default_selection() -> CliParameterSelection {
    CliParameterSelection::Inherit
}

fn default_ownership() -> CliParameterOwnership {
    CliParameterOwnership::UserEditable
}

fn default_maturity() -> CliParameterMaturity {
    CliParameterMaturity::Stable
}

fn default_risk() -> CliParameterRisk {
    CliParameterRisk::Normal
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CliParameterDefinition {
    pub(crate) id: String,
    #[serde(skip_deserializing, default)]
    pub(crate) agent_id: String,
    pub(crate) category: CliParameterCategory,
    #[serde(default = "default_ownership")]
    pub(crate) ownership: CliParameterOwnership,
    #[serde(default = "default_maturity")]
    pub(crate) maturity: CliParameterMaturity,
    pub(crate) control: CliParameterControl,
    pub(crate) label_key: String,
    pub(crate) description_key: String,
    #[serde(default = "default_selection")]
    pub(crate) default_selection: CliParameterSelection,
    pub(crate) launch_scopes: Vec<CliLaunchScope>,
    #[serde(default = "default_risk")]
    pub(crate) risk: CliParameterRisk,
    #[serde(default)]
    pub(crate) advanced: bool,
    #[serde(default)]
    pub(crate) options: Vec<CliParameterOption>,
    pub(crate) renderer: CliParameterRenderer,
    #[serde(default)]
    pub(crate) constraints: CliParameterConstraints,
    #[serde(default)]
    pub(crate) compatibility: CliParameterCompatibility,
    #[serde(default)]
    pub(crate) dependencies: CliParameterDependencies,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) diagnostics: Vec<String>,
    /// Native-only review provenance. It is deliberately not serialized: neither the frontend
    /// contract nor a command response carries audit prose.
    #[serde(skip_serializing)]
    pub(crate) audit: CliParameterAudit,
}

impl CliParameterDefinition {
    pub(crate) fn is_user_editable(&self) -> bool {
        self.ownership == CliParameterOwnership::UserEditable
    }

    pub(crate) fn applies_to(&self, scope: CliLaunchScope) -> bool {
        self.launch_scopes.contains(&scope)
    }

    pub(crate) fn allows_custom_values(&self) -> bool {
        matches!(
            self.control,
            CliParameterControl::CustomText
                | CliParameterControl::OrderedStringList
                | CliParameterControl::PathList
        )
    }

    pub(crate) fn option_values(&self) -> Vec<&str> {
        self.options
            .iter()
            .map(|option| option.value.as_str())
            .collect()
    }
}
