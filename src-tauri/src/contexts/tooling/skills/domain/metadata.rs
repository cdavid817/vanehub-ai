use super::{SkillDomainError, SkillId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMetadata {
    pub(crate) id: SkillId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: String,
    pub(crate) version: String,
    pub(crate) triggers: Vec<String>,
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
        let metadata = Self {
            id: SkillId::parse(id)?,
            name: name.into(),
            description: description.into(),
            category: category.into(),
            version: version.into(),
            triggers,
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

        assert_eq!(
            SkillMetadata::new("bad_id", "Name", "Description", "test", "1", Vec::new()),
            Err(SkillDomainError::InvalidId)
        );
        assert_eq!(
            SkillMetadata::new("valid", " ", "Description", "test", "1", Vec::new()),
            Err(SkillDomainError::MissingMetadataFields)
        );
    }
}
