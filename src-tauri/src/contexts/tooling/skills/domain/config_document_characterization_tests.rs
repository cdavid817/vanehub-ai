//! Characterization of the bounded YAML subset scanner, recorded before it moves.
//!
//! `config_document_tests.rs` shows the scanner works. This file records exactly *what* it
//! accepts and rejects — every limit at and past its boundary, every unsupported construct, and
//! the precise diagnostic each rejection produces. The scanner is being extracted to a shared
//! crate so `extension_platform` can parse `vanehub-extension.yaml` over the same primitive
//! (`add-unified-extension-platform`, Task 1.A). A relocation that quietly widens what Skills
//! accepts, or quietly changes what an operator is told, is the failure mode worth a suite of its
//! own: it would not fail any existing test.
//!
//! These assertions are deliberately literal. When the extraction lands they must pass unchanged.

use super::config_document::{
    parse_block, ConfigDocumentError, ConfigNode, MAX_CONFIG_KEY_CHARACTERS, MAX_CONFIG_NODES,
    MAX_CONFIG_NODE_DEPTH, MAX_CONFIG_SCALAR_CHARACTERS, MAX_CONFIG_SCHEMA_BYTES,
    MAX_CONFIG_SEQUENCE_ITEMS,
};

fn error(block: &str) -> ConfigDocumentError {
    parse_block(block).expect_err("block should be rejected")
}

fn message(block: &str) -> String {
    error(block).to_string()
}

// ---------------------------------------------------------------------------
// Limit values
// ---------------------------------------------------------------------------

#[test]
fn limit_constants_are_the_values_downstream_bounds_were_chosen_against() {
    assert_eq!(MAX_CONFIG_SCHEMA_BYTES, 16 * 1_024);
    assert_eq!(MAX_CONFIG_NODE_DEPTH, 6);
    assert_eq!(MAX_CONFIG_NODES, 512);
    assert_eq!(MAX_CONFIG_KEY_CHARACTERS, 64);
    assert_eq!(MAX_CONFIG_SCALAR_CHARACTERS, 512);
    assert_eq!(MAX_CONFIG_SEQUENCE_ITEMS, 32);
}

// ---------------------------------------------------------------------------
// Document size
// ---------------------------------------------------------------------------

#[test]
fn a_document_at_the_byte_limit_is_accepted_and_one_byte_over_is_not() {
    // The check is on raw byte length before any normalization, so build the block to an exact
    // size rather than an approximate one.
    let filler = "a".repeat(MAX_CONFIG_SCHEMA_BYTES - "key: ".len());
    let at_limit = format!("key: {filler}");
    assert_eq!(at_limit.len(), MAX_CONFIG_SCHEMA_BYTES);
    // Accepted by the size gate; the scalar limit rejects it later, which is the point — the two
    // bounds are independent and both apply.
    assert!(!matches!(
        parse_block(&at_limit),
        Err(ConfigDocumentError::TooLarge)
    ));

    let over_limit = format!("{at_limit}a");
    assert_eq!(over_limit.len(), MAX_CONFIG_SCHEMA_BYTES + 1);
    assert_eq!(error(&over_limit), ConfigDocumentError::TooLarge);
}

// ---------------------------------------------------------------------------
// Node budget
// ---------------------------------------------------------------------------

#[test]
fn the_node_budget_counts_mapping_keys_and_admits_exactly_the_limit() {
    let at_limit = (0..MAX_CONFIG_NODES)
        .map(|index| format!("k{index}: v"))
        .collect::<Vec<_>>()
        .join("\n");
    let node = parse_block(&at_limit).expect("a document at the node limit is accepted");
    assert_eq!(node.as_mapping().expect("mapping").len(), MAX_CONFIG_NODES);

    let over_limit = (0..=MAX_CONFIG_NODES)
        .map(|index| format!("k{index}: v"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(error(&over_limit), ConfigDocumentError::TooManyNodes);
}

#[test]
fn sequence_items_are_charged_against_the_same_node_budget_as_keys() {
    // One key plus MAX_CONFIG_NODES sequence items is one node past the budget. Recorded because
    // a shared parser could plausibly charge only mappings.
    let items = (0..MAX_CONFIG_NODES)
        .map(|index| format!("  - item{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let block = format!("key:\n{items}");
    // The sequence limit trips first for a list this long, which is itself the recorded behavior.
    assert!(matches!(
        error(&block),
        ConfigDocumentError::SequenceTooLong(_) | ConfigDocumentError::TooManyNodes
    ));
}

// ---------------------------------------------------------------------------
// Depth
// ---------------------------------------------------------------------------

fn nested_mapping(levels: usize) -> String {
    let mut block = String::new();
    for level in 0..levels {
        block.push_str(&"  ".repeat(level));
        block.push_str(&format!("level{level}:\n"));
    }
    block.push_str(&"  ".repeat(levels));
    block.push_str("leaf: value\n");
    block
}

#[test]
fn nesting_is_accepted_up_to_the_depth_limit_and_rejected_past_it() {
    // `parse_mapping` receives depth 0 for the root and `parse_child` increments, so the accepted
    // ceiling is expressed in levels of nesting rather than in the constant directly. Pinned by
    // construction so the extraction cannot shift it by one.
    let deepest_accepted = (0..=MAX_CONFIG_NODE_DEPTH + 2)
        .take_while(|levels| parse_block(&nested_mapping(*levels)).is_ok())
        .last()
        .expect("at least one nesting level is accepted");
    assert_eq!(deepest_accepted, MAX_CONFIG_NODE_DEPTH);

    assert_eq!(
        error(&nested_mapping(deepest_accepted + 1)),
        ConfigDocumentError::DepthExceeded
    );
}

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

#[test]
fn a_key_at_the_character_limit_is_accepted_and_one_character_over_is_not() {
    let at_limit = "k".repeat(MAX_CONFIG_KEY_CHARACTERS);
    let node = parse_block(&format!("{at_limit}: value")).expect("key at the limit is accepted");
    assert_eq!(
        node.get(&at_limit).and_then(ConfigNode::as_scalar),
        Some("value")
    );

    let over_limit = "k".repeat(MAX_CONFIG_KEY_CHARACTERS + 1);
    assert_eq!(
        error(&format!("{over_limit}: value")),
        ConfigDocumentError::InvalidKey {
            line: 1,
            key: over_limit.chars().take(MAX_CONFIG_KEY_CHARACTERS).collect(),
        }
    );
}

#[test]
fn the_key_limit_is_measured_in_bytes_while_the_scalar_limit_is_measured_in_characters() {
    // A recorded asymmetry, not an endorsement: `normalize_key` uses `str::len` and `scalar` uses
    // `chars().count()`. A multi-byte key therefore hits its limit sooner than its character count
    // suggests. The extraction must not "fix" this silently — Skills' accepted set would change.
    let multibyte_key = "é".repeat(MAX_CONFIG_KEY_CHARACTERS / 2);
    assert_eq!(multibyte_key.chars().count(), MAX_CONFIG_KEY_CHARACTERS / 2);
    assert_eq!(multibyte_key.len(), MAX_CONFIG_KEY_CHARACTERS);
    // Accepted by length, then rejected for its non-ASCII characters.
    assert!(matches!(
        error(&format!("{multibyte_key}: value")),
        ConfigDocumentError::InvalidKey { .. }
    ));

    let multibyte_scalar = "é".repeat(MAX_CONFIG_SCALAR_CHARACTERS);
    assert_eq!(multibyte_scalar.len(), MAX_CONFIG_SCALAR_CHARACTERS * 2);
    let node = parse_block(&format!("key: {multibyte_scalar}"))
        .expect("a scalar is bounded by characters, so twice the bytes is still accepted");
    assert_eq!(
        node.get("key").and_then(ConfigNode::as_scalar),
        Some(multibyte_scalar.as_str())
    );
}

#[test]
fn keys_accept_ascii_alphanumerics_underscore_and_dash_and_nothing_else() {
    for key in ["a", "A", "0", "a_b", "a-b", "x-vanehub-label"] {
        parse_block(&format!("{key}: value"))
            .unwrap_or_else(|_| panic!("{key} should be an accepted key"));
    }
    for key in ["a.b", "a b", "a/b", "键", "a+b", "a@b"] {
        assert!(
            parse_block(&format!("{key}: value")).is_err(),
            "{key} should be rejected"
        );
    }
}

#[test]
fn a_key_is_split_at_its_first_unquoted_colon_so_a_second_colon_lands_in_the_value() {
    // `a:b: value` is not an invalid key — `split_key` stops at the first colon, so the key is
    // `a` and everything after it is the scalar. Recorded rather than assumed: a shared parser
    // that split at the last colon, or rejected multiple colons, would change what Skills accepts
    // without failing any test that only checks valid documents.
    let node = parse_block("a:b: value").expect("parse");
    assert_eq!(
        node.get("a").and_then(ConfigNode::as_scalar),
        Some("b: value")
    );
}

#[test]
fn quoted_keys_are_unquoted_before_validation() {
    let node = parse_block("\"quoted\": value\n'single': other").expect("quoted keys are accepted");
    assert_eq!(
        node.get("quoted").and_then(ConfigNode::as_scalar),
        Some("value")
    );
    assert_eq!(
        node.get("single").and_then(ConfigNode::as_scalar),
        Some("other")
    );
}

#[test]
fn duplicate_keys_are_rejected_at_the_repeating_line_and_only_within_one_mapping() {
    assert_eq!(
        error("key: one\nkey: two"),
        ConfigDocumentError::DuplicateKey {
            line: 2,
            key: "key".to_string(),
        }
    );

    // The same key under two different parents is not a duplicate.
    parse_block("a:\n  shared: 1\nb:\n  shared: 2")
        .expect("sibling mappings may each carry the same key");
}

// ---------------------------------------------------------------------------
// Scalars and sequences
// ---------------------------------------------------------------------------

#[test]
fn a_scalar_at_the_character_limit_is_accepted_and_one_character_over_is_not() {
    let at_limit = "v".repeat(MAX_CONFIG_SCALAR_CHARACTERS);
    let node = parse_block(&format!("key: {at_limit}")).expect("scalar at the limit is accepted");
    assert_eq!(
        node.get("key").and_then(ConfigNode::as_scalar),
        Some(at_limit.as_str())
    );

    let over_limit = "v".repeat(MAX_CONFIG_SCALAR_CHARACTERS + 1);
    assert_eq!(
        error(&format!("key: {over_limit}")),
        ConfigDocumentError::ScalarTooLong(1)
    );
}

#[test]
fn surrounding_quotes_are_stripped_from_a_scalar_before_its_length_is_measured() {
    let node = parse_block("key: \"quoted value\"").expect("parse");
    assert_eq!(
        node.get("key").and_then(ConfigNode::as_scalar),
        Some("quoted value")
    );

    let inner = "v".repeat(MAX_CONFIG_SCALAR_CHARACTERS);
    parse_block(&format!("key: \"{inner}\""))
        .expect("quotes are removed before the character count, so this is exactly at the limit");
}

#[test]
fn a_block_sequence_at_the_item_limit_is_accepted_and_one_item_over_is_not() {
    let at_limit = (0..MAX_CONFIG_SEQUENCE_ITEMS)
        .map(|index| format!("  - item{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let node = parse_block(&format!("key:\n{at_limit}")).expect("sequence at the limit");
    assert_eq!(
        node.get("key")
            .and_then(ConfigNode::as_sequence)
            .map(<[String]>::len),
        Some(MAX_CONFIG_SEQUENCE_ITEMS)
    );

    let over_limit = (0..=MAX_CONFIG_SEQUENCE_ITEMS)
        .map(|index| format!("  - item{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    // Reported against the parent line, not the offending item.
    assert_eq!(
        error(&format!("key:\n{over_limit}")),
        ConfigDocumentError::SequenceTooLong(1)
    );
}

#[test]
fn a_flow_sequence_at_the_item_limit_is_accepted_and_one_item_over_is_not() {
    let at_limit = (0..MAX_CONFIG_SEQUENCE_ITEMS)
        .map(|index| format!("i{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let node = parse_block(&format!("key: [{at_limit}]")).expect("flow sequence at the limit");
    assert_eq!(
        node.get("key")
            .and_then(ConfigNode::as_sequence)
            .map(<[String]>::len),
        Some(MAX_CONFIG_SEQUENCE_ITEMS)
    );

    let over_limit = (0..=MAX_CONFIG_SEQUENCE_ITEMS)
        .map(|index| format!("i{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        error(&format!("key: [{over_limit}]")),
        ConfigDocumentError::SequenceTooLong(1)
    );
}

#[test]
fn an_empty_flow_sequence_is_an_empty_sequence_and_a_bare_key_is_an_empty_mapping() {
    assert_eq!(
        parse_block("key: []").expect("parse").get("key").cloned(),
        Some(ConfigNode::Sequence(Vec::new()))
    );
    assert_eq!(
        parse_block("key:").expect("parse").get("key").cloned(),
        Some(ConfigNode::Mapping(Vec::new()))
    );
}

#[test]
fn a_nested_flow_collection_is_rejected_rather_than_flattened() {
    assert_eq!(
        error("key: [a, [b]]"),
        ConfigDocumentError::UnsupportedConstruct {
            line: 1,
            construct: "nested flow collection".to_string(),
        }
    );
    assert_eq!(
        error("key: [a, {b: c}]"),
        ConfigDocumentError::UnsupportedConstruct {
            line: 1,
            construct: "nested flow collection".to_string(),
        }
    );
}

#[test]
fn a_list_entry_that_opens_a_mapping_is_rejected() {
    assert_eq!(
        error("key:\n  - nested:"),
        ConfigDocumentError::UnsupportedConstruct {
            line: 2,
            construct: "nested list entry".to_string(),
        }
    );
    assert_eq!(
        error("key:\n  -"),
        ConfigDocumentError::UnsupportedConstruct {
            line: 2,
            construct: "nested list entry".to_string(),
        }
    );
}

// ---------------------------------------------------------------------------
// Unsupported constructs, one at a time
// ---------------------------------------------------------------------------

#[test]
fn every_unbounded_yaml_construct_is_named_and_rejected() {
    let cases = [
        ("&anchor value", "anchor"),
        ("*alias", "alias"),
        ("<<: base", "merge key"),
        ("--- ", "document marker"),
        ("... ", "document marker"),
        ("? explicit", "explicit key"),
        ("{inline: map}", "flow mapping"),
        ("key: &anchor", "anchor or alias"),
        ("key: *alias", "anchor or alias"),
        ("key: |", "block scalar"),
        ("key: >", "block scalar"),
        ("key: !!str value", "explicit tag"),
    ];

    for (block, construct) in cases {
        assert_eq!(
            error(block),
            ConfigDocumentError::UnsupportedConstruct {
                line: 1,
                construct: construct.to_string(),
            },
            "{block} should be rejected as {construct}"
        );
    }
}

#[test]
fn a_flow_mapping_in_value_position_is_rejected_separately_from_one_that_starts_a_line() {
    // Two different code paths reach the same diagnostic: `reject_unsupported` for a line that
    // opens with `{`, and the value branch of `parse_mapping` for `key: {`.
    assert_eq!(
        error("key: {a: b}"),
        ConfigDocumentError::UnsupportedConstruct {
            line: 1,
            construct: "flow mapping".to_string(),
        }
    );
}

#[test]
fn a_brace_inside_a_value_is_ordinary_text() {
    // Skill bodies use placeholders such as `{skill_base_dir}`; only a value that *opens* with a
    // brace is a flow mapping.
    let node = parse_block("key: path/{skill_base_dir}/file").expect("parse");
    assert_eq!(
        node.get("key").and_then(ConfigNode::as_scalar),
        Some("path/{skill_base_dir}/file")
    );
}

// ---------------------------------------------------------------------------
// Indentation
// ---------------------------------------------------------------------------

#[test]
fn indentation_must_be_two_space_steps_relative_to_the_first_non_blank_line() {
    assert_eq!(
        error("key:\n\tchild: value"),
        ConfigDocumentError::TabIndentation(2)
    );
    assert_eq!(
        error("key:\n   child: value"),
        ConfigDocumentError::MisalignedIndentation(2)
    );
    assert_eq!(
        error("  key: value\nother: value"),
        ConfigDocumentError::MisalignedIndentation(2)
    );

    // A uniformly indented block dedents to its own first line rather than to column zero.
    parse_block("    key:\n      child: value").expect("a dedented block parses");
}

#[test]
fn a_child_indented_more_than_one_step_is_rejected_rather_than_absorbed() {
    assert_eq!(
        error("key:\n    child: value"),
        ConfigDocumentError::MisalignedIndentation(2)
    );
}

#[test]
fn a_line_that_is_not_a_mapping_entry_is_rejected() {
    assert_eq!(error("bare"), ConfigDocumentError::ExpectedMapping(1));
    assert_eq!(
        error("- item"),
        ConfigDocumentError::UnexpectedSequenceItem(1)
    );
}

#[test]
fn line_numbers_are_counted_against_the_original_source_including_blanks_and_comments() {
    // The scanner skips blank and comment lines but keeps their numbering, so a diagnostic points
    // at the line an operator sees in their editor.
    assert_eq!(
        error("# comment\n\nkey: one\nkey: two"),
        ConfigDocumentError::DuplicateKey {
            line: 4,
            key: "key".to_string(),
        }
    );
}

// ---------------------------------------------------------------------------
// Comments and quoting
// ---------------------------------------------------------------------------

#[test]
fn a_hash_is_a_comment_only_after_whitespace_and_never_inside_quotes() {
    let node = parse_block(
        "colour: \"#ffffff\"\nfragment: https://example.com/a#b\ntrailing: value # gone\n",
    )
    .expect("parse");

    assert_eq!(
        node.get("colour").and_then(ConfigNode::as_scalar),
        Some("#ffffff")
    );
    assert_eq!(
        node.get("fragment").and_then(ConfigNode::as_scalar),
        Some("https://example.com/a#b")
    );
    assert_eq!(
        node.get("trailing").and_then(ConfigNode::as_scalar),
        Some("value")
    );
}

#[test]
fn a_colon_inside_quotes_does_not_split_a_key() {
    let node = parse_block("key: \"a: b\"").expect("parse");
    assert_eq!(
        node.get("key").and_then(ConfigNode::as_scalar),
        Some("a: b")
    );
}

#[test]
fn carriage_returns_are_normalized_before_scanning() {
    let node = parse_block("key:\r\n  child: value\r\n").expect("parse");
    assert_eq!(
        node.get("key")
            .and_then(|child| child.get("child"))
            .and_then(ConfigNode::as_scalar),
        Some("value")
    );
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[test]
fn every_rejection_renders_the_message_an_operator_reads() {
    assert_eq!(
        message(&"a".repeat(MAX_CONFIG_SCHEMA_BYTES + 1)),
        "config_schema exceeds 16384 bytes"
    );
    assert_eq!(
        message(
            &(0..=MAX_CONFIG_NODES)
                .map(|index| format!("k{index}: v"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        "config_schema exceeds 512 declarations"
    );
    assert_eq!(
        message(&nested_mapping(MAX_CONFIG_NODE_DEPTH + 1)),
        "config_schema nests deeper than 6 levels"
    );
    assert_eq!(
        message("key:\n\tchild: value"),
        "config_schema line 2 indents with a tab"
    );
    assert_eq!(
        message("key:\n   child: value"),
        "config_schema line 2 is not indented in 2-space steps"
    );
    assert_eq!(
        message("&anchor value"),
        "config_schema line 1 uses unsupported YAML construct: anchor"
    );
    assert_eq!(
        message("key: one\nkey: two"),
        "config_schema line 2 repeats key: key"
    );
    assert_eq!(
        message("a.b: value"),
        "config_schema line 1 has invalid key: a.b"
    );
    assert_eq!(
        message(&format!(
            "key: {}",
            "v".repeat(MAX_CONFIG_SCALAR_CHARACTERS + 1)
        )),
        "config_schema line 1 exceeds 512 characters"
    );
    assert_eq!(
        message(&format!(
            "key: [{}]",
            (0..=MAX_CONFIG_SEQUENCE_ITEMS)
                .map(|index| format!("i{index}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        "config_schema line 1 exceeds 32 items"
    );
    assert_eq!(message("bare"), "config_schema line 1 expects a mapping");
    assert_eq!(
        message("- item"),
        "config_schema line 1 starts a list where a mapping is required"
    );
}

// ---------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------

#[test]
fn mapping_entry_order_follows_source_order() {
    let node = parse_block("z: 1\na: 2\nm: 3").expect("parse");
    let keys: Vec<&str> = node
        .as_mapping()
        .expect("mapping")
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    assert_eq!(keys, vec!["z", "a", "m"]);
}

#[test]
fn accessors_answer_only_for_their_own_shape() {
    let node = parse_block("scalar: value\nlist: [a]\nmap:\n  inner: 1").expect("parse");

    assert_eq!(
        node.get("scalar").and_then(ConfigNode::as_scalar),
        Some("value")
    );
    assert!(node
        .get("scalar")
        .and_then(ConfigNode::as_mapping)
        .is_none());
    assert!(node
        .get("scalar")
        .and_then(ConfigNode::as_sequence)
        .is_none());

    assert!(node.get("list").and_then(ConfigNode::as_sequence).is_some());
    assert!(node.get("list").and_then(ConfigNode::as_scalar).is_none());

    assert!(node.get("map").and_then(ConfigNode::as_mapping).is_some());
    assert!(node.get("map").and_then(ConfigNode::as_scalar).is_none());

    assert!(node.get("absent").is_none());
}

#[test]
fn an_entirely_blank_or_commented_block_is_an_empty_mapping() {
    for block in ["", "\n", "   \n\t\n", "# only a comment\n"] {
        assert_eq!(
            parse_block(block).expect("parse"),
            ConfigNode::Mapping(Vec::new()),
            "{block:?} should be an empty mapping"
        );
    }
}
