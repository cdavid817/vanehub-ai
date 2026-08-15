#![cfg_attr(not(test), allow(dead_code))]

use std::fmt;

/// Bounds are enforced by the scanner rather than by a downstream validator so that a hostile
/// `config_schema` cannot cost more than these limits to reject. A general YAML parser would
/// accept anchors, aliases, and merge keys, whose expansion is unbounded before any validator
/// runs; this subset has no construct that expands.
pub(crate) const MAX_CONFIG_SCHEMA_BYTES: usize = 16 * 1_024;
pub(crate) const MAX_CONFIG_NODE_DEPTH: usize = 6;
pub(crate) const MAX_CONFIG_NODES: usize = 512;
pub(crate) const MAX_CONFIG_KEY_CHARACTERS: usize = 64;
pub(crate) const MAX_CONFIG_SCALAR_CHARACTERS: usize = 512;
pub(crate) const MAX_CONFIG_SEQUENCE_ITEMS: usize = 32;
const INDENT_UNIT: usize = 2;

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

#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    number: usize,
    indent: usize,
    content: &'a str,
}

struct Scanner<'a> {
    lines: Vec<SourceLine<'a>>,
    cursor: usize,
    nodes: usize,
}

/// Parses the bounded mapping subset used by `config_schema`. `block` is the raw frontmatter
/// block with its own base indentation still attached; the scanner dedents to the first
/// non-blank line.
pub(crate) fn parse_block(block: &str) -> Result<ConfigNode, ConfigDocumentError> {
    if block.len() > MAX_CONFIG_SCHEMA_BYTES {
        return Err(ConfigDocumentError::TooLarge);
    }
    let normalized = block.replace("\r\n", "\n");
    let mut lines = Vec::new();
    for (offset, raw) in normalized.lines().enumerate() {
        let number = offset + 1;
        let trimmed_end = raw.trim_end();
        let without_comment = strip_comment(trimmed_end);
        if without_comment.trim().is_empty() {
            continue;
        }
        let indent = without_comment.len() - without_comment.trim_start().len();
        if without_comment[..indent].contains('\t') {
            return Err(ConfigDocumentError::TabIndentation(number));
        }
        lines.push(SourceLine {
            number,
            indent,
            content: without_comment.trim_start(),
        });
    }
    if lines.is_empty() {
        return Ok(ConfigNode::Mapping(Vec::new()));
    }
    let base_indent = lines[0].indent;
    for line in &lines {
        if line.indent < base_indent {
            return Err(ConfigDocumentError::MisalignedIndentation(line.number));
        }
    }
    for line in &mut lines {
        line.indent -= base_indent;
        if line.indent % INDENT_UNIT != 0 {
            return Err(ConfigDocumentError::MisalignedIndentation(line.number));
        }
    }
    let mut scanner = Scanner {
        lines,
        cursor: 0,
        nodes: 0,
    };
    let node = scanner.parse_mapping(0, 0)?;
    if scanner.cursor < scanner.lines.len() {
        return Err(ConfigDocumentError::MisalignedIndentation(
            scanner.lines[scanner.cursor].number,
        ));
    }
    Ok(node)
}

impl<'a> Scanner<'a> {
    fn parse_mapping(
        &mut self,
        indent: usize,
        depth: usize,
    ) -> Result<ConfigNode, ConfigDocumentError> {
        if depth > MAX_CONFIG_NODE_DEPTH {
            return Err(ConfigDocumentError::DepthExceeded);
        }
        let mut entries: Vec<(String, ConfigNode)> = Vec::new();
        while let Some(line) = self.lines.get(self.cursor).copied() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(ConfigDocumentError::MisalignedIndentation(line.number));
            }
            reject_unsupported(&line)?;
            if line.content.starts_with('-') {
                return Err(ConfigDocumentError::UnexpectedSequenceItem(line.number));
            }
            let Some((raw_key, remainder)) = split_key(line.content) else {
                return Err(ConfigDocumentError::ExpectedMapping(line.number));
            };
            let key = normalize_key(raw_key, line.number)?;
            if entries.iter().any(|(existing, _)| existing == &key) {
                return Err(ConfigDocumentError::DuplicateKey {
                    line: line.number,
                    key,
                });
            }
            self.charge_node()?;
            self.cursor += 1;
            let value = if remainder.is_empty() {
                self.parse_child(indent, depth, line.number)?
            } else if remainder.starts_with('{') {
                // Only in value position: `{` inside a label or help string is ordinary text,
                // and Skill bodies already use placeholders such as `{skill_base_dir}`.
                return Err(ConfigDocumentError::UnsupportedConstruct {
                    line: line.number,
                    construct: "flow mapping".to_string(),
                });
            } else if let Some(items) = parse_flow_sequence(remainder, line.number)? {
                ConfigNode::Sequence(items)
            } else {
                ConfigNode::Scalar(scalar(remainder, line.number)?)
            };
            entries.push((key, value));
        }
        Ok(ConfigNode::Mapping(entries))
    }

    fn parse_child(
        &mut self,
        indent: usize,
        depth: usize,
        parent_line: usize,
    ) -> Result<ConfigNode, ConfigDocumentError> {
        let Some(next) = self.lines.get(self.cursor).copied() else {
            return Ok(ConfigNode::Mapping(Vec::new()));
        };
        if next.indent <= indent {
            // `key:` with nothing under it is an empty mapping, matching YAML's null-ish shape
            // without introducing a null scalar the schema layer would have to special-case.
            return Ok(ConfigNode::Mapping(Vec::new()));
        }
        if next.indent != indent + INDENT_UNIT {
            return Err(ConfigDocumentError::MisalignedIndentation(next.number));
        }
        if next.content.starts_with('-') {
            return self.parse_sequence(next.indent, parent_line);
        }
        self.parse_mapping(next.indent, depth + 1)
    }

    fn parse_sequence(
        &mut self,
        indent: usize,
        parent_line: usize,
    ) -> Result<ConfigNode, ConfigDocumentError> {
        let mut items = Vec::new();
        while let Some(line) = self.lines.get(self.cursor).copied() {
            if line.indent < indent || !line.content.starts_with('-') {
                break;
            }
            if line.indent > indent {
                return Err(ConfigDocumentError::MisalignedIndentation(line.number));
            }
            reject_unsupported(&line)?;
            let raw = line.content[1..].trim();
            if raw.is_empty() || raw.ends_with(':') {
                return Err(ConfigDocumentError::UnsupportedConstruct {
                    line: line.number,
                    construct: "nested list entry".to_string(),
                });
            }
            self.charge_node()?;
            items.push(scalar(raw, line.number)?);
            if items.len() > MAX_CONFIG_SEQUENCE_ITEMS {
                return Err(ConfigDocumentError::SequenceTooLong(parent_line));
            }
            self.cursor += 1;
        }
        Ok(ConfigNode::Sequence(items))
    }

    fn charge_node(&mut self) -> Result<(), ConfigDocumentError> {
        self.nodes += 1;
        if self.nodes > MAX_CONFIG_NODES {
            return Err(ConfigDocumentError::TooManyNodes);
        }
        Ok(())
    }
}

fn reject_unsupported(line: &SourceLine<'_>) -> Result<(), ConfigDocumentError> {
    let content = line.content;
    let unsupported = if content.starts_with('&') {
        Some("anchor")
    } else if content.starts_with('*') {
        Some("alias")
    } else if content.starts_with("<<") {
        Some("merge key")
    } else if content.starts_with("---") || content.starts_with("...") {
        Some("document marker")
    } else if content.starts_with('?') {
        Some("explicit key")
    } else if content.starts_with('{') {
        Some("flow mapping")
    } else if content.contains(": &") || content.contains(": *") {
        Some("anchor or alias")
    } else if content.ends_with('|') || content.ends_with('>') {
        Some("block scalar")
    } else if content.contains("!!") {
        Some("explicit tag")
    } else {
        None
    };
    match unsupported {
        Some(construct) => Err(ConfigDocumentError::UnsupportedConstruct {
            line: line.number,
            construct: construct.to_string(),
        }),
        None => Ok(()),
    }
}

fn strip_comment(line: &str) -> &str {
    // Only a comment that starts a line or follows whitespace is a comment; `#` inside a value
    // such as a colour or a URL fragment stays part of the scalar.
    if line.trim_start().starts_with('#') {
        return "";
    }
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    for (index, byte) in bytes.iter().enumerate() {
        match quote {
            Some(open) if *byte == open => quote = None,
            Some(_) => {}
            None if *byte == b'"' || *byte == b'\'' => quote = Some(*byte),
            None if *byte == b'#' && index > 0 && bytes[index - 1].is_ascii_whitespace() => {
                return &line[..index];
            }
            None => {}
        }
    }
    line
}

fn split_key(content: &str) -> Option<(&str, &str)> {
    let mut quote: Option<u8> = None;
    for (index, byte) in content.as_bytes().iter().enumerate() {
        match quote {
            Some(open) if *byte == open => quote = None,
            Some(_) => {}
            None if *byte == b'"' || *byte == b'\'' => quote = Some(*byte),
            None if *byte == b':' => {
                let remainder = content[index + 1..].trim();
                return Some((content[..index].trim(), remainder));
            }
            None => {}
        }
    }
    None
}

fn normalize_key(raw: &str, line: usize) -> Result<String, ConfigDocumentError> {
    let key = raw.trim().trim_matches('"').trim_matches('\'').trim();
    let valid = !key.is_empty()
        && key.len() <= MAX_CONFIG_KEY_CHARACTERS
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'));
    if valid {
        Ok(key.to_string())
    } else {
        Err(ConfigDocumentError::InvalidKey {
            line,
            key: key.chars().take(MAX_CONFIG_KEY_CHARACTERS).collect(),
        })
    }
}

fn parse_flow_sequence(raw: &str, line: usize) -> Result<Option<Vec<String>>, ConfigDocumentError> {
    let Some(inner) = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return Ok(None);
    };
    if inner.contains('[') || inner.contains('{') {
        return Err(ConfigDocumentError::UnsupportedConstruct {
            line,
            construct: "nested flow collection".to_string(),
        });
    }
    if inner.trim().is_empty() {
        return Ok(Some(Vec::new()));
    }
    let mut items = Vec::new();
    for item in inner.split(',') {
        items.push(scalar(item.trim(), line)?);
        if items.len() > MAX_CONFIG_SEQUENCE_ITEMS {
            return Err(ConfigDocumentError::SequenceTooLong(line));
        }
    }
    Ok(Some(items))
}

fn scalar(raw: &str, line: usize) -> Result<String, ConfigDocumentError> {
    let value = if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    if value.chars().count() > MAX_CONFIG_SCALAR_CHARACTERS {
        return Err(ConfigDocumentError::ScalarTooLong(line));
    }
    Ok(value.to_string())
}
