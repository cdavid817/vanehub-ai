use super::{SkillToolDispatchOutcome, SkillToolSchemaValidationPort};
use crate::contexts::tooling::skill_tools::domain::BoundedJsonSchema;
use serde_json::Value;

pub(crate) struct SkillToolPayloadValidator<'a> {
    schemas: &'a dyn SkillToolSchemaValidationPort,
}

impl<'a> SkillToolPayloadValidator<'a> {
    pub(crate) fn new(schemas: &'a dyn SkillToolSchemaValidationPort) -> Self {
        Self { schemas }
    }

    pub(crate) fn validate_input(
        &self,
        schema: &BoundedJsonSchema,
        value: &Value,
        maximum_bytes: u64,
    ) -> Result<(), SkillToolDispatchOutcome> {
        self.validate(schema, value, maximum_bytes, "input")
    }

    pub(crate) fn validate_output(
        &self,
        schema: &BoundedJsonSchema,
        value: &Value,
        maximum_bytes: u64,
    ) -> Result<u64, SkillToolDispatchOutcome> {
        self.validate(schema, value, maximum_bytes, "output")?;
        Ok(encoded_len(value))
    }

    fn validate(
        &self,
        schema: &BoundedJsonSchema,
        value: &Value,
        maximum_bytes: u64,
        phase: &str,
    ) -> Result<(), SkillToolDispatchOutcome> {
        if encoded_len(value) > maximum_bytes {
            return Err(failed(&format!("{phase}-too-large")));
        }
        self.schemas
            .validate_instance(schema, value)
            .map_err(|_| failed(&format!("invalid-{phase}")))
    }
}

fn encoded_len(value: &Value) -> u64 {
    u64::try_from(value.to_string().len()).unwrap_or(u64::MAX)
}

fn failed(code: &str) -> SkillToolDispatchOutcome {
    SkillToolDispatchOutcome::Failed {
        code: code.chars().take(64).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::application::SkillToolSchemaViolation;
    use crate::contexts::tooling::skill_tools::domain::{
        validate_bounded_schema, DEFAULT_MANIFEST_LIMITS,
    };
    use serde_json::json;

    struct Schemas;

    impl SkillToolSchemaValidationPort for Schemas {
        fn validate_instance(
            &self,
            _schema: &BoundedJsonSchema,
            instance: &Value,
        ) -> Result<(), Vec<SkillToolSchemaViolation>> {
            instance
                .get("ok")
                .and_then(Value::as_bool)
                .is_some_and(|value| value)
                .then_some(())
                .ok_or_else(|| {
                    vec![SkillToolSchemaViolation {
                        pointer: "/secret/unbounded/path".to_string(),
                        code: "fixture".to_string(),
                    }]
                })
        }
    }

    fn schema() -> BoundedJsonSchema {
        validate_bounded_schema(
            &json!({"type": "object", "properties": {"ok": {"type": "boolean"}}}),
            &DEFAULT_MANIFEST_LIMITS,
        )
        .expect("schema")
    }

    #[test]
    fn validation_errors_are_fixed_bounded_codes_without_payload_details() {
        let validator = SkillToolPayloadValidator::new(&Schemas);
        assert_eq!(
            validator.validate_input(&schema(), &json!({"ok": false}), 1024),
            Err(SkillToolDispatchOutcome::Failed {
                code: "invalid-input".to_string()
            })
        );
        assert_eq!(
            validator.validate_input(&schema(), &json!({"ok": true}), 4),
            Err(SkillToolDispatchOutcome::Failed {
                code: "input-too-large".to_string()
            })
        );
        assert_eq!(
            validator.validate_output(&schema(), &json!({"ok": true}), 4),
            Err(SkillToolDispatchOutcome::Failed {
                code: "output-too-large".to_string()
            })
        );
    }
}
