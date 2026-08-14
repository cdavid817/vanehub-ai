use super::FetchBody;
use chrono::Utc;
use html5ever::tokenizer::{
    BufferQueue, CharacterTokens, EndTag, StartTag, TagToken, Token, TokenSink, TokenSinkResult,
    Tokenizer,
};
use std::cell::RefCell;

const EXTRACTION_CONTRACT_VERSION: u16 = 1;
const CONTROLLER_MAX_TEXT_CHARS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtractionLimits {
    pub(crate) max_text_chars: usize,
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_text_chars: 30_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedPage {
    pub(crate) contract_version: u16,
    pub(crate) provider: String,
    pub(crate) evidence_kind: String,
    pub(crate) normalized_url: String,
    pub(crate) final_url: String,
    pub(crate) title: Option<String>,
    pub(crate) media_type: String,
    pub(crate) captured_at: String,
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtractionError {
    InvalidLimit,
    InvalidEncoding,
    UnsupportedMediaType,
}

pub(crate) struct WebContentExtractor;

impl WebContentExtractor {
    pub(crate) fn extract(
        fetched: FetchBody,
        limits: ExtractionLimits,
    ) -> Result<ExtractedPage, ExtractionError> {
        if limits.max_text_chars == 0 || limits.max_text_chars > CONTROLLER_MAX_TEXT_CHARS {
            return Err(ExtractionError::InvalidLimit);
        }
        let decoded =
            std::str::from_utf8(&fetched.bytes).map_err(|_| ExtractionError::InvalidEncoding)?;
        let (title, source_text) = match fetched.media_type.as_str() {
            "text/html" | "application/xhtml+xml" => extract_html(decoded),
            "text/plain" | "application/json" => (None, decoded.to_string()),
            "application/xml" | "text/xml" => (None, extract_markup_text(decoded)),
            _ => return Err(ExtractionError::UnsupportedMediaType),
        };
        let (text, truncated) = normalize_and_bound(&source_text, limits.max_text_chars);
        let title = title.and_then(|value| {
            let (bounded, _) = normalize_and_bound(&value, 512);
            (!bounded.is_empty()).then_some(bounded)
        });
        Ok(ExtractedPage {
            contract_version: EXTRACTION_CONTRACT_VERSION,
            provider: "guarded_http".to_string(),
            evidence_kind: "fetched_content".to_string(),
            normalized_url: fetched.normalized_url,
            final_url: fetched.final_url,
            title,
            media_type: fetched.media_type,
            captured_at: Utc::now().to_rfc3339(),
            text,
            truncated,
        })
    }
}

#[derive(Debug, Default)]
struct HtmlState {
    hidden_depth: usize,
    title_depth: usize,
    title: String,
    text: String,
}

#[derive(Debug, Default)]
struct TextSink(RefCell<HtmlState>);

impl TokenSink for TextSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        let mut state = self.0.borrow_mut();
        match token {
            TagToken(tag) => {
                let name = tag.name.as_ref();
                match tag.kind {
                    StartTag => {
                        if is_hidden_tag(name) {
                            state.hidden_depth = state.hidden_depth.saturating_add(1);
                        }
                        if name == "title" {
                            state.title_depth = state.title_depth.saturating_add(1);
                        }
                        if state.hidden_depth == 0 && is_block_tag(name) {
                            state.text.push('\n');
                        }
                    }
                    EndTag => {
                        if name == "title" {
                            state.title_depth = state.title_depth.saturating_sub(1);
                        }
                        if is_hidden_tag(name) {
                            state.hidden_depth = state.hidden_depth.saturating_sub(1);
                        }
                        if state.hidden_depth == 0 && is_block_tag(name) {
                            state.text.push('\n');
                        }
                    }
                }
            }
            CharacterTokens(value) => {
                if state.title_depth > 0 && state.hidden_depth == 0 {
                    state.title.push_str(&value);
                }
                if state.hidden_depth == 0 {
                    state.text.push_str(&value);
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

fn extract_html(html: &str) -> (Option<String>, String) {
    let input = BufferQueue::default();
    input.push_back(html.to_string().into());
    let tokenizer = Tokenizer::new(TextSink::default(), Default::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    let state = tokenizer.sink.0.into_inner();
    ((!state.title.is_empty()).then_some(state.title), state.text)
}

fn extract_markup_text(markup: &str) -> String {
    extract_html(markup).1
}

fn is_hidden_tag(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "noscript" | "template" | "svg" | "canvas"
    )
}

fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "article"
            | "aside"
            | "blockquote"
            | "br"
            | "div"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "li"
            | "main"
            | "nav"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "td"
            | "th"
            | "tr"
    )
}

fn normalize_and_bound(input: &str, max_chars: usize) -> (String, bool) {
    let mut output = String::new();
    let mut pending_space = false;
    let mut pending_newline = false;
    let mut truncated = false;
    let mut character_count = 0_usize;
    for character in input.chars() {
        if character.is_whitespace() {
            if character == '\n' || character == '\r' {
                pending_newline = true;
            } else {
                pending_space = true;
            }
            continue;
        }
        let separator = if output.is_empty() {
            None
        } else if pending_newline {
            Some('\n')
        } else if pending_space {
            Some(' ')
        } else {
            None
        };
        let required = 1 + usize::from(separator.is_some());
        if character_count.saturating_add(required) > max_chars {
            truncated = true;
            break;
        }
        if let Some(separator) = separator {
            output.push(separator);
            character_count += 1;
        }
        pending_space = false;
        pending_newline = false;
        output.push(character);
        character_count += 1;
    }
    (output, truncated)
}

#[cfg(test)]
#[path = "extraction_tests.rs"]
mod tests;
