#![cfg_attr(not(test), allow(dead_code))]

//! Skills' view of the bounded YAML subset used by `config_schema`.
//!
//! The scanner itself moved to `vanehub-bounded-yaml` so extension manifests parse untrusted text
//! through the same reviewed primitive (`add-unified-extension-platform`, Task 1.B). What stays
//! here is what belongs to Skills: the limit profile, the domain node type, and the wording an
//! operator reads. Those messages name `config_schema`, which the shared crate has no business
//! knowing about.

use std::fmt;
use vanehub_bounded_yaml::{BoundedYamlError, BoundedYamlLimits, BoundedYamlValue};

/// Bounds are enforced by the scanner rather than by a downstream validator so that a hostile
/// `config_schema` cannot cost more than these limits to reject. A general YAML parser would
/// accept anchors, aliases, and merge keys, whose expansion is unbounded before any validator
/// runs; this subset has no construct that expands.
///
/// This profile is Skills'. Another consumer needing more room supplies its own rather than
/// widening these — a manifest that wants deeper nesting must not silently raise what a Skill
/// config may contain.
pub(crate) const MAX_CONFIG_SCHEMA_BYTES: usize = 16 * 1_024;
pub(crate) const MAX_CONFIG_NODE_DEPTH: usize = 6;
pub(crate) const MAX_CONFIG_NODES: usize = 512;
pub(crate) const MAX_CONFIG_KEY_CHARACTERS: usize = 64;
pub(crate) const MAX_CONFIG_SCALAR_CHARACTERS: usize = 512;
pub(crate) const MAX_CONFIG_SEQUENCE_ITEMS: usize = 32;
const INDENT_UNIT: usize = vanehub_bounded_yaml::INDENT_UNIT;

const SKILL_CONFIG_LIMITS: BoundedYamlLimits = BoundedYamlLimits {
    max_bytes: MAX_CONFIG_SCHEMA_BYTES,
    max_depth: MAX_CONFIG_NODE_DEPTH,
    max_nodes: MAX_CONFIG_NODES,
    max_key_bytes: MAX_CONFIG_KEY_CHARACTERS,
    max_scalar_characters: MAX_CONFIG_SCALAR_CHARACTERS,
    max_sequence_items: MAX_CONFIG_SEQUENCE_ITEMS,
    // Skill config keys are plain names. Unchanged from before the extraction, and the
    // characterization suite is what proves it stayed that way.
    allow_dotted_keys: false,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigNode {
    Scalar(String),
    Mapping(Vec<(String, ConfigNode)>),
    Sequence(Vec<String>),
}

impl ConfigNode {
    pub(crate) fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub(crate) fn as_mapping(&self) -> Option<&[(String, ConfigNode)]> {
        match self {
            Self::Mapping(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub(crate) fn as_sequence(&self) -> Option<&[String]> {
        match self {
            Self::Sequence(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&ConfigNode> {
        self.as_mapping()?
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value)
    }
}

impl From<BoundedYamlValue> for ConfigNode {
    fn from(value: BoundedYamlValue) -> Self {
        match value {
            BoundedYamlValue::Scalar(scalar) => Self::Scalar(scalar),
            BoundedYamlValue::Sequence(items) => Self::Sequence(items),
            BoundedYamlValue::Mapping(entries) => Self::Mapping(
                entries
                    .into_iter()
                    .map(|(key, child)| (key, Self::from(child)))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigDocumentError {
    TooLarge,
    TooManyNodes,
    DepthExceeded,
    TabIndentation(usize),
    MisalignedIndentation(usize),
    UnsupportedConstruct { line: usize, construct: String },
    DuplicateKey { line: usize, key: String },
    InvalidKey { line: usize, key: String },
    ScalarTooLong(usize),
    SequenceTooLong(usize),
    ExpectedMapping(usize),
    UnexpectedSequenceItem(usize),
}

impl From<BoundedYamlError> for ConfigDocumentError {
    fn from(error: BoundedYamlError) -> Self {
        match error {
            BoundedYamlError::TooLarge => Self::TooLarge,
            BoundedYamlError::TooManyNodes => Self::TooManyNodes,
            BoundedYamlError::DepthExceeded => Self::DepthExceeded,
            BoundedYamlError::TabIndentation(line) => Self::TabIndentation(line),
            BoundedYamlError::MisalignedIndentation(line) => Self::MisalignedIndentation(line),
            BoundedYamlError::UnsupportedConstruct { line, construct } => {
                Self::UnsupportedConstruct { line, construct }
            }
            BoundedYamlError::DuplicateKey { line, key } => Self::DuplicateKey { line, key },
            BoundedYamlError::InvalidKey { line, key } => Self::InvalidKey { line, key },
            BoundedYamlError::ScalarTooLong(line) => Self::ScalarTooLong(line),
            BoundedYamlError::SequenceTooLong(line) => Self::SequenceTooLong(line),
            BoundedYamlError::ExpectedMapping(line) => Self::ExpectedMapping(line),
            BoundedYamlError::UnexpectedSequenceItem(line) => Self::UnexpectedSequenceItem(line),
        }
    }
}

impl fmt::Display for ConfigDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(
                formatter,
                "config_schema exceeds {MAX_CONFIG_SCHEMA_BYTES} bytes"
            ),
            Self::TooManyNodes => write!(
                formatter,
                "config_schema exceeds {MAX_CONFIG_NODES} declarations"
            ),
            Self::DepthExceeded => write!(
                formatter,
                "config_schema nests deeper than {MAX_CONFIG_NODE_DEPTH} levels"
            ),
            Self::TabIndentation(line) => {
                write!(formatter, "config_schema line {line} indents with a tab")
            }
            Self::MisalignedIndentation(line) => write!(
                formatter,
                "config_schema line {line} is not indented in {INDENT_UNIT}-space steps"
            ),
            Self::UnsupportedConstruct { line, construct } => write!(
                formatter,
                "config_schema line {line} uses unsupported YAML construct: {construct}"
            ),
            Self::DuplicateKey { line, key } => {
                write!(formatter, "config_schema line {line} repeats key: {key}")
            }
            Self::InvalidKey { line, key } => {
                write!(
                    formatter,
                    "config_schema line {line} has invalid key: {key}"
                )
            }
            Self::ScalarTooLong(line) => write!(
                formatter,
                "config_schema line {line} exceeds {MAX_CONFIG_SCALAR_CHARACTERS} characters"
            ),
            Self::SequenceTooLong(line) => write!(
                formatter,
                "config_schema line {line} exceeds {MAX_CONFIG_SEQUENCE_ITEMS} items"
            ),
            Self::ExpectedMapping(line) => {
                write!(formatter, "config_schema line {line} expects a mapping")
            }
            Self::UnexpectedSequenceItem(line) => write!(
                formatter,
                "config_schema line {line} starts a list where a mapping is required"
            ),
        }
    }
}

impl std::error::Error for ConfigDocumentError {}

/// Parses the bounded mapping subset used by `config_schema`. `block` is the raw frontmatter
/// block with its own base indentation still attached; the scanner dedents to the first
/// non-blank line.
pub(crate) fn parse_block(block: &str) -> Result<ConfigNode, ConfigDocumentError> {
    vanehub_bounded_yaml::parse_block(block, SKILL_CONFIG_LIMITS)
        .map(ConfigNode::from)
        .map_err(ConfigDocumentError::from)
}
