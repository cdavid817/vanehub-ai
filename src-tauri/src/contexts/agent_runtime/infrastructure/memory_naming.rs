use crate::contexts::agent_runtime::domain::MemoryMetadata;
use std::collections::HashSet;

/// Words of the memory's own text folded into a generated name. Long enough to stay recognizable
/// in the index, short enough to leave room for a collision suffix inside the name length cap.
const NAME_WORD_BUDGET: usize = 6;
const MAX_GENERATED_DESCRIPTION_CHARACTERS: usize = 200;

// Deterministic naming and description derivation for memories whose writer did not supply them
// (`migrate-agent-memory-to-file-store`).
//
// Shared by row migration and by every save path that has content but no model-chosen name, so a
// write can always produce a valid addressable file instead of failing. One implementation, not
// one per caller: a second copy would drift and start generating names that collide across paths.

/// Leading sentence, truncated on a character boundary. Control characters are folded to spaces
/// rather than rejected: content is arbitrary text that never passed through a one-line
/// constraint, and a newline in a description would split one memory into two index rows.
pub(crate) fn derive_description(body: &str) -> Option<String> {
    let sentence = body
        .split(['.', '\n', '。', '!', '?', '！', '？'])
        .find(|segment| !segment.trim().is_empty())
        .unwrap_or(body);
    let flattened = sentence
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let collapsed = flattened.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    Some(
        collapsed
            .chars()
            .take(MAX_GENERATED_DESCRIPTION_CHARACTERS)
            .collect(),
    )
}

/// A name derived from the content, unique against `taken_names`.
///
/// The slug keeps ASCII alphanumerics only, so text written entirely in Chinese — which this pool
/// has plenty of — slugs to nothing and falls back to `fallback_seed`. The fallback also catches a
/// slug that is itself invalid as a file stem, such as content beginning with the word "con",
/// which is a reserved device name on Windows.
pub(crate) fn derive_name(
    content: &str,
    fallback_seed: &str,
    taken_names: &HashSet<String>,
) -> String {
    let base = slug(content)
        .filter(|candidate| MemoryMetadata::new(candidate, "placeholder", None).is_ok())
        .unwrap_or_else(|| fallback_name(fallback_seed));
    if !taken_names.contains(&base) {
        return base;
    }
    // Bounded by the number of names already taken, and every candidate is checked against the
    // same set, so this terminates on the first free suffix.
    let mut suffix = 2usize;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !taken_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn slug(content: &str) -> Option<String> {
    let words = content
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .take(NAME_WORD_BUDGET)
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    Some(words.join("-"))
}

fn fallback_name(seed: &str) -> String {
    let suffix = seed
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>();
    if suffix.is_empty() {
        "memory".to_string()
    } else {
        format!("memory-{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_description_is_the_leading_sentence_collapsed_and_truncated() {
        assert_eq!(
            derive_description("First sentence. Second sentence."),
            Some("First sentence".to_string())
        );
        assert_eq!(
            derive_description("Line one\nLine two"),
            Some("Line one".to_string())
        );
        assert_eq!(
            derive_description("  spaced   out   words  "),
            Some("spaced out words".to_string())
        );
        assert_eq!(
            derive_description(&"a".repeat(500))
                .expect("truncated")
                .chars()
                .count(),
            MAX_GENERATED_DESCRIPTION_CHARACTERS
        );
        assert_eq!(derive_description("   "), None);
    }

    #[test]
    fn names_fall_back_when_the_slug_is_empty_or_invalid() {
        let taken = HashSet::new();
        // No ASCII at all.
        assert_eq!(
            derive_name("别把提示词改成给路径", "abc123de-f456", &taken),
            "memory-abc123de"
        );
        // A Windows reserved device name is not a creatable file stem.
        assert_eq!(derive_name("con", "row-9", &taken), "memory-row9");
        // A seed with no usable characters still yields a valid stem.
        assert_eq!(derive_name("", "---", &taken), "memory");
    }

    #[test]
    fn an_ascii_identifier_inside_chinese_prose_becomes_the_name() {
        // Far more useful than the seed fallback, and the common shape in this pool.
        assert_eq!(
            derive_name("别把 compose_prompt 改成给路径。", "row-1", &HashSet::new()),
            "compose-prompt"
        );
    }

    #[test]
    fn collisions_get_the_first_free_numeric_suffix() {
        // The taken entries must be the slug this input actually produces — NAME_WORD_BUDGET is 6,
        // so a 5-word entry never collides and the suffix path is never reached.
        let taken = HashSet::from([
            "same-words-here-in-all-rows".to_string(),
            "same-words-here-in-all-rows-2".to_string(),
        ]);

        assert_eq!(
            derive_name("Same words here in all rows.", "row-3", &taken),
            "same-words-here-in-all-rows-3"
        );
    }
}
