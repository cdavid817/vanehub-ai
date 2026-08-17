use crate::contexts::tooling::skill_tools::application::{
    SkillToolSchemaValidationPort, SkillToolSchemaViolation,
};
use crate::contexts::tooling::skill_tools::domain::BoundedJsonSchema;
use serde_json::{Map, Value};

const MAX_VIOLATIONS: usize = 8;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BoundedSkillToolSchemaValidator;

impl SkillToolSchemaValidationPort for BoundedSkillToolSchemaValidator {
    fn validate_instance(
        &self,
        schema: &BoundedJsonSchema,
        instance: &Value,
    ) -> Result<(), Vec<SkillToolSchemaViolation>> {
        let mut violations = Vec::new();
        validate_node(schema.as_value(), instance, "", &mut violations);
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

fn validate_node(
    schema: &Value,
    instance: &Value,
    pointer: &str,
    violations: &mut Vec<SkillToolSchemaViolation>,
) {
    if violations.len() >= MAX_VIOLATIONS {
        return;
    }
    let Some(schema) = schema.as_object() else {
        return;
    };
    if !matches_type(schema.get("type").and_then(Value::as_str), instance) {
        push(violations, pointer, "type");
        return;
    }
    if schema
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.contains(instance))
    {
        push(violations, pointer, "enum");
    }
    match instance {
        Value::Object(object) => validate_object(schema, object, pointer, violations),
        Value::Array(items) => validate_array(schema, items, pointer, violations),
        Value::String(value) => validate_string(schema, value, pointer, violations),
        Value::Number(value) => validate_number(schema, value.as_f64(), pointer, violations),
        _ => {}
    }
}

fn validate_object(
    schema: &Map<String, Value>,
    object: &Map<String, Value>,
    pointer: &str,
    violations: &mut Vec<SkillToolSchemaViolation>,
) {
    let properties = schema.get("properties").and_then(Value::as_object);
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(name) {
                push(violations, &join(pointer, name), "required");
            }
        }
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
        for name in object.keys() {
            if !properties.is_some_and(|items| items.contains_key(name)) {
                push(violations, &join(pointer, name), "additional-property");
            }
        }
    }
    if let Some(properties) = properties {
        for (name, child_schema) in properties {
            if let Some(value) = object.get(name) {
                validate_node(child_schema, value, &join(pointer, name), violations);
            }
        }
    }
}

fn validate_array(
    schema: &Map<String, Value>,
    items: &[Value],
    pointer: &str,
    violations: &mut Vec<SkillToolSchemaViolation>,
) {
    let length = items.len() as f64;
    if below(schema, "minItems", length) {
        push(violations, pointer, "min-items");
    }
    if above(schema, "maxItems", length) {
        push(violations, pointer, "max-items");
    }
    if let Some(item_schema) = schema.get("items") {
        for (index, item) in items.iter().enumerate() {
            validate_node(item_schema, item, &format!("{pointer}/{index}"), violations);
        }
    }
}

fn validate_string(
    schema: &Map<String, Value>,
    value: &str,
    pointer: &str,
    violations: &mut Vec<SkillToolSchemaViolation>,
) {
    let length = value.chars().count() as f64;
    if below(schema, "minLength", length) {
        push(violations, pointer, "min-length");
    }
    if above(schema, "maxLength", length) {
        push(violations, pointer, "max-length");
    }
}

fn validate_number(
    schema: &Map<String, Value>,
    value: Option<f64>,
    pointer: &str,
    violations: &mut Vec<SkillToolSchemaViolation>,
) {
    let Some(value) = value else { return };
    if below(schema, "minimum", value) {
        push(violations, pointer, "minimum");
    }
    if above(schema, "maximum", value) {
        push(violations, pointer, "maximum");
    }
}

fn matches_type(expected: Option<&str>, value: &Value) -> bool {
    match expected {
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("integer") => value.as_i64().is_some() || value.as_u64().is_some(),
        Some("boolean") => value.is_boolean(),
        _ => false,
    }
}

fn below(schema: &Map<String, Value>, keyword: &str, actual: f64) -> bool {
    schema
        .get(keyword)
        .and_then(Value::as_f64)
        .is_some_and(|minimum| actual < minimum)
}

fn above(schema: &Map<String, Value>, keyword: &str, actual: f64) -> bool {
    schema
        .get(keyword)
        .and_then(Value::as_f64)
        .is_some_and(|maximum| actual > maximum)
}

fn join(pointer: &str, name: &str) -> String {
    format!("{pointer}/{}", name.replace('~', "~0").replace('/', "~1"))
}

fn push(violations: &mut Vec<SkillToolSchemaViolation>, pointer: &str, code: &str) {
    if violations.len() < MAX_VIOLATIONS {
        violations.push(SkillToolSchemaViolation {
            pointer: pointer.chars().take(128).collect(),
            code: code.chars().take(64).collect(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::{
        validate_bounded_schema, DEFAULT_MANIFEST_LIMITS,
    };
    use serde_json::json;

    #[test]
    fn supported_constraints_validate_and_errors_are_bounded() {
        let schema = validate_bounded_schema(
            &json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "minLength": 2, "maxLength": 4},
                    "items": {"type": "array", "maxItems": 1, "items": {"type": "integer"}}
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            &DEFAULT_MANIFEST_LIMITS,
        )
        .expect("schema");
        let validator = BoundedSkillToolSchemaValidator;
        assert!(validator
            .validate_instance(&schema, &json!({"name": "good", "items": [1]}))
            .is_ok());
        let errors = validator
            .validate_instance(
                &schema,
                &json!({"name": "x", "items": [1, 2.5], "unexpected": true}),
            )
            .expect_err("invalid");
        assert!(errors.len() <= MAX_VIOLATIONS);
        assert!(errors.iter().all(|error| error.pointer.len() <= 128));
    }
}
