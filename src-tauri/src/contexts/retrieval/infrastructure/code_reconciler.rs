use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::super::domain::code_index::CODE_INDEX_VERSION;
use super::super::domain::{
    CodeFileManifest, CodeIndexAuditEvent, CodeIndexAuditReason, CodeIndexConfigurationUpdate,
    CodeIndexPhase, CodeWorkspace, RetrievalError,
};
use super::code_chunker::{chunk_code, DEFAULT_MAX_CHUNK_BYTES};
use super::code_inventory::{inspect_workspace_path, normalize_explicit_path};
use super::code_inventory::{inventory_workspace, InventoryFile};
use super::code_parser::load_and_parse;
use super::code_symbols::extract_symbols;
use crate::contexts::retrieval::application::CodeIndexRepository;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CodeReconcileOutcome {
    pub(crate) discovered: u64,
    pub(crate) unchanged: u64,
    pub(crate) metadata_updated: u64,
    pub(crate) replaced: u64,
    pub(crate) deleted: u64,
    pub(crate) failed: u64,
    pub(crate) cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the follow-up filesystem watcher constructs targeted path changes"
    )
)]
pub(crate) enum CodePathChange {
    Upsert(String),
    Delete(String),
    Rename { from: String, to: String },
}

const MAX_TARGETED_PATHS: usize = 512;

pub(crate) trait CodeIndexCancellation {
    fn is_cancelled(&self) -> bool;
}

impl CodeIndexCancellation for AtomicBool {
    fn is_cancelled(&self) -> bool {
        self.load(Ordering::SeqCst)
    }
}

pub(crate) fn reconcile_workspace(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
) -> Result<CodeReconcileOutcome, RetrievalError> {
    reconcile_workspace_cancellable(repository, workspace, &AtomicBool::new(false))
}

pub(crate) fn reconcile_workspace_cancellable(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
    cancellation: &dyn CodeIndexCancellation,
) -> Result<CodeReconcileOutcome, RetrievalError> {
    let configuration = workspace.configuration()?;
    if !configuration.enabled {
        return Ok(CodeReconcileOutcome::default());
    }
    if should_stop(repository, workspace, cancellation)? {
        repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
        return Ok(CodeReconcileOutcome {
            cancelled: true,
            ..CodeReconcileOutcome::default()
        });
    }
    repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Scanning)?;
    let inventory = inventory_workspace(Path::new(&workspace.canonical_root), &configuration)?;
    for (reason, count) in inventory.skip_counts.ordered() {
        let reason = CodeIndexAuditReason::parse(reason).ok_or_else(|| {
            RetrievalError::Storage("unsupported code admission audit reason".to_string())
        })?;
        repository.record_audit(
            &workspace.workspace_id,
            None,
            CodeIndexAuditEvent::Skipped,
            Some(reason),
            count,
        )?;
    }
    repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Parsing)?;
    let mut existing = repository
        .list_file_manifests(&workspace.workspace_id)?
        .into_iter()
        .map(|manifest| (manifest.relative_path.clone(), manifest))
        .collect::<BTreeMap<_, _>>();
    let mut outcome = CodeReconcileOutcome {
        discovered: inventory.files.len() as u64,
        ..CodeReconcileOutcome::default()
    };

    for file in inventory.files {
        if should_stop(repository, workspace, cancellation)? {
            outcome.cancelled = true;
            repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
            return Ok(outcome);
        }
        let previous = existing.remove(&file.relative_path);
        if previous.as_ref().is_some_and(|manifest| {
            manifest.byte_size == file.byte_size
                && manifest.modified_ns == file.modified_ns
                && manifest.index_version == CODE_INDEX_VERSION
        }) {
            outcome.unchanged += 1;
            continue;
        }
        reconcile_target_file(
            repository,
            workspace,
            &configuration,
            previous.as_ref(),
            file,
            cancellation,
            &mut outcome,
        )?;
        if outcome.cancelled {
            repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
            return Ok(outcome);
        }
    }

    for relative_path in existing.keys() {
        if should_stop(repository, workspace, cancellation)? {
            outcome.cancelled = true;
            repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
            return Ok(outcome);
        }
        repository.delete_code_file(&workspace.workspace_id, relative_path)?;
        repository.record_audit(
            &workspace.workspace_id,
            Some(relative_path),
            CodeIndexAuditEvent::Deleted,
            None,
            1,
        )?;
        outcome.deleted += 1;
    }
    repository.set_workspace_phase(
        &workspace.workspace_id,
        if outcome.failed == 0 {
            CodeIndexPhase::Ready
        } else {
            CodeIndexPhase::Degraded
        },
    )?;
    Ok(outcome)
}

fn should_stop(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
    cancellation: &dyn CodeIndexCancellation,
) -> Result<bool, RetrievalError> {
    if cancellation.is_cancelled() {
        return Ok(true);
    }
    Ok(repository.workspace_generation(&workspace.workspace_id)? != Some(workspace.generation))
}

pub(crate) fn reconcile_paths(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
    changes: &[CodePathChange],
) -> Result<CodeReconcileOutcome, RetrievalError> {
    reconcile_paths_cancellable(repository, workspace, changes, &AtomicBool::new(false))
}

pub(crate) fn reconcile_paths_cancellable(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
    changes: &[CodePathChange],
    cancellation: &dyn CodeIndexCancellation,
) -> Result<CodeReconcileOutcome, RetrievalError> {
    if changes.len() > MAX_TARGETED_PATHS {
        return Err(RetrievalError::Validation(
            "too many targeted code paths".to_string(),
        ));
    }
    let configuration = workspace.configuration()?;
    if !configuration.enabled {
        return Ok(CodeReconcileOutcome::default());
    }
    if should_stop(repository, workspace, cancellation)? {
        repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
        return Ok(CodeReconcileOutcome {
            cancelled: true,
            ..CodeReconcileOutcome::default()
        });
    }
    repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Parsing)?;
    let mut actions = BTreeMap::new();
    for change in changes {
        match change {
            CodePathChange::Upsert(path) => {
                actions.insert(normalize_explicit_path(path)?, true);
            }
            CodePathChange::Delete(path) => {
                actions.insert(normalize_explicit_path(path)?, false);
            }
            CodePathChange::Rename { from, to } => {
                actions.insert(normalize_explicit_path(from)?, false);
                actions.insert(normalize_explicit_path(to)?, true);
            }
        }
    }
    let existing = repository
        .list_file_manifests(&workspace.workspace_id)?
        .into_iter()
        .map(|manifest| (manifest.relative_path.clone(), manifest))
        .collect::<BTreeMap<_, _>>();
    let mut outcome = CodeReconcileOutcome::default();
    for (relative_path, upsert) in actions {
        if should_stop(repository, workspace, cancellation)? {
            outcome.cancelled = true;
            repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
            return Ok(outcome);
        }
        let previous = existing.get(&relative_path);
        if !upsert {
            if previous.is_some() {
                repository.delete_code_file(&workspace.workspace_id, &relative_path)?;
                repository.record_audit(
                    &workspace.workspace_id,
                    Some(&relative_path),
                    CodeIndexAuditEvent::Deleted,
                    None,
                    1,
                )?;
                outcome.deleted += 1;
            }
            continue;
        }
        let Some(file) = inspect_workspace_path(
            Path::new(&workspace.canonical_root),
            &relative_path,
            &configuration,
        )?
        else {
            if previous.is_some() {
                repository.delete_code_file(&workspace.workspace_id, &relative_path)?;
                repository.record_audit(
                    &workspace.workspace_id,
                    Some(&relative_path),
                    CodeIndexAuditEvent::Deleted,
                    None,
                    1,
                )?;
                outcome.deleted += 1;
            }
            continue;
        };
        outcome.discovered += 1;
        reconcile_target_file(
            repository,
            workspace,
            &configuration,
            previous,
            file,
            cancellation,
            &mut outcome,
        )?;
        if outcome.cancelled {
            repository.set_workspace_phase(&workspace.workspace_id, CodeIndexPhase::Cancelling)?;
            return Ok(outcome);
        }
    }
    repository.set_workspace_phase(
        &workspace.workspace_id,
        if outcome.failed == 0 {
            CodeIndexPhase::Ready
        } else {
            CodeIndexPhase::Degraded
        },
    )?;
    Ok(outcome)
}

fn reconcile_target_file(
    repository: &dyn CodeIndexRepository,
    workspace: &CodeWorkspace,
    configuration: &CodeIndexConfigurationUpdate,
    previous: Option<&CodeFileManifest>,
    file: InventoryFile,
    cancellation: &dyn CodeIndexCancellation,
    outcome: &mut CodeReconcileOutcome,
) -> Result<(), RetrievalError> {
    let parsed = match load_and_parse(
        &file.absolute_path,
        &file.relative_path,
        file.language,
        configuration.max_file_bytes,
    ) {
        Ok(parsed) => parsed,
        Err(_) => {
            outcome.failed += 1;
            repository.record_audit(
                &workspace.workspace_id,
                Some(&file.relative_path),
                CodeIndexAuditEvent::Failed,
                Some(CodeIndexAuditReason::Parse),
                1,
            )?;
            return Ok(());
        }
    };
    let manifest = CodeFileManifest {
        workspace_id: workspace.workspace_id.clone(),
        relative_path: file.relative_path.clone(),
        language: file.language.as_str().to_string(),
        byte_size: file.byte_size,
        modified_ns: file.modified_ns,
        content_hash: parsed.raw_content_hash.clone(),
        index_version: CODE_INDEX_VERSION.to_string(),
    };
    if previous.is_some_and(|previous| {
        previous.content_hash == manifest.content_hash
            && previous.index_version == CODE_INDEX_VERSION
    }) {
        if should_stop(repository, workspace, cancellation)? {
            outcome.cancelled = true;
            return Ok(());
        }
        repository.update_file_fingerprint(&manifest)?;
        outcome.metadata_updated += 1;
        return Ok(());
    }
    let symbols = extract_symbols(&file.relative_path, file.language, &parsed)
        .map_err(|error| RetrievalError::Storage(error.as_str().to_string()))?;
    let chunks = chunk_code(
        &workspace.workspace_id,
        &file.relative_path,
        file.language,
        &parsed,
        &symbols,
        DEFAULT_MAX_CHUNK_BYTES,
    );
    let symbol_rows = symbols
        .into_iter()
        .map(|symbol| symbol.symbol)
        .collect::<Vec<_>>();
    if should_stop(repository, workspace, cancellation)? {
        outcome.cancelled = true;
        return Ok(());
    }
    let chunk_count = chunks.len() as u64;
    repository.replace_code_file(&manifest, &chunks, &symbol_rows)?;
    repository.record_audit(
        &workspace.workspace_id,
        Some(&file.relative_path),
        CodeIndexAuditEvent::Admitted,
        None,
        chunk_count,
    )?;
    outcome.replaced += 1;
    Ok(())
}

#[cfg(test)]
#[path = "code_reconciler_tests.rs"]
mod tests;
