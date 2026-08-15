#![cfg_attr(not(test), allow(dead_code))]

use super::config_document::{parse_block, ConfigDocumentError, ConfigNode};
use sha2::{Digest, Sha256};
use std::fmt;

pub(crate) const MAX_CONFIG_FIELDS: usize = 64;
pub(crate) const MAX_CONFIG_GROUPS: usize = 8;
pub(crate) const MAX_CONFIG_LABEL_CHARACTERS: usize = 128;
pub(crate) const MAX_CONFIG_HELP_CHARACTERS: usize = 512;

const ROOT_KEYWORDS: [&str; 2] = ["properties", "required"];
const FIELD_KEYWORDS: [&str; 18] = [
    "type",
    "items",
    "default",
    "enum",
    "description",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "properties",
    "required",
    "x-vanehub-label",
    "x-vanehub-label-key",
    "x-vanehub-help",
    "x-vanehub-order",
    "x-vanehub-advanced",
];
const SECRET_KEYWORD: &str = "x-vanehub-secret";
const MULTILINE_KEYWORD: &str = "x-vanehub-multiline";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigScalarType {
    Text,
    Integer,
    Number,
    Boolean,
}

impl SkillConfigScalarType {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "string" => Some(Self::Text),
            "integer" => Some(Self::Integer),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "string",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigFieldType {
    Scalar(SkillConfigScalarType),
    List(SkillConfigScalarType),
}

impl SkillConfigFieldType {
    pub(crate) fn item_type(self) -> SkillConfigScalarType {
        match self {
            Self::Scalar(scalar) | Self::List(scalar) => scalar,
        }
    }

    fn canonical(self) -> String {
        match self {
            Self::Scalar(scalar) => scalar.as_str().to_string(),
            Self::List(scalar) => format!("array<{}>", scalar.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SkillConfigValue {
    Text(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    List(Vec<SkillConfigValue>),
}

impl SkillConfigValue {
    /// Canonical form feeds the schema hash, so it must be stable across platforms and must not
    /// depend on float formatting defaults.
    pub(crate) fn canonical(&self) -> String {
        match self {
            Self::Text(value) => format!("s:{value}"),
            Self::Integer(value) => format!("i:{value}"),
            Self::Number(value) => format!("n:{value:?}"),
            Self::Boolean(value) => format!("b:{value}"),
            Self::List(values) => {
                let items = values
                    .iter()
                    .map(Self::canonical)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("l:[{items}]")
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SkillConfigConstraints {
    pub(crate) minimum: Option<f64>,
    pub(crate) maximum: Option<f64>,
    pub(crate) min_length: Option<usize>,
    pub(crate) max_length: Option<usize>,
    pub(crate) min_items: Option<usize>,
    pub(crate) max_items: Option<usize>,
}

impl SkillConfigConstraints {
    fn canonical(&self) -> String {
        let render = |label: &str, value: Option<String>| match value {
            Some(value) => format!("{label}={value};"),
            None => String::new(),
        };
        format!(
            "{}{}{}{}{}{}",
            render("min", self.minimum.map(|value| format!("{value:?}"))),
            render("max", self.maximum.map(|value| format!("{value:?}"))),
            render("minlen", self.min_length.map(|value| value.to_string())),
            render("maxlen", self.max_length.map(|value| value.to_string())),
            render("minitems", self.min_items.map(|value| value.to_string())),
            render("maxitems", self.max_items.map(|value| value.to_string())),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SkillConfigPresentation {
    pub(crate) label: Option<String>,
    pub(crate) label_key: Option<String>,
    pub(crate) help: Option<String>,
    pub(crate) order: Option<u32>,
    pub(crate) advanced: bool,
    pub(crate) multiline: bool,
}

impl SkillConfigPresentation {
    fn canonical(&self) -> String {
        format!(
            "label={};key={};help={};order={};advanced={};multiline={};",
            self.label.clone().unwrap_or_default(),
            self.label_key.clone().unwrap_or_default(),
            self.help.clone().unwrap_or_default(),
            self.order
                .map(|order| order.to_string())
                .unwrap_or_default(),
            self.advanced,
            self.multiline,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigField {
    pub(crate) key: String,
    pub(crate) declared_key: String,
    pub(crate) group: Option<String>,
    pub(crate) field_type: SkillConfigFieldType,
    pub(crate) required: bool,
    pub(crate) secret: bool,
    pub(crate) default: Option<SkillConfigValue>,
    pub(crate) choices: Vec<SkillConfigValue>,
    pub(crate) constraints: SkillConfigConstraints,
    pub(crate) presentation: SkillConfigPresentation,
    pub(crate) description: Option<String>,
}

impl SkillConfigField {
    fn canonical(&self) -> String {
        let choices = self
            .choices
            .iter()
            .map(SkillConfigValue::canonical)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "field:{}|group:{}|type:{}|required:{}|secret:{}|default:{}|enum:[{}]|{}|{}|description:{}",
            self.key,
            self.group.clone().unwrap_or_default(),
            self.field_type.canonical(),
            self.required,
            self.secret,
            self.default
                .as_ref()
                .map(SkillConfigValue::canonical)
                .unwrap_or_default(),
            choices,
            self.constraints.canonical(),
            self.presentation.canonical(),
            self.description.clone().unwrap_or_default(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillConfigGroup {
    pub(crate) key: String,
    pub(crate) declared_key: String,
    pub(crate) presentation: SkillConfigPresentation,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigSchema {
    pub(crate) groups: Vec<SkillConfigGroup>,
    pub(crate) fields: Vec<SkillConfigField>,
    pub(crate) hash: String,
}

impl SkillConfigSchema {
    pub(crate) fn field(&self, key: &str) -> Option<&SkillConfigField> {
        self.fields.iter().find(|field| field.key == key)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillConfigSchemaError {
    Document(ConfigDocumentError),
    UnsupportedKeyword { field: String, keyword: String },
    MissingProperties,
    MissingType(String),
    UnsupportedType { field: String, declared: String },
    DuplicateNormalizedKey(String),
    TooManyFields,
    TooManyGroups,
    NestedGroup(String),
    InvalidDefault { field: String, reason: String },
    InvalidEnum { field: String, reason: String },
    InvalidConstraint { field: String, reason: String },
    InvalidAnnotation { field: String, reason: String },
    UnknownRequiredKey(String),
    SecretNotSupported(String),
}

impl fmt::Display for SkillConfigSchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => write!(formatter, "{error}"),
            Self::UnsupportedKeyword { field, keyword } => write!(
                formatter,
                "config_schema property '{field}' uses unsupported keyword: {keyword}"
            ),
            Self::MissingProperties => {
                formatter.write_str("config_schema requires a properties mapping")
            }
            Self::MissingType(field) => {
                write!(formatter, "config_schema property '{field}' requires a type")
            }
            Self::UnsupportedType { field, declared } => write!(
                formatter,
                "config_schema property '{field}' declares unsupported type: {declared}"
            ),
            Self::DuplicateNormalizedKey(key) => write!(
                formatter,
                "config_schema declares duplicate normalized key: {key}"
            ),
            Self::TooManyFields => write!(
                formatter,
                "config_schema declares more than {MAX_CONFIG_FIELDS} properties"
            ),
            Self::TooManyGroups => write!(
                formatter,
                "config_schema declares more than {MAX_CONFIG_GROUPS} groups"
            ),
            Self::NestedGroup(field) => write!(
                formatter,
                "config_schema group '{field}' nests another group; only one level is supported"
            ),
            Self::InvalidDefault { field, reason } => write!(
                formatter,
                "config_schema property '{field}' has an invalid default: {reason}"
            ),
            Self::InvalidEnum { field, reason } => write!(
                formatter,
                "config_schema property '{field}' has an invalid enum: {reason}"
            ),
            Self::InvalidConstraint { field, reason } => write!(
                formatter,
                "config_schema property '{field}' has an invalid constraint: {reason}"
            ),
            Self::InvalidAnnotation { field, reason } => write!(
                formatter,
                "config_schema property '{field}' has an invalid annotation: {reason}"
            ),
            Self::UnknownRequiredKey(key) => write!(
                formatter,
                "config_schema marks unknown property as required: {key}"
            ),
            Self::SecretNotSupported(field) => write!(
                formatter,
                "config_schema property '{field}' cannot be secret; only string scalars support secrets"
            ),
        }
    }
}

impl std::error::Error for SkillConfigSchemaError {}

impl From<ConfigDocumentError> for SkillConfigSchemaError {
    fn from(error: ConfigDocumentError) -> Self {
        Self::Document(error)
    }
}

/// Case and separator folding is what makes "duplicate normalized keys" detectable: `api-key`,
/// `api_key`, and `API_KEY` address one stored property, so a schema declaring two of them would
/// otherwise produce records that silently overwrite each other.
pub(crate) fn normalize_key(raw: &str) -> String {
    raw.to_ascii_lowercase().replace('-', "_")
}

pub(crate) fn parse_config_schema(
    block: &str,
) -> Result<SkillConfigSchema, SkillConfigSchemaError> {
    let root = parse_block(block)?;
    let entries = root
        .as_mapping()
        .ok_or(SkillConfigSchemaError::MissingProperties)?;
    for (key, _) in entries {
        if !ROOT_KEYWORDS.contains(&key.as_str()) {
            return Err(SkillConfigSchemaError::UnsupportedKeyword {
                field: "(root)".to_string(),
                keyword: key.clone(),
            });
        }
    }
    let properties = root
        .get("properties")
        .and_then(ConfigNode::as_mapping)
        .ok_or(SkillConfigSchemaError::MissingProperties)?;
    let required_root = required_keys(&root, "(root)")?;

    let mut groups = Vec::new();
    let mut fields = Vec::new();
    let mut seen = Vec::new();
    for (declared_key, node) in properties {
        let normalized = normalize_key(declared_key);
        if is_group(node) {
            let group_required = required_keys(node, declared_key)?;
            let nested = node
                .get("properties")
                .and_then(ConfigNode::as_mapping)
                .unwrap_or_default();
            for (nested_key, nested_node) in nested {
                if is_group(nested_node) {
                    return Err(SkillConfigSchemaError::NestedGroup(nested_key.clone()));
                }
                let nested_normalized = normalize_key(nested_key);
                let key = format!("{normalized}.{nested_normalized}");
                register(&mut seen, &key)?;
                fields.push(build_field(
                    key,
                    nested_key,
                    Some(normalized.clone()),
                    nested_node,
                    group_required.contains(&nested_normalized),
                )?);
            }
            groups.push(SkillConfigGroup {
                key: normalized.clone(),
                declared_key: declared_key.clone(),
                presentation: presentation(node, declared_key)?,
            });
            if groups.len() > MAX_CONFIG_GROUPS {
                return Err(SkillConfigSchemaError::TooManyGroups);
            }
            continue;
        }
        register(&mut seen, &normalized)?;
        fields.push(build_field(
            normalized.clone(),
            declared_key,
            None,
            node,
            required_root.contains(&normalized),
        )?);
    }
    if fields.len() > MAX_CONFIG_FIELDS {
        return Err(SkillConfigSchemaError::TooManyFields);
    }
    for key in &required_root {
        let addresses_group = groups.iter().any(|group| &group.key == key);
        if !addresses_group && !fields.iter().any(|field| &field.key == key) {
            return Err(SkillConfigSchemaError::UnknownRequiredKey(key.clone()));
        }
    }

    // Sorting before hashing keeps the hash independent of declaration order, so reordering
    // frontmatter for readability does not present as schema drift to stored configuration.
    let mut ordered = fields.clone();
    ordered.sort_by(|left, right| left.key.cmp(&right.key));
    let mut ordered_groups = groups.clone();
    ordered_groups.sort_by(|left, right| left.key.cmp(&right.key));
    let canonical = ordered_groups
        .iter()
        .map(|group| format!("group:{}|{}", group.key, group.presentation.canonical()))
        .chain(ordered.iter().map(SkillConfigField::canonical))
        .collect::<Vec<_>>()
        .join("\n");
    let hash = Sha256::digest(canonical.as_bytes()).iter().fold(
        String::with_capacity(64),
        |mut encoded, byte| {
            use fmt::Write;
            let _ = write!(encoded, "{byte:02x}");
            encoded
        },
    );

    Ok(SkillConfigSchema {
        groups,
        fields,
        hash,
    })
}

fn is_group(node: &ConfigNode) -> bool {
    let declares_object = node
        .get("type")
        .and_then(ConfigNode::as_scalar)
        .is_some_and(|declared| declared == "object");
    declares_object || (node.get("type").is_none() && node.get("properties").is_some())
}

fn register(seen: &mut Vec<String>, key: &str) -> Result<(), SkillConfigSchemaError> {
    if seen.iter().any(|existing| existing == key) {
        return Err(SkillConfigSchemaError::DuplicateNormalizedKey(
            key.to_string(),
        ));
    }
    seen.push(key.to_string());
    Ok(())
}

fn required_keys(node: &ConfigNode, field: &str) -> Result<Vec<String>, SkillConfigSchemaError> {
    match node.get("required") {
        None => Ok(Vec::new()),
        Some(ConfigNode::Sequence(items)) => {
            Ok(items.iter().map(|item| normalize_key(item)).collect())
        }
        Some(_) => Err(SkillConfigSchemaError::InvalidAnnotation {
            field: field.to_string(),
            reason: "required must be a list of property keys".to_string(),
        }),
    }
}

fn build_field(
    key: String,
    declared_key: &str,
    group: Option<String>,
    node: &ConfigNode,
    required: bool,
) -> Result<SkillConfigField, SkillConfigSchemaError> {
    let entries = node
        .as_mapping()
        .ok_or_else(|| SkillConfigSchemaError::MissingType(key.clone()))?;
    for (keyword, _) in entries {
        let supported = FIELD_KEYWORDS.contains(&keyword.as_str())
            || keyword == SECRET_KEYWORD
            || keyword == MULTILINE_KEYWORD;
        if !supported {
            return Err(SkillConfigSchemaError::UnsupportedKeyword {
                field: key.clone(),
                keyword: keyword.clone(),
            });
        }
    }
    let declared_type = node
        .get("type")
        .and_then(ConfigNode::as_scalar)
        .ok_or_else(|| SkillConfigSchemaError::MissingType(key.clone()))?;
    let field_type = if declared_type == "array" {
        SkillConfigFieldType::List(item_type(node, &key)?)
    } else {
        SkillConfigFieldType::Scalar(SkillConfigScalarType::parse(declared_type).ok_or_else(
            || SkillConfigSchemaError::UnsupportedType {
                field: key.clone(),
                declared: declared_type.to_string(),
            },
        )?)
    };
    let secret = flag(node, SECRET_KEYWORD, &key)?;
    if secret && field_type != SkillConfigFieldType::Scalar(SkillConfigScalarType::Text) {
        return Err(SkillConfigSchemaError::SecretNotSupported(key.clone()));
    }
    let constraints = constraints(node, &key)?;
    let choices = choices(node, field_type, &key)?;
    let default = match node.get("default") {
        None => None,
        Some(raw) => {
            let value = coerce(raw, field_type, &key)?;
            validate_value(&value, field_type, &constraints, &choices, &key)?;
            Some(value)
        }
    };
    if secret && default.is_some() {
        return Err(SkillConfigSchemaError::InvalidDefault {
            field: key.clone(),
            reason: "secret properties cannot declare a default".to_string(),
        });
    }
    Ok(SkillConfigField {
        key,
        declared_key: declared_key.to_string(),
        group,
        field_type,
        required,
        secret,
        default,
        choices,
        constraints,
        presentation: presentation(node, declared_key)?,
        description: node
            .get("description")
            .and_then(ConfigNode::as_scalar)
            .map(str::to_string),
    })
}

fn item_type(
    node: &ConfigNode,
    key: &str,
) -> Result<SkillConfigScalarType, SkillConfigSchemaError> {
    let declared = match node.get("items") {
        Some(ConfigNode::Scalar(value)) => value.clone(),
        Some(mapping @ ConfigNode::Mapping(_)) => mapping
            .get("type")
            .and_then(ConfigNode::as_scalar)
            .ok_or_else(|| SkillConfigSchemaError::MissingType(format!("{key}.items")))?
            .to_string(),
        _ => {
            return Err(SkillConfigSchemaError::MissingType(format!("{key}.items")));
        }
    };
    SkillConfigScalarType::parse(&declared).ok_or(SkillConfigSchemaError::UnsupportedType {
        field: format!("{key}.items"),
        declared,
    })
}

fn flag(node: &ConfigNode, keyword: &str, key: &str) -> Result<bool, SkillConfigSchemaError> {
    match node.get(keyword).and_then(ConfigNode::as_scalar) {
        None => Ok(false),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(SkillConfigSchemaError::InvalidAnnotation {
            field: key.to_string(),
            reason: format!("{keyword} must be true or false, found {other}"),
        }),
    }
}

fn presentation(
    node: &ConfigNode,
    key: &str,
) -> Result<SkillConfigPresentation, SkillConfigSchemaError> {
    let text = |keyword: &str, limit: usize| -> Result<Option<String>, SkillConfigSchemaError> {
        match node.get(keyword).and_then(ConfigNode::as_scalar) {
            None => Ok(None),
            Some(value) if value.chars().count() > limit => {
                Err(SkillConfigSchemaError::InvalidAnnotation {
                    field: key.to_string(),
                    reason: format!("{keyword} exceeds {limit} characters"),
                })
            }
            Some(value) => Ok(Some(value.to_string())),
        }
    };
    let order =
        match node.get("x-vanehub-order").and_then(ConfigNode::as_scalar) {
            None => None,
            Some(value) => Some(value.parse::<u32>().map_err(|_| {
                SkillConfigSchemaError::InvalidAnnotation {
                    field: key.to_string(),
                    reason: format!(
                        "x-vanehub-order must be a non-negative integer, found {value}"
                    ),
                }
            })?),
        };
    Ok(SkillConfigPresentation {
        label: text("x-vanehub-label", MAX_CONFIG_LABEL_CHARACTERS)?,
        label_key: text("x-vanehub-label-key", MAX_CONFIG_LABEL_CHARACTERS)?,
        help: text("x-vanehub-help", MAX_CONFIG_HELP_CHARACTERS)?,
        order,
        advanced: flag(node, "x-vanehub-advanced", key)?,
        multiline: flag(node, MULTILINE_KEYWORD, key)?,
    })
}

fn constraints(
    node: &ConfigNode,
    key: &str,
) -> Result<SkillConfigConstraints, SkillConfigSchemaError> {
    let number = |keyword: &str| -> Result<Option<f64>, SkillConfigSchemaError> {
        match node.get(keyword).and_then(ConfigNode::as_scalar) {
            None => Ok(None),
            Some(value) => value.parse::<f64>().map(Some).map_err(|_| {
                SkillConfigSchemaError::InvalidConstraint {
                    field: key.to_string(),
                    reason: format!("{keyword} must be numeric, found {value}"),
                }
            }),
        }
    };
    let count = |keyword: &str| -> Result<Option<usize>, SkillConfigSchemaError> {
        match node.get(keyword).and_then(ConfigNode::as_scalar) {
            None => Ok(None),
            Some(value) => value.parse::<usize>().map(Some).map_err(|_| {
                SkillConfigSchemaError::InvalidConstraint {
                    field: key.to_string(),
                    reason: format!("{keyword} must be a non-negative integer, found {value}"),
                }
            }),
        }
    };
    let constraints = SkillConfigConstraints {
        minimum: number("minimum")?,
        maximum: number("maximum")?,
        min_length: count("minLength")?,
        max_length: count("maxLength")?,
        min_items: count("minItems")?,
        max_items: count("maxItems")?,
    };
    let inverted = |low: Option<f64>, high: Option<f64>| match (low, high) {
        (Some(low), Some(high)) => low > high,
        _ => false,
    };
    let inverted_count = |low: Option<usize>, high: Option<usize>| match (low, high) {
        (Some(low), Some(high)) => low > high,
        _ => false,
    };
    if inverted(constraints.minimum, constraints.maximum)
        || inverted_count(constraints.min_length, constraints.max_length)
        || inverted_count(constraints.min_items, constraints.max_items)
    {
        return Err(SkillConfigSchemaError::InvalidConstraint {
            field: key.to_string(),
            reason: "lower bound exceeds upper bound".to_string(),
        });
    }
    Ok(constraints)
}

fn choices(
    node: &ConfigNode,
    field_type: SkillConfigFieldType,
    key: &str,
) -> Result<Vec<SkillConfigValue>, SkillConfigSchemaError> {
    let Some(raw) = node.get("enum") else {
        return Ok(Vec::new());
    };
    let items = raw
        .as_sequence()
        .ok_or_else(|| SkillConfigSchemaError::InvalidEnum {
            field: key.to_string(),
            reason: "enum must be a list".to_string(),
        })?;
    if items.is_empty() {
        return Err(SkillConfigSchemaError::InvalidEnum {
            field: key.to_string(),
            reason: "enum must declare at least one choice".to_string(),
        });
    }
    if field_type.item_type() == SkillConfigScalarType::Boolean {
        return Err(SkillConfigSchemaError::InvalidEnum {
            field: key.to_string(),
            reason: "boolean properties cannot declare an enum".to_string(),
        });
    }
    let mut choices = Vec::new();
    for item in items {
        let value = scalar_value(item, field_type.item_type(), key, "enum")?;
        if choices.contains(&value) {
            return Err(SkillConfigSchemaError::InvalidEnum {
                field: key.to_string(),
                reason: format!("duplicate choice: {item}"),
            });
        }
        choices.push(value);
    }
    Ok(choices)
}

fn coerce(
    raw: &ConfigNode,
    field_type: SkillConfigFieldType,
    key: &str,
) -> Result<SkillConfigValue, SkillConfigSchemaError> {
    match field_type {
        SkillConfigFieldType::Scalar(scalar) => match raw {
            ConfigNode::Scalar(value) => scalar_value(value, scalar, key, "default"),
            _ => Err(SkillConfigSchemaError::InvalidDefault {
                field: key.to_string(),
                reason: "expected a scalar value".to_string(),
            }),
        },
        SkillConfigFieldType::List(scalar) => {
            let items =
                raw.as_sequence()
                    .ok_or_else(|| SkillConfigSchemaError::InvalidDefault {
                        field: key.to_string(),
                        reason: "expected a list value".to_string(),
                    })?;
            let values = items
                .iter()
                .map(|item| scalar_value(item, scalar, key, "default"))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SkillConfigValue::List(values))
        }
    }
}

fn scalar_value(
    raw: &str,
    scalar: SkillConfigScalarType,
    key: &str,
    origin: &str,
) -> Result<SkillConfigValue, SkillConfigSchemaError> {
    let invalid = |reason: String| match origin {
        "enum" => SkillConfigSchemaError::InvalidEnum {
            field: key.to_string(),
            reason,
        },
        _ => SkillConfigSchemaError::InvalidDefault {
            field: key.to_string(),
            reason,
        },
    };
    match scalar {
        SkillConfigScalarType::Text => Ok(SkillConfigValue::Text(raw.to_string())),
        SkillConfigScalarType::Integer => raw
            .parse::<i64>()
            .map(SkillConfigValue::Integer)
            .map_err(|_| invalid(format!("expected an integer, found {raw}"))),
        SkillConfigScalarType::Number => raw
            .parse::<f64>()
            .map(SkillConfigValue::Number)
            .map_err(|_| invalid(format!("expected a number, found {raw}")))
            .and_then(|value| match value {
                SkillConfigValue::Number(number) if !number.is_finite() => {
                    Err(invalid(format!("expected a finite number, found {raw}")))
                }
                other => Ok(other),
            }),
        SkillConfigScalarType::Boolean => match raw {
            "true" => Ok(SkillConfigValue::Boolean(true)),
            "false" => Ok(SkillConfigValue::Boolean(false)),
            other => Err(invalid(format!("expected true or false, found {other}"))),
        },
    }
}

pub(crate) fn validate_value(
    value: &SkillConfigValue,
    field_type: SkillConfigFieldType,
    constraints: &SkillConfigConstraints,
    choices: &[SkillConfigValue],
    key: &str,
) -> Result<(), SkillConfigSchemaError> {
    let invalid = |reason: String| SkillConfigSchemaError::InvalidDefault {
        field: key.to_string(),
        reason,
    };
    match (value, field_type) {
        (SkillConfigValue::List(items), SkillConfigFieldType::List(_)) => {
            if let Some(minimum) = constraints.min_items {
                if items.len() < minimum {
                    return Err(invalid(format!("expected at least {minimum} items")));
                }
            }
            if let Some(maximum) = constraints.max_items {
                if items.len() > maximum {
                    return Err(invalid(format!("expected at most {maximum} items")));
                }
            }
            for item in items {
                validate_scalar(item, constraints, choices, key)?;
            }
            Ok(())
        }
        (_, SkillConfigFieldType::List(_)) => Err(invalid("expected a list value".to_string())),
        (SkillConfigValue::List(_), _) => Err(invalid("expected a scalar value".to_string())),
        _ => validate_scalar(value, constraints, choices, key),
    }
}

fn validate_scalar(
    value: &SkillConfigValue,
    constraints: &SkillConfigConstraints,
    choices: &[SkillConfigValue],
    key: &str,
) -> Result<(), SkillConfigSchemaError> {
    let invalid = |reason: String| SkillConfigSchemaError::InvalidDefault {
        field: key.to_string(),
        reason,
    };
    if !choices.is_empty() && !choices.contains(value) {
        return Err(invalid(
            "value is not one of the declared choices".to_string(),
        ));
    }
    match value {
        SkillConfigValue::Text(text) => {
            let length = text.chars().count();
            if let Some(minimum) = constraints.min_length {
                if length < minimum {
                    return Err(invalid(format!("expected at least {minimum} characters")));
                }
            }
            if let Some(maximum) = constraints.max_length {
                if length > maximum {
                    return Err(invalid(format!("expected at most {maximum} characters")));
                }
            }
            Ok(())
        }
        SkillConfigValue::Integer(number) => bounds(*number as f64, constraints, key),
        SkillConfigValue::Number(number) => bounds(*number, constraints, key),
        SkillConfigValue::Boolean(_) => Ok(()),
        SkillConfigValue::List(_) => Err(invalid("nested lists are not supported".to_string())),
    }
}

fn bounds(
    number: f64,
    constraints: &SkillConfigConstraints,
    key: &str,
) -> Result<(), SkillConfigSchemaError> {
    let invalid = |reason: String| SkillConfigSchemaError::InvalidDefault {
        field: key.to_string(),
        reason,
    };
    if let Some(minimum) = constraints.minimum {
        if number < minimum {
            return Err(invalid(format!("expected at least {minimum}")));
        }
    }
    if let Some(maximum) = constraints.maximum {
        if number > maximum {
            return Err(invalid(format!("expected at most {maximum}")));
        }
    }
    Ok(())
}
