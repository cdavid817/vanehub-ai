use std::collections::{BTreeMap, BTreeSet};

use unicode_normalization::UnicodeNormalization;

const MAX_DESCRIPTION_CHARS: usize = 1_024;
const MAX_LIST_ITEMS: usize = 64;
const MAX_ITEM_CHARS: usize = 128;
const MAX_INSTRUCTION_CHARS: usize = 8_192;
const MAX_TOKENS_PER_FIELD: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LexicalFieldClass {
    Capability,
    Tag,
    Description,
    Heading,
    Instruction,
}

impl LexicalFieldClass {
    pub(crate) fn weight(self) -> u8 {
        match self {
            Self::Capability => 7,
            Self::Tag => 6,
            Self::Description => 5,
            Self::Heading => 3,
            Self::Instruction => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Every string in this projection is untrusted Skill data. Indexing is deliberately pure: it
/// normalizes bounded characters and never interprets commands, placeholders, links, or paths.
pub(crate) struct LexicalDocument {
    pub(crate) skill_id: String,
    pub(crate) revision_hash: String,
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) headings: Vec<String>,
    pub(crate) instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LexicalPosting {
    pub(crate) skill_id: String,
    pub(crate) revision_hash: String,
    pub(crate) field: LexicalFieldClass,
    pub(crate) weight: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LocalLexicalIndex {
    postings: BTreeMap<String, Vec<LexicalPosting>>,
}

impl LocalLexicalIndex {
    pub(crate) fn postings(&self, value: &str) -> &[LexicalPosting] {
        let tokens = tokenize(value, MAX_TOKENS_PER_FIELD);
        if tokens.len() != 1 {
            return &[];
        }
        self.postings.get(&tokens[0]).map_or(&[], Vec::as_slice)
    }

    pub(super) fn score(
        &self,
        skill_id: &str,
        revision_hash: &str,
        terms: &[String],
    ) -> (u8, Vec<LexicalFieldClass>) {
        let tokens = terms
            .iter()
            .flat_map(|term| tokenize(term, MAX_TOKENS_PER_FIELD))
            .collect::<BTreeSet<_>>();
        let mut score = 0_u8;
        let mut fields = BTreeSet::new();
        for token in tokens {
            for posting in self
                .postings
                .get(&token)
                .into_iter()
                .flatten()
                .filter(|posting| {
                    posting.skill_id == skill_id && posting.revision_hash == revision_hash
                })
            {
                score = score.saturating_add(posting.weight).min(20);
                fields.insert(posting.field);
            }
        }
        (score, fields.into_iter().collect())
    }

    #[cfg(test)]
    pub(crate) fn tokens(&self) -> Vec<&str> {
        self.postings.keys().map(String::as_str).collect()
    }
}

pub(crate) fn build_local_lexical_index(documents: &[LexicalDocument]) -> LocalLexicalIndex {
    let mut postings = BTreeMap::<String, Vec<LexicalPosting>>::new();
    for document in documents {
        index_field(
            &mut postings,
            document,
            LexicalFieldClass::Description,
            [bounded(&document.description, MAX_DESCRIPTION_CHARS)],
        );
        index_field(
            &mut postings,
            document,
            LexicalFieldClass::Tag,
            bounded_items(&document.tags),
        );
        index_field(
            &mut postings,
            document,
            LexicalFieldClass::Capability,
            bounded_items(&document.capabilities),
        );
        index_field(
            &mut postings,
            document,
            LexicalFieldClass::Heading,
            bounded_items(&document.headings),
        );
        index_field(
            &mut postings,
            document,
            LexicalFieldClass::Instruction,
            [bounded(&document.instructions, MAX_INSTRUCTION_CHARS)],
        );
    }
    for values in postings.values_mut() {
        values.sort();
        values.dedup();
    }
    LocalLexicalIndex { postings }
}

fn index_field(
    postings: &mut BTreeMap<String, Vec<LexicalPosting>>,
    document: &LexicalDocument,
    field: LexicalFieldClass,
    values: impl IntoIterator<Item = String>,
) {
    let mut tokens = values
        .into_iter()
        .flat_map(|value| tokenize(&value, MAX_TOKENS_PER_FIELD))
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.dedup();
    tokens.truncate(MAX_TOKENS_PER_FIELD);
    for token in tokens {
        postings.entry(token).or_default().push(LexicalPosting {
            skill_id: document.skill_id.clone(),
            revision_hash: document.revision_hash.clone(),
            field,
            weight: field.weight(),
        });
    }
}

fn bounded_items(values: &[String]) -> Vec<String> {
    values
        .iter()
        .take(MAX_LIST_ITEMS)
        .map(|value| bounded(value, MAX_ITEM_CHARS))
        .collect()
}

fn bounded(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn tokenize(value: &str, limit: usize) -> Vec<String> {
    let normalized = value
        .nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut cjk_run = Vec::new();
    for character in normalized.chars() {
        if is_cjk(character) {
            flush_word(&mut word, &mut tokens, limit);
            cjk_run.push(character);
        } else {
            flush_cjk(&mut cjk_run, &mut tokens, limit);
            if character.is_alphanumeric() {
                word.push(character);
            } else {
                flush_word(&mut word, &mut tokens, limit);
            }
        }
        if tokens.len() >= limit {
            break;
        }
    }
    flush_word(&mut word, &mut tokens, limit);
    flush_cjk(&mut cjk_run, &mut tokens, limit);
    tokens.retain(|token| !is_stop_word(token));
    tokens.truncate(limit);
    tokens
}

fn flush_word(word: &mut String, tokens: &mut Vec<String>, limit: usize) {
    if !word.is_empty() && tokens.len() < limit {
        tokens.push(std::mem::take(word));
    } else {
        word.clear();
    }
}

fn flush_cjk(run: &mut Vec<char>, tokens: &mut Vec<String>, limit: usize) {
    if run.is_empty() {
        return;
    }
    for pair in run.windows(2) {
        if tokens.len() >= limit {
            break;
        }
        tokens.push(pair.iter().collect());
    }
    if run.len() == 1 && tokens.len() < limit {
        tokens.push(run[0].to_string());
    }
    run.clear();
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an" | "and" | "for" | "in" | "of" | "on" | "or" | "the" | "to"
    )
}
