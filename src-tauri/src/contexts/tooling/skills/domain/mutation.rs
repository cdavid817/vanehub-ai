use super::{
    builtin_definition, SkillDomainError, SkillId, SkillLocation, SkillMetadata, SkillScope,
    SkillSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillDeletionPolicy {
    pub(crate) record_builtin_tombstone: bool,
    pub(crate) remove_bindings: bool,
    pub(crate) remove_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillRestorePlan {
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: &'static str,
    pub(crate) location: SkillLocation,
    pub(crate) source: SkillSource,
    pub(crate) enabled: bool,
}

pub(crate) fn validate_create_identity(
    requested_id: &SkillId,
    metadata: &SkillMetadata,
) -> Result<(), SkillDomainError> {
    if requested_id == &metadata.id {
        Ok(())
    } else {
        Err(SkillDomainError::CreateIdMismatch)
    }
}

pub(crate) fn validate_update_identity(
    existing_id: &SkillId,
    metadata: &SkillMetadata,
) -> Result<(), SkillDomainError> {
    if existing_id == &metadata.id {
        Ok(())
    } else {
        Err(SkillDomainError::UpdateIdChanged)
    }
}

pub(crate) fn source_for_user_create(
    requested: Option<SkillSource>,
) -> Result<SkillSource, SkillDomainError> {
    match requested {
        None | Some(SkillSource::User) => Ok(SkillSource::User),
        Some(source) => Err(SkillDomainError::InvalidUserSource(
            source.as_str().to_string(),
        )),
    }
}

pub(crate) fn deletion_policy(source: SkillSource) -> SkillDeletionPolicy {
    SkillDeletionPolicy {
        record_builtin_tombstone: source == SkillSource::Builtin,
        remove_bindings: true,
        remove_source: true,
    }
}

pub(crate) fn builtin_restore_plan(id: &SkillId) -> Result<SkillRestorePlan, SkillDomainError> {
    let definition = builtin_definition(id)
        .ok_or_else(|| SkillDomainError::UnknownBuiltin(id.as_str().to_string()))?;
    Ok(SkillRestorePlan {
        metadata: definition.metadata()?,
        body: definition.body,
        location: SkillLocation::new(SkillScope::Global, None)?,
        source: SkillSource::Builtin,
        enabled: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(id: &str) -> SkillMetadata {
        SkillMetadata::new(id, "Name", "Description", "test", "1.0.0", Vec::new())
            .expect("metadata")
    }

    #[test]
    fn create_and_update_keep_skill_identity_immutable() {
        let id = SkillId::parse("stable-id").expect("id");
        assert_eq!(
            validate_create_identity(&id, &metadata("stable-id")),
            Ok(())
        );
        assert_eq!(
            validate_create_identity(&id, &metadata("changed-id")),
            Err(SkillDomainError::CreateIdMismatch)
        );
        assert_eq!(
            validate_update_identity(&id, &metadata("changed-id")),
            Err(SkillDomainError::UpdateIdChanged)
        );
    }

    #[test]
    fn source_and_deletion_rules_reserve_builtin_and_imported_lifecycles() {
        assert_eq!(source_for_user_create(None), Ok(SkillSource::User));
        assert_eq!(
            source_for_user_create(Some(SkillSource::User)),
            Ok(SkillSource::User)
        );
        assert!(matches!(
            source_for_user_create(Some(SkillSource::Builtin)),
            Err(SkillDomainError::InvalidUserSource(source)) if source == "builtin"
        ));
        assert!(matches!(
            source_for_user_create(Some(SkillSource::Imported)),
            Err(SkillDomainError::InvalidUserSource(source)) if source == "imported"
        ));
        assert!(deletion_policy(SkillSource::Builtin).record_builtin_tombstone);
        assert!(!deletion_policy(SkillSource::User).record_builtin_tombstone);
        assert!(!deletion_policy(SkillSource::Imported).record_builtin_tombstone);
    }

    #[test]
    fn builtin_restore_is_catalog_only_global_enabled_and_builtin() {
        let plan = builtin_restore_plan(&SkillId::parse("code-review").expect("id"))
            .expect("restore plan");
        assert_eq!(plan.metadata.id.as_str(), "code-review");
        assert_eq!(plan.location.scope, SkillScope::Global);
        assert_eq!(plan.location.workspace_path, None);
        assert_eq!(plan.source, SkillSource::Builtin);
        assert!(plan.enabled);

        let error = builtin_restore_plan(&SkillId::parse("unknown-skill").expect("id"))
            .expect_err("unknown builtin");
        assert_eq!(
            error,
            SkillDomainError::UnknownBuiltin("unknown-skill".to_string())
        );
    }
}
