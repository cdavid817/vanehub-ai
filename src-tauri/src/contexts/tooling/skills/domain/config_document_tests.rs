use super::config_document::{
    parse_block, ConfigDocumentError, ConfigNode, MAX_CONFIG_SCALAR_CHARACTERS,
    MAX_CONFIG_SCHEMA_BYTES, MAX_CONFIG_SEQUENCE_ITEMS,
};

fn mapping_keys(node: &ConfigNode) -> Vec<String> {
    node.as_mapping()
        .expect("mapping")
        .iter()
        .map(|(key, _)| key.clone())
        .collect()
}

#[test]
fn parses_nested_mappings_sequences_and_flow_lists() {
    let node = parse_block(
        "  properties:\n    endpoint:\n      type: string\n      default: https://example.com\n      x-vanehub-label: Endpoint\n    mode:\n      type: string\n      enum: [fast, thorough]\n  required:\n    - endpoint\n",
    )
    .expect("parse");

    assert_eq!(mapping_keys(&node), vec!["properties", "required"]);
    let endpoint = node
        .get("properties")
        .and_then(|properties| properties.get("endpoint"))
        .expect("endpoint");
    assert_eq!(
        endpoint.get("type").and_then(ConfigNode::as_scalar),
        Some("string")
    );
    assert_eq!(
        endpoint.get("default").and_then(ConfigNode::as_scalar),
        Some("https://example.com")
    );
    assert_eq!(
        node.get("properties")
            .and_then(|properties| properties.get("mode"))
            .and_then(|mode| mode.get("enum"))
            .and_then(ConfigNode::as_sequence),
        Some(["fast".to_string(), "thorough".to_string()].as_slice())
    );
    assert_eq!(
        node.get("required").and_then(ConfigNode::as_sequence),
        Some(["endpoint".to_string()].as_slice())
    );
}

#[test]
fn empty_block_is_an_empty_mapping() {
    assert_eq!(
        parse_block("\n   \n").expect("parse"),
        ConfigNode::Mapping(Vec::new())
    );
}

#[test]
fn key_without_children_is_an_empty_mapping() {
    let node = parse_block("  properties:\n").expect("parse");
    assert_eq!(
        node.get("properties"),
        Some(&ConfigNode::Mapping(Vec::new()))
    );
}

#[test]
fn rejects_anchors_aliases_and_merge_keys() {
    for (block, construct) in [
        ("  base: &anchor\n    type: string\n", "anchor or alias"),
        ("  copy: *anchor\n", "anchor or alias"),
        ("  <<: other\n", "merge key"),
    ] {
        assert!(
            matches!(
                parse_block(block),
                Err(ConfigDocumentError::UnsupportedConstruct { construct: found, .. })
                    if found == construct
            ),
            "expected {construct} rejection for {block:?}, got {:?}",
            parse_block(block)
        );
    }
}

#[test]
fn rejects_block_scalars_tags_flow_mappings_and_document_markers() {
    for block in [
        "  help: |\n",
        "  help: >\n",
        "  value: !!python/object\n",
        "  properties: {endpoint: string}\n",
        "  ---\n",
    ] {
        assert!(
            matches!(
                parse_block(block),
                Err(ConfigDocumentError::UnsupportedConstruct { .. })
            ),
            "expected rejection for {block:?}, got {:?}",
            parse_block(block)
        );
    }
}

#[test]
fn rejects_tab_indentation_and_misaligned_steps() {
    assert_eq!(
        parse_block("  properties:\n\t\tendpoint: string\n"),
        Err(ConfigDocumentError::TabIndentation(2))
    );
    assert_eq!(
        parse_block("  properties:\n     endpoint: string\n"),
        Err(ConfigDocumentError::MisalignedIndentation(2))
    );
}

#[test]
fn rejects_duplicate_and_invalid_keys() {
    assert!(matches!(
        parse_block("  type: string\n  type: integer\n"),
        Err(ConfigDocumentError::DuplicateKey { key, .. }) if key == "type"
    ));
    assert!(matches!(
        parse_block("  bad key!: string\n"),
        Err(ConfigDocumentError::InvalidKey { .. })
    ));
}

#[test]
fn rejects_oversized_documents_scalars_and_sequences() {
    let oversized = format!("  help: {}\n", "a".repeat(MAX_CONFIG_SCHEMA_BYTES));
    assert_eq!(parse_block(&oversized), Err(ConfigDocumentError::TooLarge));

    let long_scalar = format!("  help: {}\n", "a".repeat(MAX_CONFIG_SCALAR_CHARACTERS + 1));
    assert_eq!(
        parse_block(&long_scalar),
        Err(ConfigDocumentError::ScalarTooLong(1))
    );

    let items = (0..=MAX_CONFIG_SEQUENCE_ITEMS)
        .map(|index| format!("    - value{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        parse_block(&format!("  enum:\n{items}\n")),
        Err(ConfigDocumentError::SequenceTooLong(1))
    );
}

#[test]
fn rejects_nesting_beyond_the_supported_depth() {
    let mut block = String::new();
    for level in 0..10 {
        block.push_str(&"  ".repeat(level + 1));
        block.push_str(&format!("level{level}:\n"));
    }
    assert_eq!(parse_block(&block), Err(ConfigDocumentError::DepthExceeded));
}

#[test]
fn strips_trailing_comments_but_keeps_hashes_inside_values() {
    let node = parse_block("  colour: \"#ff8800\" # accent\n  # standalone\n  label: plain\n")
        .expect("parse");
    assert_eq!(
        node.get("colour").and_then(ConfigNode::as_scalar),
        Some("#ff8800")
    );
    assert_eq!(
        node.get("label").and_then(ConfigNode::as_scalar),
        Some("plain")
    );
}

#[test]
fn rejects_a_list_where_a_mapping_is_required() {
    assert_eq!(
        parse_block("  - endpoint\n"),
        Err(ConfigDocumentError::UnexpectedSequenceItem(1))
    );
}
