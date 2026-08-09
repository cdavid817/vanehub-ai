use tree_sitter::Node;

use super::super::domain::{content_hash, redact_code, CodeChunk, CodeLanguage};
use super::code_parser::ParsedCodeFile;
use super::code_symbols::ExtractedSymbol;

pub(crate) const DEFAULT_MAX_CHUNK_BYTES: usize = 6 * 1024;

pub(crate) fn chunk_code(
    workspace_id: &str,
    relative_path: &str,
    language: CodeLanguage,
    parsed: &ParsedCodeFile,
    symbols: &[ExtractedSymbol],
    max_chunk_bytes: usize,
) -> Vec<CodeChunk> {
    if max_chunk_bytes == 0 || parsed.content.is_empty() {
        return Vec::new();
    }
    let max_chunk_bytes = max_chunk_bytes.min(DEFAULT_MAX_CHUNK_BYTES);
    let mut chunks = Vec::new();
    if symbols.is_empty() {
        append_range_chunks(
            &mut chunks,
            workspace_id,
            relative_path,
            language,
            parsed,
            0,
            parsed.content.len(),
            None,
            max_chunk_bytes,
        );
    } else {
        for symbol in symbols {
            append_range_chunks(
                &mut chunks,
                workspace_id,
                relative_path,
                language,
                parsed,
                symbol.start_byte,
                symbol.end_byte,
                Some(symbol),
                max_chunk_bytes,
            );
        }
    }
    for (ordinal, chunk) in chunks.iter_mut().enumerate() {
        chunk.ordinal = ordinal as u32;
    }
    chunks
}

#[allow(clippy::too_many_arguments)]
fn append_range_chunks(
    chunks: &mut Vec<CodeChunk>,
    workspace_id: &str,
    relative_path: &str,
    language: CodeLanguage,
    parsed: &ParsedCodeFile,
    start: usize,
    end: usize,
    symbol: Option<&ExtractedSymbol>,
    max_chunk_bytes: usize,
) {
    if start >= end || parsed.content.get(start..end).is_none() {
        return;
    }
    let context = structural_context(relative_path, language, symbol);
    let payload_limit = max_chunk_bytes.saturating_sub(context.len()).max(1);
    let cut_points = named_child_cut_points(parsed.tree.root_node(), start, end);
    for (part, (slice_start, slice_end)) in
        split_range(&parsed.content, start, end, payload_limit, &cut_points)
            .into_iter()
            .enumerate()
    {
        let Some(body) = parsed.content.get(slice_start..slice_end) else {
            continue;
        };
        let redacted = redact_code(&format!("{context}{body}"));
        let symbol_key = symbol.map_or_else(
            || "fallback".to_string(),
            |value| value.symbol.symbol_id.clone(),
        );
        let chunk_key = format!("{symbol_key}:{part}");
        let source_id = content_hash(&format!("{workspace_id}\0{relative_path}\0{chunk_key}"));
        chunks.push(CodeChunk {
            source_id,
            content_hash: content_hash(&redacted.text),
            content: redacted.text,
            language: language.as_str().to_string(),
            start_line: line_at(&parsed.content, slice_start),
            end_line: end_line_at(&parsed.content, slice_start, slice_end),
            symbol_name: symbol.map(|value| value.symbol.display_name.clone()),
            symbol_kind: symbol.map(|value| value.symbol.symbol_kind.clone()),
            ordinal: 0,
            chunk_key,
            redaction_count: redacted.count,
        });
    }
}

fn structural_context(
    relative_path: &str,
    language: CodeLanguage,
    symbol: Option<&ExtractedSymbol>,
) -> String {
    let symbol_context = symbol.map_or_else(String::new, |value| {
        let qualified = value.symbol.container_name.as_ref().map_or_else(
            || value.symbol.display_name.clone(),
            |container| format!("{container}::{}", value.symbol.display_name),
        );
        format!("symbol {} {qualified}\n", value.symbol.symbol_kind)
    });
    format!(
        "file {relative_path}\nlanguage {}\n{symbol_context}\n",
        language.as_str()
    )
}

fn named_child_cut_points(root: Node<'_>, start: usize, end: usize) -> Vec<usize> {
    let Some(node) = root.descendant_for_byte_range(start, end.saturating_sub(1)) else {
        return Vec::new();
    };
    let mut cursor = node.walk();
    let mut points = node
        .named_children(&mut cursor)
        .map(|child| child.end_byte())
        .filter(|point| *point > start && *point < end)
        .collect::<Vec<_>>();
    points.sort_unstable();
    points.dedup();
    points
}

fn split_range(
    source: &str,
    start: usize,
    end: usize,
    max_bytes: usize,
    named_cut_points: &[usize],
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let target = cursor.saturating_add(max_bytes).min(end);
        if target == end {
            ranges.push((cursor, end));
            break;
        }
        let named_cut = named_cut_points
            .iter()
            .copied()
            .rfind(|point| *point > cursor && *point <= target);
        let cut = named_cut.unwrap_or_else(|| line_or_character_cut(source, cursor, target));
        if cut <= cursor {
            break;
        }
        ranges.push((cursor, cut));
        cursor = cut;
    }
    ranges
}

fn line_or_character_cut(source: &str, start: usize, target: usize) -> usize {
    let mut boundary = target.min(source.len());
    while boundary > start && !source.is_char_boundary(boundary) {
        boundary -= 1;
    }
    source[start..boundary]
        .rfind('\n')
        .map(|offset| start + offset + 1)
        .filter(|cut| *cut > start)
        .unwrap_or(boundary)
}

fn line_at(source: &str, offset: usize) -> u32 {
    (source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1) as u32
}

fn end_line_at(source: &str, start: usize, end: usize) -> u32 {
    if end <= start {
        return line_at(source, start);
    }
    let preceding = end.saturating_sub(1);
    line_at(source, preceding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::infrastructure::code_parser::load_and_parse;
    use crate::contexts::retrieval::infrastructure::code_symbols::extract_symbols;
    use crate::test_support::TempDirectory;

    fn parsed_chunks(
        language: CodeLanguage,
        path: &str,
        source: &str,
        max_bytes: usize,
    ) -> Vec<CodeChunk> {
        let directory = TempDirectory::new("code-chunks");
        let absolute = directory.write(path, source);
        let parsed = load_and_parse(&absolute, path, language, source.len() as u64 + 1)
            .expect("parse source");
        let symbols = extract_symbols(path, language, &parsed).expect("extract symbols");
        chunk_code("workspace", path, language, &parsed, &symbols, max_bytes)
    }

    #[test]
    fn symbol_chunks_carry_context_locations_and_redacted_content() {
        let chunks = parsed_chunks(
            CodeLanguage::Rust,
            "src/lib.rs",
            "fn login() {\n let api_key = \"SENSITIVE-CHUNK\";\n}\n",
            512,
        );
        assert_eq!(chunks.len(), 1);
        let chunk = &chunks[0];
        assert_eq!(chunk.symbol_name.as_deref(), Some("login"));
        assert_eq!(chunk.start_line, 1);
        assert_eq!(chunk.end_line, 3);
        assert!(chunk.content.contains("file src/lib.rs"));
        assert!(chunk.content.contains("[REDACTED]"));
        assert!(!chunk.content.contains("SENSITIVE-CHUNK"));
        assert_eq!(chunk.redaction_count, 1);
    }

    #[test]
    fn oversized_symbols_split_at_bounded_utf8_safe_ranges() {
        let body = (0..80)
            .map(|index| format!("    let value_{index} = \"多字节-{index}\";\n"))
            .collect::<String>();
        let source = format!("fn large() {{\n{body}}}\n");
        let chunks = parsed_chunks(CodeLanguage::Rust, "large.rs", &source, 256);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.content.len() <= 256));
        assert!(chunks
            .windows(2)
            .all(|pair| pair[0].end_line <= pair[1].start_line));
    }

    #[test]
    fn files_without_symbols_and_partial_syntax_errors_get_fallback_chunks() {
        let no_symbols = parsed_chunks(
            CodeLanguage::JavaScript,
            "values.js",
            "const value = 42;\n",
            128,
        );
        assert_eq!(no_symbols.len(), 1);
        assert!(no_symbols[0].symbol_name.is_none());

        let broken = parsed_chunks(
            CodeLanguage::Rust,
            "broken.rs",
            "fn valid() {}\nfn broken( {\n",
            128,
        );
        assert!(!broken.is_empty());
    }

    #[test]
    fn duplicate_symbol_names_produce_distinct_deterministic_chunk_keys() {
        let source = "mod a { fn same() {} }\nmod b { fn same() {} }";
        let first = parsed_chunks(CodeLanguage::Rust, "lib.rs", source, 256);
        let second = parsed_chunks(CodeLanguage::Rust, "lib.rs", source, 256);
        assert_eq!(first, second);
        let same = first
            .iter()
            .filter(|chunk| chunk.symbol_name.as_deref() == Some("same"))
            .collect::<Vec<_>>();
        assert_eq!(same.len(), 2);
        assert_ne!(same[0].chunk_key, same[1].chunk_key);
    }
}
