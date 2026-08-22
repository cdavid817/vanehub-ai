//! A restricted YAML subset scanner with caller-supplied resource limits.
//!
//! Extracted from `skills`' `config_schema` scanner so that Skill configuration and extension
//! manifests parse untrusted text through one reviewed primitive rather than two. The original
//! rationale still governs the design: a general YAML parser accepts anchors, aliases, and merge
//! keys whose expansion is unbounded *before* any validator runs, so the bounds live in the
//! scanner and the subset contains no construct that expands.
//!
//! This crate is deliberately ignorant of what it is parsing. It owns the lexer, the grammar, the
//! resource limits, duplicate-key detection, and a generic AST. It owns no field names, no
//! defaults, no wording for user-facing messages, and no I/O. Each consumer decodes
//! [`BoundedYamlValue`] into its own domain type and renders [`BoundedYamlError`] in its own
//! vocabulary — which is why the error carries structured data instead of a formatted sentence.
//!
//! Limits are supplied per call. A consumer that needs more nodes than another cannot widen the
//! other's bound as a side effect.

use std::fmt;

/// Indentation is a fixed two-space step. Part of the grammar rather than a limit: making it
/// configurable would let one consumer accept a document another rejects for reasons that have
/// nothing to do with resources.
pub const INDENT_UNIT: usize = 2;

/// Resource ceilings for one parse. Every consumer pins its own profile and tests it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedYamlLimits {
    /// Maximum raw byte length of the block, checked before any other work.
    pub max_bytes: usize,
    /// Maximum nesting depth of mappings.
    pub max_depth: usize,
    /// Maximum total number of mapping entries and sequence items combined.
    pub max_nodes: usize,
    /// Maximum key length **in bytes**. Distinct from `max_scalar_characters` on purpose; see
    /// `BoundedYamlError::InvalidKey`.
    pub max_key_bytes: usize,
    /// Maximum scalar length in Unicode characters.
    pub max_scalar_characters: usize,
    /// Maximum items in one sequence, block or flow.
    pub max_sequence_items: usize,
    /// Whether a key may contain `.`.
    ///
    /// Off by default in spirit: a consumer whose keys are plain names does not want a dotted key,
    /// and diagnostics that address fields as `a.b.c` become ambiguous once a key can contain the
    /// separator. On for a consumer whose keys are themselves dotted identifiers.
    pub allow_dotted_keys: bool,
}

/// The parsed document.
///
/// Sequences hold scalars only. The subset has no sequence-of-mappings and no nested sequence,
/// because both are shapes whose size is not bounded by the node budget alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedYamlValue {
    Scalar(String),
    Mapping(Vec<(String, BoundedYamlValue)>),
    Sequence(Vec<String>),
}

impl BoundedYamlValue {
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_mapping(&self) -> Option<&[(String, BoundedYamlValue)]> {
        match self {
            Self::Mapping(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&[String]> {
        match self {
            Self::Sequence(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    /// First entry with this key. Keys are unique within a mapping, so "first" is "the one".
    pub fn get(&self, key: &str) -> Option<&BoundedYamlValue> {
        self.as_mapping()?
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value)
    }
}

/// A rejection, as structured data.
///
/// No consumer-facing wording lives here. A caller renders these in its own vocabulary, so the
/// same scanner can report "config_schema line 3 ..." to one caller and something else to another
/// without either wording leaking into the shared crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedYamlError {
    TooLarge,
    TooManyNodes,
    DepthExceeded,
    TabIndentation(usize),
    MisalignedIndentation(usize),
    UnsupportedConstruct {
        line: usize,
        construct: String,
    },
    DuplicateKey {
        line: usize,
        key: String,
    },
    /// The key as far as it was read, truncated to the caller's key limit so a hostile document
    /// cannot make the diagnostic itself unbounded.
    InvalidKey {
        line: usize,
        key: String,
    },
    ScalarTooLong(usize),
    /// Reported against the line that *opens* the sequence, not the offending item.
    SequenceTooLong(usize),
    ExpectedMapping(usize),
    UnexpectedSequenceItem(usize),
}

impl BoundedYamlError {
    /// Stable machine-readable code. Distinct from the caller's message, which may be localized
    /// or domain-specific.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "too_large",
            Self::TooManyNodes => "too_many_nodes",
            Self::DepthExceeded => "depth_exceeded",
            Self::TabIndentation(_) => "tab_indentation",
            Self::MisalignedIndentation(_) => "misaligned_indentation",
            Self::UnsupportedConstruct { .. } => "unsupported_construct",
            Self::DuplicateKey { .. } => "duplicate_key",
            Self::InvalidKey { .. } => "invalid_key",
            Self::ScalarTooLong(_) => "scalar_too_long",
            Self::SequenceTooLong(_) => "sequence_too_long",
            Self::ExpectedMapping(_) => "expected_mapping",
            Self::UnexpectedSequenceItem(_) => "unexpected_sequence_item",
        }
    }

    /// Source line the rejection points at, when it points at one. Size and budget failures are
    /// whole-document facts and have none.
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::TooLarge | Self::TooManyNodes | Self::DepthExceeded => None,
            Self::TabIndentation(line)
            | Self::MisalignedIndentation(line)
            | Self::ScalarTooLong(line)
            | Self::SequenceTooLong(line)
            | Self::ExpectedMapping(line)
            | Self::UnexpectedSequenceItem(line) => Some(*line),
            Self::UnsupportedConstruct { line, .. }
            | Self::DuplicateKey { line, .. }
            | Self::InvalidKey { line, .. } => Some(*line),
        }
    }
}

impl fmt::Display for BoundedYamlError {
    /// A last-resort rendering. Consumers are expected to supply their own wording; this exists so
    /// the type satisfies `Error` and so a stray `{}` in a log is not empty.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line() {
            Some(line) => write!(formatter, "{} at line {line}", self.code()),
            None => formatter.write_str(self.code()),
        }
    }
}

impl std::error::Error for BoundedYamlError {}

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
    limits: BoundedYamlLimits,
}

/// Parses the bounded mapping subset.
///
/// `block` may carry its own base indentation; the scanner dedents to the first non-blank line, so
/// a block lifted out of surrounding frontmatter parses without the caller reindenting it.
pub fn parse_block(
    block: &str,
    limits: BoundedYamlLimits,
) -> Result<BoundedYamlValue, BoundedYamlError> {
    if block.len() > limits.max_bytes {
        return Err(BoundedYamlError::TooLarge);
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
            return Err(BoundedYamlError::TabIndentation(number));
        }
        lines.push(SourceLine {
            number,
            indent,
            content: without_comment.trim_start(),
        });
    }
    if lines.is_empty() {
        return Ok(BoundedYamlValue::Mapping(Vec::new()));
    }
    let base_indent = lines[0].indent;
    for line in &lines {
        if line.indent < base_indent {
            return Err(BoundedYamlError::MisalignedIndentation(line.number));
        }
    }
    for line in &mut lines {
        line.indent -= base_indent;
        if line.indent % INDENT_UNIT != 0 {
            return Err(BoundedYamlError::MisalignedIndentation(line.number));
        }
    }
    let mut scanner = Scanner {
        lines,
        cursor: 0,
        nodes: 0,
        limits,
    };
    let node = scanner.parse_mapping(0, 0)?;
    if scanner.cursor < scanner.lines.len() {
        return Err(BoundedYamlError::MisalignedIndentation(
            scanner.lines[scanner.cursor].number,
        ));
    }
    Ok(node)
}

impl Scanner<'_> {
    fn parse_mapping(
        &mut self,
        indent: usize,
        depth: usize,
    ) -> Result<BoundedYamlValue, BoundedYamlError> {
        if depth > self.limits.max_depth {
            return Err(BoundedYamlError::DepthExceeded);
        }
        let mut entries: Vec<(String, BoundedYamlValue)> = Vec::new();
        while let Some(line) = self.lines.get(self.cursor).copied() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(BoundedYamlError::MisalignedIndentation(line.number));
            }
            reject_unsupported(&line)?;
            if line.content.starts_with('-') {
                return Err(BoundedYamlError::UnexpectedSequenceItem(line.number));
            }
            let Some((raw_key, remainder)) = split_key(line.content) else {
                return Err(BoundedYamlError::ExpectedMapping(line.number));
            };
            let key = normalize_key(raw_key, line.number, self.limits)?;
            if entries.iter().any(|(existing, _)| existing == &key) {
                return Err(BoundedYamlError::DuplicateKey {
                    line: line.number,
                    key,
                });
            }
            self.charge_node()?;
            self.cursor += 1;
            let value = if remainder.is_empty() {
                self.parse_child(indent, depth, line.number)?
            } else if remainder.starts_with('{') {
                // Only in value position: a `{` inside a label or help string is ordinary text,
                // and callers embed placeholders such as `{skill_base_dir}` in scalars.
                return Err(BoundedYamlError::UnsupportedConstruct {
                    line: line.number,
                    construct: "flow mapping".to_string(),
                });
            } else if let Some(items) = parse_flow_sequence(remainder, line.number, self.limits)? {
                BoundedYamlValue::Sequence(items)
            } else {
                BoundedYamlValue::Scalar(scalar(
                    remainder,
                    line.number,
                    self.limits.max_scalar_characters,
                )?)
            };
            entries.push((key, value));
        }
        Ok(BoundedYamlValue::Mapping(entries))
    }

    fn parse_child(
        &mut self,
        indent: usize,
        depth: usize,
        parent_line: usize,
    ) -> Result<BoundedYamlValue, BoundedYamlError> {
        let Some(next) = self.lines.get(self.cursor).copied() else {
            return Ok(BoundedYamlValue::Mapping(Vec::new()));
        };
        if next.indent <= indent {
            // `key:` with nothing under it is an empty mapping, matching YAML's null-ish shape
            // without introducing a null scalar every consumer would have to special-case.
            return Ok(BoundedYamlValue::Mapping(Vec::new()));
        }
        if next.indent != indent + INDENT_UNIT {
            return Err(BoundedYamlError::MisalignedIndentation(next.number));
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
    ) -> Result<BoundedYamlValue, BoundedYamlError> {
        let mut items = Vec::new();
        while let Some(line) = self.lines.get(self.cursor).copied() {
            if line.indent < indent || !line.content.starts_with('-') {
                break;
            }
            if line.indent > indent {
                return Err(BoundedYamlError::MisalignedIndentation(line.number));
            }
            reject_unsupported(&line)?;
            let raw = line.content[1..].trim();
            if raw.is_empty() || raw.ends_with(':') {
                return Err(BoundedYamlError::UnsupportedConstruct {
                    line: line.number,
                    construct: "nested list entry".to_string(),
                });
            }
            self.charge_node()?;
            items.push(scalar(raw, line.number, self.limits.max_scalar_characters)?);
            if items.len() > self.limits.max_sequence_items {
                return Err(BoundedYamlError::SequenceTooLong(parent_line));
            }
            self.cursor += 1;
        }
        Ok(BoundedYamlValue::Sequence(items))
    }

    fn charge_node(&mut self) -> Result<(), BoundedYamlError> {
        self.nodes += 1;
        if self.nodes > self.limits.max_nodes {
            return Err(BoundedYamlError::TooManyNodes);
        }
        Ok(())
    }
}

fn reject_unsupported(line: &SourceLine<'_>) -> Result<(), BoundedYamlError> {
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
        Some(construct) => Err(BoundedYamlError::UnsupportedConstruct {
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

fn normalize_key(
    raw: &str,
    line: usize,
    limits: BoundedYamlLimits,
) -> Result<String, BoundedYamlError> {
    let key = raw.trim().trim_matches('"').trim_matches('\'').trim();
    let permitted = |character: char| {
        character.is_ascii_alphanumeric()
            || matches!(character, '_' | '-')
            || (limits.allow_dotted_keys && character == '.')
    };
    let valid = !key.is_empty() && key.len() <= limits.max_key_bytes && key.chars().all(permitted);
    if valid {
        Ok(key.to_string())
    } else {
        Err(BoundedYamlError::InvalidKey {
            line,
            key: key.chars().take(limits.max_key_bytes).collect(),
        })
    }
}

fn parse_flow_sequence(
    raw: &str,
    line: usize,
    limits: BoundedYamlLimits,
) -> Result<Option<Vec<String>>, BoundedYamlError> {
    let Some(inner) = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return Ok(None);
    };
    if inner.contains('[') || inner.contains('{') {
        return Err(BoundedYamlError::UnsupportedConstruct {
            line,
            construct: "nested flow collection".to_string(),
        });
    }
    if inner.trim().is_empty() {
        return Ok(Some(Vec::new()));
    }
    let mut items = Vec::new();
    for item in inner.split(',') {
        items.push(scalar(item.trim(), line, limits.max_scalar_characters)?);
        if items.len() > limits.max_sequence_items {
            return Err(BoundedYamlError::SequenceTooLong(line));
        }
    }
    Ok(Some(items))
}

fn scalar(raw: &str, line: usize, max_characters: usize) -> Result<String, BoundedYamlError> {
    let value = if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    if value.chars().count() > max_characters {
        return Err(BoundedYamlError::ScalarTooLong(line));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests;
