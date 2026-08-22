// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The validated shape of `vanehub-extension.yaml`.
//!
//! Every field is already a domain type. Nothing here holds a `String` where an id, path, version,
//! or event exists, so a value that reached this struct has been validated exactly once, at the
//! decoder boundary, rather than at each use.
//!
//! The struct is versioned rather than extended in place. A manifest declaring a schema version
//! this build does not know is incompatible, not "mostly readable": guessing at the security
//! semantics of fields added later is how a package gets more authority than its author declared.

use super::{
    ActivationEvent, ContributionKind, ContributionLocalId, ExtensionId, NetworkOrigin,
    PortablePackagePath, PublisherId,
};
use semver::{Version, VersionReq};

/// Schema versions this build understands. A single entry today; the list exists so adding one
/// is a decision rather than a bump.
pub(crate) const SUPPORTED_SCHEMA_VERSIONS: [u32; 1] = [1];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionedExtensionManifest {
    V1(ExtensionManifestV1),
}

impl VersionedExtensionManifest {
    pub(crate) fn id(&self) -> &ExtensionId {
        match self {
            Self::V1(manifest) => &manifest.id,
        }
    }

    pub(crate) const fn schema_version(&self) -> u32 {
        match self {
            Self::V1(_) => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionManifestV1 {
    pub(crate) id: ExtensionId,
    /// Never an identifier. Shown to the user, matched against nothing.
    pub(crate) display_name: String,
    pub(crate) publisher: PublisherId,
    pub(crate) version: Version,
    pub(crate) description: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) min_vanehub_version: VersionReq,
    pub(crate) runtime: RuntimeDeclaration,
    pub(crate) activation_events: Vec<ActivationEvent>,
    pub(crate) requires: ExtensionRequirements,
    pub(crate) permissions: CapabilityRequest,
    pub(crate) contributes: ContributionManifest,
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// How an extension's executable half runs, if it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeKind {
    /// A reviewed Rust adapter compiled into the application. Not selectable from a `.vhext`.
    Builtin,
    /// A WebAssembly core module, executed on the Skill tool engine.
    WasmModule,
    /// A separate process speaking the sidecar protocol.
    Sidecar,
    /// Declared but not runnable: the pinned Wasmtime build has no component-model support, so a
    /// package asking for one is told exactly that instead of failing as a malformed module.
    WasmComponentReserved,
    /// Contributions only. No entrypoint, nothing to activate.
    None,
}

impl RuntimeKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::WasmModule => "wasm-module",
            Self::Sidecar => "sidecar",
            Self::WasmComponentReserved => "wasm-component",
            Self::None => "none",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "builtin" => Some(Self::Builtin),
            "wasm-module" => Some(Self::WasmModule),
            "sidecar" => Some(Self::Sidecar),
            "wasm-component" => Some(Self::WasmComponentReserved),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// Whether an external package may select it. `builtin` is compile-time reviewed code, so a
    /// downloaded manifest naming it is claiming an identity it cannot have.
    pub(crate) const fn is_selectable_by_external_package(self) -> bool {
        match self {
            Self::Builtin => false,
            Self::WasmModule | Self::Sidecar | Self::WasmComponentReserved | Self::None => true,
        }
    }

    pub(crate) const fn requires_entry(self) -> bool {
        matches!(self, Self::WasmModule | Self::Sidecar | Self::Builtin)
    }
}

/// Reviewed placement, not unrestricted authority. `Trusted` means the publisher and runtime were
/// vetted; every call still passes the permission floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TrustProfile {
    Strict,
    Standard,
    Trusted,
}

impl TrustProfile {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Standard => "standard",
            Self::Trusted => "trusted",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "strict" => Some(Self::Strict),
            "standard" => Some(Self::Standard),
            "trusted" => Some(Self::Trusted),
            _ => None,
        }
    }

    /// Maximum callback budget in seconds, per `design.md`'s trust matrix.
    pub(crate) const fn max_callback_seconds(self) -> u32 {
        match self {
            Self::Strict => 5,
            Self::Standard => 10,
            Self::Trusted => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDeclaration {
    pub(crate) kind: RuntimeKind,
    pub(crate) entry: Option<PortablePackagePath>,
    /// What the manifest asks for. The installer may tighten it and never widen it.
    pub(crate) trust_profile: TrustProfile,
}

// ---------------------------------------------------------------------------
// Requirements and requested capabilities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExtensionRequirements {
    pub(crate) extensions: Vec<ExtensionDependency>,
    pub(crate) skills: Vec<SkillDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionDependency {
    pub(crate) id: ExtensionId,
    pub(crate) version: VersionReq,
    /// An optional dependency that is missing leaves dependent contributions ineligible without
    /// blocking the rest of the package.
    pub(crate) optional: bool,
}

/// A Skill this extension needs but does not own. Resolved through the published Skill API; the
/// extension never installs a competing copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDependency {
    pub(crate) id: String,
    pub(crate) version: VersionReq,
    pub(crate) optional: bool,
}

/// What the package asks to be allowed to do. An upper bound, never a grant: policy still decides
/// each concrete operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CapabilityRequest {
    pub(crate) filesystem_read: Vec<String>,
    pub(crate) filesystem_write: Vec<String>,
    /// Canonical `scheme://host[:port]`. Validated at decode so the text a reviewer approves and
    /// the value the broker matches on are the same thing.
    pub(crate) network_origins: Vec<NetworkOrigin>,
    pub(crate) process_commands: Vec<String>,
    pub(crate) secret_ids: Vec<String>,
}

impl CapabilityRequest {
    /// Whether the package asks for anything beyond pure computation. Drives the install review:
    /// a package requesting nothing needs a much shorter conversation with the user.
    pub(crate) fn is_empty(&self) -> bool {
        self.filesystem_read.is_empty()
            && self.filesystem_write.is_empty()
            && self.network_origins.is_empty()
            && self.process_commands.is_empty()
            && self.secret_ids.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Contributions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ContributionManifest {
    pub(crate) tools: Vec<ToolContribution>,
    pub(crate) skills: Vec<SkillContribution>,
    pub(crate) mcp_definitions: Vec<McpContribution>,
    pub(crate) modes: Vec<ModePresetContribution>,
    pub(crate) hooks: Vec<HookContribution>,
    pub(crate) authorization_rules: Vec<AuthorizationRuleContribution>,
    pub(crate) connectors: Vec<ConnectorContribution>,
    pub(crate) configuration: Vec<ConfigurationContribution>,
    pub(crate) transforms: Vec<TransformContribution>,
}

impl ContributionManifest {
    /// Every declared id paired with its kind, in a stable order.
    ///
    /// The one place duplicate detection can be complete: a tool and a Skill may share a local id,
    /// but two tools may not, and only a view across all kinds can say which case applies.
    pub(crate) fn declared_ids(&self) -> Vec<(ContributionKind, &ContributionLocalId)> {
        let mut declared: Vec<(ContributionKind, &ContributionLocalId)> = Vec::new();
        declared.extend(
            self.tools
                .iter()
                .map(|item| (ContributionKind::Tool, &item.id)),
        );
        declared.extend(
            self.skills
                .iter()
                .map(|item| (ContributionKind::Skill, &item.id)),
        );
        declared.extend(
            self.mcp_definitions
                .iter()
                .map(|item| (ContributionKind::Mcp, &item.id)),
        );
        declared.extend(
            self.modes
                .iter()
                .map(|item| (ContributionKind::Mode, &item.id)),
        );
        declared.extend(
            self.hooks
                .iter()
                .map(|item| (ContributionKind::Hook, &item.id)),
        );
        declared.extend(
            self.authorization_rules
                .iter()
                .map(|item| (ContributionKind::Rule, &item.id)),
        );
        declared.extend(
            self.connectors
                .iter()
                .map(|item| (ContributionKind::Connector, &item.id)),
        );
        declared.extend(
            self.configuration
                .iter()
                .map(|item| (ContributionKind::Configuration, &item.id)),
        );
        declared.extend(
            self.transforms
                .iter()
                .map(|item| (ContributionKind::Transform, &item.id)),
        );
        declared
    }

    /// Every path the manifest points at, for snapshot-containment and collision checks.
    pub(crate) fn declared_paths(&self) -> Vec<&PortablePackagePath> {
        let mut paths: Vec<&PortablePackagePath> = Vec::new();
        for tool in &self.tools {
            paths.extend(tool.input_schema.iter());
            paths.extend(tool.output_schema.iter());
        }
        paths.extend(self.skills.iter().map(|skill| &skill.path));
        paths.extend(
            self.configuration
                .iter()
                .map(|configuration| &configuration.schema),
        );
        paths
    }

    pub(crate) fn total(&self) -> usize {
        self.declared_ids().len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) display_name: String,
    pub(crate) description: Option<String>,
    pub(crate) input_schema: Option<PortablePackagePath>,
    pub(crate) output_schema: Option<PortablePackagePath>,
    /// Entry point inside the extension's runtime. Meaningless without a runtime, which the
    /// decoder checks.
    pub(crate) handler: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillContribution {
    pub(crate) id: ContributionLocalId,
    /// The `SKILL.md` inside the package. Validated by the Skill parser, not here.
    pub(crate) path: PortablePackagePath,
}

/// Transport and non-secret configuration only. Credential values are supplied by the user
/// through MCP's own flows and never travel inside a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) display_name: String,
    pub(crate) transport: McpTransportDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpTransportDeclaration {
    Stdio {
        command: String,
        args: Vec<String>,
        /// Names only. A value here would be a secret in a package.
        env_keys: Vec<String>,
    },
    Http {
        url: String,
        header_keys: Vec<String>,
    },
}

/// Data only. A preset composes strategies the application already registers; it cannot supply
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModePresetContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) display_name: String,
    pub(crate) strategy: String,
    pub(crate) default_policy_template: Option<String>,
    pub(crate) required_tool_groups: Vec<String>,
    pub(crate) required_skills: Vec<String>,
    pub(crate) required_hooks: Vec<ContributionLocalId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) event: String,
    pub(crate) matcher: Vec<(String, Vec<String>)>,
    pub(crate) handler: HookHandlerDeclaration,
    pub(crate) failure_mode: HookFailureMode,
    pub(crate) priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HookHandlerDeclaration {
    /// Runs in the extension's own runtime.
    ExtensionRuntime { entry: String },
    /// Invokes an MCP tool this extension contributed.
    McpTool { tool: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookFailureMode {
    FailClosed,
    FailOpen,
}

impl HookFailureMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::FailClosed => "fail_closed",
            Self::FailOpen => "fail_open",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "fail_closed" => Some(Self::FailClosed),
            "fail_open" => Some(Self::FailOpen),
            _ => None,
        }
    }
}

/// A downloaded package may only ask or deny. `Allow` is not representable here, so a manifest
/// cannot express self-authorization even before policy compilation sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContributedRuleEffect {
    Ask,
    Deny,
}

impl ContributedRuleEffect {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Deny => "deny",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ask" => Some(Self::Ask),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizationRuleContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) operation: String,
    pub(crate) matcher: Vec<(String, Vec<String>)>,
    pub(crate) effect: ContributedRuleEffect,
    pub(crate) risk: String,
    pub(crate) allowed_scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) display_name: String,
    pub(crate) connector_type: String,
    pub(crate) driver: String,
    pub(crate) auth_strategy: String,
    pub(crate) capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigurationContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) schema: PortablePackagePath,
}

/// A bounded, provenance-labelled prompt or message transform. Whether the event admits one is
/// decided by the Hook engine, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformContribution {
    pub(crate) id: ContributionLocalId,
    pub(crate) event: String,
    pub(crate) handler: String,
}
