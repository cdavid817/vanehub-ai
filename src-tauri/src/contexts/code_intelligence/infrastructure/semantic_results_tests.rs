use super::semantic_results::{
    query_status_label, SemanticResultNormalizer, MAX_DEFINITIONS, MAX_HOVER_DOCUMENTATION_BYTES,
    MAX_PREVIEW_BYTES, MAX_REFERENCES,
};
use crate::contexts::code_intelligence::domain::models::{
    DiagnosticSnapshot, DocumentVersion, NormalizedDiagnostic, NormalizedRange, PositionEncoding,
    QueryStatus,
};
use crate::test_support::TempDirectory;
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, LanguageString, Location, LocationLink,
    MarkedString, MarkupContent, MarkupKind, Position, Range, Uri,
};
use url::Url;

#[test]
fn definition_null_single_array_and_location_links_share_one_shape() {
    let directory = TempDirectory::new("semantic-definition-shapes");
    let target = directory.write("src/lib.rs", "fn alpha() {}\nfn beta() {}\n");
    let normalizer = normalizer(&directory);
    assert!(normalizer.definitions(None).locations.is_empty());

    let scalar = normalizer.definitions(Some(GotoDefinitionResponse::Scalar(location(&target, 0))));
    assert_eq!(scalar.total, 1);
    assert_eq!(scalar.locations[0].file(), "src/lib.rs");

    let array = normalizer.definitions(Some(GotoDefinitionResponse::Array(vec![
        location(&target, 0),
        location(&target, 1),
    ])));
    assert_eq!(array.total, 2);

    let link = LocationLink {
        origin_selection_range: None,
        target_uri: file_uri(&target),
        target_range: range(0),
        target_selection_range: range(1),
    };
    let linked = normalizer.definitions(Some(GotoDefinitionResponse::Link(vec![link])));
    assert_eq!(linked.locations[0].range.start_line, 2);
}

#[test]
fn locations_outside_the_workspace_are_filtered_and_counted() {
    let directory = TempDirectory::new("semantic-filter-inside");
    let outside = TempDirectory::new("semantic-filter-outside");
    let inside_path = directory.write("src/lib.rs", "fn inside() {}\n");
    let outside_path = outside.write("src/outside.rs", "fn outside() {}\n");

    let result = normalizer(&directory).definitions(Some(GotoDefinitionResponse::Array(vec![
        location(&inside_path, 0),
        location(&outside_path, 0),
    ])));

    assert_eq!(result.total, 1);
    assert_eq!(result.filtered_count, 1);
    assert_eq!(result.locations[0].file(), "src/lib.rs");
}

#[test]
fn non_file_uri_results_are_rejected_and_counted() {
    let directory = TempDirectory::new("semantic-non-file-uri");
    let target = directory.write("src/lib.rs", "fn inside() {}\n");
    let result = normalizer(&directory).definitions(Some(GotoDefinitionResponse::Array(vec![
        location(&target, 0),
        Location::new(uri("https://attacker.invalid/private.rs"), range(0)),
        Location::new(uri("untitled:private-buffer.rs"), range(0)),
    ])));

    assert_eq!(result.total, 1);
    assert_eq!(result.filtered_count, 2);
    assert_eq!(result.locations[0].file(), "src/lib.rs");
}

#[test]
fn oversized_definition_results_preserve_total_and_stop_at_the_hard_cap() {
    let directory = TempDirectory::new("semantic-definition-limit");
    let target = directory.write("src/lib.rs", "fn inside() {}\n");
    let locations = (0..100).map(|_| location(&target, 0)).collect();

    let result = normalizer(&directory).definitions(Some(GotoDefinitionResponse::Array(locations)));

    assert_eq!(result.total, 100);
    assert_eq!(result.locations.len(), MAX_DEFINITIONS);
    assert!(result.truncated);
}

#[test]
fn references_are_sorted_then_truncated_with_the_accepted_total_preserved() {
    let directory = TempDirectory::new("semantic-reference-order");
    let content = (0..60)
        .map(|line| format!("fn item_{line}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = directory.write("src/lib.rs", &content);
    let locations = (0..60).rev().map(|line| location(&target, line)).collect();

    let result = normalizer(&directory).references(locations);

    assert_eq!(result.total, 60);
    assert_eq!(result.locations.len(), MAX_REFERENCES);
    assert!(result.truncated);
    assert_eq!(result.locations[0].range.start_line, 1);
    assert_eq!(result.locations[49].range.start_line, 50);
}

#[test]
fn location_truncation_changes_only_after_each_exact_cap() {
    let directory = TempDirectory::new("semantic-exact-limits");
    let content = (0..=MAX_REFERENCES)
        .map(|line| format!("fn item_{line}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = directory.write("src/lib.rs", &content);
    let normalizer = normalizer(&directory);

    let exact_definitions = (0..MAX_DEFINITIONS as u32)
        .map(|line| location(&target, line))
        .collect();
    let definitions =
        normalizer.definitions(Some(GotoDefinitionResponse::Array(exact_definitions)));
    assert_eq!(definitions.total, MAX_DEFINITIONS);
    assert_eq!(definitions.locations.len(), MAX_DEFINITIONS);
    assert!(!definitions.truncated);

    let over_references = (0..=MAX_REFERENCES as u32)
        .rev()
        .map(|line| location(&target, line))
        .collect();
    let references = normalizer.references(over_references);
    assert_eq!(references.total, MAX_REFERENCES + 1);
    assert_eq!(references.locations.len(), MAX_REFERENCES);
    assert!(references.truncated);
    assert_eq!(references.locations[0].range.start_line, 1);
    assert_eq!(
        references.locations[MAX_REFERENCES - 1].range.start_line,
        50
    );
}

#[test]
fn previews_are_utf8_safe_and_hard_bounded() {
    let directory = TempDirectory::new("semantic-preview-bound");
    let target = directory.write("src/lib.rs", &format!("{}\n", "😀".repeat(300)));

    let result = normalizer(&directory)
        .definitions(Some(GotoDefinitionResponse::Scalar(location(&target, 0))));
    let preview = result.locations[0].preview.as_deref().expect("preview");

    assert!(preview.len() <= MAX_PREVIEW_BYTES);
    assert!(preview.is_char_boundary(preview.len()));
}

#[test]
fn hover_normalizes_marked_signature_markdown_range_and_executable_html() {
    let directory = TempDirectory::new("semantic-hover");
    directory.write("src/lib.rs", "fn alpha() {}\n");
    let hover = Hover {
        contents: HoverContents::Array(vec![
            MarkedString::LanguageString(LanguageString {
                language: "rust".to_owned(),
                value: "fn alpha()".to_owned(),
            }),
            MarkedString::String("Docs <script>alert(1)</script>".to_owned()),
        ]),
        range: Some(range(0)),
    };

    let result = normalizer(&directory)
        .hover("fn alpha() {}\n", Some(hover))
        .expect("hover");

    assert_eq!(result.signature.as_deref(), Some("fn alpha()"));
    assert!(result
        .documentation
        .as_deref()
        .is_some_and(|value| { value.contains("&lt;script") && !value.contains("<script") }));
    assert_eq!(result.range.expect("range").start_line, 1);
}

#[test]
fn hover_markup_and_multibyte_documentation_are_bounded() {
    let directory = TempDirectory::new("semantic-hover-limit");
    directory.write("src/lib.rs", "fn alpha() {}\n");
    let hover = Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "界".repeat(MAX_HOVER_DOCUMENTATION_BYTES),
        }),
        range: None,
    };

    let result = normalizer(&directory)
        .hover("fn alpha() {}\n", Some(hover))
        .expect("hover");

    assert!(result.truncated);
    assert!(result
        .documentation
        .as_deref()
        .is_some_and(|value| value.len() <= MAX_HOVER_DOCUMENTATION_BYTES));
}

#[test]
fn diagnostic_snapshots_preserve_empty_and_current_version_semantics() {
    let version = DocumentVersion::new(7);
    let empty = DiagnosticSnapshot::new(Some(version), version, Vec::new(), 10);
    assert!(empty.is_current_for(version));
    assert!(empty.diagnostics().is_empty());

    let diagnostic = NormalizedDiagnostic {
        range: NormalizedRange::new(1, 1, 1, 2).expect("range"),
        severity: None,
        message: "bounded".to_owned(),
        source: Some("fixture".to_owned()),
        code: None,
        related_information: Vec::new(),
    };
    let populated = DiagnosticSnapshot::new(None, version, vec![diagnostic], 11);
    assert_eq!(populated.diagnostics().len(), 1);
}

#[test]
fn every_query_status_has_a_stable_fail_soft_label() {
    assert_eq!(query_status_label(QueryStatus::Ready), "ready");
    assert_eq!(query_status_label(QueryStatus::Warming), "warming");
    assert_eq!(query_status_label(QueryStatus::Timeout), "timeout");
    assert_eq!(query_status_label(QueryStatus::Unavailable), "unavailable");
    assert_eq!(query_status_label(QueryStatus::Failed), "failed");
}

fn normalizer(directory: &TempDirectory) -> SemanticResultNormalizer {
    SemanticResultNormalizer::new(directory.path(), PositionEncoding::Utf16).expect("normalizer")
}

fn location(path: &std::path::Path, line: u32) -> Location {
    Location::new(file_uri(path), range(line))
}

fn file_uri(path: &std::path::Path) -> Uri {
    Url::from_file_path(path)
        .expect("file url")
        .as_str()
        .parse()
        .expect("file uri")
}

fn uri(value: &str) -> Uri {
    value.parse().expect("URI")
}

fn range(line: u32) -> Range {
    Range::new(Position::new(line, 0), Position::new(line, 2))
}
