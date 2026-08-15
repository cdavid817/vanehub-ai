use super::{SkillAvailability, SkillTrust, SkillType};
use std::collections::BTreeMap;

/// Platform ceilings a Utility package can lower but never raise. Package authors must not
/// control platform resource budgets, so every declared limit is clamped to these values and
/// the clamped fields are reported so management surfaces can explain the difference.
pub(crate) const DELEGATION_MODEL_ROUNDS_CEILING: u16 = 8;
pub(crate) const DELEGATION_TIMEOUT_SECONDS_CEILING: u32 = 120;
pub(crate) const DELEGATION_CONTEXT_CHARS_CEILING: u32 = 16_000;
pub(crate) const DELEGATION_OUTPUT_CHARS_CEILING: u32 = 12_000;

/// Capability ids are VaneHub-owned stable categories mapped to existing tool operations. They
/// are never executable entry points, so a Utility cannot name an arbitrary tool and have it
/// treated as an authorization grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillDelegationCapabilityId {
    FileRead,
    FileWrite,
    ScopedEdit,
    ContentSearch,
    FilenameSearch,
    MemoryRead,
    MemoryWrite,
    SkillRead,
    Shell,
}

impl SkillDelegationCapabilityId {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "file-read" => Some(Self::FileRead),
            "file-write" => Some(Self::FileWrite),
            "scoped-edit" => Some(Self::ScopedEdit),
            "content-search" => Some(Self::ContentSearch),
            "filename-search" => Some(Self::FilenameSearch),
            "memory-read" => Some(Self::MemoryRead),
            "memory-write" => Some(Self::MemoryWrite),
            "skill-read" => Some(Self::SkillRead),
            "shell" => Some(Self::Shell),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::FileRead => "file-read",
            Self::FileWrite => "file-write",
            Self::ScopedEdit => "scoped-edit",
            Self::ContentSearch => "content-search",
            Self::FilenameSearch => "filename-search",
            Self::MemoryRead => "memory-read",
            Self::MemoryWrite => "memory-write",
            Self::SkillRead => "skill-read",
            Self::Shell => "shell",
        }
    }

    pub(crate) fn is_read_only(self) -> bool {
        matches!(
            self,
            Self::FileRead
                | Self::ContentSearch
                | Self::FilenameSearch
                | Self::MemoryRead
                | Self::SkillRead
        )
    }
}

/// A Utility that omits `delegation` receives this read-only set rather than nothing, so a
/// content-only Utility package stays useful without declaring a contract.
pub(crate) const DEFAULT_DELEGATION_CAPABILITIES: &[SkillDelegationCapabilityId] = &[
    SkillDelegationCapabilityId::FileRead,
    SkillDelegationCapabilityId::ContentSearch,
    SkillDelegationCapabilityId::FilenameSearch,
    SkillDelegationCapabilityId::SkillRead,
];

/// Categories a Utility may never request. They are rejected with a distinct reason instead of
/// being reported as unknown, because naming one is an authorization mistake rather than a typo.
const PROHIBITED_CAPABILITY_IDS: &[&str] = &[
    "delegate-skill",
    "delegate-utility-skill",
    "mcp",
    "script",
    "dynamic-script",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkillDelegationLimitField {
    MaxRounds,
    TimeoutSeconds,
    MaxContextChars,
    MaxOutputChars,
}

impl SkillDelegationLimitField {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MaxRounds => "max-rounds",
            Self::TimeoutSeconds => "timeout-seconds",
            Self::MaxContextChars => "max-context-chars",
            Self::MaxOutputChars => "max-output-chars",
        }
    }

    fn declaration_key(self) -> &'static str {
        match self {
            Self::MaxRounds => "max_rounds",
            Self::TimeoutSeconds => "timeout_seconds",
            Self::MaxContextChars => "max_context_chars",
            Self::MaxOutputChars => "max_output_chars",
        }
    }
}

const DELEGATION_LIMIT_FIELDS: &[SkillDelegationLimitField] = &[
    SkillDelegationLimitField::MaxRounds,
    SkillDelegationLimitField::TimeoutSeconds,
    SkillDelegationLimitField::MaxContextChars,
    SkillDelegationLimitField::MaxOutputChars,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillDelegationLimits {
    pub(crate) max_rounds: u16,
    pub(crate) timeout_seconds: u32,
    pub(crate) max_context_chars: u32,
    pub(crate) max_output_chars: u32,
}

impl SkillDelegationLimits {
    pub(crate) const PLATFORM: Self = Self {
        max_rounds: DELEGATION_MODEL_ROUNDS_CEILING,
        timeout_seconds: DELEGATION_TIMEOUT_SECONDS_CEILING,
        max_context_chars: DELEGATION_CONTEXT_CHARS_CEILING,
        max_output_chars: DELEGATION_OUTPUT_CHARS_CEILING,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SkillDelegationRequestedLimits {
    pub(crate) max_rounds: Option<u16>,
    pub(crate) timeout_seconds: Option<u32>,
    pub(crate) max_context_chars: Option<u32>,
    pub(crate) max_output_chars: Option<u32>,
}

/// Safe, content-free reasons. Every value is a fixed identifier so a refusal can be persisted,
/// logged, and rendered without leaking Skill instructions, paths, or declared values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillDelegationUnavailableReason {
    UnknownCapability,
    ProhibitedCapability,
    DuplicateCapability,
    EmptyCapabilityList,
    UnknownContractField,
    InvalidLimit,
    Untrusted,
    SkillUnavailable,
    UnsupportedCliAgent,
    UnsupportedApiRuntime,
}

impl SkillDelegationUnavailableReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::UnknownCapability => "unknown-capability",
            Self::ProhibitedCapability => "prohibited-capability",
            Self::DuplicateCapability => "duplicate-capability",
            Self::EmptyCapabilityList => "empty-capability-list",
            Self::UnknownContractField => "unknown-contract-field",
            Self::InvalidLimit => "invalid-limit",
            Self::Untrusted => "untrusted",
            Self::SkillUnavailable => "skill-unavailable",
            Self::UnsupportedCliAgent => "unsupported-cli-agent",
            Self::UnsupportedApiRuntime => "unsupported-api-runtime",
        }
    }
}

/// The `delegation` frontmatter block exactly as declared. Keeping the untyped form on the
/// metadata lets an invalid contract stay visible for repair instead of making the whole Skill
/// unparseable, and lets `compose` round-trip a block the editor never learned to render.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RawSkillDelegation {
    pub(crate) tools: Vec<String>,
    pub(crate) fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillDelegationDeclaration {
    declared: Option<RawSkillDelegation>,
}

impl SkillDelegationDeclaration {
    pub(crate) fn declared(raw: RawSkillDelegation) -> Self {
        Self {
            declared: Some(raw),
        }
    }

    pub(crate) fn is_absent(&self) -> bool {
        self.declared.is_none()
    }

    pub(crate) fn raw(&self) -> Option<&RawSkillDelegation> {
        self.declared.as_ref()
    }

    pub(crate) fn resolve(
        &self,
    ) -> Result<SkillDelegationContract, SkillDelegationUnavailableReason> {
        let Some(raw) = &self.declared else {
            return Ok(SkillDelegationContract::platform_default());
        };
        let declared_capabilities = parse_capabilities(&raw.tools)?;
        let requested_limits = parse_requested_limits(&raw.fields)?;
        Ok(SkillDelegationContract::from_declaration(
            declared_capabilities,
            requested_limits,
        ))
    }
}

fn parse_capabilities(
    tools: &[String],
) -> Result<Vec<SkillDelegationCapabilityId>, SkillDelegationUnavailableReason> {
    if tools.is_empty() {
        return Err(SkillDelegationUnavailableReason::EmptyCapabilityList);
    }
    let mut capabilities = Vec::with_capacity(tools.len());
    for tool in tools {
        let tool = tool.trim();
        if PROHIBITED_CAPABILITY_IDS.contains(&tool) {
            return Err(SkillDelegationUnavailableReason::ProhibitedCapability);
        }
        let capability = SkillDelegationCapabilityId::parse(tool)
            .ok_or(SkillDelegationUnavailableReason::UnknownCapability)?;
        if capabilities.contains(&capability) {
            return Err(SkillDelegationUnavailableReason::DuplicateCapability);
        }
        capabilities.push(capability);
    }
    Ok(capabilities)
}

fn parse_requested_limits(
    fields: &BTreeMap<String, String>,
) -> Result<SkillDelegationRequestedLimits, SkillDelegationUnavailableReason> {
    for key in fields.keys() {
        if !DELEGATION_LIMIT_FIELDS
            .iter()
            .any(|field| field.declaration_key() == key)
        {
            return Err(SkillDelegationUnavailableReason::UnknownContractField);
        }
    }
    Ok(SkillDelegationRequestedLimits {
        max_rounds: parse_limit(fields, SkillDelegationLimitField::MaxRounds)?,
        timeout_seconds: parse_limit(fields, SkillDelegationLimitField::TimeoutSeconds)?,
        max_context_chars: parse_limit(fields, SkillDelegationLimitField::MaxContextChars)?,
        max_output_chars: parse_limit(fields, SkillDelegationLimitField::MaxOutputChars)?,
    })
}

fn parse_limit<T>(
    fields: &BTreeMap<String, String>,
    field: SkillDelegationLimitField,
) -> Result<Option<T>, SkillDelegationUnavailableReason>
where
    T: std::str::FromStr + PartialEq + From<u8>,
{
    let Some(raw) = fields.get(field.declaration_key()) else {
        return Ok(None);
    };
    let value = raw
        .trim()
        .parse::<T>()
        .map_err(|_| SkillDelegationUnavailableReason::InvalidLimit)?;
    if value == T::from(0u8) {
        return Err(SkillDelegationUnavailableReason::InvalidLimit);
    }
    Ok(Some(value))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDelegationContract {
    pub(crate) declared_capabilities: Vec<SkillDelegationCapabilityId>,
    pub(crate) effective_capabilities: Vec<SkillDelegationCapabilityId>,
    pub(crate) requested_limits: SkillDelegationRequestedLimits,
    pub(crate) effective_limits: SkillDelegationLimits,
    pub(crate) capped_limits: Vec<SkillDelegationLimitField>,
    pub(crate) uses_platform_default: bool,
}

impl SkillDelegationContract {
    fn platform_default() -> Self {
        Self {
            declared_capabilities: DEFAULT_DELEGATION_CAPABILITIES.to_vec(),
            effective_capabilities: DEFAULT_DELEGATION_CAPABILITIES.to_vec(),
            requested_limits: SkillDelegationRequestedLimits::default(),
            effective_limits: SkillDelegationLimits::PLATFORM,
            capped_limits: Vec::new(),
            uses_platform_default: true,
        }
    }

    fn from_declaration(
        declared_capabilities: Vec<SkillDelegationCapabilityId>,
        requested_limits: SkillDelegationRequestedLimits,
    ) -> Self {
        let mut capped_limits = Vec::new();
        let effective_limits = SkillDelegationLimits {
            max_rounds: cap(
                requested_limits.max_rounds,
                DELEGATION_MODEL_ROUNDS_CEILING,
                SkillDelegationLimitField::MaxRounds,
                &mut capped_limits,
            ),
            timeout_seconds: cap(
                requested_limits.timeout_seconds,
                DELEGATION_TIMEOUT_SECONDS_CEILING,
                SkillDelegationLimitField::TimeoutSeconds,
                &mut capped_limits,
            ),
            max_context_chars: cap(
                requested_limits.max_context_chars,
                DELEGATION_CONTEXT_CHARS_CEILING,
                SkillDelegationLimitField::MaxContextChars,
                &mut capped_limits,
            ),
            max_output_chars: cap(
                requested_limits.max_output_chars,
                DELEGATION_OUTPUT_CHARS_CEILING,
                SkillDelegationLimitField::MaxOutputChars,
                &mut capped_limits,
            ),
        };
        Self {
            effective_capabilities: declared_capabilities.clone(),
            declared_capabilities,
            requested_limits,
            effective_limits,
            capped_limits,
            uses_platform_default: false,
        }
    }

    pub(crate) fn is_read_only(&self) -> bool {
        self.effective_capabilities
            .iter()
            .all(|capability| capability.is_read_only())
    }
}

fn cap<T: Ord + Copy>(
    requested: Option<T>,
    ceiling: T,
    field: SkillDelegationLimitField,
    capped: &mut Vec<SkillDelegationLimitField>,
) -> T {
    match requested {
        Some(value) if value > ceiling => {
            capped.push(field);
            ceiling
        }
        Some(value) => value,
        None => ceiling,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillDelegationEligibility {
    /// Role Skills are delivered as prompts; delegation fields must not imply otherwise.
    NotApplicable,
    Available(SkillDelegationContract),
    Unavailable {
        reason: SkillDelegationUnavailableReason,
        /// Present when the contract parsed but the Skill itself is not currently delegatable,
        /// so management surfaces can still show declared capabilities for repair.
        contract: Option<SkillDelegationContract>,
    },
}

pub(crate) fn evaluate_delegation_eligibility(
    skill_type: SkillType,
    trust: SkillTrust,
    availability: SkillAvailability,
    declaration: &SkillDelegationDeclaration,
) -> SkillDelegationEligibility {
    if skill_type != SkillType::Utility {
        return SkillDelegationEligibility::NotApplicable;
    }
    let contract = match declaration.resolve() {
        Ok(contract) => contract,
        Err(reason) => {
            return SkillDelegationEligibility::Unavailable {
                reason,
                contract: None,
            }
        }
    };
    if trust != SkillTrust::Trusted {
        return SkillDelegationEligibility::Unavailable {
            reason: SkillDelegationUnavailableReason::Untrusted,
            contract: Some(contract),
        };
    }
    // `Unsupported` is the availability an effective Utility carries while the delegation
    // runtime is still gated; it is not a repairable package defect, so it stays delegatable
    // metadata rather than an unavailable reason.
    if !matches!(
        availability,
        SkillAvailability::Available | SkillAvailability::Unsupported
    ) {
        return SkillDelegationEligibility::Unavailable {
            reason: SkillDelegationUnavailableReason::SkillUnavailable,
            contract: Some(contract),
        };
    }
    SkillDelegationEligibility::Available(contract)
}

/// How an Agent participates in delegated Utility assignment. Only native API runtimes able to
/// receive the fixed delegation tool can hold a delegated Utility assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillDelegationAgentRuntime {
    NativeApi,
    UnsupportedApi,
    Cli,
}

impl SkillDelegationAgentRuntime {
    pub(crate) fn classify(is_api: bool, interface_format: Option<&str>) -> Self {
        if !is_api {
            return Self::Cli;
        }
        match interface_format {
            Some("anthropic") | Some("openai-compatible") => Self::NativeApi,
            _ => Self::UnsupportedApi,
        }
    }

    pub(crate) fn supports_delegation(self) -> bool {
        self == Self::NativeApi
    }
}

/// Evaluated only for `type: utility` Skills. Role prompt bindings and CLI mount bindings keep
/// their existing behavior because they never reach this check.
pub(crate) fn evaluate_delegated_assignment(
    runtime: SkillDelegationAgentRuntime,
) -> Result<(), SkillDelegationUnavailableReason> {
    match runtime {
        SkillDelegationAgentRuntime::NativeApi => Ok(()),
        SkillDelegationAgentRuntime::UnsupportedApi => {
            Err(SkillDelegationUnavailableReason::UnsupportedApiRuntime)
        }
        SkillDelegationAgentRuntime::Cli => {
            Err(SkillDelegationUnavailableReason::UnsupportedCliAgent)
        }
    }
}
