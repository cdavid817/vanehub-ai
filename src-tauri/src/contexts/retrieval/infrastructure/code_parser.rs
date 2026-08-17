use std::path::Path;
use std::{fs::File, io::Read};
use tree_sitter::{Language, Parser, Query, Tree};

use super::super::domain::{content_hash_bytes, CodeLanguage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeParseFailure {
    Unreadable,
    SizeLimit,
    InvalidUtf8,
    Grammar,
    Parser,
}

impl CodeParseFailure {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::SizeLimit => "size_limit",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::Grammar => "grammar",
            Self::Parser => "parser",
        }
    }
}

pub(crate) struct ParsedCodeFile {
    pub(crate) content: String,
    pub(crate) raw_content_hash: String,
    pub(crate) tree: Tree,
    #[cfg(test)]
    pub(crate) has_syntax_errors: bool,
}

pub(crate) fn load_and_parse(
    path: &Path,
    relative_path: &str,
    language: CodeLanguage,
    max_file_bytes: u64,
) -> Result<ParsedCodeFile, CodeParseFailure> {
    if max_file_bytes == 0 {
        return Err(CodeParseFailure::SizeLimit);
    }
    let metadata = path.metadata().map_err(|_| CodeParseFailure::Unreadable)?;
    if metadata.len() > max_file_bytes {
        return Err(CodeParseFailure::SizeLimit);
    }
    let capacity = usize::try_from(metadata.len())
        .unwrap_or(usize::MAX)
        .min(max_file_bytes as usize);
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .map_err(|_| CodeParseFailure::Unreadable)?
        .take(max_file_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| CodeParseFailure::Unreadable)?;
    if bytes.len() as u64 > max_file_bytes {
        return Err(CodeParseFailure::SizeLimit);
    }
    let raw_content_hash = content_hash_bytes(&bytes);
    let content = String::from_utf8(bytes).map_err(|_| CodeParseFailure::InvalidUtf8)?;
    let grammar = grammar_for(language, relative_path)?;
    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|_| CodeParseFailure::Grammar)?;
    let tree = parser
        .parse(&content, None)
        .ok_or(CodeParseFailure::Parser)?;
    #[cfg(test)]
    let has_syntax_errors = tree.root_node().has_error();
    Ok(ParsedCodeFile {
        content,
        raw_content_hash,
        tree,
        #[cfg(test)]
        has_syntax_errors,
    })
}

pub(crate) fn grammar_for(
    language: CodeLanguage,
    relative_path: &str,
) -> Result<Language, CodeParseFailure> {
    let grammar = match language {
        CodeLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        CodeLanguage::TypeScript if relative_path.to_ascii_lowercase().ends_with(".tsx") => {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        }
        CodeLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        CodeLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        CodeLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        CodeLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        CodeLanguage::Java => tree_sitter_java::LANGUAGE.into(),
        CodeLanguage::C => tree_sitter_c::LANGUAGE.into(),
        CodeLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
    };
    Ok(grammar)
}

pub(crate) fn symbol_query(
    language: CodeLanguage,
    relative_path: &str,
) -> Result<Query, CodeParseFailure> {
    let grammar = grammar_for(language, relative_path)?;
    Query::new(&grammar, query_source(language)).map_err(|_| CodeParseFailure::Grammar)
}

fn query_source(language: CodeLanguage) -> &'static str {
    match language {
        CodeLanguage::JavaScript => include_str!("../queries/javascript.scm"),
        CodeLanguage::TypeScript => include_str!("../queries/typescript.scm"),
        CodeLanguage::Python => include_str!("../queries/python.scm"),
        CodeLanguage::Rust => include_str!("../queries/rust.scm"),
        CodeLanguage::Go => include_str!("../queries/go.scm"),
        CodeLanguage::Java => include_str!("../queries/java.scm"),
        CodeLanguage::C => include_str!("../queries/c.scm"),
        CodeLanguage::Cpp => include_str!("../queries/cpp.scm"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::domain::content_hash;
    use crate::test_support::TempDirectory;
    use std::time::Instant;
    use tree_sitter::{InputEdit, Point};

    #[test]
    fn every_parser_and_symbol_query_loads_for_a_minimal_definition() {
        for (language, path, source) in parser_fixtures() {
            let directory = TempDirectory::new("code-parser-language");
            let absolute = directory.write(path, source);
            let parsed = load_and_parse(&absolute, path, language, 1024).expect("parse source");
            assert!(!parsed.has_syntax_errors, "syntax error for {path}");
            assert_eq!(parsed.raw_content_hash, content_hash(source));
            assert_eq!(parsed.content, source);
            symbol_query(language, path).expect("compile symbol query");
        }
    }

    fn parser_fixtures() -> [(CodeLanguage, &'static str, &'static str); 9] {
        [
            (
                CodeLanguage::JavaScript,
                "a.js",
                include_str!("../../../../tests/fixtures/code_index/javascript.js"),
            ),
            (
                CodeLanguage::TypeScript,
                "a.ts",
                include_str!("../../../../tests/fixtures/code_index/typescript.ts"),
            ),
            (
                CodeLanguage::TypeScript,
                "a.tsx",
                include_str!("../../../../tests/fixtures/code_index/typescript.tsx"),
            ),
            (
                CodeLanguage::Python,
                "a.py",
                include_str!("../../../../tests/fixtures/code_index/python.py"),
            ),
            (
                CodeLanguage::Rust,
                "a.rs",
                include_str!("../../../../tests/fixtures/code_index/rust.rs"),
            ),
            (
                CodeLanguage::Go,
                "a.go",
                include_str!("../../../../tests/fixtures/code_index/go.go"),
            ),
            (
                CodeLanguage::Java,
                "A.java",
                include_str!("../../../../tests/fixtures/code_index/java.java"),
            ),
            (
                CodeLanguage::C,
                "a.c",
                include_str!("../../../../tests/fixtures/code_index/c.c"),
            ),
            (
                CodeLanguage::Cpp,
                "a.cpp",
                include_str!("../../../../tests/fixtures/code_index/cpp.cpp"),
            ),
        ]
    }

    #[test]
    fn bounded_loading_rejects_growth_and_invalid_utf8_with_safe_categories() {
        let directory = TempDirectory::new("code-parser-bounds");
        let oversized = directory.write("large.rs", "fn main() {}");
        assert_eq!(
            load_and_parse(&oversized, "large.rs", CodeLanguage::Rust, 4).err(),
            Some(CodeParseFailure::SizeLimit)
        );
        let invalid = directory.path().join("invalid.rs");
        std::fs::write(&invalid, [0xff, 0xfe]).expect("write invalid utf8");
        assert_eq!(
            load_and_parse(&invalid, "invalid.rs", CodeLanguage::Rust, 10).err(),
            Some(CodeParseFailure::InvalidUtf8)
        );
        assert_eq!(CodeParseFailure::InvalidUtf8.as_str(), "invalid_utf8");
    }

    #[test]
    fn recoverable_syntax_errors_return_a_tree_for_best_effort_chunking() {
        let directory = TempDirectory::new("code-parser-error-recovery");
        let path = directory.write("broken.rs", "fn broken( {");
        let parsed =
            load_and_parse(&path, "broken.rs", CodeLanguage::Rust, 1024).expect("parse tree");
        assert!(parsed.has_syntax_errors);
    }

    #[test]
    fn performance_tree_sitter_incremental_updates_emit_scale_and_percentile_evidence() {
        for (dataset, functions) in [
            ("repo-small", 32_usize),
            ("repo-medium", 512),
            ("repo-large", 2_048),
        ] {
            let mut source = (0..functions)
                .map(|index| format!("fn f{index}() -> usize {{ {index} }}\n"))
                .collect::<String>();
            let language = grammar_for(CodeLanguage::Rust, "fixture.rs").expect("grammar");
            let mut parser = Parser::new();
            parser.set_language(&language).expect("language");
            let mut tree = parser.parse(&source, None).expect("initial parse");
            let mut samples = Vec::new();
            for iteration in 0..7 {
                source.replace_range(3..4, if iteration % 2 == 0 { "g" } else { "f" });
                tree.edit(&InputEdit {
                    start_byte: 3,
                    old_end_byte: 4,
                    new_end_byte: 4,
                    start_position: Point::new(0, 3),
                    old_end_position: Point::new(0, 4),
                    new_end_position: Point::new(0, 4),
                });
                let started = Instant::now();
                tree = parser
                    .parse(&source, Some(&tree))
                    .expect("incremental parse");
                samples.push(started.elapsed().as_micros());
            }
            assert!(!tree.root_node().has_error());
            samples.sort_unstable();
            eprintln!(
                "TREE_SITTER_PERFORMANCE dataset={dataset}@1 files={} bytes={} symbols={functions} p50Micros={} p95Micros={}",
                functions,
                source.len(),
                samples[3],
                samples[6]
            );
        }
    }
}
