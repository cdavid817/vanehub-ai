use super::{
    validate_bounded_schema, BoundedJsonSchema, SkillToolDomainError, SkillToolId,
    SkillToolLimitOverrides, SkillToolLimits, SkillToolManifestLimits, SkillToolOwnerId,
    DEFAULT_SKILL_TOOL_LIMITS, MODULE_DIRECTORY, SUPPORTED_MANIFEST_VERSION,
};
use serde_json::{Map, Value};

/// Implementation kinds a Skill package may not ship, matched case-insensitively on the declared
/// kind so `Python` and `PowerShell` fail the same way.
///
/// Listed explicitly instead of relying on the unknown-kind fallback because these produce a
/// distinct diagnostic: the manifest is well-formed and the author's intent is clear, and it is
/// refused on policy (`design.md` non-goals), not because the runtime failed to understand it.
const FORBIDDEN_IMPLEMENTATION_KINDS: &[&str] = &[
    "python",
    "python3",
    "shell",
    "sh",
    "bash",
    "zsh",
    "powershell",
    "pwsh",
    "batch",
    "cmd",
    "node",
    "javascript",
    "typescript",
    "deno",
    "bun",
    "ruby",
    "perl",
    "exec",
    "process",
    "command",
    "native",
    "binary",
    "dylib",
    "dll",
    "so",
];

const MANIFEST_FIELDS: &[&str] = &["version", "skillId", "tools"];
const TOOL_FIELDS: &[&str] = &[
    "id",
    "description",
    "implementation",
    "input",
    "output",
    "capabilities",
    "limits",
];
const LIMIT_FIELDS: &[&str] = &[
    "wallTimeMs",
    "fuel",
    "memoryBytes",
    "inputBytes",
    "outputBytes",
    "hostCalls",
    "delegationDepth",
    "concurrency",
];

/// A host operation a tool is allowed to request. Only registered tool operations are
/// expressible: a capability is an upper bound on delegation, never a filesystem or network grant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillToolCapability {
    Tool(String),
}

impl SkillToolCapability {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillToolDomainError> {
        let operation = value
            .strip_prefix("tool:")
            .filter(|operation| {
                !operation.is_empty()
                    && operation.len() <= 64
                    && operation.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            })
            .ok_or_else(|| {
                SkillToolDomainError::UnknownCapability(value.chars().take(64).collect())
            })?;
        Ok(Self::Tool(operation.to_string()))
    }

    pub(crate) fn as_declaration(&self) -> String {
        match self {
            Self::Tool(operation) => format!("tool:{operation}"),
        }
    }

    pub(crate) fn operation(&self) -> &str {
        match self {
            Self::Tool(operation) => operation,
        }
    }
}

/// A `sha256:<64 hex>` content hash. Parsed rather than stored as a free string so a manifest
/// cannot bind an implementation to an algorithm the verifier does not compute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContentHash(String);

impl ContentHash {
    pub(crate) fn parse(value: &str) -> Result<Self, SkillToolDomainError> {
        let digest = value
            .strip_prefix("sha256:")
            .map(str::to_ascii_lowercase)
            .filter(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| {
                SkillToolDomainError::InvalidContentHash(value.chars().take(80).collect())
            })?;
        Ok(Self(format!("sha256:{digest}")))
    }

    pub(crate) fn from_digest(digest: &str) -> Self {
        Self(format!("sha256:{}", digest.to_ascii_lowercase()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeclarativeFieldSource {
    /// Projects a top-level property of the validated input.
    Input(String),
    /// A manifest-authored constant.
    Constant(Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarativeField {
    pub(crate) name: String,
    pub(crate) source: DeclarativeFieldSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarativeImplementation {
    pub(crate) target: SkillToolCapability,
    pub(crate) template: Vec<DeclarativeField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleImplementation {
    pub(crate) path: String,
    pub(crate) export: String,
    pub(crate) content_hash: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolImplementation {
    Declarative(DeclarativeImplementation),
    Module(ModuleImplementation),
}

impl SkillToolImplementation {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Declarative(_) => "declarative",
            Self::Module(_) => "wasm",
        }
    }

    /// The integrity-bound file this implementation reads, if any. Declarative tools have none:
    /// they carry no bytes beyond the manifest that already covers them.
    pub(crate) fn implementation_path(&self) -> Option<&str> {
        match self {
            Self::Declarative(_) => None,
            Self::Module(module) => Some(module.path.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolDeclaration {
    pub(crate) id: SkillToolId,
    pub(crate) description: String,
    pub(crate) implementation: SkillToolImplementation,
    pub(crate) input: BoundedJsonSchema,
    pub(crate) output: BoundedJsonSchema,
    pub(crate) capabilities: Vec<SkillToolCapability>,
    pub(crate) limits: SkillToolLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolManifest {
    pub(crate) version: u32,
    pub(crate) owner: SkillToolOwnerId,
    pub(crate) tools: Vec<SkillToolDeclaration>,
}

impl SkillToolManifest {
    /// Relative paths of every file this manifest integrity-binds, sorted. Discovery compares the
    /// package's actual tool directory against this list so an undeclared file is never loaded.
    pub(crate) fn declared_implementation_paths(&self) -> Vec<String> {
        let mut paths = self
            .tools
            .iter()
            .filter_map(|tool| tool.implementation.implementation_path())
            .map(str::to_string)
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }
}

/// Parses raw manifest bytes.
///
/// The byte budget is checked before `serde_json` sees the input: a multi-megabyte manifest is a
/// resource decision, and paying full parse cost to discover that would defeat the limit.
pub(crate) fn parse_manifest_bytes(
    bytes: &[u8],
    limits: &SkillToolManifestLimits,
) -> Result<SkillToolManifest, SkillToolDomainError> {
    if bytes.len() as u64 > limits.maximum_manifest_bytes {
        return Err(SkillToolDomainError::TooLarge {
            maximum: limits.maximum_manifest_bytes,
            actual: bytes.len() as u64,
        });
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| SkillToolDomainError::ManifestNotAnObject)?;
    parse_manifest(&value, limits)
}

pub(crate) fn parse_manifest(
    value: &Value,
    limits: &SkillToolManifestLimits,
) -> Result<SkillToolManifest, SkillToolDomainError> {
    let object = value
        .as_object()
        .ok_or(SkillToolDomainError::ManifestNotAnObject)?;
    reject_unknown_fields(object, MANIFEST_FIELDS)?;

    let version = object
        .get("version")
        .ok_or(SkillToolDomainError::MissingField("version"))?
        .as_u64()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("version"))?;
    if version != u64::from(SUPPORTED_MANIFEST_VERSION) {
        return Err(SkillToolDomainError::UnsupportedManifestVersion {
            supported: SUPPORTED_MANIFEST_VERSION,
            actual: version,
        });
    }

    let owner = SkillToolOwnerId::parse(
        object
            .get("skillId")
            .ok_or(SkillToolDomainError::MissingField("skillId"))?
            .as_str()
            .ok_or(SkillToolDomainError::UnexpectedFieldType("skillId"))?,
    )?;

    let entries = object
        .get("tools")
        .ok_or(SkillToolDomainError::MissingField("tools"))?
        .as_array()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("tools"))?;
    if entries.is_empty() {
        return Err(SkillToolDomainError::EmptyToolList);
    }
    if entries.len() > limits.maximum_tools {
        return Err(SkillToolDomainError::TooManyTools {
            maximum: limits.maximum_tools,
            actual: entries.len(),
        });
    }

    let mut tools = Vec::with_capacity(entries.len());
    for entry in entries {
        let tool = parse_tool(entry, limits)?;
        if tools
            .iter()
            .any(|existing: &SkillToolDeclaration| existing.id == tool.id)
        {
            return Err(SkillToolDomainError::DuplicateToolId(
                tool.id.as_str().to_string(),
            ));
        }
        tools.push(tool);
    }
    Ok(SkillToolManifest {
        version: SUPPORTED_MANIFEST_VERSION,
        owner,
        tools,
    })
}

fn parse_tool(
    value: &Value,
    limits: &SkillToolManifestLimits,
) -> Result<SkillToolDeclaration, SkillToolDomainError> {
    let object = value
        .as_object()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("tools"))?;
    reject_unknown_fields(object, TOOL_FIELDS)?;

    let id = SkillToolId::parse(
        object
            .get("id")
            .ok_or(SkillToolDomainError::MissingField("id"))?
            .as_str()
            .ok_or(SkillToolDomainError::UnexpectedFieldType("id"))?,
    )?;
    let description = object
        .get("description")
        .ok_or(SkillToolDomainError::MissingField("description"))?
        .as_str()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("description"))?;
    let described = description.chars().count();
    if described == 0 || described > limits.maximum_description_characters {
        return Err(SkillToolDomainError::DescriptionTooLong {
            maximum: limits.maximum_description_characters,
            actual: described,
        });
    }

    let input = validate_bounded_schema(
        object
            .get("input")
            .ok_or(SkillToolDomainError::MissingField("input"))?,
        limits,
    )?;
    let output = validate_bounded_schema(
        object
            .get("output")
            .ok_or(SkillToolDomainError::MissingField("output"))?,
        limits,
    )?;

    let capabilities = parse_capabilities(object.get("capabilities"), limits)?;
    let overrides = parse_limit_overrides(object.get("limits"))?;
    let implementation = parse_implementation(
        object
            .get("implementation")
            .ok_or(SkillToolDomainError::MissingField("implementation"))?,
        &capabilities,
        &input,
        limits,
    )?;

    Ok(SkillToolDeclaration {
        id,
        description: description.to_string(),
        implementation,
        input,
        output,
        capabilities,
        limits: DEFAULT_SKILL_TOOL_LIMITS.tighten(overrides)?,
    })
}

fn parse_capabilities(
    value: Option<&Value>,
    limits: &SkillToolManifestLimits,
) -> Result<Vec<SkillToolCapability>, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("capabilities"))?;
    if entries.len() > limits.maximum_capabilities {
        return Err(SkillToolDomainError::TooManyCapabilities {
            maximum: limits.maximum_capabilities,
            actual: entries.len(),
        });
    }
    let mut capabilities = Vec::with_capacity(entries.len());
    for entry in entries {
        let capability = SkillToolCapability::parse(
            entry
                .as_str()
                .ok_or(SkillToolDomainError::UnexpectedFieldType("capabilities"))?,
        )?;
        if capabilities.contains(&capability) {
            return Err(SkillToolDomainError::DuplicateCapability(
                capability.as_declaration(),
            ));
        }
        capabilities.push(capability);
    }
    capabilities.sort();
    Ok(capabilities)
}

fn parse_limit_overrides(
    value: Option<&Value>,
) -> Result<SkillToolLimitOverrides, SkillToolDomainError> {
    let Some(value) = value else {
        return Ok(SkillToolLimitOverrides::default());
    };
    let object = value
        .as_object()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("limits"))?;
    reject_unknown_fields(object, LIMIT_FIELDS)?;
    let number = |field: &'static str| -> Result<Option<u64>, SkillToolDomainError> {
        object
            .get(field)
            .map(|value| {
                value
                    .as_u64()
                    .ok_or(SkillToolDomainError::UnexpectedFieldType("limits"))
            })
            .transpose()
    };
    let count = |field: &'static str| -> Result<Option<u32>, SkillToolDomainError> {
        Ok(match number(field)? {
            Some(value) => Some(
                u32::try_from(value)
                    .map_err(|_| SkillToolDomainError::LimitAboveCeiling { limit: field })?,
            ),
            None => None,
        })
    };
    Ok(SkillToolLimitOverrides {
        wall_time_milliseconds: number("wallTimeMs")?,
        fuel: number("fuel")?,
        memory_bytes: number("memoryBytes")?,
        input_bytes: number("inputBytes")?,
        output_bytes: number("outputBytes")?,
        host_calls: count("hostCalls")?,
        delegation_depth: count("delegationDepth")?,
        concurrency: count("concurrency")?,
    })
}

fn parse_implementation(
    value: &Value,
    capabilities: &[SkillToolCapability],
    input: &BoundedJsonSchema,
    limits: &SkillToolManifestLimits,
) -> Result<SkillToolImplementation, SkillToolDomainError> {
    let object = value
        .as_object()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("implementation"))?;
    let kind = object
        .get("kind")
        .ok_or(SkillToolDomainError::MissingField("kind"))?
        .as_str()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("kind"))?;
    let normalized = kind.to_ascii_lowercase();
    match normalized.as_str() {
        "declarative" => parse_declarative(object, capabilities, input, limits),
        "wasm" => parse_module(object),
        _ if FORBIDDEN_IMPLEMENTATION_KINDS.contains(&normalized.as_str()) => Err(
            SkillToolDomainError::ForbiddenImplementationKind(normalized),
        ),
        _ => Err(SkillToolDomainError::UnknownImplementationKind(
            normalized.chars().take(64).collect(),
        )),
    }
}

fn parse_declarative(
    object: &Map<String, Value>,
    capabilities: &[SkillToolCapability],
    input: &BoundedJsonSchema,
    limits: &SkillToolManifestLimits,
) -> Result<SkillToolImplementation, SkillToolDomainError> {
    reject_unknown_fields(object, &["kind", "target", "template"])?;
    let target = SkillToolCapability::parse(
        object
            .get("target")
            .ok_or(SkillToolDomainError::MissingField("target"))?
            .as_str()
            .ok_or(SkillToolDomainError::UnexpectedFieldType("target"))?,
    )?;
    if !capabilities.contains(&target) {
        return Err(SkillToolDomainError::UndeclaredCapability(
            target.as_declaration(),
        ));
    }
    let template = object
        .get("template")
        .ok_or(SkillToolDomainError::MissingField("template"))?
        .as_object()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("template"))?;
    if template.len() > limits.maximum_template_fields {
        return Err(SkillToolDomainError::TooManyTemplateFields {
            maximum: limits.maximum_template_fields,
            actual: template.len(),
        });
    }

    let declared_input_properties = input
        .as_value()
        .get("properties")
        .and_then(Value::as_object);
    let mut fields = Vec::with_capacity(template.len());
    for (name, node) in template {
        fields.push(DeclarativeField {
            name: name.clone(),
            source: parse_template_field(name, node, declared_input_properties)?,
        });
    }
    fields.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(SkillToolImplementation::Declarative(
        DeclarativeImplementation {
            target,
            template: fields,
        },
    ))
}

/// A template field is exactly one of `{"$input": "<property>"}` or `{"$const": <json>}`.
///
/// Anything else — a bare string, a second key, `$if`, `$each`, `$expr`, `$shell` — is refused.
/// Making the accepted shape a closed two-member set is what keeps loops, conditionals,
/// expressions, shell expansion, and pipelines out without needing to enumerate them.
fn parse_template_field(
    name: &str,
    node: &Value,
    declared_input_properties: Option<&Map<String, Value>>,
) -> Result<DeclarativeFieldSource, SkillToolDomainError> {
    let object = node
        .as_object()
        .filter(|object| object.len() == 1)
        .ok_or_else(|| SkillToolDomainError::InvalidTemplate(name.chars().take(64).collect()))?;
    if let Some(property) = object.get("$input") {
        let property = property.as_str().ok_or_else(|| {
            SkillToolDomainError::InvalidTemplate(name.chars().take(64).collect())
        })?;
        let declared =
            declared_input_properties.is_some_and(|properties| properties.contains_key(property));
        if !declared {
            return Err(SkillToolDomainError::InvalidTemplate(
                property.chars().take(64).collect(),
            ));
        }
        return Ok(DeclarativeFieldSource::Input(property.to_string()));
    }
    if let Some(constant) = object.get("$const") {
        return Ok(DeclarativeFieldSource::Constant(constant.clone()));
    }
    Err(SkillToolDomainError::InvalidTemplate(
        object
            .keys()
            .next()
            .map(|key| key.chars().take(64).collect())
            .unwrap_or_default(),
    ))
}

fn parse_module(
    object: &Map<String, Value>,
) -> Result<SkillToolImplementation, SkillToolDomainError> {
    reject_unknown_fields(object, &["kind", "module", "export", "hash"])?;
    let path = object
        .get("module")
        .ok_or(SkillToolDomainError::MissingField("module"))?
        .as_str()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("module"))?;
    validate_module_path(path)?;
    let export = object
        .get("export")
        .ok_or(SkillToolDomainError::MissingField("export"))?
        .as_str()
        .ok_or(SkillToolDomainError::UnexpectedFieldType("export"))?;
    if export.is_empty()
        || export.len() > 64
        || !export
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(SkillToolDomainError::UnexpectedFieldType("export"));
    }
    let content_hash = ContentHash::parse(
        object
            .get("hash")
            .ok_or(SkillToolDomainError::MissingField("hash"))?
            .as_str()
            .ok_or(SkillToolDomainError::UnexpectedFieldType("hash"))?,
    )?;
    Ok(SkillToolImplementation::Module(ModuleImplementation {
        path: path.to_string(),
        export: export.to_string(),
        content_hash,
    }))
}

/// Module paths are validated as text before any filesystem call: the traversal, absolute-path,
/// separator, and extension rules must hold for a manifest that is only being inspected, on a
/// machine where the referenced file may not exist at all.
fn validate_module_path(path: &str) -> Result<(), SkillToolDomainError> {
    let unsafe_path =
        || SkillToolDomainError::UnsafeImplementationPath(path.chars().take(80).collect());
    if !path.starts_with(MODULE_DIRECTORY) || !path.ends_with(".wasm") {
        return Err(unsafe_path());
    }
    if path.contains('\\') || path.contains(':') || path.contains('%') {
        return Err(unsafe_path());
    }
    let components = path.split('/').collect::<Vec<_>>();
    if components.len() != 3 {
        return Err(unsafe_path());
    }
    let file = components[2];
    if file.len() <= ".wasm".len()
        || file.starts_with('.')
        || !file.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
        || file.matches('.').count() != 1
    {
        return Err(unsafe_path());
    }
    Ok(())
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), SkillToolDomainError> {
    match object.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(unknown) => Err(SkillToolDomainError::UnknownField(
            unknown.chars().take(64).collect(),
        )),
        None => Ok(()),
    }
}
