use super::{
    AgentMountConfiguration, EffectiveSkill, ManagedSkillSource, OverlayValidationDiagnostic,
    SkillAgentBinding, SkillApplicationError, SkillCompatibleAgent, SkillDocument,
    SkillDriftReport, SkillFilesystemTransaction, SkillImportedSource, SkillLogEvent,
    SkillMountRepair, SkillPackageDescriptor, SkillPackageResource, SkillRecord,
    SkillResourceDocument, SkillSourceRefresh, SkillUsageActivity, SkillUsageIdentity,
    SkillUsageMutation, SkillUsageRead,
};

pub(crate) trait SkillPackageReader: Send + Sync {
    fn read_document(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<SkillDocument, SkillApplicationError>;

    fn list_resources(
        &self,
        _package: &SkillPackageDescriptor,
    ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
        Ok(Vec::new())
    }

    fn read_resource(
        &self,
        package: &SkillPackageDescriptor,
        _relative_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        Err(SkillApplicationError::NotFound(
            package.metadata.id.as_str().to_string(),
        ))
    }

    fn read_resource_bytes(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        self.read_resource(package, relative_path)
            .map(|resource| resource.content.into_bytes())
    }
}

pub(crate) trait SkillPackageMaterializer: Send + Sync {
    fn materialize(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError>;
}

pub(crate) trait SkillReconciliationRepository: Send + Sync {
    fn builtin_reconciliation(
        &self,
        id: &crate::contexts::tooling::skills::domain::SkillId,
    ) -> Result<Option<super::BuiltinReconciliationState>, SkillApplicationError>;
    fn save_builtin_reconciliation(
        &self,
        state: &super::BuiltinReconciliationState,
        record: Option<&SkillRecord>,
        clear_tombstone: bool,
    ) -> Result<(), SkillApplicationError>;
    fn complete_builtin_cleanup(
        &self,
        id: &crate::contexts::tooling::skills::domain::SkillId,
        backup_path: Option<&str>,
        updated_at: &str,
    ) -> Result<(), SkillApplicationError>;
}

pub(crate) trait SkillLegacySourcePort: Send + Sync {
    fn read_legacy_document(
        &self,
        location: &crate::contexts::tooling::skills::domain::SkillLocation,
        id: &crate::contexts::tooling::skills::domain::SkillId,
    ) -> Result<SkillDocument, SkillApplicationError>;
    fn archive_legacy_source(
        &self,
        location: &crate::contexts::tooling::skills::domain::SkillLocation,
        id: &crate::contexts::tooling::skills::domain::SkillId,
        reconciliation_version: u32,
    ) -> Result<Option<String>, SkillApplicationError>;
}

pub(crate) trait SkillLayerProvider: Send + Sync {
    fn inventory(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPackageDescriptor>, SkillApplicationError>;
}

pub(crate) trait EffectiveSkillCatalogPort: Send + Sync {
    fn effective_catalog(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<EffectiveSkill>, SkillApplicationError>;
    fn invalidate(&self, workspace_path: Option<&str>);
}
use crate::contexts::tooling::skills::domain::{
    SkillBindingPlan, SkillDriftInspection, SkillDriftIssue, SkillId, SkillKey, SkillLocation,
    SkillMountPath,
};

pub(crate) trait SkillRepository: Send + Sync {
    fn list(&self, location: &SkillLocation) -> Result<Vec<SkillRecord>, SkillApplicationError>;
    fn get(&self, key: &SkillKey) -> Result<Option<SkillRecord>, SkillApplicationError>;
    fn deleted_builtin_ids(&self) -> Result<Vec<SkillId>, SkillApplicationError>;
    fn agent_mount_configurations(
        &self,
    ) -> Result<Vec<AgentMountConfiguration>, SkillApplicationError>;
    fn is_api_agent(&self, agent_id: &str) -> Result<bool, SkillApplicationError>;
    fn compatible_agents(&self) -> Result<Vec<SkillCompatibleAgent>, SkillApplicationError>;
    fn api_agent_bindings_for_location(
        &self,
        location: &SkillLocation,
    ) -> Result<std::collections::BTreeMap<String, Vec<String>>, SkillApplicationError>;
    fn enabled_skills_bound_to(
        &self,
        agent_id: &str,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError>;
    fn save_skills(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
    ) -> Result<(), SkillApplicationError>;
    fn delete_skill(
        &self,
        key: &SkillKey,
        record_builtin_tombstone: bool,
        deleted_at: &str,
    ) -> Result<(), SkillApplicationError>;
    fn save_mount_path(
        &self,
        agent_id: &str,
        mount_path: &SkillMountPath,
        affected_records: &[SkillRecord],
        updated_at: &str,
    ) -> Result<(), SkillApplicationError>;
    fn save_drift_snapshot(&self, report: &SkillDriftReport) -> Result<(), SkillApplicationError>;
    fn save_synchronization(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
        report: &SkillDriftReport,
    ) -> Result<(), SkillApplicationError>;
}

/// What is actually on disk where a Skill's source would live.
///
/// Seeding needs this because "absent from the registry" and "absent from disk" are different
/// questions, and answering the second with a `Conflict` from `create_source` leaves the caller
/// unable to tell an adoptable source from a genuine failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillSourceProbe {
    /// Nothing is there; the source can be created.
    Absent,
    /// A readable, parseable source is there and can be adopted as-is, described by what the file
    /// says rather than by what the caller expected it to say.
    Present(Box<SkillImportedSource>),
    /// Something is there but cannot be read or parsed as a Skill.
    Unusable(String),
}

pub(crate) trait SkillFilesystemPort: Send + Sync {
    fn begin_mutation(&self) -> Result<SkillFilesystemTransaction, SkillApplicationError>;
    fn commit_mutation(&self, transaction: SkillFilesystemTransaction);
    fn rollback_mutation(&self, transaction: SkillFilesystemTransaction);

    /// Reports what exists at a Skill's source path without modifying it.
    fn probe_source(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<SkillSourceProbe, SkillApplicationError>;

    /// The content hash a document would have if it were written to disk.
    ///
    /// Lets a caller compare an adopted source against the definition that was supposed to be
    /// there, without duplicating the filesystem's rules for serializing and hashing a document.
    fn content_hash_for(&self, document: &SkillDocument) -> String;

    fn create_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError>;
    fn replace_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        document: &SkillDocument,
        expected_content_hash: &str,
    ) -> Result<ManagedSkillSource, SkillApplicationError>;
    fn inspect_import_metadata(
        &self,
        source_path: &str,
    ) -> Result<crate::contexts::tooling::skills::domain::SkillMetadata, SkillApplicationError>;
    fn import_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        source_path: &str,
    ) -> Result<SkillImportedSource, SkillApplicationError>;
    fn remove_skill(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        remove_source: bool,
    ) -> Result<(), SkillApplicationError>;
    fn reconcile_bindings(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        plan: &SkillBindingPlan,
        mount_paths: &[AgentMountConfiguration],
    ) -> Result<Vec<SkillAgentBinding>, SkillApplicationError>;
    fn migrate_binding(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        old_mount_path: &SkillMountPath,
        new_mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError>;
    fn read_source(&self, record: &SkillRecord) -> Result<String, SkillApplicationError>;
    fn observe_bindings(&self, records: &mut [SkillRecord]) -> Result<(), SkillApplicationError>;
    fn inspect_drift(
        &self,
        location: &SkillLocation,
        records: &[SkillRecord],
        deleted_builtin_ids: &[SkillId],
    ) -> Result<SkillDriftInspection, SkillApplicationError>;
    fn repair_binding(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError>;
    fn refresh_source(
        &self,
        record: &SkillRecord,
        issue: &SkillDriftIssue,
    ) -> Result<SkillSourceRefresh, SkillApplicationError>;
}

pub(crate) trait SkillWorkspaceSelectionPort: Send + Sync {
    fn select_workspace_directory(&self) -> Result<Option<String>, SkillApplicationError>;
}

pub(crate) trait SkillClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait SkillLoggingPort: Send + Sync {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError>;

    // The Overlay preparation pipeline is introduced after its security and storage ports.
    #[allow(dead_code)]
    fn record_overlay_validation(
        &self,
        diagnostic: &OverlayValidationDiagnostic,
    ) -> Result<(), SkillApplicationError> {
        self.record(&diagnostic.to_log_event())
    }
}

pub(crate) trait SkillUsageRepository: Send + Sync {
    fn summaries(
        &self,
        location: &crate::contexts::tooling::skills::domain::SkillLocation,
        identities: &[SkillUsageIdentity],
    ) -> Result<SkillUsageRead, SkillApplicationError>;

    fn bump(
        &self,
        location: &crate::contexts::tooling::skills::domain::SkillLocation,
        identity: &SkillUsageIdentity,
        activity: SkillUsageActivity,
        timestamp: &str,
        revision_witness: &str,
    ) -> Result<SkillUsageMutation, SkillApplicationError>;
}

/// Non-mount binding boundary for Skills bound to API-based Agents (`add-agent-skill-support`) —
/// a separate, simpler relationship from `SkillRepository`'s CLI mount-path binding, which this
/// does not read or write. Presence of a binding means "bound"; whether it's active still gates
/// on the Skill's own `enabled` flag, matching `SkillRepository::enabled_skills_bound_to`.
pub(crate) trait SkillApiBindingRepository: Send + Sync {
    fn bind_api_agent(
        &self,
        key: &SkillKey,
        agent_id: &str,
        now: &str,
    ) -> Result<(), SkillApplicationError>;

    fn unbind_api_agent(&self, key: &SkillKey, agent_id: &str)
        -> Result<(), SkillApplicationError>;

    fn api_agent_bindings(&self, key: &SkillKey) -> Result<Vec<String>, SkillApplicationError>;

    fn enabled_skills_bound_to_api_agent(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError>;
}
