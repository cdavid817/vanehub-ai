use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::models::{
    CreateMemoryInput, DiscoveredLegacySource, LegacyMemoryFields, MigrationRunOutcome,
    WorkspaceIdentityRequest,
};
use super::ports::{
    ClockPort, LegacyAddressAliasPort, LegacyMemorySourcePort, MemoryIdGeneratorPort,
    MemoryProjectionPort, MemoryRepository, MigrationJournalPort, WorkspaceIdentityPort,
};
use crate::contexts::personalization::domain::{
    AgentId, LegacyAddressKey, LegacyMemorySaveSource, LegacySourceFingerprint, LegacySourceId,
    MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope, MemorySensitivity,
    MemorySource, MemoryStatus, MemoryType, MigrationJournalEntry, MigrationStage, WorkspaceKey,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Migrates every v1 memory source into the governed v2 store, resumably.
///
/// # Ordering
///
/// The sequence below is not a preference. Every step is journalled *before* the action it
/// authorizes, so an interruption leaves a state the next run can read and continue from rather
/// than one it has to guess about:
///
/// ```text
/// discover + claim a journal entry (fingerprint, and the target id it will use)
///   -> raw byte backup
///   -> backup verified against the source fingerprint
///   -> v2 written at the journalled id
///   -> v2 re-read and verified
///   -> projection written
///   -> compatibility alias persisted
///   -> source fingerprint re-checked
///   -> legacy source removed
///   -> derived pending -> completed
/// ```
///
/// The legacy source is removed only after its backup is verified *and* its v2 record is verified
/// *and* the projection and mapping are persisted. Every earlier ordering has a window where the
/// original is gone and nothing readable has replaced it.
///
/// The target id is allocated and journalled before the file is written, which is what makes a
/// crash between write and journal recoverable: the next run finds the id it intended to use and
/// re-verifies that file instead of creating a second one.
pub(crate) struct LegacyMemoryMigrationService {
    sources: Arc<dyn LegacyMemorySourcePort>,
    repository: Arc<dyn MemoryRepository>,
    projection: Arc<dyn MemoryProjectionPort>,
    journal: Arc<dyn MigrationJournalPort>,
    aliases: Arc<dyn LegacyAddressAliasPort>,
    identity: Arc<dyn WorkspaceIdentityPort>,
    ids: Arc<dyn MemoryIdGeneratorPort>,
    clock: Arc<dyn ClockPort>,
    /// Test-only interruption point.
    ///
    /// A seam rather than a real crash: killing a process mid-write is not reproducible on every
    /// platform, and the property under test — that each journal boundary is recoverable — has to
    /// hold on every developer's machine.
    #[cfg(test)]
    interrupt_after: std::sync::Mutex<Option<MigrationStage>>,
}

/// The eight collaborators migration needs, named rather than positional.
///
/// A struct because four of them are `Arc<dyn ...>` over traits with overlapping shapes, and a
/// positional list of those is a swap waiting to happen that the type checker cannot catch.
pub(crate) struct LegacyMemoryMigrationPorts {
    pub(crate) sources: Arc<dyn LegacyMemorySourcePort>,
    pub(crate) repository: Arc<dyn MemoryRepository>,
    pub(crate) projection: Arc<dyn MemoryProjectionPort>,
    pub(crate) journal: Arc<dyn MigrationJournalPort>,
    pub(crate) aliases: Arc<dyn LegacyAddressAliasPort>,
    pub(crate) identity: Arc<dyn WorkspaceIdentityPort>,
    pub(crate) ids: Arc<dyn MemoryIdGeneratorPort>,
    pub(crate) clock: Arc<dyn ClockPort>,
}

impl LegacyMemoryMigrationService {
    pub(crate) fn new(ports: LegacyMemoryMigrationPorts) -> Self {
        Self {
            sources: ports.sources,
            repository: ports.repository,
            projection: ports.projection,
            journal: ports.journal,
            aliases: ports.aliases,
            identity: ports.identity,
            ids: ports.ids,
            clock: ports.clock,
            #[cfg(test)]
            interrupt_after: std::sync::Mutex::new(None),
        }
    }

    /// Aborts the run immediately after the named stage is journalled.
    #[cfg(test)]
    pub(crate) fn interrupt_after(&self, stage: MigrationStage) {
        *self.interrupt_after.lock().expect("interrupt") = Some(stage);
    }

    fn should_interrupt(&self, stage: MigrationStage) -> bool {
        #[cfg(test)]
        {
            return *self.interrupt_after.lock().expect("interrupt") == Some(stage);
        }
        #[cfg(not(test))]
        {
            let _ = stage;
            false
        }
    }

    /// Runs one migration pass over every discovered source.
    ///
    /// One source failing never stops the others: a directory with one unreadable file must still
    /// migrate the rest, or a single bad file would hold the whole store hostage.
    pub(crate) fn run(&self) -> Result<MigrationRunOutcome> {
        let discovered = self.sources.enumerate_sources()?;
        let mut outcome = MigrationRunOutcome {
            discovered: discovered.len(),
            ..MigrationRunOutcome::default()
        };

        let mut seen = std::collections::BTreeSet::new();
        for source in discovered {
            seen.insert(source.source_id());
            match self.migrate_one(&source) {
                Ok(SourceOutcome::Migrated) => outcome.migrated += 1,
                Ok(SourceOutcome::AlreadyDone) => outcome.already_done += 1,
                Ok(SourceOutcome::Quarantined) => outcome.quarantined += 1,
                Ok(SourceOutcome::SourceChanged) => outcome.source_changed += 1,
                // Terminal from an earlier run. Counted and reported again rather than forgotten,
                // so repair has something to act on instead of a memory that is simply absent.
                Ok(SourceOutcome::PreviouslyFailed(code)) => {
                    outcome.failed += 1;
                    record_code(&mut outcome, code);
                }
                Ok(SourceOutcome::Interrupted) => return Ok(outcome),
                // The directory is held by someone else. Directory-wide, so the rest of this run
                // would contend too — and nothing is marked failed, because nothing failed.
                Err(PersonalizationApplicationError::MaintenanceBusy) => {
                    outcome.deferred += 1;
                    record_code(&mut outcome, "maintenance_busy".to_string());
                    return Ok(outcome);
                }
                Err(error) => {
                    outcome.failed += 1;
                    let code = failure_code(&error);
                    self.mark_failed(&source, &code)?;
                    record_code(&mut outcome, code);
                }
            }
        }

        self.finish_removed_sources(&seen, &mut outcome)?;
        Ok(outcome)
    }

    /// Completes journal entries whose source is no longer on disk.
    ///
    /// Enumeration cannot find these: a run interrupted after the legacy file was removed leaves an
    /// entry that is resumable but has nothing left to discover, and without this pass it would stay
    /// resumable forever — the record would exist while migration never reported itself finished,
    /// which is the state that keeps memory unavailable.
    ///
    /// A source that vanished *before* it was safely removed is a different thing entirely and is
    /// marked failed rather than completed: something outside this migration deleted it, and saying
    /// "migrated" about text nobody can produce would be a lie.
    fn finish_removed_sources(
        &self,
        seen: &std::collections::BTreeSet<LegacySourceId>,
        outcome: &mut MigrationRunOutcome,
    ) -> Result<()> {
        for mut entry in self.journal.list_all()? {
            if !entry.stage.is_resumable() || seen.contains(&entry.source_id) {
                continue;
            }
            let record = match entry.target_memory_id.as_ref() {
                Some(id) => self.repository.get(id)?,
                None => None,
            };
            let usable = entry.stage >= MigrationStage::LegacyRemoved && record.is_some();
            if !usable {
                outcome.failed += 1;
                let code = if entry.stage >= MigrationStage::LegacyRemoved {
                    "v2_record_missing"
                } else {
                    "source_vanished"
                };
                entry.stage = MigrationStage::Failed;
                entry.last_error_code = Some(code.to_string());
                self.write_entry(&entry)?;
                record_code(outcome, code.to_string());
                continue;
            }

            if entry.stage < MigrationStage::DerivedPending {
                entry.stage = MigrationStage::DerivedPending;
                self.write_entry(&entry)?;
                if self.should_interrupt(MigrationStage::DerivedPending) {
                    return Ok(());
                }
            }
            entry.stage = MigrationStage::Completed;
            self.write_entry(&entry)?;
            outcome.migrated += 1;
        }
        Ok(())
    }

    fn migrate_one(&self, source: &DiscoveredLegacySource) -> Result<SourceOutcome> {
        let source_id = source.source_id();

        if let Some(entry) = self.journal.get(&source_id)? {
            if !entry.stage.is_resumable() {
                return Ok(match entry.stage {
                    MigrationStage::Completed => SourceOutcome::AlreadyDone,
                    MigrationStage::SourceChanged => SourceOutcome::SourceChanged,
                    _ => SourceOutcome::PreviouslyFailed(
                        entry
                            .last_error_code
                            .clone()
                            .unwrap_or_else(|| "migration_step_failed".to_string()),
                    ),
                });
            }
        }

        // A source that will not parse is journalled and quarantined rather than skipped. The
        // previous store's parse-dependent scan made these invisible, which is how a reset could
        // report success while leaving them behind.
        let Some(fields) = source.fields.as_ref() else {
            return self.quarantine(source, &source_id);
        };

        // Claimed rather than inserted. Two migrators over one directory both reach this, and
        // whichever loses adopts the winner's target id instead of allocating a second one.
        let mut entry = self.journal.claim(
            &MigrationJournalEntry {
                source_id,
                locator: source.locator.clone(),
                // Allocated before anything is written, so a crash between the write and the
                // journal still leaves the next run able to recognize its own file.
                target_memory_id: Some(self.ids.generate()),
                stage: MigrationStage::Discovered,
                backup_relative_path: None,
                source_fingerprint: source.fingerprint.clone(),
                last_error_code: None,
            },
            self.clock.now(),
        )?;
        if entry.stage == MigrationStage::Discovered
            && self.should_interrupt(MigrationStage::Discovered)
        {
            return Ok(SourceOutcome::Interrupted);
        }

        let target_id = entry.target_memory_id.clone().ok_or_else(|| {
            PersonalizationApplicationError::Storage(
                "a resumable journal entry has no target memory id".to_string(),
            )
        })?;
        // The fingerprint the journal holds, not the one this scan just took. Comparing the source
        // against itself would make every check pass by construction.
        let expected = entry.source_fingerprint.clone().ok_or_else(|| {
            PersonalizationApplicationError::Storage(
                "a resumable journal entry has no source fingerprint".to_string(),
            )
        })?;

        if let Some(outcome) = self.back_up(source, &mut entry, &expected)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.write_v2(fields, &mut entry, &target_id)? {
            return Ok(outcome);
        }

        let record = self.repository.get(&target_id)?.ok_or_else(|| {
            PersonalizationApplicationError::Storage("v2_record_missing".to_string())
        })?;
        if entry.stage < MigrationStage::V2Verified {
            verify_migrated_record(&record, fields)?;
            entry.stage = MigrationStage::V2Verified;
            self.write_entry(&entry)?;
            if self.should_interrupt(MigrationStage::V2Verified) {
                return Ok(SourceOutcome::Interrupted);
            }
        }

        if entry.stage < MigrationStage::ProjectionWritten {
            self.projection.upsert(&record, &record.content_hash())?;
            entry.stage = MigrationStage::ProjectionWritten;
            self.write_entry(&entry)?;
            if self.should_interrupt(MigrationStage::ProjectionWritten) {
                return Ok(SourceOutcome::Interrupted);
            }
        }

        // Best effort by design: a name that could never have been a v1 filename has no legacy
        // address, and that is not a migration failure. An address that already points somewhere is
        // left alone — the first migrated memory of a given name keeps the old address.
        if let Ok(address) = LegacyAddressKey::from_display_name(&fields.name) {
            if self.aliases.get(&address)?.is_none() {
                self.aliases.put(&address, &target_id, self.clock.now())?;
            }
        }

        if let Some(outcome) = self.remove_legacy(source, &mut entry, &expected)? {
            return Ok(outcome);
        }

        if entry.stage < MigrationStage::DerivedPending {
            entry.stage = MigrationStage::DerivedPending;
            self.write_entry(&entry)?;
            if self.should_interrupt(MigrationStage::DerivedPending) {
                return Ok(SourceOutcome::Interrupted);
            }
        }

        entry.stage = MigrationStage::Completed;
        self.write_entry(&entry)?;
        Ok(SourceOutcome::Migrated)
    }

    /// Copies the source's raw bytes aside, then proves the copy reproduces them.
    fn back_up(
        &self,
        source: &DiscoveredLegacySource,
        entry: &mut MigrationJournalEntry,
        expected: &LegacySourceFingerprint,
    ) -> Result<Option<SourceOutcome>> {
        if entry.stage < MigrationStage::BackupWritten {
            let raw = self.sources.read_raw(&source.locator)?;
            if !LegacySourceFingerprint::of(&raw).matches(expected) {
                return self.mark_source_changed(entry).map(Some);
            }
            entry.backup_relative_path = Some(self.sources.write_backup(&source.locator, &raw)?);
            entry.stage = MigrationStage::BackupWritten;
            self.write_entry(entry)?;
            if self.should_interrupt(MigrationStage::BackupWritten) {
                return Ok(Some(SourceOutcome::Interrupted));
            }
        }

        if entry.stage < MigrationStage::BackupVerified {
            let backup_path = entry.backup_relative_path.clone().ok_or_else(|| {
                PersonalizationApplicationError::Storage(
                    "a backed-up entry has no backup path".to_string(),
                )
            })?;
            let stored = self.sources.read_backup(&backup_path)?;
            // Verified against the source's own raw fingerprint, not against a re-read of the
            // source: a backup that does not reproduce the original bytes cannot restore a file an
            // older build could read, and that is the only thing this backup is for.
            if !LegacySourceFingerprint::of(&stored).matches(expected) {
                return Err(PersonalizationApplicationError::Storage(
                    "backup_verification_failed".to_string(),
                ));
            }
            entry.stage = MigrationStage::BackupVerified;
            self.write_entry(entry)?;
            if self.should_interrupt(MigrationStage::BackupVerified) {
                return Ok(Some(SourceOutcome::Interrupted));
            }
        }
        Ok(None)
    }

    fn write_v2(
        &self,
        fields: &LegacyMemoryFields,
        entry: &mut MigrationJournalEntry,
        target_id: &MemoryId,
    ) -> Result<Option<SourceOutcome>> {
        if entry.stage < MigrationStage::V2Written {
            let created_at = fields.created_at.unwrap_or_else(|| self.clock.now());
            // v1 ordered by modification time, so dropping it would put a memory the model had just
            // corrected behind every stale one it never touched.
            let updated_at = fields.modified_at.unwrap_or(created_at);
            if let Err(error) = self.repository.create_with_id(
                target_id,
                self.build_input(fields),
                created_at,
                updated_at,
            ) {
                // An existing file at the journalled id is this source's own record, written by a
                // run that died before it could journal the write, or by a racing migrator that
                // claimed the same id. Verification below is what decides whether it is usable —
                // treating it as a failure here would strand a source whose record already exists.
                if self.repository.get(target_id)?.is_none() {
                    return Err(error);
                }
            }
            entry.stage = MigrationStage::V2Written;
            self.write_entry(entry)?;
            if self.should_interrupt(MigrationStage::V2Written) {
                return Ok(Some(SourceOutcome::Interrupted));
            }
        }
        Ok(None)
    }

    fn remove_legacy(
        &self,
        source: &DiscoveredLegacySource,
        entry: &mut MigrationJournalEntry,
        expected: &LegacySourceFingerprint,
    ) -> Result<Option<SourceOutcome>> {
        if entry.stage < MigrationStage::LegacyRemoved {
            // Re-checked immediately before the irreversible step. Something may have edited the
            // file since the backup was taken, and removing it then would destroy an edit no backup
            // holds.
            let current = self.sources.read_raw(&source.locator)?;
            if !LegacySourceFingerprint::of(&current).matches(expected) {
                return self.mark_source_changed(entry).map(Some);
            }
            self.sources.remove_source(&source.locator)?;
            entry.stage = MigrationStage::LegacyRemoved;
            self.write_entry(entry)?;
            if self.should_interrupt(MigrationStage::LegacyRemoved) {
                return Ok(Some(SourceOutcome::Interrupted));
            }
        }
        Ok(None)
    }

    fn quarantine(
        &self,
        source: &DiscoveredLegacySource,
        source_id: &LegacySourceId,
    ) -> Result<SourceOutcome> {
        let mut entry = MigrationJournalEntry {
            source_id: source_id.clone(),
            locator: source.locator.clone(),
            target_memory_id: None,
            stage: MigrationStage::Discovered,
            backup_relative_path: None,
            source_fingerprint: source.fingerprint.clone(),
            last_error_code: Some("unreadable_source".to_string()),
        };
        self.write_entry(&entry)?;

        match self.sources.quarantine_source(&source.locator) {
            Ok(path) => {
                entry.backup_relative_path = Some(path);
                entry.stage = MigrationStage::Failed;
                entry.last_error_code = Some("quarantined".to_string());
                self.write_entry(&entry)?;
                Ok(SourceOutcome::Quarantined)
            }
            Err(PersonalizationApplicationError::MaintenanceBusy) => {
                Err(PersonalizationApplicationError::MaintenanceBusy)
            }
            Err(_) => {
                // The original stays exactly where it is. Losing a user's text because this build
                // could not move it is worse than any amount of maintenance noise.
                entry.stage = MigrationStage::Failed;
                entry.last_error_code = Some("quarantine_failed".to_string());
                self.write_entry(&entry)?;
                Err(PersonalizationApplicationError::Storage(
                    "quarantine_failed".to_string(),
                ))
            }
        }
    }

    fn mark_source_changed(&self, entry: &mut MigrationJournalEntry) -> Result<SourceOutcome> {
        entry.stage = MigrationStage::SourceChanged;
        entry.last_error_code = Some("source_changed".to_string());
        self.write_entry(entry)?;
        Ok(SourceOutcome::SourceChanged)
    }

    fn mark_failed(&self, source: &DiscoveredLegacySource, code: &str) -> Result<()> {
        let source_id = source.source_id();
        let mut entry = self
            .journal
            .get(&source_id)?
            .unwrap_or_else(|| MigrationJournalEntry {
                source_id,
                locator: source.locator.clone(),
                target_memory_id: None,
                stage: MigrationStage::Discovered,
                backup_relative_path: None,
                source_fingerprint: source.fingerprint.clone(),
                last_error_code: None,
            });
        entry.stage = MigrationStage::Failed;
        entry.last_error_code = Some(code.to_string());
        self.write_entry(&entry)
    }

    fn write_entry(&self, entry: &MigrationJournalEntry) -> Result<()> {
        self.journal.upsert(entry, self.clock.now())
    }

    /// The v2 shape every migrated record takes.
    ///
    /// Global scope and an all-Agents audience preserve what a v1 memory could see, so nothing a
    /// user could read before becomes invisible after. An unrecognized legacy type migrates as
    /// explicitly untyped rather than guessed — a wrong type is worse than a missing one.
    fn build_input(&self, fields: &LegacyMemoryFields) -> CreateMemoryInput {
        CreateMemoryInput {
            name: fields.name.clone(),
            description: fields.description.clone(),
            memory_type: fields
                .memory_type
                .as_deref()
                .and_then(|value| MemoryType::parse(value).ok())
                .unwrap_or(MemoryType::Untyped),
            content: fields.content.clone(),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
            status: MemoryStatus::Active,
            source: MemorySource::LegacyMigration,
            provenance: MemoryProvenance {
                source_agent_id: fields
                    .source_agent_id
                    .as_deref()
                    .and_then(|id| AgentId::parse(id).ok()),
                source_session_id: None,
                source_message_id: None,
                // Both are recorded, and they answer different questions. The key is what scope
                // comparisons would use if one can be derived; the raw folder is what the user's
                // file actually said. Keeping only the key would lose the origin wherever it cannot
                // be resolved, and keeping only the folder would leave nothing comparable.
                source_workspace_key: self.workspace_key_for(fields.folder.as_deref()),
                // Mapped explicitly, and an unrecognized or absent value stays absent. Defaulting
                // to `Automatic` would relabel a fact the user stated as one an Agent inferred, and
                // there is no second chance to tell them apart after the source is gone.
                legacy_original_save_source: fields
                    .save_source
                    .as_deref()
                    .map(str::trim)
                    .and_then(|value| LegacyMemorySaveSource::parse(value).ok()),
                legacy_folder: fields.folder.clone(),
                legacy_source_relative_path: fields.source_relative_path.clone(),
            },
            sensitivity: MemorySensitivity::Normal,
        }
    }

    /// The stable key for the raw path v1 recorded, when that path identifies a workspace precisely.
    ///
    /// v1 stored a display path, and a display path is not a workspace identity. It is only enough
    /// when it is unambiguously one root: an absolute local path, or a remote URI carrying a host.
    /// A relative path depends on a working directory nobody recorded, and a UNC path names a share
    /// that the same host can expose under more than one spelling — deriving a key from either
    /// would produce a value that compares equal to workspaces it is not, which is worse than
    /// having no key. Those keep `legacy_folder` and no key, and that pairing is itself the
    /// diagnostic: an origin was recorded, and it could not be resolved.
    fn workspace_key_for(&self, folder: Option<&str>) -> Option<WorkspaceKey> {
        let request = legacy_workspace_request(folder?)?;
        self.identity
            .resolve(&request)
            .ok()
            .flatten()
            .map(|identity| identity.key().clone())
    }
}

/// Turns a raw v1 `folder` into an identity request, or refuses to guess.
///
/// Shared with the pre-governance compatibility save path: both receive the same kind of value from
/// the same frozen contract, and two rules for one input is how they would come to disagree about
/// which workspace a memory belongs to.
pub(crate) fn legacy_workspace_request(folder: &str) -> Option<WorkspaceIdentityRequest> {
    let folder = folder.trim();
    if folder.is_empty() {
        return None;
    }
    // A scheme means connection identity is present; the resolver discards any password component
    // and returns nothing when the host is empty, so an unusable URI still resolves to no key.
    if folder.contains("://") {
        return Some(WorkspaceIdentityRequest {
            remote_uri: Some(folder.to_string()),
            ..WorkspaceIdentityRequest::default()
        });
    }
    if is_unc_path(folder) || !is_absolute_local_path(folder) {
        return None;
    }
    Some(WorkspaceIdentityRequest {
        project_path: Some(folder.to_string()),
        ..WorkspaceIdentityRequest::default()
    })
}

/// A UNC path — `\\server\share`, or the same thing spelled with forward slashes.
///
/// Refused rather than treated as a local root: the same share reached through two mappings, or
/// through a drive letter on one machine and a UNC path on another, would otherwise derive two keys
/// for one workspace, or one key for two.
fn is_unc_path(folder: &str) -> bool {
    folder.starts_with(r"\\") || folder.starts_with("//")
}

/// Whether the path names one root on its own, with no working directory to resolve against.
///
/// A single leading backslash is deliberately not absolute: on Windows `\projects` is relative to
/// the current drive, so the same string names a different directory depending on where the process
/// happened to be.
fn is_absolute_local_path(folder: &str) -> bool {
    if folder.starts_with('/') {
        return true;
    }
    // A drive-relative path such as `C:notes` is not absolute either; the separator is what makes
    // it one.
    let mut characters = folder.chars();
    matches!(
        (characters.next(), characters.next(), characters.next()),
        (Some(letter), Some(':'), Some('/' | '\\')) if letter.is_ascii_alphabetic()
    )
}

enum SourceOutcome {
    Migrated,
    AlreadyDone,
    Quarantined,
    SourceChanged,
    PreviouslyFailed(String),
    Interrupted,
}

fn record_code(outcome: &mut MigrationRunOutcome, code: String) {
    if !outcome.failure_codes.contains(&code) {
        outcome.failure_codes.push(code);
    }
}

/// Codes only. This reaches diagnostics and logs, so a path or a memory body must never travel in
/// it.
fn failure_code(error: &PersonalizationApplicationError) -> String {
    match error {
        PersonalizationApplicationError::Storage(message)
            if message
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_') =>
        {
            message.clone()
        }
        PersonalizationApplicationError::MaintenanceBusy => "maintenance_busy".to_string(),
        PersonalizationApplicationError::Domain(_) => "domain_validation_failed".to_string(),
        _ => "migration_step_failed".to_string(),
    }
}

/// Re-reading proves the file parses; this proves it says what the source said.
///
/// A record that round-tripped through a broken writer would still parse, so a hash check alone is
/// not enough — the fields have to match the source they came from.
fn verify_migrated_record(record: &MemoryRecord, fields: &LegacyMemoryFields) -> Result<()> {
    let expected_content =
        crate::contexts::personalization::domain::content_hash(&normalize(&fields.content));
    if record.content_hash() != expected_content {
        return Err(PersonalizationApplicationError::Storage(
            "v2_content_mismatch".to_string(),
        ));
    }
    if record.name != fields.name || record.description != fields.description {
        return Err(PersonalizationApplicationError::Storage(
            "v2_metadata_mismatch".to_string(),
        ));
    }
    if !matches!(record.source, MemorySource::LegacyMigration) {
        return Err(PersonalizationApplicationError::Storage(
            "v2_provenance_mismatch".to_string(),
        ));
    }
    if record.revision != 1 {
        return Err(PersonalizationApplicationError::Storage(
            "v2_revision_mismatch".to_string(),
        ));
    }
    if let Some(created_at) = fields.created_at {
        if record.created_at != created_at {
            return Err(PersonalizationApplicationError::Storage(
                "v2_timestamp_mismatch".to_string(),
            ));
        }
    }
    // Provenance is verified from the re-read file, not from the record that was just built: the
    // point is that it survived the write, and a check against the in-memory value would pass even
    // if the writer dropped the field.
    if record.provenance.legacy_folder.as_deref() != fields.folder.as_deref()
        || record.provenance.legacy_source_relative_path.as_deref()
            != fields.source_relative_path.as_deref()
    {
        return Err(PersonalizationApplicationError::Storage(
            "v2_legacy_provenance_mismatch".to_string(),
        ));
    }
    Ok(())
}

/// Mirrors the store's own body normalization, so the expected hash is computed the same way the
/// stored one was.
fn normalize(content: &str) -> String {
    content.replace("\r\n", "\n")
}
