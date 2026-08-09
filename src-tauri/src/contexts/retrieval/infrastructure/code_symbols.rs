use tree_sitter::{QueryCursor, StreamingIterator};

use super::super::domain::{content_hash, CodeLanguage, CodeSymbol};
use super::code_parser::{symbol_query, CodeParseFailure, ParsedCodeFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedSymbol {
    pub(crate) symbol: CodeSymbol,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

pub(crate) fn extract_symbols(
    relative_path: &str,
    language: CodeLanguage,
    parsed: &ParsedCodeFile,
) -> Result<Vec<ExtractedSymbol>, CodeParseFailure> {
    let query = symbol_query(language, relative_path)?;
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, parsed.tree.root_node(), parsed.content.as_bytes());
    let mut extracted = Vec::new();
    while let Some(query_match) = matches.next() {
        let mut definition = None;
        let mut name = None;
        let mut symbol_kind = None;
        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            if capture_name == "symbol.name" {
                name = capture
                    .node
                    .utf8_text(parsed.content.as_bytes())
                    .ok()
                    .map(str::to_string);
            } else if let Some(kind) = capture_name.strip_prefix("symbol.") {
                definition = Some(capture.node);
                symbol_kind = Some(kind.to_string());
            }
        }
        let (Some(definition), Some(display_name), Some(symbol_kind)) =
            (definition, name, symbol_kind)
        else {
            continue;
        };
        let start_byte = definition.start_byte();
        let end_byte = definition.end_byte();
        let occurrence_key =
            format!("{relative_path}\0{symbol_kind}\0{display_name}\0{start_byte}\0{end_byte}");
        extracted.push(ExtractedSymbol {
            symbol: CodeSymbol {
                symbol_id: content_hash(&occurrence_key),
                normalized_name: display_name.to_lowercase(),
                display_name,
                symbol_kind,
                container_name: None,
                start_line: (definition.start_position().row + 1) as u32,
                end_line: (definition.end_position().row + 1) as u32,
            },
            start_byte,
            end_byte,
        });
    }
    extracted.sort_by_key(|symbol| (symbol.start_byte, symbol.end_byte));
    assign_containers(&mut extracted);
    Ok(extracted)
}

fn assign_containers(symbols: &mut [ExtractedSymbol]) {
    for index in 0..symbols.len() {
        let start = symbols[index].start_byte;
        let end = symbols[index].end_byte;
        symbols[index].symbol.container_name = symbols
            .iter()
            .enumerate()
            .filter(|(candidate_index, candidate)| {
                *candidate_index != index
                    && is_container_kind(&candidate.symbol.symbol_kind)
                    && candidate.start_byte <= start
                    && candidate.end_byte >= end
                    && (candidate.start_byte < start || candidate.end_byte > end)
            })
            .min_by_key(|(_, candidate)| candidate.end_byte - candidate.start_byte)
            .map(|(_, candidate)| candidate.symbol.display_name.clone());
    }
}

fn is_container_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class" | "interface" | "struct" | "trait" | "enum" | "type"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::infrastructure::code_parser::load_and_parse;
    use crate::test_support::TempDirectory;

    fn symbols(language: CodeLanguage, path: &str, source: &str) -> Vec<ExtractedSymbol> {
        let directory = TempDirectory::new("code-symbols");
        let absolute = directory.write(path, source);
        let parsed = load_and_parse(&absolute, path, language, 4096).expect("parse");
        extract_symbols(path, language, &parsed).expect("extract")
    }

    #[test]
    fn extracts_definitions_for_every_parser_family() {
        for (language, path, source, expected) in [
            (CodeLanguage::JavaScript, "a.js", "function f() {}", "f"),
            (CodeLanguage::TypeScript, "a.ts", "interface Box {}", "Box"),
            (
                CodeLanguage::TypeScript,
                "a.tsx",
                "function View() { return <div />; }",
                "View",
            ),
            (CodeLanguage::Python, "a.py", "def f():\n    pass\n", "f"),
            (CodeLanguage::Rust, "a.rs", "fn f() {}", "f"),
            (CodeLanguage::Go, "a.go", "package p\nfunc f() {}", "f"),
            (CodeLanguage::Java, "A.java", "class A {}", "A"),
            (CodeLanguage::C, "a.c", "void f(void) {}", "f"),
            (CodeLanguage::Cpp, "a.cpp", "class A {};", "A"),
        ] {
            let extracted = symbols(language, path, source);
            assert!(
                extracted
                    .iter()
                    .any(|symbol| symbol.symbol.display_name == expected),
                "missing {expected} in {path}: {extracted:?}"
            );
        }
    }

    #[test]
    fn duplicate_names_have_deterministic_distinct_occurrence_ids() {
        let source = "mod first { fn same() {} }\nmod second { fn same() {} }";
        let first = symbols(CodeLanguage::Rust, "src/lib.rs", source);
        let second = symbols(CodeLanguage::Rust, "src/lib.rs", source);
        let ids = first
            .iter()
            .filter(|symbol| symbol.symbol.display_name == "same")
            .map(|symbol| symbol.symbol.symbol_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        assert_eq!(first, second);
    }

    #[test]
    fn nested_methods_include_the_nearest_container_and_one_based_lines() {
        let extracted = symbols(
            CodeLanguage::Java,
            "Example.java",
            "class Example {\n  void run() {}\n}",
        );
        let method = extracted
            .iter()
            .find(|symbol| symbol.symbol.display_name == "run")
            .expect("method");
        assert_eq!(method.symbol.container_name.as_deref(), Some("Example"));
        assert_eq!(method.symbol.start_line, 2);
        assert_eq!(method.symbol.end_line, 2);
    }
}
