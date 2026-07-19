use super::{
    PromptHookBindings, PromptHookCategory, PromptHookDomainError, PromptHookId, PromptHookName,
    PromptHookOrder, PromptHookOrderSlot, PromptHookSource, PromptHookStage, PromptHookTemplate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookManifest {
    id: PromptHookId,
    name: PromptHookName,
    category: PromptHookCategory,
    stage: PromptHookStage,
    order: PromptHookOrder,
    template: PromptHookTemplate,
    bindings: PromptHookBindings,
}

impl PromptHookManifest {
    pub(crate) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: PromptHookCategory,
        stage: PromptHookStage,
        order: i64,
        template: impl Into<String>,
        bindings: &[String],
    ) -> Result<Self, PromptHookDomainError> {
        Ok(Self {
            id: PromptHookId::parse(id)?,
            name: PromptHookName::new(name)?,
            category,
            stage,
            order: PromptHookOrder::new(order)?,
            template: PromptHookTemplate::new(template)?,
            bindings: PromptHookBindings::new(bindings)?,
        })
    }

    pub(crate) fn id(&self) -> &PromptHookId {
        &self.id
    }

    pub(crate) fn name(&self) -> &PromptHookName {
        &self.name
    }

    pub(crate) fn category(&self) -> PromptHookCategory {
        self.category
    }

    pub(crate) fn stage(&self) -> PromptHookStage {
        self.stage
    }

    pub(crate) fn order(&self) -> PromptHookOrder {
        self.order
    }

    pub(crate) fn order_slot(&self) -> PromptHookOrderSlot {
        PromptHookOrderSlot::new(self.stage, self.category, self.order)
    }

    pub(crate) fn template(&self) -> &PromptHookTemplate {
        &self.template
    }

    pub(crate) fn bindings(&self) -> &PromptHookBindings {
        &self.bindings
    }

    pub(crate) fn with_bindings(mut self, bindings: PromptHookBindings) -> Self {
        self.bindings = bindings;
        self
    }
}

pub(crate) fn ensure_identity_unchanged(
    current_id: &str,
    requested_id: &str,
) -> Result<(), PromptHookDomainError> {
    if current_id == requested_id {
        Ok(())
    } else {
        Err(PromptHookDomainError::IdentityChanged)
    }
}

pub(crate) fn ensure_content_editable(
    source: PromptHookSource,
) -> Result<(), PromptHookDomainError> {
    match source {
        PromptHookSource::Builtin => Err(PromptHookDomainError::BuiltinContentImmutable),
        PromptHookSource::User => Ok(()),
    }
}

pub(crate) fn ensure_deletable(source: PromptHookSource) -> Result<(), PromptHookDomainError> {
    match source {
        PromptHookSource::Builtin => Err(PromptHookDomainError::BuiltinCannotBeDeleted),
        PromptHookSource::User => Ok(()),
    }
}

pub(crate) fn ensure_enablement(
    disableable: bool,
    requested_enabled: bool,
) -> Result<(), PromptHookDomainError> {
    if !requested_enabled && !disableable {
        Err(PromptHookDomainError::CannotBeDisabled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_immutable_during_update() {
        assert_eq!(ensure_identity_unchanged("hook-a", "hook-a"), Ok(()));
        assert_eq!(
            ensure_identity_unchanged("hook-a", "hook-b"),
            Err(PromptHookDomainError::IdentityChanged)
        );
    }

    #[test]
    fn manifest_constructs_all_user_controlled_invariants_together() {
        let manifest = PromptHookManifest::new(
            "review-focus",
            "  Review Focus  ",
            PromptHookCategory::Dynamic,
            PromptHookStage::PerTurn,
            450,
            "Review for {{agentId}}",
            &["codex-cli".to_string(), "codex-cli".to_string()],
        )
        .expect("manifest");
        assert_eq!(manifest.id().as_str(), "review-focus");
        assert_eq!(manifest.name().as_str(), "Review Focus");
        assert_eq!(manifest.order().value(), 450);
        assert_eq!(manifest.bindings().to_strings(), ["codex-cli"]);
    }

    #[test]
    fn builtin_content_and_deletion_are_immutable_while_user_hooks_are_mutable() {
        assert_eq!(
            ensure_content_editable(PromptHookSource::Builtin),
            Err(PromptHookDomainError::BuiltinContentImmutable)
        );
        assert_eq!(
            ensure_deletable(PromptHookSource::Builtin),
            Err(PromptHookDomainError::BuiltinCannotBeDeleted)
        );
        assert_eq!(ensure_content_editable(PromptHookSource::User), Ok(()));
        assert_eq!(ensure_deletable(PromptHookSource::User), Ok(()));
    }

    #[test]
    fn only_non_disableable_hooks_reject_a_disable_request() {
        assert_eq!(
            ensure_enablement(false, false),
            Err(PromptHookDomainError::CannotBeDisabled)
        );
        assert_eq!(ensure_enablement(false, true), Ok(()));
        assert_eq!(ensure_enablement(true, false), Ok(()));
    }
}
