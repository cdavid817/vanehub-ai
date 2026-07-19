use super::{
    AgentMountConfiguration, ManagedSkillSource, SkillAgentBinding, SkillApplicationError,
    SkillDocument, SkillDriftReport, SkillFilesystemTransaction, SkillImportedSource,
    SkillLogEvent, SkillMountRepair, SkillRecord, SkillSourceRefresh,
};
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

pub(crate) trait SkillFilesystemPort: Send + Sync {
    fn begin_mutation(&self) -> Result<SkillFilesystemTransaction, SkillApplicationError>;
    fn commit_mutation(&self, transaction: SkillFilesystemTransaction);
    fn rollback_mutation(&self, transaction: SkillFilesystemTransaction);

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
    ) -> Result<ManagedSkillSource, SkillApplicationError>;
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
}
