use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::{
    AgentMemoryDeletionGateway, AgentRuntimeApi, AgentRuntimeApplicationError,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::retrieval::api::RetrievalApi;
use crate::contexts::retrieval::domain::{RetrievalError, SourceKind};
use crate::platform::logging::fallback_log_dir;
use std::collections::BTreeMap;
use tauri::State;

const REVOCATION_FAILURE_CATEGORY: &str = "retrieval.indexing.revocation";

/// Boundary this command needs from `retrieval` to revoke a memory's index entry after its delete
/// succeeds (`add-onepiece-vector-search` design doc §5.3). A trait — rather than calling
/// `RetrievalApi::remove` directly — so this file's own tests can substitute a fake instead of
/// constructing a real `RetrievalApi`, which would otherwise need a document repository, a
/// configuration repository, and a search service just to exercise one delegate call.
///
/// `agent_runtime` and `retrieval` never import each other (`bootstrap` is their only meeting
/// point); this command file is neither of those contexts, so it is free to depend on both
/// published `api` modules directly, the same way `commands::desktop::open_session_folder`
/// already composes `desktop::api` and `workspaces::api`.
pub(crate) trait AgentMemoryIndexRevocationPort {
    fn remove(&self, source_kind: SourceKind, source_id: &str) -> Result<(), RetrievalError>;
}

/// The real implementation also logs a failed revocation (`log_revocation_failure`): warn
/// severity, category-only context, and it never affects the return value seen by
/// `delete_agent_memory_with`'s caller (below), which discards it — a failed revocation must
/// never fail the delete. Two independent backstops make swallowing it safe: `retrieval::search`
/// resolves every hit against the source table (a vanished source is skipped, so a deleted memory
/// can never be returned even with a stale index row), and `reconcile`'s orphan cleanup
/// eventually removes the residual row.
impl AgentMemoryIndexRevocationPort for RetrievalApi {
    fn remove(&self, source_kind: SourceKind, source_id: &str) -> Result<(), RetrievalError> {
        let outcome = RetrievalApi::remove(self, source_kind, source_id);
        if let Err(error) = &outcome {
            log_revocation_failure(error);
        }
        outcome
    }
}

#[tauri::command]
pub(crate) fn delete_agent_memory(
    api: State<'_, AgentRuntimeApi>,
    retrieval: State<'_, RetrievalApi>,
    memory_id: String,
) -> Result<(), CommandError> {
    delete_agent_memory_with(&*api, &*retrieval, &memory_id).map_err(map_command_error)
}

/// Deletes the memory, then best-effort revokes its retrieval index entry — the revocation
/// outcome is deliberately discarded (see `AgentMemoryIndexRevocationPort for RetrievalApi`'s own
/// doc comment for why swallowing it is safe). Kept separate from the `#[tauri::command]` above
/// purely for testability with fakes; production and tests share this exact function.
fn delete_agent_memory_with(
    memories: &dyn AgentMemoryDeletionGateway,
    retrieval: &dyn AgentMemoryIndexRevocationPort,
    memory_id: &str,
) -> Result<(), AgentRuntimeApplicationError> {
    memories.delete_agent_memory(memory_id)?;
    let _ = retrieval.remove(SourceKind::AgentMemory, memory_id);
    Ok(())
}

/// Fixed message, category-only context (design doc §8.2) — never interpolates the underlying
/// `RetrievalError` payload (which may carry storage-layer error text, e.g. `RetrievalError::
/// Storage`) or the memory id into anything written to disk.
fn log_revocation_failure(error: &RetrievalError) {
    let logging = UnifiedLoggingAdapter::active(fallback_log_dir());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Warn,
        category: REVOCATION_FAILURE_CATEGORY.to_string(),
        message: "Retrieval index revocation failed after a memory delete; the residual index row will be cleaned up by the next reconcile pass"
            .to_string(),
        context: BTreeMap::from([(
            "category".to_string(),
            error_category(error).to_string(),
        )]),
    });
}

fn error_category(error: &RetrievalError) -> &'static str {
    match error {
        RetrievalError::Storage(_) => "storage",
        RetrievalError::Embedding(_) => "embedding",
        RetrievalError::NotConfigured => "not_configured",
        RetrievalError::InvalidScope => "invalid_scope",
        RetrievalError::Validation(_) => "validation",
        RetrievalError::Unavailable => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeMemories {
        deleted: Mutex<Vec<String>>,
    }

    impl AgentMemoryDeletionGateway for FakeMemories {
        fn delete_agent_memory(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            self.deleted
                .lock()
                .expect("lock")
                .push(memory_id.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeRetrieval {
        removed: Mutex<Vec<String>>,
        remove_fails: bool,
    }

    impl AgentMemoryIndexRevocationPort for FakeRetrieval {
        fn remove(&self, _source_kind: SourceKind, source_id: &str) -> Result<(), RetrievalError> {
            if self.remove_fails {
                return Err(RetrievalError::Storage("simulated failure".to_string()));
            }
            self.removed
                .lock()
                .expect("lock")
                .push(source_id.to_string());
            Ok(())
        }
    }

    #[test]
    fn deleting_a_memory_revokes_its_retrieval_index_entry() {
        let memories = FakeMemories::default();
        let retrieval = FakeRetrieval::default();

        delete_agent_memory_with(&memories, &retrieval, "m1").expect("delete");

        assert_eq!(
            *retrieval.removed.lock().expect("lock"),
            vec!["m1".to_string()]
        );
    }

    #[test]
    fn a_failed_revocation_does_not_fail_the_delete() {
        // Two independent backstops make it safe to swallow this error here: the search path
        // resolves every hit against the source table (a deleted memory is never returned even
        // with a stale index row), and `reconcile`'s orphan cleanup eventually removes the
        // residual row.
        let memories = FakeMemories::default();
        let retrieval = FakeRetrieval {
            remove_fails: true,
            ..FakeRetrieval::default()
        };

        let result = delete_agent_memory_with(&memories, &retrieval, "m1");

        assert!(result.is_ok());
    }

    /// Guards the ordering, not just the outcome: revocation must only ever run after a
    /// *successful* delete. `PanicsIfCalled` turns any call into a hard test failure rather than a
    /// silent pass, so this would fail loudly if the two steps were ever reordered or the early
    /// `?` removed.
    #[test]
    fn a_failed_delete_never_reaches_the_revocation_step() {
        struct AlwaysFailingMemories;
        impl AgentMemoryDeletionGateway for AlwaysFailingMemories {
            fn delete_agent_memory(
                &self,
                _memory_id: &str,
            ) -> Result<(), AgentRuntimeApplicationError> {
                Err(AgentRuntimeApplicationError::Validation("boom".to_string()))
            }
        }
        struct PanicsIfCalled;
        impl AgentMemoryIndexRevocationPort for PanicsIfCalled {
            fn remove(
                &self,
                _source_kind: SourceKind,
                _source_id: &str,
            ) -> Result<(), RetrievalError> {
                panic!("revocation must not run when the delete itself failed")
            }
        }

        let result = delete_agent_memory_with(&AlwaysFailingMemories, &PanicsIfCalled, "m1");

        assert!(result.is_err());
    }

    /// Mirrors `bootstrap::retrieval`'s own `error_category` test: proves the mapping never
    /// forwards the wrapped error text, which is the only thing standing between a revocation
    /// failure and a storage error string reaching the unified log.
    #[test]
    fn error_category_maps_each_variant_without_the_payload_text() {
        assert_eq!(
            error_category(&RetrievalError::Storage("SENSITIVE-SENTINEL".to_string())),
            "storage"
        );
        assert_eq!(
            error_category(&RetrievalError::Embedding("SENSITIVE-SENTINEL".to_string())),
            "embedding"
        );
        assert_eq!(
            error_category(&RetrievalError::NotConfigured),
            "not_configured"
        );
        assert_eq!(error_category(&RetrievalError::Unavailable), "unavailable");
    }
}
