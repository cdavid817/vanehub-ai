use serde_json::Value;
use std::collections::HashSet;

/// Upper bound on memories one selection may contribute.
///
/// A bound is what makes the selection a budget rather than a second unbounded injection. Five is
/// enough to cover a turn's real context without the bodies crowding out the conversation.
pub(crate) const MAX_SELECTED_MEMORIES: usize = 5;

/// Instruction for the relevance selector. Lives beside the parser so the shape asked for and the
/// shape accepted cannot drift apart.
///
/// The "return nothing" clause is load-bearing. A selector that always returns something turns the
/// relevance budget into a random sample, which is worse than injecting no bodies at all: the
/// model treats a confidently-surfaced irrelevant memory as if it were relevant.
pub(crate) const MEMORY_SELECTION_INSTRUCTION: &str = r#"You are selecting which stored memories are worth reading in full for the request below.

You will be given the request and a list of available memories, each with its type, name, age, and a one-line description.

Return a JSON array of memory names and nothing else. Select only memories that are clearly useful for this request, at most 5.
- If you are unsure whether a memory is useful, leave it out.
- If none are clearly useful, return an empty array. That is the expected answer most of the time and is never wrong.
- Judge from the description alone. You are not being shown the memories' contents."#;

/// Parses the selector's response into names that exist in the manifest it was shown.
///
/// A name absent from `available` is discarded rather than treated as an error: a hallucinated
/// name costs nothing once dropped, while failing the whole selection over one would throw away
/// the valid choices beside it. Order follows the selector's own, so its ranking survives.
pub(crate) fn parse_memory_selection(raw: &str, available: &HashSet<String>) -> Vec<String> {
    let Some(array) = extract_json_array(raw) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<Value>>(&array) else {
        return Vec::new();
    };
    let mut selected: Vec<String> = Vec::new();
    for value in values {
        if selected.len() >= MAX_SELECTED_MEMORIES {
            break;
        }
        let Some(name) = value.as_str().map(str::trim) else {
            continue;
        };
        if !available.contains(name) || selected.iter().any(|existing| existing == name) {
            continue;
        }
        selected.push(name.to_string());
    }
    selected
}

/// Finds the outermost JSON array in a response that may carry prose or a code fence around it.
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
