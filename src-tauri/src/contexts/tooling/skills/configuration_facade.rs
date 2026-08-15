#![cfg_attr(not(test), allow(dead_code))]

use super::application::{SkillApplicationError, SkillLoggingPort};
use super::domain::{SkillConfigScope, SkillConfigSnapshot};
use super::infrastructure::{
    apply_deletion_retention, reconcile, reset_property, reset_scope, resolve_activation_snapshot,
    resolve_stored_configuration, save, ActivationRefusal, DeletionRetention, ReconciliationPlan,
    ResolvedSkillConfiguration, SecretRecovery, SkillConfigurationError, SkillConfigurationRequest,
    SkillConfigurationSaveResult, SkillConfigurationSecrets, SkillSecretStore,
    SqliteSkillConfigurationRepository,
};
use crate::platform::credentials::OsCredentialStore;
use std::sync::Arc;

/// Bundles the three resources a configuration operation needs. They are grouped here rather than
/// passed separately through every command because the compensation sequence only makes sense
/// when the same repository, credential store, and log sink take part in all of it.
/// The credential store is a type parameter so the facade can be exercised against a fake. It was
/// concrete first, and the dead-code check made the cost visible: nothing could call these methods
/// in a test without writing to the developer's real keychain.
#[derive(Clone)]
pub(crate) struct SkillConfigurationFacade<S: SkillSecretStore = OsCredentialStore> {
    repository: SqliteSkillConfigurationRepository,
    secrets: SkillConfigurationSecrets<S>,
    logging: Arc<dyn SkillLoggingPort>,
}

impl<S: SkillSecretStore> SkillConfigurationFacade<S> {
    pub(crate) fn new(
        repository: SqliteSkillConfigurationRepository,
        credential_store: S,
        logging: Arc<dyn SkillLoggingPort>,
    ) -> Self {
        Self {
            repository,
            secrets: SkillConfigurationSecrets::new(credential_store),
            logging,
        }
    }

    pub(crate) fn read(
        &self,
        schema: &super::domain::SkillConfigSchema,
        skill_id: &str,
        workspace_identity: &str,
    ) -> Result<ResolvedSkillConfiguration, SkillApplicationError> {
        resolve_stored_configuration(&self.repository, schema, skill_id, workspace_identity)
    }

    pub(crate) fn save(
        &self,
        schema: &super::domain::SkillConfigSchema,
        request: &SkillConfigurationRequest,
    ) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
        save(
            &self.repository,
            &self.secrets,
            self.logging.as_ref(),
            schema,
            request,
        )
    }

    pub(crate) fn reset_property(
        &self,
        schema: &super::domain::SkillConfigSchema,
        request: &SkillConfigurationRequest,
        key: &str,
    ) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
        reset_property(
            &self.repository,
            &self.secrets,
            self.logging.as_ref(),
            schema,
            request,
            key,
        )
    }

    pub(crate) fn reset_scope(
        &self,
        schema: &super::domain::SkillConfigSchema,
        skill_id: &str,
        scope: SkillConfigScope,
        workspace_identity: &str,
    ) -> Result<ResolvedSkillConfiguration, SkillConfigurationError> {
        reset_scope(
            &self.repository,
            &self.secrets,
            self.logging.as_ref(),
            schema,
            skill_id,
            scope,
            workspace_identity,
        )
    }

    pub(crate) fn reconcile(
        &self,
        schema: &super::domain::SkillConfigSchema,
        request: &SkillConfigurationRequest,
        plan: &ReconciliationPlan,
    ) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
        reconcile(
            &self.repository,
            &self.secrets,
            self.logging.as_ref(),
            schema,
            request,
            plan,
        )
    }

    pub(crate) fn apply_deletion_retention(
        &self,
        skill_id: &str,
        workspace_identity: &str,
        retention: DeletionRetention,
    ) -> Result<SecretRecovery, SkillConfigurationError> {
        apply_deletion_retention(
            &self.repository,
            &self.secrets,
            self.logging.as_ref(),
            skill_id,
            workspace_identity,
            retention,
        )
    }

    pub(crate) fn activation_snapshot(
        &self,
        schema: &super::domain::SkillConfigSchema,
        skill_id: &str,
        base_revision: &str,
        workspace_identity: &str,
    ) -> Result<Result<SkillConfigSnapshot, ActivationRefusal>, SkillApplicationError> {
        resolve_activation_snapshot(
            &self.repository,
            schema,
            skill_id,
            base_revision,
            workspace_identity,
        )
    }
}

#[cfg(test)]
#[path = "configuration_facade_tests.rs"]
mod tests;
