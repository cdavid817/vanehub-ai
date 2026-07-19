mod document;
mod paths;
mod transaction;

use self::paths::{SkillPathResolver, SKILL_FILE};
use self::transaction::{path_exists, FileTransactions};
use crate::contexts::tooling::skills::application::{
    AgentMountConfiguration, ManagedSkillSource, SkillAgentBinding, SkillApplicationError,
    SkillBackupEntry, SkillDocument, SkillFilesystemPort, SkillFilesystemTransaction,
    SkillImportedSource, SkillMountRepair, SkillRecord, SkillSourceRefresh,
};
use crate::contexts::tooling::skills::domain::{
    SkillBindingInspection, SkillBindingPlan, SkillDriftInspection, SkillDriftIssue, SkillId,
    SkillLocation, SkillMountObservation, SkillMountPath, SkillSourceInspection,
    UnregisteredSkillInspection,
};
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
    fn with_home_root(home_root: PathBuf) -> Self {
        Self {
            paths: SkillPathResolver::with_home_root(home_root),
            transactions: Arc::new(FileTransactions::default()),
        }
    }

    fn write_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let (directory, skill_file) = self.paths.source_paths(location, id)?;
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

    fn mount(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        let checkpoint = self.transactions.checkpoint(transaction)?;
        let result = (|| {
            let (source, skill_file) = self
                .paths
                .source_paths(&record.key.location, &record.key.id)?;
            if !source.is_dir() || !skill_file.is_file() {
                return Err(SkillApplicationError::Filesystem(format!(
                    "Skill source is missing: {}",
                    skill_file.display()
                )));
            }
            let target =
                self.paths
                    .mount_target(&record.key.location, &record.key.id, mount_path)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(filesystem_error)?;
            }
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
        let (source, _) = self
            .paths
            .source_paths(&record.key.location, &record.key.id)?;
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
}

impl SkillFilesystemPort for ManagedSkillFilesystem {
    fn begin_mutation(&self) -> Result<SkillFilesystemTransaction, SkillApplicationError> {
        Ok(self.transactions.begin())
    }

    fn commit_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.transactions.commit(transaction);
    }

    fn rollback_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.transactions.rollback(transaction);
    }

    fn create_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.write_source(transaction, location, id, document)
    }

    fn replace_source(
        &self,
        transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.write_source(transaction, &record.key.location, &record.key.id, document)
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
        let content = std::fs::read_to_string(source.join(SKILL_FILE))
            .map_err(|error| invalid_import(error.to_string()))?;
        let metadata = document::parse(&content)?;
        let (target, skill_file) = self.paths.source_paths(location, &metadata.id)?;
        if source == target {
            return Err(invalid_import(
                "source is already in the managed Skill directory",
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
    ) -> Result<(), SkillApplicationError> {
        for binding in &record.bindings {
            self.remove_managed_mount(transaction, record, &binding.mount_path)?;
        }
        let (source, _) = self
            .paths
            .source_paths(&record.key.location, &record.key.id)?;
        self.transactions.stage_remove(transaction, &source)?;
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
        let (_, skill_file) = self
            .paths
            .source_paths(&record.key.location, &record.key.id)?;
        std::fs::read_to_string(skill_file).map_err(filesystem_error)
    }

    fn observe_bindings(&self, records: &mut [SkillRecord]) -> Result<(), SkillApplicationError> {
        for record in records {
            let (source, _) = self
                .paths
                .source_paths(&record.key.location, &record.key.id)?;
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
            let (source, skill_file) = self
                .paths
                .source_paths(&record.key.location, &record.key.id)?;
            let source_inspection = if skill_file.is_file() {
                let content = std::fs::read_to_string(&skill_file).map_err(filesystem_error)?;
                SkillSourceInspection::Present {
                    path: skill_file.to_string_lossy().to_string(),
                    content_hash: document::content_hash(&content),
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
        let (_, skill_file) = self
            .paths
            .source_paths(&record.key.location, &record.key.id)?;
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

        let replace = filesystem.begin_mutation().expect("replace transaction");
        filesystem
            .replace_source(
                &replace,
                &existing,
                &document("fixture-skill", "replacement"),
            )
            .expect("replace source");
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
            .remove_skill(&remove, &existing)
            .expect("stage removal");
        assert!(!Path::new(&source.skill_dir).exists());
        filesystem.rollback_mutation(remove);
        assert!(Path::new(&source.skill_md_path).is_file());
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
