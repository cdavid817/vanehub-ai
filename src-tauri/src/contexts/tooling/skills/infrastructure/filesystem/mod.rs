mod document;
mod overlay_history;
mod overlay_import;
mod overlay_layout;
mod overlay_manifest;
mod overlay_payload;
mod overlay_transaction;
mod paths;
mod provider;
mod transaction;
mod usage;

#[cfg(test)]
mod overlay_history_tests;
#[cfg(test)]
mod overlay_layout_tests;
#[cfg(test)]
mod overlay_manifest_tests;
#[cfg(test)]
mod overlay_payload_tests;
#[cfg(test)]
mod overlay_transaction_tests;

pub(crate) use document::{
    compose as compose_document, content_hash as document_content_hash, parse_document,
    read_import_document as read_bounded_document,
};
pub(super) use paths::default_home_root;

pub(crate) use overlay_history::FilesystemOverlayHistoryRepository;
pub(crate) use overlay_import::FilesystemOverlayImportParser;
pub(crate) use overlay_manifest::FilesystemOverlayManifestRepository;
pub(crate) use overlay_payload::OverlayPayloadStore;
pub(crate) use overlay_transaction::FilesystemOverlayTransactionExecutor;
pub(crate) use provider::{EmptyRegistrySkillProvider, FilesystemSkillLayerProvider};
pub(crate) use usage::FilesystemSkillUsageRepository;

use self::paths::{SkillPathResolver, SKILL_FILE};
use self::transaction::{path_exists, FileTransactions};
use crate::contexts::tooling::skills::application::{
    AgentMountConfiguration, ManagedSkillSource, SkillAgentBinding, SkillApplicationError,
    SkillBackupEntry, SkillDocument, SkillFilesystemPort, SkillFilesystemTransaction,
    SkillImportedSource, SkillLegacySourcePort, SkillMountRepair, SkillRecord, SkillSourceProbe,
    SkillSourceRefresh,
};
use crate::contexts::tooling::skills::domain::{
    SkillBindingInspection, SkillBindingPlan, SkillDriftInspection, SkillDriftIssue, SkillId,
    SkillLocation, SkillMetadata, SkillMountObservation, SkillMountPath, SkillSource,
    SkillSourceInspection, UnregisteredSkillInspection,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct ManagedSkillFilesystem {
    paths: SkillPathResolver,
    transactions: Arc<FileTransactions>,
}

impl ManagedSkillFilesystem {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(super) fn with_home_root(home_root: PathBuf) -> Self {
        Self {
            paths: SkillPathResolver::with_home_root(home_root),
            transactions: Arc::new(FileTransactions::default()),
        }
    }

    /// Reads what is already at a Skill's source path. Deliberately read-only: seeding uses it to
    /// decide between creating and adopting, and adopting must leave a user's file untouched.
    fn probe_document(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<SkillSourceProbe, SkillApplicationError> {
        let (directory, skill_file) = self.paths.source_paths(location, id)?;
        if !path_exists(&directory) {
            return Ok(SkillSourceProbe::Absent);
        }
        let content = match std::fs::read_to_string(&skill_file) {
            Ok(content) => content,
            Err(error) => {
                return Ok(SkillSourceProbe::Unusable(format!(
                    "source directory exists but its SKILL.md could not be read: {error}"
                )))
            }
        };
        match document::parse(&content) {
            Ok(metadata) => Ok(SkillSourceProbe::Present(Box::new(SkillImportedSource {
                metadata,
                source: managed_source(directory, skill_file, &content),
            }))),
            Err(error) => Ok(SkillSourceProbe::Unusable(format!(
                "source directory exists but its SKILL.md could not be parsed: {error}"
            ))),
        }
    }

    fn create_document(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let (directory, skill_file) = self.paths.source_paths(location, id)?;
        if path_exists(&directory) {
            return Err(SkillApplicationError::Conflict(id.as_str().to_string()));
        }
        if let Some(parent) = directory.parent() {
            std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        }
        self.transactions
            .stage_replace_or_create(transaction, &directory)?;
        std::fs::create_dir_all(&directory).map_err(filesystem_error)?;
        let content = document::compose(document);
        std::fs::write(&skill_file, &content).map_err(filesystem_error)?;
        Ok(managed_source(directory, skill_file, &content))
    }

    fn replace_document(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        document: &SkillDocument,
        expected_content_hash: &str,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let (directory, skill_file) = self
            .paths
            .source_paths(&record.key.location, &record.key.id)?;
        let current = std::fs::read_to_string(&skill_file).map_err(filesystem_error)?;
        if document::content_hash(&current) != expected_content_hash {
            return Err(SkillApplicationError::ConcurrentModification(
                record.key.id.as_str().to_string(),
            ));
        }
        let content = document::compose(document);
        self.transactions
            .stage_replace_or_create(transaction, &skill_file)?;
        std::fs::write(&skill_file, &content).map_err(filesystem_error)?;
        Ok(managed_source(directory, skill_file, &content))
    }

    fn record_source_paths(
        &self,
        record: &SkillRecord,
    ) -> Result<(PathBuf, PathBuf), SkillApplicationError> {
        let persisted_dir = PathBuf::from(&record.managed_source.skill_dir);
        let persisted_file = PathBuf::from(&record.managed_source.skill_md_path);
        if record.resolved_metadata.is_some() && persisted_file.is_file() {
            let global = SkillLocation::new(
                crate::contexts::tooling::skills::domain::SkillScope::Global,
                None,
            )?;
            let cache_candidate = self
                .paths
                .scope_root(&global)?
                .join(".vanehub")
                .join("cache")
                .join("skills")
                .join("effective");
            if cache_candidate.is_dir() {
                let directory = persisted_dir.canonicalize().map_err(filesystem_error)?;
                let skill_file = persisted_file.canonicalize().map_err(filesystem_error)?;
                let cache_root = cache_candidate.canonicalize().map_err(filesystem_error)?;
                let expected_parent = cache_root.join(record.key.id.as_str());
                if path_is_descendant(&directory, &cache_root)
                    && directory.parent() == Some(expected_parent.as_path())
                    && skill_file == directory.join(SKILL_FILE)
                    && directory.file_name().and_then(|value| value.to_str())
                        == Some(record.managed_source.content_hash.as_str())
                {
                    return Ok((directory, skill_file));
                }
                if path_is_descendant(&directory, &cache_root) {
                    return Err(SkillApplicationError::Filesystem(
                        "Effective Skill cache source failed boundary validation".to_string(),
                    ));
                }
            }
        }
        if let Some(effective) = &record.resolved_metadata {
            if matches!(
                effective.layer,
                crate::contexts::tooling::skills::domain::SkillLayer::User
                    | crate::contexts::tooling::skills::domain::SkillLayer::Project
            ) && persisted_file.is_file()
            {
                let directory = persisted_dir.canonicalize().map_err(filesystem_error)?;
                let skill_file = persisted_file.canonicalize().map_err(filesystem_error)?;
                let boundary_location = if effective.layer
                    == crate::contexts::tooling::skills::domain::SkillLayer::User
                {
                    SkillLocation::new(
                        crate::contexts::tooling::skills::domain::SkillScope::Global,
                        None,
                    )?
                } else {
                    record.key.location.clone()
                };
                let root = self.paths.source_root(&boundary_location)?;
                if path_is_descendant(&directory, &root) && skill_file == directory.join(SKILL_FILE)
                {
                    return Ok((directory, skill_file));
                }
                return Err(SkillApplicationError::Filesystem(
                    "Effective Skill source failed boundary validation".to_string(),
                ));
            }
        }
        if record.source == SkillSource::Builtin && persisted_file.is_file() {
            let global = SkillLocation::new(
                crate::contexts::tooling::skills::domain::SkillScope::Global,
                None,
            )?;
            let cache_candidate = self
                .paths
                .scope_root(&global)?
                .join(".vanehub")
                .join("cache")
                .join("skills")
                .join("system");
            if !cache_candidate.is_dir() {
                return self
                    .paths
                    .source_paths(&record.key.location, &record.key.id);
            }
            let directory = persisted_dir.canonicalize().map_err(filesystem_error)?;
            let skill_file = persisted_file.canonicalize().map_err(filesystem_error)?;
            let cache_root = cache_candidate.canonicalize().map_err(filesystem_error)?;
            if path_is_descendant(&directory, &cache_root)
                && skill_file == directory.join(SKILL_FILE)
                && directory.file_name().and_then(|value| value.to_str())
                    == Some(record.managed_source.content_hash.as_str())
            {
                return Ok((directory, skill_file));
            }
            return Err(SkillApplicationError::Filesystem(
                "System Skill cache source failed boundary validation".to_string(),
            ));
        }
        self.paths
            .source_paths(&record.key.location, &record.key.id)
    }

    fn mount(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        let checkpoint = self.transactions.checkpoint(transaction)?;
        let result = (|| {
            let (source, skill_file) = self.record_source_paths(record)?;
            if !source.is_dir() || !skill_file.is_file() {
                return Err(SkillApplicationError::Filesystem(format!(
                    "Skill source is missing: {}",
                    skill_file.display()
                )));
            }
            let target =
                self.paths
                    .mount_target(&record.key.location, &record.key.id, mount_path)?;
            if paths_overlap(&source, &target) {
                return Err(SkillApplicationError::Validation(format!(
                    "Skill mount target overlaps its managed source: {}",
                    target.display()
                )));
            }
            let mount_root =
                self.preflight_mount_root(&record.key.location, agent_id, mount_path)?;
            std::fs::create_dir_all(&mount_root).map_err(|error| {
                SkillApplicationError::Filesystem(format!(
                    "Unable to prepare the Skill root for {agent_id}: {error}"
                ))
            })?;
            let mut overwritten = Vec::new();
            let mut backed_up = Vec::new();
            if path_exists(&target) {
                if is_managed_link(&target, &source) {
                    return Ok(repair_binding(
                        agent_id,
                        mount_path,
                        target,
                        overwritten,
                        backed_up,
                    ));
                }
                let backup = self.paths.durable_backup(
                    &record.key.location,
                    &record.key.id,
                    agent_id,
                    transaction,
                )?;
                self.transactions
                    .stage_permanent_replacement(transaction, &target, &backup)?;
                overwritten.push(target.to_string_lossy().to_string());
                backed_up.push(SkillBackupEntry {
                    original_path: target.to_string_lossy().to_string(),
                    backup_path: backup.to_string_lossy().to_string(),
                });
            } else {
                self.transactions
                    .stage_replace_or_create(transaction, &target)?;
            }
            create_dir_link(&source, &target)?;
            Ok(repair_binding(
                agent_id,
                mount_path,
                target,
                overwritten,
                backed_up,
            ))
        })();
        if result.is_err() {
            self.transactions.rollback_to(transaction, checkpoint);
        }
        result
    }

    fn remove_managed_mount(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        mount_path: &SkillMountPath,
    ) -> Result<Option<String>, SkillApplicationError> {
        let (source, _) = self.record_source_paths(record)?;
        let target = self
            .paths
            .mount_target(&record.key.location, &record.key.id, mount_path)?;
        if is_managed_link(&target, &source) {
            self.transactions.stage_remove(transaction, &target)?;
            Ok(Some(target.to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    }

    fn mount_path<'a>(
        configurations: &'a [AgentMountConfiguration],
        agent_id: &str,
    ) -> Result<&'a SkillMountPath, SkillApplicationError> {
        configurations
            .iter()
            .find(|configuration| configuration.agent_id == agent_id)
            .and_then(|configuration| configuration.configured_path.as_ref())
            .ok_or_else(|| {
                SkillApplicationError::Filesystem(format!(
                    "Agent mount path is unavailable: {agent_id}"
                ))
            })
    }

    fn preflight_mount_root(
        &self,
        location: &SkillLocation,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<PathBuf, SkillApplicationError> {
        let scope_root = self.paths.scope_root(location)?;
        let mount_root = self.paths.mount_root(location, mount_path)?;
        let mut current = scope_root;
        for component in Path::new(mount_path.as_str()).components() {
            current.push(component.as_os_str());
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if is_link_or_reparse_point(&metadata) => {
                    return if current.canonicalize().is_ok_and(|target| target.is_dir()) {
                        Err(SkillApplicationError::MountRootExternalLink(
                            agent_id.to_string(),
                        ))
                    } else {
                        Err(SkillApplicationError::MountRootBrokenLink(
                            agent_id.to_string(),
                        ))
                    };
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(SkillApplicationError::MountRootNotDirectory(
                        agent_id.to_string(),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => {
                    return Err(SkillApplicationError::Filesystem(format!(
                        "Unable to inspect the Skill root for {agent_id}: {error}"
                    )));
                }
            }
        }
        Ok(mount_root)
    }
}

impl SkillLegacySourcePort for ManagedSkillFilesystem {
    fn read_legacy_document(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<SkillDocument, SkillApplicationError> {
        let (_, skill_file) = self.paths.source_paths(location, id)?;
        let content = document::read_import_document(&skill_file)?;
        document::parse_document(&content)
    }

    fn archive_legacy_source(
        &self,
        location: &SkillLocation,
        id: &SkillId,
        reconciliation_version: u32,
    ) -> Result<Option<String>, SkillApplicationError> {
        let (source, _) = self.paths.source_paths(location, id)?;
        let skills_root = source.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Invalid legacy Skill source".to_string())
        })?;
        let vane_root = skills_root.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Invalid legacy Skill root".to_string())
        })?;
        let backup = vane_root
            .join("skill-migration-backups")
            .join(format!("v{reconciliation_version}"))
            .join(id.as_str());
        if !path_exists(&source) {
            return Ok(path_exists(&backup).then(|| normalize_path(&backup)));
        }

        let transaction = self.transactions.begin()?;
        let staged = if path_exists(&backup) {
            self.transactions
                .stage_remove(&transaction, &source)
                .map(|_| ())
        } else {
            self.transactions
                .stage_permanent_replacement(&transaction, &source, &backup)
        };
        match staged {
            Ok(()) => {
                self.transactions.commit(transaction);
                Ok(Some(normalize_path(&backup)))
            }
            Err(error) => {
                self.transactions.rollback(transaction);
                Err(error)
            }
        }
    }
}

impl SkillFilesystemPort for ManagedSkillFilesystem {
    fn begin_mutation(&self) -> Result<SkillFilesystemTransaction, SkillApplicationError> {
        self.transactions.begin()
    }

    fn commit_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.transactions.commit(transaction);
    }

    fn rollback_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.transactions.rollback(transaction);
    }

    fn probe_source(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<SkillSourceProbe, SkillApplicationError> {
        self.probe_document(location, id)
    }

    fn content_hash_for(&self, document: &SkillDocument) -> String {
        document::content_hash(&document::compose(document))
    }

    fn create_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.create_document(transaction, location, id, document)
    }

    fn replace_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        document: &SkillDocument,
        expected_content_hash: &str,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.replace_document(transaction, record, document, expected_content_hash)
    }

    fn inspect_import_metadata(
        &self,
        source_path: &str,
    ) -> Result<SkillMetadata, SkillApplicationError> {
        let source = PathBuf::from(source_path)
            .canonicalize()
            .map_err(|error| invalid_import(error.to_string()))?;
        if !source.is_dir() {
            return Err(invalid_import("source is not a directory"));
        }
        let content = document::read_import_document(&source.join(SKILL_FILE))
            .map_err(|error| invalid_import(error.to_string()))?;
        document::parse(&content)
    }

    fn import_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        source_path: &str,
    ) -> Result<SkillImportedSource, SkillApplicationError> {
        let source = PathBuf::from(source_path)
            .canonicalize()
            .map_err(|error| invalid_import(error.to_string()))?;
        if !source.is_dir() {
            return Err(invalid_import("source is not a directory"));
        }
        let content = document::read_import_document(&source.join(SKILL_FILE))
            .map_err(|error| invalid_import(error.to_string()))?;
        let metadata = document::parse(&content)?;
        let (target, skill_file) = self.paths.source_paths(location, &metadata.id)?;
        if paths_overlap(&source, &target) {
            return Err(invalid_import(
                "source overlaps the managed Skill destination",
            ));
        }
        if path_exists(&target) {
            return Err(SkillApplicationError::Conflict(
                metadata.id.as_str().to_string(),
            ));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        }
        self.transactions
            .stage_replace_or_create(transaction, &target)?;
        document::copy_directory(&source, &target)?;
        Ok(SkillImportedSource {
            source: ManagedSkillSource {
                skill_dir: target.to_string_lossy().to_string(),
                skill_md_path: skill_file.to_string_lossy().to_string(),
                content_hash: document::content_hash(&content),
            },
            metadata,
        })
    }

    fn remove_skill(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        remove_source: bool,
    ) -> Result<(), SkillApplicationError> {
        for binding in &record.bindings {
            self.remove_managed_mount(transaction, record, &binding.mount_path)?;
        }
        if remove_source {
            let (source, _) = self
                .paths
                .source_paths(&record.key.location, &record.key.id)?;
            self.transactions.stage_remove(transaction, &source)?;
        }
        Ok(())
    }

    fn reconcile_bindings(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        plan: &SkillBindingPlan,
        mount_paths: &[AgentMountConfiguration],
    ) -> Result<Vec<SkillAgentBinding>, SkillApplicationError> {
        for agent_id in &plan.unmount {
            let mount_path = record
                .bindings
                .iter()
                .find(|binding| binding.agent_id == *agent_id)
                .map(|binding| &binding.mount_path)
                .unwrap_or(Self::mount_path(mount_paths, agent_id)?);
            self.remove_managed_mount(transaction, record, mount_path)?;
        }
        let mut bindings = Vec::with_capacity(plan.desired_agent_ids.len());
        for agent_id in &plan.desired_agent_ids {
            let mount_path = Self::mount_path(mount_paths, agent_id)?;
            if plan.mount.contains(agent_id) {
                bindings.push(
                    self.mount(transaction, record, agent_id, mount_path)?
                        .binding,
                );
            } else {
                self.remove_managed_mount(transaction, record, mount_path)?;
                let target =
                    self.paths
                        .mount_target(&record.key.location, &record.key.id, mount_path)?;
                bindings.push(SkillAgentBinding {
                    agent_id: agent_id.clone(),
                    mount_path: mount_path.clone(),
                    mounted_path: target.to_string_lossy().to_string(),
                    mounted: false,
                });
            }
        }
        Ok(bindings)
    }

    fn migrate_binding(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        old_mount_path: &SkillMountPath,
        new_mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        if old_mount_path == new_mount_path {
            return self.mount(transaction, record, agent_id, new_mount_path);
        }
        let checkpoint = self.transactions.checkpoint(transaction)?;
        let result = (|| {
            let mut repair = self.mount(transaction, record, agent_id, new_mount_path)?;
            repair.removed_path = self.remove_managed_mount(transaction, record, old_mount_path)?;
            Ok(repair)
        })();
        if result.is_err() {
            self.transactions.rollback_to(transaction, checkpoint);
        }
        result
    }

    fn read_source(&self, record: &SkillRecord) -> Result<String, SkillApplicationError> {
        let (_, skill_file) = self.record_source_paths(record)?;
        std::fs::read_to_string(skill_file).map_err(filesystem_error)
    }

    fn observe_bindings(&self, records: &mut [SkillRecord]) -> Result<(), SkillApplicationError> {
        for record in records {
            let (source, _) = self.record_source_paths(record)?;
            for binding in &mut record.bindings {
                let target = self.paths.mount_target(
                    &record.key.location,
                    &record.key.id,
                    &binding.mount_path,
                )?;
                binding.mounted_path = target.to_string_lossy().to_string();
                binding.mounted = is_managed_link(&target, &source);
            }
        }
        Ok(())
    }

    fn inspect_drift(
        &self,
        location: &SkillLocation,
        records: &[SkillRecord],
        deleted_builtin_ids: &[SkillId],
    ) -> Result<SkillDriftInspection, SkillApplicationError> {
        let mut registered = Vec::with_capacity(records.len());
        let registered_ids = records
            .iter()
            .map(|record| record.key.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        for record in records {
            let (source, skill_file) = self.record_source_paths(record)?;
            let source_inspection = if skill_file.is_file() {
                let content = std::fs::read_to_string(&skill_file).map_err(filesystem_error)?;
                SkillSourceInspection::Present {
                    path: skill_file.to_string_lossy().to_string(),
                    content_hash: observed_content_hash(
                        &content,
                        &record.managed_source.content_hash,
                    ),
                }
            } else {
                SkillSourceInspection::Missing {
                    path: skill_file.to_string_lossy().to_string(),
                }
            };
            let bindings = record
                .bindings
                .iter()
                .map(|binding| {
                    let target = self.paths.mount_target(
                        &record.key.location,
                        &record.key.id,
                        &binding.mount_path,
                    )?;
                    let observation = if !path_exists(&target) {
                        SkillMountObservation::Missing
                    } else if is_managed_link(&target, &source) {
                        SkillMountObservation::Managed
                    } else {
                        SkillMountObservation::Conflict
                    };
                    Ok(SkillBindingInspection {
                        agent_id: binding.agent_id.clone(),
                        mounted_path: target.to_string_lossy().to_string(),
                        observation,
                    })
                })
                .collect::<Result<Vec<_>, SkillApplicationError>>()?;
            registered.push(
                crate::contexts::tooling::skills::domain::RegisteredSkillInspection {
                    id: record.key.id.clone(),
                    enabled: record.enabled,
                    expected_content_hash: record.managed_source.content_hash.clone(),
                    source: source_inspection,
                    bindings,
                },
            );
        }
        let source_root = self.paths.source_root(location)?;
        let mut unregistered_sources = Vec::new();
        match std::fs::read_dir(&source_root) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(filesystem_error)?;
                    let path = entry.path();
                    if !path.join(SKILL_FILE).is_file() {
                        continue;
                    }
                    let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
                        continue;
                    };
                    if !registered_ids.contains(id) {
                        unregistered_sources.push(UnregisteredSkillInspection {
                            id: id.to_string(),
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(filesystem_error(error)),
        }
        unregistered_sources.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(SkillDriftInspection {
            location: location.clone(),
            registered,
            unregistered_sources,
            deleted_builtin_ids: deleted_builtin_ids.to_vec(),
        })
    }

    fn repair_binding(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        self.mount(transaction, record, agent_id, mount_path)
    }

    fn refresh_source(
        &self,
        record: &SkillRecord,
        _issue: &SkillDriftIssue,
    ) -> Result<SkillSourceRefresh, SkillApplicationError> {
        let (_, skill_file) = self.record_source_paths(record)?;
        let content = std::fs::read_to_string(skill_file).map_err(filesystem_error)?;
        Ok(SkillSourceRefresh {
            metadata: document::parse(&content)?,
            content_hash: document::content_hash(&content),
        })
    }
}

fn managed_source(directory: PathBuf, skill_file: PathBuf, content: &str) -> ManagedSkillSource {
    ManagedSkillSource {
        skill_dir: directory.to_string_lossy().to_string(),
        skill_md_path: skill_file.to_string_lossy().to_string(),
        content_hash: document::content_hash(content),
    }
}

fn repair_binding(
    agent_id: &str,
    mount_path: &SkillMountPath,
    target: PathBuf,
    overwritten: Vec<String>,
    backed_up: Vec<SkillBackupEntry>,
) -> SkillMountRepair {
    SkillMountRepair {
        binding: SkillAgentBinding {
            agent_id: agent_id.to_string(),
            mount_path: mount_path.clone(),
            mounted_path: target.to_string_lossy().to_string(),
            mounted: true,
        },
        removed_path: None,
        overwritten,
        backed_up,
    }
}

fn is_managed_link(target: &Path, source: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(target) else {
        return false;
    };
    if !metadata.file_type().is_symlink() {
        return false;
    }
    let Ok(link_target) = std::fs::read_link(target) else {
        return false;
    };
    paths_equal(&link_target, source)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
    } else {
        left == right
    }
}

fn path_is_descendant(path: &Path, root: &Path) -> bool {
    if cfg!(windows) {
        path.to_string_lossy()
            .to_lowercase()
            .starts_with(&root.to_string_lossy().to_lowercase())
    } else {
        path.starts_with(root)
    }
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        let left = PathBuf::from(left.to_string_lossy().to_lowercase());
        let right = PathBuf::from(right.to_string_lossy().to_lowercase());
        left.starts_with(&right) || right.starts_with(&left)
    } else {
        left.starts_with(&right) || right.starts_with(&left)
    }
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn observed_content_hash(content: &str, expected: &str) -> String {
    if expected.len() == 64 && expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Sha256::digest(content.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    } else {
        document::content_hash(content)
    }
}

fn is_link_or_reparse_point(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

fn create_dir_link(source: &Path, target: &Path) -> Result<(), SkillApplicationError> {
    #[cfg(windows)]
    {
        match std::os::windows::fs::symlink_dir(source, target) {
            Ok(()) => Ok(()),
            Err(_) => create_windows_junction(source, target),
        }
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).map_err(filesystem_error)
    }
}

#[cfg(windows)]
fn create_windows_junction(source: &Path, target: &Path) -> Result<(), SkillApplicationError> {
    for path in [source, target] {
        if path.to_string_lossy().chars().any(|character| {
            matches!(
                character,
                '&' | '|' | '<' | '>' | '^' | '%' | '!' | '(' | ')' | '"' | '\r' | '\n'
            )
        }) {
            return Err(SkillApplicationError::Filesystem(
                "Skill link path contains characters unsupported by the Windows junction fallback"
                    .to_string(),
            ));
        }
    }
    let status = crate::platform::process::std_command("cmd")
        .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?
        .args([
            "/D",
            "/C",
            "mklink",
            "/J",
            &target.to_string_lossy(),
            &source.to_string_lossy(),
        ])
        .status()
        .map_err(filesystem_error)?;
    if status.success() {
        Ok(())
    } else {
        Err(SkillApplicationError::Filesystem(format!(
            "Failed to create managed Skill link: {}",
            target.display()
        )))
    }
}

fn invalid_import(message: impl Into<String>) -> SkillApplicationError {
    SkillApplicationError::Validation(format!("Invalid Skill source: {}", message.into()))
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::transaction::remove_path;
    use super::*;
    use crate::contexts::tooling::skills::domain::{
        SkillId, SkillKey, SkillLocation, SkillMetadata, SkillScope, SkillSource,
    };
    use crate::test_support::TempDirectory;

    fn location() -> SkillLocation {
        SkillLocation::new(SkillScope::Global, None).expect("global location")
    }

    fn metadata(value: &str) -> SkillMetadata {
        SkillMetadata::new(
            value,
            "Fixture Skill",
            "Fixture description",
            "testing",
            "1.0.0",
            vec!["fixture".to_string()],
        )
        .expect("metadata")
    }

    fn document(value: &str, body: &str) -> SkillDocument {
        SkillDocument {
            metadata: metadata(value),
            body: body.to_string(),
        }
    }

    fn record(value: &str, source: ManagedSkillSource) -> SkillRecord {
        SkillRecord {
            key: SkillKey::new(SkillId::parse(value).expect("Skill id"), location()),
            source: SkillSource::User,
            enabled: true,
            managed_source: source,
            metadata: metadata(value),
            bindings: Vec::new(),
            created_at: "2026-07-18T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
            resolved_metadata: None,
        }
    }

    #[test]
    fn source_create_replace_and_remove_are_reversible_until_commit() {
        let home = TempDirectory::new("Skill filesystem transaction");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("fixture-skill").expect("Skill id");

        let create = filesystem.begin_mutation().expect("create transaction");
        let source = filesystem
            .create_source(
                &create,
                &location(),
                &id,
                &document("fixture-skill", "first"),
            )
            .expect("create source");
        assert!(Path::new(&source.skill_md_path).is_file());
        filesystem.rollback_mutation(create);
        assert!(!Path::new(&source.skill_dir).exists());

        let create = filesystem.begin_mutation().expect("create transaction");
        let source = filesystem
            .create_source(
                &create,
                &location(),
                &id,
                &document("fixture-skill", "first"),
            )
            .expect("create source");
        filesystem.commit_mutation(create);
        let existing = record("fixture-skill", source.clone());
        std::fs::write(
            Path::new(&source.skill_dir).join("template.txt"),
            "preserve-me",
        )
        .expect("attachment");

        let replace = filesystem.begin_mutation().expect("replace transaction");
        filesystem
            .replace_source(
                &replace,
                &existing,
                &document("fixture-skill", "replacement"),
                &source.content_hash,
            )
            .expect("replace source");
        assert_eq!(
            std::fs::read_to_string(Path::new(&source.skill_dir).join("template.txt"))
                .expect("preserved attachment"),
            "preserve-me"
        );
        assert!(filesystem
            .read_source(&existing)
            .expect("replacement content")
            .contains("replacement"));
        filesystem.rollback_mutation(replace);
        assert!(filesystem
            .read_source(&existing)
            .expect("restored content")
            .contains("first"));

        let remove = filesystem.begin_mutation().expect("remove transaction");
        filesystem
            .remove_skill(&remove, &existing, true)
            .expect("stage removal");
        assert!(!Path::new(&source.skill_dir).exists());
        filesystem.rollback_mutation(remove);
        assert!(Path::new(&source.skill_md_path).is_file());
    }

    #[test]
    fn replacement_rejects_a_stale_hash_without_touching_the_document() {
        let home = TempDirectory::new("Skill stale edit");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("stale-skill").expect("Skill id");
        let create = filesystem.begin_mutation().expect("create transaction");
        let source = filesystem
            .create_source(&create, &location(), &id, &document("stale-skill", "newer"))
            .expect("source");
        filesystem.commit_mutation(create);
        let existing = record("stale-skill", source);
        let replace = filesystem.begin_mutation().expect("replace transaction");

        let error = filesystem
            .replace_source(
                &replace,
                &existing,
                &document("stale-skill", "stale overwrite"),
                "older-hash",
            )
            .expect_err("stale edit");
        filesystem.rollback_mutation(replace);

        assert!(matches!(
            error,
            SkillApplicationError::ConcurrentModification(ref id) if id == "stale-skill"
        ));
        assert!(filesystem
            .read_source(&existing)
            .expect("current document")
            .contains("newer"));
    }

    #[test]
    fn create_and_import_refuse_an_existing_managed_source() {
        let home = TempDirectory::new("Skill source collision");
        let incoming = TempDirectory::new("Skill collision import");
        incoming.write(
            "SKILL.md",
            "---\nid: collision-skill\nname: Collision\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - collision\n---\n\nincoming",
        );
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("collision-skill").expect("Skill id");
        let create = filesystem.begin_mutation().expect("create transaction");
        filesystem
            .create_source(
                &create,
                &location(),
                &id,
                &document("collision-skill", "original"),
            )
            .expect("original source");
        filesystem.commit_mutation(create);

        let duplicate = filesystem.begin_mutation().expect("duplicate transaction");
        assert!(matches!(
            filesystem.create_source(
                &duplicate,
                &location(),
                &id,
                &document("collision-skill", "replacement")
            ),
            Err(SkillApplicationError::Conflict(_))
        ));
        filesystem.rollback_mutation(duplicate);

        let import = filesystem.begin_mutation().expect("import transaction");
        assert!(matches!(
            filesystem.import_source(&import, &location(), &incoming.path().to_string_lossy()),
            Err(SkillApplicationError::Conflict(_))
        ));
        filesystem.rollback_mutation(import);
    }

    #[test]
    fn import_rejects_sources_deeper_than_the_limit() {
        let home = TempDirectory::new("Skill import depth target");
        let incoming = TempDirectory::new("Skill import depth source");
        incoming.write(
            "SKILL.md",
            "---\nid: deep-skill\nname: Deep\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - deep\n---\n\nbody",
        );
        let mut nested = incoming.path().to_path_buf();
        for index in 0..17 {
            nested = nested.join(format!("level-{index}"));
            std::fs::create_dir_all(&nested).expect("nested directory");
        }
        std::fs::write(nested.join("file.txt"), "too deep").expect("deep file");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(
                &transaction,
                &location(),
                &incoming.path().to_string_lossy(),
            )
            .expect_err("depth rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error.to_string().contains("depth exceeds 16"));
        assert!(!home.path().join(".vanehub/skills/deep-skill").exists());
    }

    #[test]
    fn import_rejects_an_oversized_skill_document_before_copying() {
        let home = TempDirectory::new("Skill import document target");
        let incoming = TempDirectory::new("Skill import document source");
        incoming.write("SKILL.md", &"x".repeat(256 * 1024 + 1));
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(
                &transaction,
                &location(),
                &incoming.path().to_string_lossy(),
            )
            .expect_err("document size rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error.to_string().contains("SKILL.md exceeds 256 KiB"));
        assert!(!home.path().join(".vanehub/skills").exists());
    }

    #[test]
    fn import_rejects_more_than_512_files_and_rolls_back_the_target() {
        let home = TempDirectory::new("Skill import file-count target");
        let incoming = TempDirectory::new("Skill import file-count source");
        incoming.write(
            "SKILL.md",
            "---\nid: many-files-skill\nname: Many Files\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - files\n---\n\nbody",
        );
        for index in 0..512 {
            incoming.write(&format!("asset-{index}.txt"), "x");
        }
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(
                &transaction,
                &location(),
                &incoming.path().to_string_lossy(),
            )
            .expect_err("file-count rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error.to_string().contains("file count exceeds 512"));
        assert!(!home
            .path()
            .join(".vanehub/skills/many-files-skill")
            .exists());
    }

    #[test]
    fn import_rejects_more_than_16_mib_and_rolls_back_the_target() {
        let home = TempDirectory::new("Skill import aggregate-size target");
        let incoming = TempDirectory::new("Skill import aggregate-size source");
        incoming.write(
            "SKILL.md",
            "---\nid: oversized-import\nname: Oversized Import\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - size\n---\n\nbody",
        );
        std::fs::write(
            incoming.path().join("payload.bin"),
            vec![b'x'; 16 * 1024 * 1024],
        )
        .expect("large payload");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(
                &transaction,
                &location(),
                &incoming.path().to_string_lossy(),
            )
            .expect_err("aggregate-size rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error.to_string().contains("import size exceeds 16 MiB"));
        assert!(!home
            .path()
            .join(".vanehub/skills/oversized-import")
            .exists());
    }

    #[test]
    fn import_rejects_a_source_that_contains_its_managed_destination() {
        let home = TempDirectory::new("Skill overlapping import source");
        home.write(
            "SKILL.md",
            "---\nid: overlapping-import\nname: Overlapping Import\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - overlap\n---\n\nbody",
        );
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(&transaction, &location(), &home.path().to_string_lossy())
            .expect_err("overlap rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error
            .to_string()
            .contains("overlaps the managed Skill destination"));
        assert!(!home
            .path()
            .join(".vanehub/skills/overlapping-import")
            .exists());
    }

    #[test]
    fn source_reads_derive_the_bounded_path_instead_of_trusting_persisted_paths() {
        let home = TempDirectory::new("Skill forged persisted path");
        let outside = TempDirectory::new("Skill outside secret");
        let secret = outside.write("secret.md", "private-secret");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("bounded-skill").expect("Skill id");
        let transaction = filesystem.begin_mutation().expect("transaction");
        let source = filesystem
            .create_source(
                &transaction,
                &location(),
                &id,
                &document("bounded-skill", "managed-content"),
            )
            .expect("managed source");
        filesystem.commit_mutation(transaction);
        let mut stored = record("bounded-skill", source);
        stored.managed_source.skill_md_path = secret.to_string_lossy().to_string();
        stored.managed_source.skill_dir = outside.path().to_string_lossy().to_string();

        let content = filesystem.read_source(&stored).expect("bounded read");

        assert!(content.contains("managed-content"));
        assert!(!content.contains("private-secret"));
    }

    #[test]
    fn binding_observation_uses_the_live_managed_link_and_derived_target() {
        let home = TempDirectory::new("Skill binding observation");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("observed-skill").expect("Skill id");
        let source_transaction = filesystem.begin_mutation().expect("source transaction");
        let source = filesystem
            .create_source(
                &source_transaction,
                &location(),
                &id,
                &document("observed-skill", "body"),
            )
            .expect("source");
        filesystem.commit_mutation(source_transaction);
        let mount_path = SkillMountPath::parse(".codex/skills").expect("mount path");
        let mut stored = record("observed-skill", source);
        stored.bindings.push(SkillAgentBinding {
            agent_id: "codex-cli".to_string(),
            mount_path: mount_path.clone(),
            mounted_path: "forged/outside/path".to_string(),
            mounted: true,
        });

        filesystem
            .observe_bindings(std::slice::from_mut(&mut stored))
            .expect("missing binding observation");
        assert!(!stored.bindings[0].mounted);
        let expected_target = home
            .path()
            .canonicalize()
            .expect("canonical home")
            .join(".codex")
            .join("skills")
            .join("observed-skill");
        assert_eq!(Path::new(&stored.bindings[0].mounted_path), expected_target);

        let mount_transaction = filesystem.begin_mutation().expect("mount transaction");
        filesystem
            .repair_binding(&mount_transaction, &stored, "codex-cli", &mount_path)
            .expect("binding repair");
        filesystem.commit_mutation(mount_transaction);
        stored.bindings[0].mounted = false;

        filesystem
            .observe_bindings(std::slice::from_mut(&mut stored))
            .expect("mounted binding observation");
        assert!(stored.bindings[0].mounted);
    }

    #[test]
    fn effective_derived_cache_is_an_accepted_cli_mount_source() {
        let home = TempDirectory::new("Skill effective cache mount");
        let revision = "a".repeat(64);
        let relative_root = format!(".vanehub/cache/skills/effective/effective-mounted/{revision}");
        let skill_file = home.write(
            &format!("{relative_root}/SKILL.md"),
            "---\nid: effective-mounted\nname: Effective Mounted\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\n---\n\nEffective",
        );
        home.write(
            &format!("{relative_root}/references/guide.md"),
            "Effective resource",
        );
        let source_root = skill_file.parent().expect("cache source").to_path_buf();
        let source = ManagedSkillSource {
            skill_dir: source_root.to_string_lossy().to_string(),
            skill_md_path: skill_file.to_string_lossy().to_string(),
            content_hash: revision,
        };
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let mut stored = record("effective-mounted", source);
        stored.resolved_metadata = Some(stored.effective_metadata());
        let mount_path = SkillMountPath::parse(".codex/skills").expect("mount path");
        let transaction = filesystem.begin_mutation().expect("mount transaction");

        let repair = filesystem
            .repair_binding(&transaction, &stored, "codex-cli", &mount_path)
            .expect("effective cache mount");
        filesystem.commit_mutation(transaction);

        assert!(repair.binding.mounted);
        let mounted = Path::new(&repair.binding.mounted_path);
        assert!(is_managed_link(mounted, &source_root));
        assert_eq!(
            std::fs::read_to_string(mounted.join("references/guide.md")).expect("mounted resource"),
            "Effective resource"
        );
    }

    #[test]
    fn mount_root_preflight_allows_absent_and_normal_directories() {
        let home = TempDirectory::new("Skill normal mount root");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("normal-root-skill").expect("Skill id");
        let source_transaction = filesystem.begin_mutation().expect("source transaction");
        let source = filesystem
            .create_source(
                &source_transaction,
                &location(),
                &id,
                &document("normal-root-skill", "body"),
            )
            .expect("source");
        filesystem.commit_mutation(source_transaction);
        let stored = record("normal-root-skill", source);
        let mount_path = SkillMountPath::parse(".codex/skills").expect("mount path");

        let absent_transaction = filesystem.begin_mutation().expect("absent transaction");
        filesystem
            .repair_binding(&absent_transaction, &stored, "codex-cli", &mount_path)
            .expect("absent mount root");
        filesystem.commit_mutation(absent_transaction);

        let target = home.path().join(".codex/skills/normal-root-skill");
        remove_path(&target).expect("remove managed target");
        assert!(home.path().join(".codex/skills").is_dir());
        let normal_transaction = filesystem.begin_mutation().expect("normal transaction");
        filesystem
            .repair_binding(&normal_transaction, &stored, "codex-cli", &mount_path)
            .expect("normal mount root");
        filesystem.commit_mutation(normal_transaction);
        assert!(is_managed_link(
            &target,
            Path::new(&stored.managed_source.skill_dir)
        ));
    }

    #[test]
    fn mount_root_preflight_rejects_live_and_broken_directory_links_without_writes() {
        let home = TempDirectory::new("Skill linked mount root");
        let external = TempDirectory::new("Skill external mount root");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("linked-root-skill").expect("Skill id");
        let source_transaction = filesystem.begin_mutation().expect("source transaction");
        let source = filesystem
            .create_source(
                &source_transaction,
                &location(),
                &id,
                &document("linked-root-skill", "body"),
            )
            .expect("source");
        filesystem.commit_mutation(source_transaction);
        let stored = record("linked-root-skill", source);
        let mount_path = SkillMountPath::parse(".claude/skills").expect("mount path");
        let linked_root = home.path().join(".claude").join("skills");
        std::fs::create_dir_all(linked_root.parent().expect("mount parent")).expect("mount parent");
        create_dir_link(external.path(), &linked_root).expect("live mount-root link");

        let live_transaction = filesystem.begin_mutation().expect("live transaction");
        let live_error = filesystem
            .repair_binding(&live_transaction, &stored, "claude-code", &mount_path)
            .expect_err("live directory link rejection");
        filesystem.rollback_mutation(live_transaction);
        assert_eq!(
            live_error,
            SkillApplicationError::MountRootExternalLink("claude-code".to_string())
        );
        assert!(!external.path().join("linked-root-skill").exists());
        remove_path(&linked_root).expect("remove live link");

        let broken_target = home.path().join("removed-external-root");
        std::fs::create_dir_all(&broken_target).expect("broken target seed");
        create_dir_link(&broken_target, &linked_root).expect("breakable mount-root link");
        std::fs::remove_dir(&broken_target).expect("remove linked target");
        let broken_transaction = filesystem.begin_mutation().expect("broken transaction");
        let broken_error = filesystem
            .repair_binding(&broken_transaction, &stored, "claude-code", &mount_path)
            .expect_err("broken directory link rejection");
        filesystem.rollback_mutation(broken_transaction);
        assert_eq!(
            broken_error,
            SkillApplicationError::MountRootBrokenLink("claude-code".to_string())
        );
        assert!(std::fs::symlink_metadata(&linked_root).is_ok());
        remove_path(&linked_root).expect("remove broken link");
    }

    #[test]
    fn conflicting_mount_backup_and_link_are_restored_on_rollback() {
        let home = TempDirectory::new("Skill mount rollback");
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let id = SkillId::parse("mounted-skill").expect("Skill id");
        let source_transaction = filesystem.begin_mutation().expect("source transaction");
        let source = filesystem
            .create_source(
                &source_transaction,
                &location(),
                &id,
                &document("mounted-skill", "body"),
            )
            .expect("source");
        filesystem.commit_mutation(source_transaction);
        let stored = record("mounted-skill", source);
        let target = home.path().join(".codex/skills/mounted-skill");
        std::fs::create_dir_all(&target).expect("conflicting target");
        std::fs::write(target.join("owner.txt"), "external").expect("conflict marker");
        let mount_path = SkillMountPath::parse(".codex/skills").expect("mount path");

        let transaction = filesystem.begin_mutation().expect("mount transaction");
        let repair = filesystem
            .repair_binding(&transaction, &stored, "codex-cli", &mount_path)
            .expect("mount repair");
        assert_eq!(repair.overwritten.len(), 1);
        assert_eq!(repair.backed_up.len(), 1);
        assert!(is_managed_link(
            &target,
            Path::new(&stored.managed_source.skill_dir)
        ));

        filesystem.rollback_mutation(transaction);

        assert!(target.join("owner.txt").is_file());
        assert_eq!(
            std::fs::read_to_string(target.join("owner.txt")).expect("restored marker"),
            "external"
        );
        assert!(!Path::new(&repair.backed_up[0].backup_path).exists());
    }

    #[test]
    fn import_rejects_symbolic_links_and_rolls_back_partial_target() {
        let home = TempDirectory::new("Skill import target");
        let incoming = TempDirectory::new("Skill import source");
        incoming.write(
            "SKILL.md",
            "---\nid: imported-skill\nname: Imported\ndescription: Fixture\ncategory: testing\nversion: 1.0.0\ntriggers:\n  - import\n---\n\nbody",
        );
        let outside = incoming.write("outside.txt", "outside");
        let link = incoming.path().join("linked.txt");
        if !create_file_symlink(&outside, &link) {
            return;
        }
        let filesystem = ManagedSkillFilesystem::with_home_root(home.path().to_path_buf());
        let transaction = filesystem.begin_mutation().expect("import transaction");

        let error = filesystem
            .import_source(
                &transaction,
                &location(),
                &incoming.path().to_string_lossy(),
            )
            .expect_err("symbolic link rejection");
        filesystem.rollback_mutation(transaction);

        assert!(error.to_string().contains("symbolic links"));
        assert!(!home.path().join(".vanehub/skills/imported-skill").exists());
    }

    #[cfg(unix)]
    fn create_file_symlink(source: &Path, target: &Path) -> bool {
        std::os::unix::fs::symlink(source, target).is_ok()
    }

    #[cfg(windows)]
    fn create_file_symlink(source: &Path, target: &Path) -> bool {
        std::os::windows::fs::symlink_file(source, target).is_ok()
    }
}
