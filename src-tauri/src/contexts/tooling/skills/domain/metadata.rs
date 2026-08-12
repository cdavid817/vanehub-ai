use super::{SkillCompatibilityDefaults, SkillDelivery, SkillDomainError, SkillId, SkillType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMetadata {
    pub(crate) id: SkillId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: String,
    pub(crate) version: String,
    pub(crate) triggers: Vec<String>,
    pub(crate) aliases: Vec<SkillId>,
    pub(crate) skill_type: SkillType,
    pub(crate) delivery: SkillDelivery,
    pub(crate) compatibility_defaults: SkillCompatibilityDefaults,
}

impl SkillMetadata {
    pub(crate) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
        version: impl Into<String>,
        triggers: Vec<String>,
    ) -> Result<Self, SkillDomainError> {
        Self::with_classification(
            id,
            name,
            description,
            category,
            version,
            triggers,
            Vec::new(),
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn with_classification(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
        version: impl Into<String>,
        triggers: Vec<String>,
        aliases: Vec<String>,
        skill_type: Option<SkillType>,
        delivery: Option<SkillDelivery>,
    ) -> Result<Self, SkillDomainError> {
        let id = SkillId::parse(id)?;
        let aliases = aliases
            .into_iter()
            .map(SkillId::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if aliases.iter().any(|alias| alias == &id) {
            return Err(SkillDomainError::InvalidMetadataValue(
                "Skill aliases must not repeat the canonical id".to_string(),
            ));
        }
        let mut unique_aliases = aliases.clone();
        unique_aliases.sort();
        unique_aliases.dedup();
        if unique_aliases.len() != aliases.len() {
            return Err(SkillDomainError::InvalidMetadataValue(
                "Skill aliases must be unique".to_string(),
            ));
        }
        let compatibility_defaults = SkillCompatibilityDefaults {
            skill_type: skill_type.is_none(),
            delivery: delivery.is_none(),
        };
        let metadata = Self {
            id,
            name: name.into(),
            description: description.into(),
            category: category.into(),
            version: version.into(),
            triggers,
            aliases,
            skill_type: skill_type.unwrap_or(SkillType::Role),
            delivery: delivery.unwrap_or(SkillDelivery::Eager),
            compatibility_defaults,
        };
        if metadata.name.trim().is_empty()
            || metadata.description.trim().is_empty()
            || metadata.category.trim().is_empty()
            || metadata.version.trim().is_empty()
        {
            return Err(SkillDomainError::MissingMetadataFields);
        }
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_constructor_enforces_id_and_required_frontmatter_fields() {
        let metadata = SkillMetadata::new(
            "sample-skill",
            "Sample",
            "Description",
            "testing",
            "1.0.0",
            vec!["sample".to_string()],
        )
        .expect("metadata");
        assert_eq!(metadata.id.as_str(), "sample-skill");
        assert_eq!(metadata.triggers, vec!["sample"]);
        assert_eq!(metadata.skill_type, SkillType::Role);
        assert_eq!(metadata.delivery, SkillDelivery::Eager);
        assert!(metadata.compatibility_defaults.skill_type);
        assert!(metadata.compatibility_defaults.delivery);

        assert_eq!(
            SkillMetadata::new("bad_id", "Name", "Description", "test", "1", Vec::new()),
            Err(SkillDomainError::InvalidId)
        );
        assert_eq!(
            SkillMetadata::new("valid", " ", "Description", "test", "1", Vec::new()),
            Err(SkillDomainError::MissingMetadataFields)
        );
    }

    #[test]
    fn explicit_classification_and_aliases_do_not_use_compatibility_defaults() {
        let metadata = SkillMetadata::with_classification(
            "developer",
            "Developer",
            "Description",
            "development",
            "1.0.0",
            Vec::new(),
            vec!["dev".to_string()],
            Some(SkillType::Role),
            Some(SkillDelivery::OnDemand),
        )
        .expect("metadata");
        assert_eq!(metadata.aliases[0].as_str(), "dev");
        assert_eq!(metadata.delivery, SkillDelivery::OnDemand);
        assert_eq!(
            metadata.compatibility_defaults,
            SkillCompatibilityDefaults::default()
        );
    }
}
