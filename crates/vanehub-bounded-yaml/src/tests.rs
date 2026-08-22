//! Tests for what this crate owns: the grammar, the limits being genuinely per-call, duplicate
//! detection, and error identity.
//!
//! Consumer-specific behavior is tested by consumers. Skills keeps its own characterization suite
//! against its profile, and the extension manifest will keep one against its.

use super::{parse_block, BoundedYamlError, BoundedYamlLimits, BoundedYamlValue, INDENT_UNIT};

/// Deliberately small so a limit is reachable in a few lines and a test that trips the wrong
/// bound is obvious.
const TIGHT: BoundedYamlLimits = BoundedYamlLimits {
    max_bytes: 256,
    max_depth: 3,
    max_nodes: 8,
    max_key_bytes: 8,
    max_scalar_characters: 16,
    max_sequence_items: 3,
};

const ROOMY: BoundedYamlLimits = BoundedYamlLimits {
    max_bytes: 16 * 1_024,
    max_depth: 12,
    max_nodes: 512,
    max_key_bytes: 64,
    max_scalar_characters: 512,
    max_sequence_items: 32,
};

fn parse(block: &str, limits: BoundedYamlLimits) -> BoundedYamlValue {
    parse_block(block, limits).expect("block should parse")
}

fn error(block: &str, limits: BoundedYamlLimits) -> BoundedYamlError {
    parse_block(block, limits).expect_err("block should be rejected")
}

#[test]
fn the_indent_unit_is_two_spaces() {
    assert_eq!(INDENT_UNIT, 2);
}

#[test]
fn a_mapping_a_sequence_and_a_scalar_round_trip() {
    let node = parse(
        "name: value\nitems:\n  - one\n  - two\nnested:\n  inner: leaf\nflow: [a, b]",
        ROOMY,
    );

    assert_eq!(
        node.get("name").and_then(BoundedYamlValue::as_scalar),
        Some("value")
    );
    assert_eq!(
        node.get("items").and_then(BoundedYamlValue::as_sequence),
        Some(["one".to_string(), "two".to_string()].as_slice())
    );
    assert_eq!(
        node.get("nested")
            .and_then(|nested| nested.get("inner"))
            .and_then(BoundedYamlValue::as_scalar),
        Some("leaf")
    );
    assert_eq!(
        node.get("flow").and_then(BoundedYamlValue::as_sequence),
        Some(["a".to_string(), "b".to_string()].as_slice())
    );
}

#[test]
fn limits_are_per_call_so_one_consumer_cannot_widen_another() {
    // The whole reason limits are a parameter rather than a constant. The same document is
    // accepted under one profile and rejected under the other, with neither profile mutated.
    let block = "a: 1\nb: 2\nc: 3\nd: 4\ne: 5\nf: 6\ng: 7\nh: 8\ni: 9";

    assert_eq!(error(block, TIGHT), BoundedYamlError::TooManyNodes);
    assert_eq!(parse(block, ROOMY).as_mapping().expect("mapping").len(), 9);
}

#[test]
fn each_limit_is_enforced_at_its_own_boundary() {
    // Bytes.
    let at_bytes = format!("k: {}", "v".repeat(TIGHT.max_bytes - "k: ".len()));
    assert_eq!(at_bytes.len(), TIGHT.max_bytes);
    assert!(!matches!(
        parse_block(&at_bytes, TIGHT),
        Err(BoundedYamlError::TooLarge)
    ));
    assert_eq!(
        error(&format!("{at_bytes}v"), TIGHT),
        BoundedYamlError::TooLarge
    );

    // Nodes.
    let at_nodes = (0..TIGHT.max_nodes)
        .map(|index| format!("k{index}: v"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        parse(&at_nodes, TIGHT).as_mapping().expect("mapping").len(),
        TIGHT.max_nodes
    );
    let over_nodes = (0..=TIGHT.max_nodes)
        .map(|index| format!("k{index}: v"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(error(&over_nodes, TIGHT), BoundedYamlError::TooManyNodes);

    // Key bytes.
    let at_key = "k".repeat(TIGHT.max_key_bytes);
    parse(&format!("{at_key}: v"), TIGHT);
    let over_key = "k".repeat(TIGHT.max_key_bytes + 1);
    assert!(matches!(
        error(&format!("{over_key}: v"), TIGHT),
        BoundedYamlError::InvalidKey { .. }
    ));

    // Scalar characters.
    let at_scalar = "v".repeat(TIGHT.max_scalar_characters);
    parse(&format!("k: {at_scalar}"), TIGHT);
    let over_scalar = "v".repeat(TIGHT.max_scalar_characters + 1);
    assert_eq!(
        error(&format!("k: {over_scalar}"), TIGHT),
        BoundedYamlError::ScalarTooLong(1)
    );

    // Sequence items.
    let at_items = (0..TIGHT.max_sequence_items)
        .map(|index| format!("i{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    parse(&format!("k: [{at_items}]"), TIGHT);
    let over_items = (0..=TIGHT.max_sequence_items)
        .map(|index| format!("i{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        error(&format!("k: [{over_items}]"), TIGHT),
        BoundedYamlError::SequenceTooLong(1)
    );
}

#[test]
fn depth_is_bounded_by_the_supplied_profile() {
    fn nested(levels: usize) -> String {
        let mut block = String::new();
        for level in 0..levels {
            block.push_str(&"  ".repeat(level));
            block.push_str(&format!("l{level}:\n"));
        }
        block.push_str(&"  ".repeat(levels));
        block.push_str("leaf: v\n");
        block
    }

    parse(&nested(TIGHT.max_depth), TIGHT);
    assert_eq!(
        error(&nested(TIGHT.max_depth + 1), TIGHT),
        BoundedYamlError::DepthExceeded
    );
    // The same document a tight profile rejects for depth is fine under a roomier one.
    parse(&nested(TIGHT.max_depth + 1), ROOMY);
}

#[test]
fn duplicate_keys_are_rejected_within_a_mapping_but_not_across_siblings() {
    assert_eq!(
        error("k: 1\nk: 2", ROOMY),
        BoundedYamlError::DuplicateKey {
            line: 2,
            key: "k".to_string(),
        }
    );
    parse("a:\n  shared: 1\nb:\n  shared: 2", ROOMY);
}

#[test]
fn every_unbounded_construct_is_rejected_by_name() {
    let cases = [
        ("&anchor v", "anchor"),
        ("*alias", "alias"),
        ("<<: base", "merge key"),
        ("--- ", "document marker"),
        ("... ", "document marker"),
        ("? explicit", "explicit key"),
        ("{inline: map}", "flow mapping"),
        ("k: &anchor", "anchor or alias"),
        ("k: *alias", "anchor or alias"),
        ("k: |", "block scalar"),
        ("k: >", "block scalar"),
        ("k: !!str v", "explicit tag"),
        ("k: {a: b}", "flow mapping"),
        ("k: [a, [b]]", "nested flow collection"),
    ];

    for (block, construct) in cases {
        assert_eq!(
            error(block, ROOMY),
            BoundedYamlError::UnsupportedConstruct {
                line: 1,
                construct: construct.to_string(),
            },
            "{block} should be rejected as {construct}"
        );
    }
}

#[test]
fn indentation_must_be_two_space_steps_and_tabs_are_refused() {
    assert_eq!(
        error("k:\n\tc: v", ROOMY),
        BoundedYamlError::TabIndentation(2)
    );
    assert_eq!(
        error("k:\n   c: v", ROOMY),
        BoundedYamlError::MisalignedIndentation(2)
    );
    assert_eq!(
        error("k:\n    c: v", ROOMY),
        BoundedYamlError::MisalignedIndentation(2)
    );
    // A uniformly indented block dedents to its own first line.
    parse("    k:\n      c: v", ROOMY);
}

#[test]
fn a_non_entry_line_and_a_root_sequence_are_refused() {
    assert_eq!(error("bare", ROOMY), BoundedYamlError::ExpectedMapping(1));
    assert_eq!(
        error("- item", ROOMY),
        BoundedYamlError::UnexpectedSequenceItem(1)
    );
}

#[test]
fn comments_and_quoting_follow_the_documented_subset() {
    let node = parse(
        "colour: \"#ffffff\"\nfragment: https://e.com/a#b\ntrailing: v # gone\ncolon: \"a: b\"",
        ROOMY,
    );

    assert_eq!(
        node.get("colour").and_then(BoundedYamlValue::as_scalar),
        Some("#ffffff")
    );
    assert_eq!(
        node.get("fragment").and_then(BoundedYamlValue::as_scalar),
        Some("https://e.com/a#b")
    );
    assert_eq!(
        node.get("trailing").and_then(BoundedYamlValue::as_scalar),
        Some("v")
    );
    assert_eq!(
        node.get("colon").and_then(BoundedYamlValue::as_scalar),
        Some("a: b")
    );
}

#[test]
fn empty_shapes_are_stable() {
    assert_eq!(parse("", ROOMY), BoundedYamlValue::Mapping(Vec::new()));
    assert_eq!(
        parse("# only a comment", ROOMY),
        BoundedYamlValue::Mapping(Vec::new())
    );
    assert_eq!(
        parse("k:", ROOMY).get("k").cloned(),
        Some(BoundedYamlValue::Mapping(Vec::new()))
    );
    assert_eq!(
        parse("k: []", ROOMY).get("k").cloned(),
        Some(BoundedYamlValue::Sequence(Vec::new()))
    );
}

#[test]
fn mapping_order_follows_source_order() {
    let node = parse("z: 1\na: 2\nm: 3", ROOMY);
    let keys: Vec<&str> = node
        .as_mapping()
        .expect("mapping")
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    assert_eq!(keys, vec!["z", "a", "m"]);
}

#[test]
fn error_codes_are_distinct_and_lines_are_reported_where_they_exist() {
    let all = [
        BoundedYamlError::TooLarge,
        BoundedYamlError::TooManyNodes,
        BoundedYamlError::DepthExceeded,
        BoundedYamlError::TabIndentation(1),
        BoundedYamlError::MisalignedIndentation(1),
        BoundedYamlError::UnsupportedConstruct {
            line: 1,
            construct: "anchor".to_string(),
        },
        BoundedYamlError::DuplicateKey {
            line: 1,
            key: "k".to_string(),
        },
        BoundedYamlError::InvalidKey {
            line: 1,
            key: "k".to_string(),
        },
        BoundedYamlError::ScalarTooLong(1),
        BoundedYamlError::SequenceTooLong(1),
        BoundedYamlError::ExpectedMapping(1),
        BoundedYamlError::UnexpectedSequenceItem(1),
    ];

    let mut codes: Vec<&str> = all.iter().map(BoundedYamlError::code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total, "every variant needs its own code");

    // Whole-document failures have no line to point at; pretending otherwise would send an
    // operator to line 0.
    assert_eq!(BoundedYamlError::TooLarge.line(), None);
    assert_eq!(BoundedYamlError::TooManyNodes.line(), None);
    assert_eq!(BoundedYamlError::DepthExceeded.line(), None);
    assert_eq!(BoundedYamlError::ScalarTooLong(7).line(), Some(7));
}

#[test]
fn an_invalid_key_diagnostic_is_bounded_by_the_key_limit() {
    // A hostile document must not make the rejection itself unbounded. `@` is chosen because it
    // is invalid in a key while triggering none of the unsupported-construct checks, which run
    // first and would otherwise report a different rejection; the length stays under `max_bytes`
    // so the document-size gate does not fire first either.
    let hostile = "@".repeat(TIGHT.max_key_bytes * 8);
    let BoundedYamlError::InvalidKey { key, .. } = error(&format!("{hostile}: v"), TIGHT) else {
        panic!("expected an invalid key");
    };
    assert_eq!(key.chars().count(), TIGHT.max_key_bytes);
}
