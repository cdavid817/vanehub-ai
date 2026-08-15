use super::{validate_name, AgentRuntimeDomainError, MemoryType};
use serde_json::Value;

/// Upper bound on actions honored from one extraction response. Extraction is best-effort
/// background work; a response proposing hundreds of writes is a malfunction, not a windfall.
pub(crate) const MAX_MEMORY_ACTIONS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryActionKind {
    Create,
    Update,
    Delete,
}

impl MemoryActionKind {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "create" => Some(Self::Create),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

/// One validated instruction from an extraction response. `body` and `description` are absent only
/// for a delete, which needs nothing but the name of the memory it retracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryAction {
    pub(crate) kind: MemoryActionKind,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) body: Option<String>,
}

/// Why one action was dropped. Content-free by construction — a reason code and a position, never
/// the rejected text, because these reach the unified log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryActionRejection {
    pub(crate) index: usize,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedMemoryActions {
    pub(crate) actions: Vec<MemoryAction>,
    pub(crate) rejections: Vec<MemoryActionRejection>,
}

/// Instruction appended to the extraction request. Kept beside the parser so the shape asked for
/// and the shape accepted cannot drift apart.
pub(crate) const MEMORY_ACTIONS_INSTRUCTION: &str = r#"Return a JSON array of memory actions and nothing else.

Each element is an object:
- "action": "create", "update", or "delete"
- "name": short kebab-case identifier. To correct or replace an existing memory, reuse its name. To retract one, delete it by name.
- "description": one line describing what the memory holds, specific enough to judge later whether it is worth reading in full. Required for create and update.
- "type": one of "user", "feedback", "project", "reference". Optional.
- "body": the memory content. Required for create and update.

Save only what is not derivable from the current project state: code patterns, architecture, git history, and file structure are all derivable and must not be saved. Prefer updating an existing memory over creating a near-duplicate. Return an empty array when nothing is worth remembering."#;

/// Parses an extraction response into actions.
///
/// Rejection is per action rather than per response: one malformed element must not discard the
/// good ones alongside it. A response that is not a JSON array at all is a different failure and
/// returns `Err`, so the caller can log it as a malfunctioning extraction rather than as an
/// extraction that found nothing.
pub(crate) fn parse_memory_actions(
    raw: &str,
) -> Result<ParsedMemoryActions, AgentRuntimeDomainError> {
    let array = extract_json_array(raw).ok_or(AgentRuntimeDomainError::InvalidMemoryValue(
        "action response",
    ))?;
    let elements: Vec<Value> = serde_json::from_str(&array)
        .map_err(|_| AgentRuntimeDomainError::InvalidMemoryValue("action response"))?;

    let mut parsed = ParsedMemoryActions::default();
    for (index, element) in elements.into_iter().enumerate() {
        if parsed.actions.len() >= MAX_MEMORY_ACTIONS {
            parsed.rejections.push(MemoryActionRejection {
                index,
                reason: "action-limit",
            });
            continue;
        }
        match parse_action(&element) {
            Ok(action) => parsed.actions.push(action),
            Err(reason) => parsed
                .rejections
                .push(MemoryActionRejection { index, reason }),
        }
    }
    Ok(parsed)
}

fn parse_action(element: &Value) -> Result<MemoryAction, &'static str> {
    let kind = element
        .get("action")
        .and_then(Value::as_str)
        .and_then(MemoryActionKind::parse)
        .ok_or("unknown-action")?;
    let name = element
        .get("name")
        .and_then(Value::as_str)
        .ok_or("missing-name")?;
    // The same validation the store applies, run here so an unusable name is a counted rejection
    // rather than a write failure discovered later.
    let name = validate_name(name).map_err(|_| "invalid-name")?;
    let memory_type = element
        .get("type")
        .and_then(Value::as_str)
        .and_then(MemoryType::parse);

    if kind == MemoryActionKind::Delete {
        return Ok(MemoryAction {
            kind,
            name,
            description: None,
            memory_type,
            body: None,
        });
    }

    let body = element
        .get("body")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|body| !body.is_empty())
        .ok_or("missing-body")?;
    let description = element
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .ok_or("missing-description")?;

    Ok(MemoryAction {
        kind,
        name,
        description: Some(description.to_string()),
        memory_type,
        body: Some(body.to_string()),
    })
}

/// Finds the outermost JSON array in a response that may carry prose or a code fence around it.
/// Models wrap structured output often enough that failing on the wrapper would misreport a usable
/// response as a malfunction.
fn extract_json_array(raw: &str) -> Option<String> {
    let start = raw.find('[')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, character) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..start + offset + character.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}
