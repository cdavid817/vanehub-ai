use super::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::domain::{
    DeclarativeFieldSource, DeclarativeImplementation, SkillToolCapability, SkillToolDomainError,
};
use serde_json::{Map, Value};

pub(crate) trait SkillToolTargetCatalogPort: Send + Sync {
    fn contains_operation(&self, operation: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedDeclarativeTemplate {
    implementation: DeclarativeImplementation,
}

impl ValidatedDeclarativeTemplate {
    pub(crate) fn target_operation(&self) -> &str {
        self.implementation.target.operation()
    }

    pub(crate) fn target(&self) -> &SkillToolCapability {
        &self.implementation.target
    }

    pub(crate) fn project(&self, input: &Value) -> Result<Value, SkillToolApplicationError> {
        let input = input.as_object().ok_or_else(invalid_input)?;
        let mut projected = Map::new();
        for field in &self.implementation.template {
            let value = match &field.source {
                DeclarativeFieldSource::Input(property) => {
                    input.get(property).cloned().ok_or_else(invalid_input)?
                }
                DeclarativeFieldSource::Constant(value) => value.clone(),
            };
            projected.insert(field.name.clone(), value);
        }
        Ok(Value::Object(projected))
    }
}

pub(crate) struct SkillToolDeclarativeValidator<'a> {
    targets: &'a dyn SkillToolTargetCatalogPort,
}

impl<'a> SkillToolDeclarativeValidator<'a> {
    pub(crate) fn new(targets: &'a dyn SkillToolTargetCatalogPort) -> Self {
        Self { targets }
    }

    pub(crate) fn validate(
        &self,
        implementation: &DeclarativeImplementation,
    ) -> Result<ValidatedDeclarativeTemplate, SkillToolApplicationError> {
        let operation = implementation.target.operation();
        if !self.targets.contains_operation(operation) {
            return Err(SkillToolDomainError::UnknownCapability(
                implementation.target.as_declaration(),
            )
            .into());
        }
        Ok(ValidatedDeclarativeTemplate {
            implementation: implementation.clone(),
        })
    }
}

fn invalid_input() -> SkillToolApplicationError {
    SkillToolDomainError::InvalidTemplate("input-projection".to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::{
        parse_manifest_bytes, SkillToolImplementation, DEFAULT_MANIFEST_LIMITS,
    };
    use serde_json::json;

    const MANIFEST: &[u8] =
        include_bytes!("../../../../../tests/fixtures/skill-tools/valid-declarative.json");

    struct Catalog(&'static [&'static str]);

    impl SkillToolTargetCatalogPort for Catalog {
        fn contains_operation(&self, operation: &str) -> bool {
            self.0.contains(&operation)
        }
    }

    fn implementation() -> DeclarativeImplementation {
        let manifest = parse_manifest_bytes(MANIFEST, &DEFAULT_MANIFEST_LIMITS).expect("manifest");
        let SkillToolImplementation::Declarative(implementation) =
            manifest.tools[0].implementation.clone()
        else {
            panic!("declarative fixture")
        };
        implementation
    }

    #[test]
    fn registered_target_projects_only_declared_input_and_constants() {
        let validator = SkillToolDeclarativeValidator::new(&Catalog(&["read_file"]));
        let validated = validator.validate(&implementation()).expect("validated");
        assert_eq!(validated.target_operation(), "read_file");
        assert_eq!(
            validated
                .project(&json!({"path": "src/main.rs", "ignored": "value"}))
                .expect("projection"),
            json!({"encoding": "utf-8", "path": "src/main.rs"})
        );
    }

    #[test]
    fn unknown_target_and_missing_projection_fail_closed() {
        let unknown = SkillToolDeclarativeValidator::new(&Catalog(&[]));
        assert!(matches!(
            unknown.validate(&implementation()),
            Err(SkillToolApplicationError::Domain(
                SkillToolDomainError::UnknownCapability(_)
            ))
        ));

        let known = SkillToolDeclarativeValidator::new(&Catalog(&["read_file"]));
        let validated = known.validate(&implementation()).expect("validated");
        assert!(validated.project(&json!({})).is_err());
        assert!(validated.project(&json!(["not", "an", "object"])).is_err());
    }
}
