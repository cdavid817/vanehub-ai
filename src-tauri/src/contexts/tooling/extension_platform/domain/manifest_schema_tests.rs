//! Keeps three descriptions of one shape in step.
//!
//! The decoder enforces installation, the field inventory records what is legal, and the shipped
//! JSON Schema tells an editor the same story. None is generated from another — the decoder stays
//! explicit and the schema stays readable — so the only thing stopping them drifting is this file.
//!
//! Drift is not a safety hole: the decoder still refuses what it refuses, and a stale schema costs
//! a publisher a wrong squiggle. It is a correctness and trust problem, which is worth a test and
//! not worth a code generator.

use super::{
    field_set, ExtensionManifestV1Decoder, FieldSet, VersionedExtensionManifest,
    EXTENSION_MANIFEST_YAML_LIMITS, MANIFEST_FIELDS,
};
use semver::Version;
use serde_json::Value;
use vanehub_bounded_yaml::parse_block;

const SCHEMA: &str =
    include_str!("../../../../../resources/extension-platform/vanehub-extension.schema.json");

/// Keywords this project's schemas may use.
///
/// Fail-closed by construction: a keyword outside this list is a rejection, not something ignored.
/// A schema using a keyword the project has not reviewed would describe rules no reader here can
/// evaluate, and shipping one would tell publishers something the decoder does not do.
const ALLOWED_KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "$ref",
    "$defs",
    "title",
    "description",
    "type",
    "const",
    "enum",
    "required",
    "properties",
    "additionalProperties",
    "propertyNames",
    "items",
    "maxItems",
    "minLength",
    "maxLength",
    "pattern",
    "oneOf",
];

/// Combinators whose evaluation cost or semantics this project has decided not to take on.
const REJECTED_KEYWORDS: &[&str] = &[
    "allOf",
    "anyOf",
    "not",
    "if",
    "then",
    "else",
    "dependentSchemas",
    "unevaluatedProperties",
    "patternProperties",
    "$dynamicRef",
];

fn schema() -> Value {
    serde_json::from_str(SCHEMA).expect("the shipped schema must be valid JSON")
}

// ---------------------------------------------------------------------------
// The schema is within the supported subset
// ---------------------------------------------------------------------------

#[test]
fn the_shipped_schema_uses_no_keyword_outside_the_reviewed_set() {
    let mut unexpected: Vec<String> = Vec::new();
    walk_keywords(&schema(), &mut unexpected);

    assert!(
        unexpected.is_empty(),
        "schema uses keywords this project does not evaluate: {unexpected:?}"
    );
}

#[test]
fn a_rejected_combinator_would_be_caught() {
    // The check above only proves the current schema is clean. This proves the check itself has
    // teeth, so a later edit that reaches for `allOf` fails rather than passing quietly.
    let hostile: Value =
        serde_json::from_str(r#"{"type":"object","allOf":[{"type":"string"}]}"#).expect("fixture");
    let mut unexpected = Vec::new();
    walk_keywords(&hostile, &mut unexpected);

    assert_eq!(unexpected, vec!["allOf".to_string()]);
}

fn walk_keywords(value: &Value, unexpected: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                // Inside `properties`, `$defs`, and `propertyNames`, the keys are field or
                // definition names rather than keywords.
                let names_are_data = matches!(key.as_str(), "properties" | "$defs");
                if !names_are_data
                    && !ALLOWED_KEYWORDS.contains(&key.as_str())
                    && (REJECTED_KEYWORDS.contains(&key.as_str()) || is_keyword_position(key))
                {
                    unexpected.push(key.clone());
                }
                if names_are_data {
                    if let Value::Object(members) = child {
                        for member in members.values() {
                            walk_keywords(member, unexpected);
                        }
                    }
                } else {
                    walk_keywords(child, unexpected);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_keywords(item, unexpected);
            }
        }
        _ => {}
    }
}

/// A key is a keyword position unless it sits under `properties`/`$defs`, which `walk_keywords`
/// handles separately. Anything left that is not in the allowlist is unreviewed.
fn is_keyword_position(key: &str) -> bool {
    !ALLOWED_KEYWORDS.contains(&key)
}

// ---------------------------------------------------------------------------
// Schema and inventory agree
// ---------------------------------------------------------------------------

/// Resolves the object schema at an inventory path, following `$ref` and stepping through the
/// `additionalProperties` of a keyed collection where the path says `*`.
fn schema_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = resolve(root, root)?;
    if path.is_empty() {
        return Some(current);
    }
    for segment in path.split('.') {
        current = if segment == "*" {
            resolve(root, current.get("additionalProperties")?)?
        } else {
            resolve(root, current.get("properties")?.get(segment)?)?
        };
    }
    Some(current)
}

fn resolve<'a>(root: &'a Value, value: &'a Value) -> Option<&'a Value> {
    let Some(reference) = value.get("$ref").and_then(Value::as_str) else {
        return Some(value);
    };
    let name = reference.strip_prefix("#/$defs/")?;
    resolve(root, root.get("$defs")?.get(name)?)
}

#[test]
fn every_inventory_path_exists_in_the_schema_with_the_same_fields() {
    let root = schema();

    for set in MANIFEST_FIELDS {
        let node = schema_at(&root, set.path)
            .unwrap_or_else(|| panic!("schema has no object at {:?}", set.path));

        let properties = node
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("schema node at {:?} declares no properties", set.path));

        let mut declared: Vec<&str> = properties.keys().map(String::as_str).collect();
        let mut expected: Vec<&str> = set.all().collect();
        declared.sort_unstable();
        expected.sort_unstable();
        assert_eq!(
            declared, expected,
            "schema and inventory disagree about the fields at {:?}",
            set.path
        );

        let required = node
            .get("required")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut schema_required = required;
        let mut inventory_required: Vec<&str> = set.required.to_vec();
        schema_required.sort_unstable();
        inventory_required.sort_unstable();
        assert_eq!(
            schema_required, inventory_required,
            "schema and inventory disagree about required fields at {:?}",
            set.path
        );
    }
}

#[test]
fn every_object_in_the_schema_refuses_unknown_fields() {
    // `additionalProperties: false` on every record shape, matching the decoder's `finish`. A
    // record that allowed extras would tell a publisher a field is fine while installation
    // refuses it.
    for set in MANIFEST_FIELDS {
        let root = schema();
        let node = schema_at(&root, set.path).expect("path resolves");
        assert_eq!(
            node.get("additionalProperties"),
            Some(&Value::Bool(false)),
            "schema node at {:?} accepts unknown fields",
            set.path
        );
    }
}

// ---------------------------------------------------------------------------
// Decoder and inventory agree
// ---------------------------------------------------------------------------

const MINIMAL: &str = "\
schema_version: 1
id: acme.x
display_name: X
publisher: acme
version: 1.0.0
min_vanehub_version: \">=0.9.0\"
";

fn decodes(text: &str) -> Result<(), String> {
    let document = parse_block(text, EXTENSION_MANIFEST_YAML_LIMITS)
        .map_err(|error| format!("parse: {error:?}"))?;
    ExtensionManifestV1Decoder::new(Version::parse("1.0.0").expect("version"))
        .decode(&document)
        .map(|manifest| {
            let VersionedExtensionManifest::V1(_) = manifest;
        })
        .map_err(|error| error.to_string())
}

#[test]
fn the_decoder_refuses_a_field_the_inventory_does_not_list() {
    // The inventory's other half: `finish` already rejects extras, and this pins that the two
    // agree about which extras those are.
    let root_set = field_set("").expect("root inventory");
    assert!(!root_set.contains("surprise"));

    let error = decodes(&format!("{MINIMAL}surprise: yes\n")).expect_err("should be refused");
    assert!(error.contains("surprise"), "{error}");
}

#[test]
fn the_decoder_accepts_every_optional_root_field_the_inventory_lists() {
    // Catches the reverse drift: a field listed and shipped in the schema that the decoder never
    // learned to read would be refused as unknown, and a publisher would be told to remove
    // something the schema told them to write.
    let cases = [
        ("description", "description: text\n"),
        ("license", "license: MIT\n"),
        ("runtime", "runtime:\n  kind: none\n"),
        ("activation_events", "activation_events:\n  - manual\n"),
        (
            "requires",
            "requires:\n  skills:\n    reviewer:\n      version: \"1\"\n",
        ),
        ("permissions", "permissions:\n  secrets:\n    - a.b\n"),
        (
            "contributes",
            "contributes:\n  skills:\n    s:\n      path: s/SKILL.md\n",
        ),
    ];

    let root_set = field_set("").expect("root inventory");
    for (field, fragment) in cases {
        assert!(
            root_set.contains(field),
            "{field} is exercised here but missing from the inventory"
        );
        decodes(&format!("{MINIMAL}{fragment}"))
            .unwrap_or_else(|error| panic!("{field} should decode: {error}"));
    }

    // Every optional root field is covered by a case above.
    let covered: Vec<&str> = cases.iter().map(|(field, _)| *field).collect();
    let mut missing: Vec<&str> = root_set
        .optional
        .iter()
        .copied()
        .filter(|field| !covered.contains(field))
        .collect();
    missing.sort_unstable();
    assert!(missing.is_empty(), "no case exercises {missing:?}");
}

#[test]
fn the_inventory_has_no_duplicate_paths_and_no_duplicate_fields() {
    let mut paths: Vec<&str> = MANIFEST_FIELDS.iter().map(|set| set.path).collect();
    let total = paths.len();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(paths.len(), total, "an inventory path is listed twice");

    for set in MANIFEST_FIELDS {
        let mut fields: Vec<&str> = set.all().collect();
        let declared = fields.len();
        fields.sort_unstable();
        fields.dedup();
        assert_eq!(
            fields.len(),
            declared,
            "a field is listed twice at {:?}",
            set.path
        );
        for field in set.required {
            assert!(
                !set.optional.contains(field),
                "{field} is both required and optional at {:?}",
                set.path
            );
        }
    }
}

#[test]
fn field_set_lookup_answers_only_for_declared_paths() {
    assert!(field_set("").is_some());
    assert!(field_set("contributes.tools.*").is_some());
    assert!(field_set("contributes.tools").is_none());
    assert!(field_set("nowhere").is_none());
}

#[test]
fn a_field_set_reports_membership_across_required_and_optional() {
    let set: &FieldSet = field_set("contributes.tools.*").expect("inventory");

    assert!(set.contains("display_name"));
    assert!(set.contains("description"));
    assert!(!set.contains("surprise"));
}
