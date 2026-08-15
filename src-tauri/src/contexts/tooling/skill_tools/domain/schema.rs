use super::{BoundedSchemaError, SkillToolManifestLimits};
use serde_json::Value;

/// Keywords a Skill tool schema may use. Everything outside this set is rejected by name rather
/// than ignored, so a manifest author never believes an unsupported constraint is enforced.
const ALLOWED_KEYWORDS: &[&str] = &[
    "type",
    "title",
    "description",
    "properties",
    "required",
    "items",
    "enum",
    "additionalProperties",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
];

/// Constructs that make validation cost unbounded or recursive. Named separately from the
/// generic unsupported-keyword path because rejecting `$ref` is a security decision, not a
/// feature gap, and the diagnostic should say so.
const REJECTED_COMBINATORS: &[&str] = &[
    "$ref",
    "$defs",
    "definitions",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
    "if",
    "then",
    "else",
    "patternProperties",
    "dependentSchemas",
    "dependentRequired",
    "unevaluatedProperties",
    "unevaluatedItems",
    "propertyNames",
    "contains",
    "pattern",
];

const ALLOWED_TYPES: &[&str] = &["object", "array", "string", "number", "integer", "boolean"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundedSchemaMetrics {
    pub(crate) depth: usize,
    pub(crate) nodes: usize,
}

/// A JSON Schema that has been proven to sit inside the supported subset.
///
/// Holds the original value so later sections can compile a validator from it without re-deriving
/// what was already checked; it is constructible only through `validate_bounded_schema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundedJsonSchema {
    value: Value,
    metrics: BoundedSchemaMetrics,
}

impl BoundedJsonSchema {
    pub(crate) fn as_value(&self) -> &Value {
        &self.value
    }

    pub(crate) fn metrics(&self) -> &BoundedSchemaMetrics {
        &self.metrics
    }
}

/// Validates a declared input or output schema against the supported subset.
///
/// The root must be an object schema: a tool call's arguments are always a named-field structure,
/// and forcing the same shape on results keeps result projection and redaction field-addressable.
pub(crate) fn validate_bounded_schema(
    value: &Value,
    limits: &SkillToolManifestLimits,
) -> Result<BoundedJsonSchema, BoundedSchemaError> {
    let encoded = value.to_string();
    if encoded.len() > limits.maximum_schema_bytes {
        return Err(BoundedSchemaError::TooLarge {
            maximum: limits.maximum_schema_bytes,
            actual: encoded.len(),
        });
    }
    let Some(object) = value.as_object() else {
        return Err(BoundedSchemaError::NotAnObject);
    };
    if object.get("type").and_then(Value::as_str) != Some("object") {
        return Err(BoundedSchemaError::RootMustBeObjectType);
    }

    let mut nodes = 0_usize;
    let depth = walk(value, 1, limits, &mut nodes)?;
    Ok(BoundedJsonSchema {
        value: value.clone(),
        metrics: BoundedSchemaMetrics { depth, nodes },
    })
}

fn walk(
    value: &Value,
    depth: usize,
    limits: &SkillToolManifestLimits,
    nodes: &mut usize,
) -> Result<usize, BoundedSchemaError> {
    if depth > limits.maximum_schema_depth {
        return Err(BoundedSchemaError::TooDeep {
            maximum: limits.maximum_schema_depth,
            actual: depth,
        });
    }
    *nodes += 1;
    if *nodes > limits.maximum_schema_nodes {
        return Err(BoundedSchemaError::TooManyNodes {
            maximum: limits.maximum_schema_nodes,
            actual: *nodes,
        });
    }

    let Some(object) = value.as_object() else {
        return Err(BoundedSchemaError::NotAnObject);
    };
    for keyword in object.keys() {
        if REJECTED_COMBINATORS.contains(&keyword.as_str()) {
            return Err(BoundedSchemaError::UnsupportedCombinator(keyword.clone()));
        }
        if !ALLOWED_KEYWORDS.contains(&keyword.as_str()) {
            return Err(BoundedSchemaError::UnsupportedKeyword(keyword.clone()));
        }
    }

    let declared_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or(BoundedSchemaError::MissingType)?;
    if !ALLOWED_TYPES.contains(&declared_type) {
        return Err(BoundedSchemaError::UnsupportedType(
            declared_type.to_string(),
        ));
    }

    if let Some(values) = object.get("enum") {
        let values = values
            .as_array()
            .ok_or_else(|| BoundedSchemaError::UnsupportedKeyword("enum".to_string()))?;
        if values.len() > limits.maximum_schema_enum_values {
            return Err(BoundedSchemaError::TooManyEnumValues {
                maximum: limits.maximum_schema_enum_values,
                actual: values.len(),
            });
        }
    }

    let mut deepest = depth;
    match declared_type {
        "object" => {
            let properties = object
                .get("properties")
                .map(|properties| {
                    properties
                        .as_object()
                        .ok_or_else(|| BoundedSchemaError::UnsupportedKeyword("properties".into()))
                })
                .transpose()?;
            if let Some(properties) = properties {
                if properties.len() > limits.maximum_schema_properties {
                    return Err(BoundedSchemaError::TooManyProperties {
                        maximum: limits.maximum_schema_properties,
                        actual: properties.len(),
                    });
                }
                for property in properties.values() {
                    deepest = deepest.max(walk(property, depth + 1, limits, nodes)?);
                }
            }
            if let Some(required) = object.get("required") {
                let required = required
                    .as_array()
                    .ok_or(BoundedSchemaError::RequiredMustBeStrings)?;
                for entry in required {
                    let name = entry
                        .as_str()
                        .ok_or(BoundedSchemaError::RequiredMustBeStrings)?;
                    let declared =
                        properties.is_some_and(|properties| properties.contains_key(name));
                    if !declared {
                        return Err(BoundedSchemaError::RequiredPropertyNotDeclared(
                            name.to_string(),
                        ));
                    }
                }
            }
        }
        "array" => {
            let items = object
                .get("items")
                .ok_or(BoundedSchemaError::MissingItems)?;
            deepest = deepest.max(walk(items, depth + 1, limits, nodes)?);
        }
        _ => {}
    }
    Ok(deepest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::DEFAULT_MANIFEST_LIMITS;
    use serde_json::json;

    fn validate(value: &Value) -> Result<BoundedJsonSchema, BoundedSchemaError> {
        validate_bounded_schema(value, &DEFAULT_MANIFEST_LIMITS)
    }

    #[test]
    fn a_supported_object_schema_reports_its_depth_and_node_count() {
        let schema = validate(&json!({
            "type": "object",
            "description": "Diff request",
            "properties": {
                "path": { "type": "string", "maxLength": 512 },
                "modes": { "type": "array", "items": { "type": "string", "enum": ["fast", "full"] } }
            },
            "required": ["path"],
            "additionalProperties": false
        }))
        .expect("supported schema");
        assert_eq!(
            schema.metrics(),
            &BoundedSchemaMetrics { depth: 3, nodes: 4 }
        );
        assert_eq!(schema.as_value()["type"], json!("object"));
    }

    #[test]
    fn recursive_and_combinator_constructs_are_rejected_by_name() {
        for (schema, keyword) in [
            (json!({ "type": "object", "$ref": "#/$defs/self" }), "$ref"),
            (
                json!({ "type": "object", "allOf": [{ "type": "object" }] }),
                "allOf",
            ),
            (
                json!({ "type": "object", "properties": { "a": { "type": "string", "pattern": ".*" } } }),
                "pattern",
            ),
            (
                json!({ "type": "object", "patternProperties": {} }),
                "patternProperties",
            ),
        ] {
            assert_eq!(
                validate(&schema),
                Err(BoundedSchemaError::UnsupportedCombinator(
                    keyword.to_string()
                ))
            );
        }
        assert_eq!(
            validate(&json!({ "type": "object", "nullable": true })),
            Err(BoundedSchemaError::UnsupportedKeyword(
                "nullable".to_string()
            ))
        );
    }

    #[test]
    fn structural_requirements_fail_closed() {
        assert_eq!(validate(&json!([])), Err(BoundedSchemaError::NotAnObject));
        assert_eq!(
            validate(&json!({ "type": "string" })),
            Err(BoundedSchemaError::RootMustBeObjectType)
        );
        assert_eq!(
            validate(&json!({ "type": "object", "properties": { "a": { "description": "x" } } })),
            Err(BoundedSchemaError::MissingType)
        );
        assert_eq!(
            validate(&json!({ "type": "object", "properties": { "a": { "type": "null" } } })),
            Err(BoundedSchemaError::UnsupportedType("null".to_string()))
        );
        assert_eq!(
            validate(&json!({ "type": "object", "properties": { "a": { "type": "array" } } })),
            Err(BoundedSchemaError::MissingItems)
        );
        assert_eq!(
            validate(&json!({ "type": "object", "required": ["missing"] })),
            Err(BoundedSchemaError::RequiredPropertyNotDeclared(
                "missing".to_string()
            ))
        );
        assert_eq!(
            validate(&json!({ "type": "object", "required": [7] })),
            Err(BoundedSchemaError::RequiredMustBeStrings)
        );
    }

    #[test]
    fn depth_node_property_enum_and_size_budgets_are_enforced() {
        let mut deep = json!({ "type": "string" });
        for _ in 0..DEFAULT_MANIFEST_LIMITS.maximum_schema_depth {
            deep = json!({ "type": "object", "properties": { "next": deep } });
        }
        assert!(matches!(
            validate(&deep),
            Err(BoundedSchemaError::TooDeep { .. })
        ));

        let mut wide = serde_json::Map::new();
        for index in 0..=DEFAULT_MANIFEST_LIMITS.maximum_schema_properties {
            wide.insert(format!("field{index}"), json!({ "type": "string" }));
        }
        assert!(matches!(
            validate(&json!({ "type": "object", "properties": wide })),
            Err(BoundedSchemaError::TooManyProperties { .. })
        ));

        let enum_values = (0..=DEFAULT_MANIFEST_LIMITS.maximum_schema_enum_values)
            .map(|index| json!(format!("value{index}")))
            .collect::<Vec<_>>();
        assert!(matches!(
            validate(
                &json!({ "type": "object", "properties": { "a": { "type": "string", "enum": enum_values } } })
            ),
            Err(BoundedSchemaError::TooManyEnumValues { .. })
        ));

        let oversized = json!({
            "type": "object",
            "description": "x".repeat(DEFAULT_MANIFEST_LIMITS.maximum_schema_bytes)
        });
        assert!(matches!(
            validate(&oversized),
            Err(BoundedSchemaError::TooLarge { .. })
        ));

        let tiny_budget = SkillToolManifestLimits {
            maximum_schema_nodes: 2,
            ..DEFAULT_MANIFEST_LIMITS
        };
        let three_nodes = json!({
            "type": "object",
            "properties": { "a": { "type": "string" }, "b": { "type": "string" } }
        });
        assert!(matches!(
            validate_bounded_schema(&three_nodes, &tiny_budget),
            Err(BoundedSchemaError::TooManyNodes { maximum: 2, .. })
        ));
    }
}
