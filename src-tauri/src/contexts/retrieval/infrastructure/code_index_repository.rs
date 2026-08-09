use crate::contexts::retrieval::application::{CodeIndexRepository, EMBEDDING_BATCH_SIZE};
use crate::contexts::retrieval::domain::code_index::{
    normalized_workspace_relative_path, CODE_INDEX_VERSION,
};
use crate::contexts::retrieval::domain::{
    code_embedding_identity, content_hash, document_id, redact_code, CodeChunk,
    CodeEmbeddingConfirmation, CodeFileManifest, CodeIndexAuditEntry, CodeIndexAuditEvent,
    CodeIndexAuditReason, CodeIndexConfigurationUpdate, CodeIndexMode, CodeIndexPhase,
    CodeIndexStatus, CodeSearchCandidate, CodeSymbol, CodeWorkspace, CodeWorkspaceOrigin,
    FailureCategory, RetrievalError, SourceKind,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::{DatabaseError, NativeDatabase};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const MAX_AUDIT_ROWS_PER_WORKSPACE: usize = 200;
const MAX_AUDIT_QUERY_ROWS: usize = 100;

#[derive(Clone)]
pub(crate) struct SqliteCodeIndexRepository {
    database: NativeDatabase,
}

impl SqliteCodeIndexRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn register_workspace(
        &self,
        root: &Path,
        display_name: &str,
    ) -> Result<CodeWorkspace, RetrievalError> {
        let canonical = root.canonicalize().map_err(io_error)?;
        if !canonical.is_dir() {
            return Err(RetrievalError::InvalidScope);
        }
        let canonical_root = normalize_windows_extended_length_path(&canonical.to_string_lossy());
        let connection = self.database.connection().map_err(database_error)?;
        if let Some(existing) = load_workspace_by_root(&connection, &canonical_root)? {
            drop(connection);
            return self
                .load_workspace(&existing.workspace_id)?
                .ok_or(RetrievalError::InvalidScope);
        }

        let workspace = CodeWorkspace::new(canonical_root, display_name.trim().to_string());
        let now = SystemClock.rfc3339();
        connection
            .execute(
                r#"
                INSERT INTO code_index_workspaces
                  (workspace_id, canonical_root, display_name, origin, enabled, index_mode, selected_roots_json,
                   languages_json, exclusion_patterns_json, max_file_bytes, index_version,
                   phase, generation, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
                "#,
                params![
                    workspace.workspace_id,
                    workspace.canonical_root,
                    workspace.display_name,
                    workspace.origin.as_str(),
                    workspace.enabled,
                    workspace.mode.as_str(),
                    json(&workspace.selected_roots)?,
                    json(&workspace.languages)?,
                    json(&workspace.exclusion_patterns)?,
                    workspace.max_file_bytes as i64,
                    workspace.index_version,
                    workspace.phase.as_str(),
                    workspace.generation as i64,
                    now,
                ],
            )
            .map_err(storage_error)?;
        Ok(workspace)
    }

    pub(crate) fn ensure_automatic_workspace(
        &self,
        root: &Path,
        display_name: &str,
        mode: CodeIndexMode,
    ) -> Result<(CodeWorkspace, bool), RetrievalError> {
        let canonical = root.canonicalize().map_err(io_error)?;
        if !canonical.is_dir() {
            return Err(RetrievalError::InvalidScope);
        }
        let canonical_root = normalize_windows_extended_length_path(&canonical.to_string_lossy());
        let mut workspace =
            CodeWorkspace::new(canonical_root.clone(), display_name.trim().to_string());
        workspace.enabled = true;
        workspace.mode = mode;
        workspace.origin = CodeWorkspaceOrigin::Automatic;
        workspace.phase = CodeIndexPhase::Scanning;
        workspace.generation = 1;
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        let created = connection
            .execute(
                r#"
                INSERT OR IGNORE INTO code_index_workspaces
                  (workspace_id, canonical_root, display_name, origin, enabled, index_mode, selected_roots_json,
                   languages_json, exclusion_patterns_json, max_file_bytes, index_version,
                   phase, generation, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
                "#,
                params![
                    workspace.workspace_id,
                    workspace.canonical_root,
                    workspace.display_name,
                    workspace.origin.as_str(),
                    workspace.enabled,
                    workspace.mode.as_str(),
                    json(&workspace.selected_roots)?,
                    json(&workspace.languages)?,
                    json(&workspace.exclusion_patterns)?,
                    workspace.max_file_bytes as i64,
                    workspace.index_version,
                    workspace.phase.as_str(),
                    workspace.generation as i64,
                    now,
                ],
            )
            .map_err(storage_error)?
            == 1;
        let existing = load_workspace_by_root(&connection, &canonical_root)?
            .ok_or(RetrievalError::InvalidScope)?;
        drop(connection);
        let loaded = self
            .load_workspace(&existing.workspace_id)?
            .ok_or(RetrievalError::InvalidScope)?;
        Ok((loaded, created))
    }

    pub(crate) fn load_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<CodeWorkspace>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        if workspace_requires_rebuild(&connection, workspace_id)? {
            drop(connection);
            self.invalidate_stale_version(workspace_id)?;
            return self.load_workspace(workspace_id);
        }
        let mut workspace = connection
            .query_row(
                &workspace_select("workspace_id = ?1"),
                [workspace_id],
                read_workspace,
            )
            .optional()
            .map_err(storage_error)?;
        if let Some(workspace) = workspace.as_mut() {
            if !Path::new(&workspace.canonical_root).is_dir() {
                workspace.phase = CodeIndexPhase::Unavailable;
            }
        }
        Ok(workspace)
    }

    pub(crate) fn resolve_workspace(
        &self,
        root: &Path,
    ) -> Result<Option<CodeWorkspace>, RetrievalError> {
        let canonical = root.canonicalize().map_err(io_error)?;
        let canonical_root = normalize_windows_extended_length_path(&canonical.to_string_lossy());
        let connection = self.database.connection().map_err(database_error)?;
        let workspace = load_workspace_by_root(&connection, &canonical_root)?;
        drop(connection);
        match workspace {
            Some(workspace) => self.load_workspace(&workspace.workspace_id),
            None => Ok(None),
        }
    }

    fn invalidate_stale_version(&self, workspace_id: &str) -> Result<(), RetrievalError> {
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM code_index_files WHERE workspace_id = ?1",
                [workspace_id],
            )
            .map_err(storage_error)?;
        let changed = transaction
            .execute(
                r#"
                UPDATE code_index_workspaces
                SET index_version = ?2,
                    phase = CASE WHEN enabled THEN 'scanning' ELSE 'disabled' END,
                    generation = generation + 1,
                    embedding_confirmed_profile = NULL,
                    embedding_confirmed_model = NULL,
                    embedding_confirmed_generation = NULL,
                    updated_at = ?3
                WHERE workspace_id = ?1
                "#,
                params![workspace_id, CODE_INDEX_VERSION, SystemClock.rfc3339()],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(RetrievalError::InvalidScope);
        }
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn save_configuration(
        &self,
        workspace_id: &str,
        update: CodeIndexConfigurationUpdate,
    ) -> Result<CodeWorkspace, RetrievalError> {
        let update = update.validate()?;
        let languages = update
            .languages
            .iter()
            .map(|language| language.as_str().to_string())
            .collect::<Vec<_>>();
        let phase = if update.enabled {
            CodeIndexPhase::Scanning
        } else {
            CodeIndexPhase::Disabled
        };
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed = transaction
            .execute(
                r#"
                UPDATE code_index_workspaces
                SET enabled = ?2, index_mode = ?3, selected_roots_json = ?4, languages_json = ?5,
                    exclusion_patterns_json = ?6, max_file_bytes = ?7, phase = ?8,
                    generation = generation + 1, embedding_confirmed_profile = NULL,
                    embedding_confirmed_model = NULL, embedding_confirmed_generation = NULL,
                    updated_at = ?9
                WHERE workspace_id = ?1
                "#,
                params![
                    workspace_id,
                    update.enabled,
                    update.mode.as_str(),
                    json(&update.selected_roots)?,
                    json(&languages)?,
                    json(&update.exclusion_patterns)?,
                    update.max_file_bytes as i64,
                    phase.as_str(),
                    SystemClock.rfc3339(),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(RetrievalError::InvalidScope);
        }
        if update.mode == CodeIndexMode::Local {
            transaction
                .execute(
                    r#"
                    UPDATE retrieval_documents
                    SET index_state = 'indexed', attempt_count = 0, failure_category = NULL,
                        updated_at = ?2
                    WHERE source_kind = 'workspace_file' AND scope_folder = ?1
                    "#,
                    params![workspace_id, SystemClock.rfc3339()],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        self.load_workspace(workspace_id)?
            .ok_or(RetrievalError::InvalidScope)
    }

    pub(crate) fn replace_file(
        &self,
        manifest: &CodeFileManifest,
        chunks: &[CodeChunk],
        symbols: &[CodeSymbol],
    ) -> Result<(), RetrievalError> {
        if manifest.index_version != CODE_INDEX_VERSION {
            return Err(RetrievalError::Validation(
                "code index version is stale".to_string(),
            ));
        }
        let relative_path = normalized_relative_path(&manifest.relative_path)?;
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM code_index_files WHERE workspace_id = ?1 AND relative_path = ?2",
                params![manifest.workspace_id, relative_path],
            )
            .map_err(storage_error)?;
        let now = SystemClock.rfc3339();
        transaction
            .execute(
                r#"
                INSERT INTO code_index_files
                  (workspace_id, relative_path, language, byte_size, modified_ns, content_hash,
                   index_version, state, chunk_count, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'indexed', ?8, ?9, ?9)
                "#,
                params![
                    manifest.workspace_id,
                    relative_path,
                    manifest.language,
                    manifest.byte_size as i64,
                    manifest.modified_ns,
                    manifest.content_hash,
                    manifest.index_version,
                    chunks.len() as i64,
                    now,
                ],
            )
            .map_err(storage_error)?;
        for chunk in chunks {
            insert_chunk(&transaction, manifest, &relative_path, chunk, &now)?;
        }
        for symbol in symbols {
            transaction
                .execute(
                    r#"
                    INSERT INTO code_index_symbols
                      (symbol_id, workspace_id, relative_path, normalized_name, display_name,
                       symbol_kind, container_name, start_line, end_line)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                    params![
                        symbol.symbol_id,
                        manifest.workspace_id,
                        relative_path,
                        symbol.normalized_name,
                        symbol.display_name,
                        symbol.symbol_kind,
                        symbol.container_name,
                        symbol.start_line,
                        symbol.end_line,
                    ],
                )
                .map_err(storage_error)?;
        }
        let mode = transaction
            .query_row(
                "SELECT index_mode FROM code_index_workspaces WHERE workspace_id = ?1",
                [&manifest.workspace_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage_error)?;
        if mode == CodeIndexMode::Local.as_str() {
            transaction
                .execute(
                    r#"
                    UPDATE retrieval_documents
                    SET index_state = 'indexed', attempt_count = 0, failure_category = NULL,
                        updated_at = ?3
                    WHERE id IN (
                      SELECT document_id FROM code_index_chunks
                      WHERE workspace_id = ?1 AND relative_path = ?2
                    )
                    "#,
                    params![manifest.workspace_id, relative_path, now],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn list_file_manifests(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<CodeFileManifest>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT workspace_id, relative_path, language, byte_size, modified_ns,
                       content_hash, index_version
                FROM code_index_files
                WHERE workspace_id = ?1
                ORDER BY relative_path
                "#,
            )
            .map_err(storage_error)?;
        let manifests = statement
            .query_map([workspace_id], |row| {
                Ok(CodeFileManifest {
                    workspace_id: row.get(0)?,
                    relative_path: row.get(1)?,
                    language: row.get(2)?,
                    byte_size: row.get::<_, i64>(3)? as u64,
                    modified_ns: row.get(4)?,
                    content_hash: row.get(5)?,
                    index_version: row.get(6)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(manifests)
    }

    pub(crate) fn update_file_fingerprint(
        &self,
        manifest: &CodeFileManifest,
    ) -> Result<(), RetrievalError> {
        let relative_path = normalized_relative_path(&manifest.relative_path)?;
        let connection = self.database.connection().map_err(database_error)?;
        let changed = connection
            .execute(
                r#"
                UPDATE code_index_files
                SET language = ?3, byte_size = ?4, modified_ns = ?5, updated_at = ?6
                WHERE workspace_id = ?1 AND relative_path = ?2
                  AND content_hash = ?7 AND index_version = ?8
                "#,
                params![
                    manifest.workspace_id,
                    relative_path,
                    manifest.language,
                    manifest.byte_size as i64,
                    manifest.modified_ns,
                    SystemClock.rfc3339(),
                    manifest.content_hash,
                    CODE_INDEX_VERSION,
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(RetrievalError::Storage(
                "code file changed during reconciliation".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn delete_file(
        &self,
        workspace_id: &str,
        relative_path: &str,
    ) -> Result<(), RetrievalError> {
        let relative_path = normalized_relative_path(relative_path)?;
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .execute(
                "DELETE FROM code_index_files WHERE workspace_id = ?1 AND relative_path = ?2",
                params![workspace_id, relative_path],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(crate) fn delete_workspace(&self, workspace_id: &str) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .execute(
                "DELETE FROM code_index_workspaces WHERE workspace_id = ?1",
                [workspace_id],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(crate) fn rebuild_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<CodeWorkspace, RetrievalError> {
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM code_index_files WHERE workspace_id = ?1",
                [workspace_id],
            )
            .map_err(storage_error)?;
        let changed = transaction
            .execute(
                r#"
                UPDATE code_index_workspaces
                SET generation = generation + 1,
                    phase = CASE WHEN enabled THEN 'scanning' ELSE 'disabled' END,
                    embedding_confirmed_profile = NULL,
                    embedding_confirmed_model = NULL,
                    embedding_confirmed_generation = NULL,
                    updated_at = ?2
                WHERE workspace_id = ?1
                "#,
                params![workspace_id, SystemClock.rfc3339()],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RetrievalError::InvalidScope);
        }
        transaction.commit().map_err(storage_error)?;
        self.load_workspace(workspace_id)?
            .ok_or(RetrievalError::InvalidScope)
    }
}

impl CodeIndexRepository for SqliteCodeIndexRepository {
    fn register_workspace(
        &self,
        root: &Path,
        display_name: &str,
    ) -> Result<CodeWorkspace, RetrievalError> {
        Self::register_workspace(self, root, display_name)
    }

    fn ensure_automatic_workspace(
        &self,
        root: &Path,
        display_name: &str,
        mode: CodeIndexMode,
    ) -> Result<(CodeWorkspace, bool), RetrievalError> {
        Self::ensure_automatic_workspace(self, root, display_name, mode)
    }

    fn list_workspaces(&self) -> Result<Vec<CodeWorkspace>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id FROM code_index_workspaces ORDER BY created_at, workspace_id",
            )
            .map_err(storage_error)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(statement);
        drop(connection);
        ids.into_iter()
            .map(|workspace_id| {
                self.load_workspace(&workspace_id)?
                    .ok_or(RetrievalError::InvalidScope)
            })
            .collect()
    }

    fn load_workspace(&self, workspace_id: &str) -> Result<Option<CodeWorkspace>, RetrievalError> {
        Self::load_workspace(self, workspace_id)
    }

    fn save_workspace_configuration(
        &self,
        workspace_id: &str,
        update: CodeIndexConfigurationUpdate,
    ) -> Result<CodeWorkspace, RetrievalError> {
        self.save_configuration(workspace_id, update)
    }

    fn rebuild_workspace(&self, workspace_id: &str) -> Result<CodeWorkspace, RetrievalError> {
        Self::rebuild_workspace(self, workspace_id)
    }

    fn delete_workspace(&self, workspace_id: &str) -> Result<(), RetrievalError> {
        Self::delete_workspace(self, workspace_id)
    }

    fn workspace_generation(&self, workspace_id: &str) -> Result<Option<u64>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .query_row(
                "SELECT generation FROM code_index_workspaces WHERE workspace_id = ?1",
                [workspace_id],
                |row| row.get::<_, i64>(0).map(|generation| generation as u64),
            )
            .optional()
            .map_err(storage_error)
    }

    fn set_workspace_phase(
        &self,
        workspace_id: &str,
        phase: CodeIndexPhase,
    ) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let changed = connection
            .execute(
                "UPDATE code_index_workspaces SET phase = ?2, updated_at = ?3 WHERE workspace_id = ?1",
                params![workspace_id, phase.as_str(), SystemClock.rfc3339()],
            )
            .map_err(storage_error)?;
        (changed == 1)
            .then_some(())
            .ok_or(RetrievalError::InvalidScope)
    }

    fn workspace_status(&self, workspace_id: &str) -> Result<CodeIndexStatus, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .query_row(
                r#"
                SELECT phase, updated_at,
                  (SELECT COUNT(*) FROM code_index_files WHERE workspace_id = ?1),
                  (SELECT COUNT(*) FROM code_index_files
                   WHERE workspace_id = ?1 AND state IN ('indexed', 'failed')),
                  (SELECT COUNT(*) FROM code_index_files
                   WHERE workspace_id = ?1 AND state = 'failed'),
                  (SELECT COUNT(*) FROM code_index_chunks WHERE workspace_id = ?1),
                  (SELECT COUNT(*) FROM code_index_chunks AS chunk
                   JOIN retrieval_documents AS document ON document.id = chunk.document_id
                   WHERE chunk.workspace_id = ?1 AND document.index_state IN ('indexed', 'failed')),
                  (SELECT COUNT(*) FROM code_index_chunks AS chunk
                   JOIN retrieval_documents AS document ON document.id = chunk.document_id
                   WHERE chunk.workspace_id = ?1 AND document.index_state = 'pending'),
                  (SELECT COUNT(*) FROM code_index_chunks AS chunk
                   JOIN retrieval_documents AS document ON document.id = chunk.document_id
                   WHERE chunk.workspace_id = ?1 AND document.index_state = 'indexed'),
                  (SELECT COUNT(*) FROM code_index_chunks AS chunk
                   JOIN retrieval_documents AS document ON document.id = chunk.document_id
                   WHERE chunk.workspace_id = ?1 AND document.index_state = 'failed'),
                  (SELECT COALESCE(SUM(redaction_count), 0) FROM code_index_chunks
                   WHERE workspace_id = ?1),
                  (SELECT failure_category FROM retrieval_documents
                   WHERE source_kind = 'workspace_file' AND scope_folder = ?1
                     AND failure_category IS NOT NULL
                   ORDER BY updated_at DESC, id DESC LIMIT 1),
                  index_mode
                FROM code_index_workspaces WHERE workspace_id = ?1
                "#,
                [workspace_id],
                read_code_index_status,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(RetrievalError::InvalidScope)
    }

    fn embedding_confirmation(
        &self,
        workspace_id: &str,
    ) -> Result<Option<CodeEmbeddingConfirmation>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let confirmation = connection
            .query_row(
                r#"
                SELECT embedding_confirmed_profile, embedding_confirmed_model,
                       embedding_confirmed_generation
                FROM code_index_workspaces WHERE workspace_id = ?1
                "#,
                [workspace_id],
                |row| {
                    let profile_id: Option<String> = row.get(0)?;
                    let model: Option<String> = row.get(1)?;
                    let generation: Option<i64> = row.get(2)?;
                    match (profile_id, model, generation) {
                        (Some(profile_id), Some(model), Some(generation)) => {
                            Ok(Some(CodeEmbeddingConfirmation {
                                profile_id,
                                model,
                                generation: generation as u64,
                            }))
                        }
                        (None, None, None) => Ok(None),
                        _ => Err(rusqlite::Error::InvalidQuery),
                    }
                },
            )
            .optional()
            .map_err(storage_error)?;
        confirmation.ok_or(RetrievalError::InvalidScope)
    }

    fn confirm_embedding(
        &self,
        workspace_id: &str,
        profile_id: &str,
        model: &str,
        generation: u64,
    ) -> Result<CodeEmbeddingConfirmation, RetrievalError> {
        let profile_id = profile_id.trim();
        let model = model.trim();
        if profile_id.is_empty() || model.is_empty() {
            return Err(RetrievalError::Validation(
                "embedding profile and model are required".to_string(),
            ));
        }
        let generation = i64::try_from(generation).map_err(|_| {
            RetrievalError::Validation("workspace generation is too large".to_string())
        })?;
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed = transaction
            .execute(
                r#"
                UPDATE code_index_workspaces
                SET embedding_confirmed_profile = ?2, embedding_confirmed_model = ?3,
                    embedding_confirmed_generation = ?4, phase = 'embedding', updated_at = ?5
                WHERE workspace_id = ?1 AND enabled = 1 AND index_mode = 'semantic'
                  AND generation = ?4
                "#,
                params![
                    workspace_id,
                    profile_id,
                    model,
                    generation,
                    SystemClock.rfc3339()
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(RetrievalError::Validation(
                "workspace embedding confirmation is stale".to_string(),
            ));
        }
        transaction
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL,
                    updated_at = ?3
                WHERE source_kind = 'workspace_file' AND scope_folder = ?1
                  AND (index_state = 'failed' OR embedding_model IS NULL OR embedding_model <> ?2)
                "#,
                params![
                    workspace_id,
                    code_embedding_identity(profile_id, model),
                    SystemClock.rfc3339()
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(CodeEmbeddingConfirmation {
            profile_id: profile_id.to_string(),
            model: model.to_string(),
            generation: generation as u64,
        })
    }

    fn record_audit(
        &self,
        workspace_id: &str,
        relative_path: Option<&str>,
        event: CodeIndexAuditEvent,
        reason: Option<CodeIndexAuditReason>,
        item_count: u64,
    ) -> Result<(), RetrievalError> {
        let relative_path = relative_path.map(normalized_relative_path).transpose()?;
        let item_count = i64::try_from(item_count)
            .map_err(|_| RetrievalError::Validation("audit count is too large".to_string()))?;
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                r#"
                INSERT INTO code_index_audit
                  (workspace_id, relative_path, event_kind, reason_category, item_count, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    workspace_id,
                    relative_path,
                    event.as_str(),
                    reason.map(CodeIndexAuditReason::as_str),
                    item_count,
                    SystemClock.rfc3339(),
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"
                DELETE FROM code_index_audit
                WHERE workspace_id = ?1 AND audit_id NOT IN (
                  SELECT audit_id FROM code_index_audit WHERE workspace_id = ?1
                  ORDER BY audit_id DESC LIMIT ?2
                )
                "#,
                params![workspace_id, MAX_AUDIT_ROWS_PER_WORKSPACE as i64],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn list_audit(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<CodeIndexAuditEntry>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM code_index_workspaces WHERE workspace_id = ?1)",
                [workspace_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !exists {
            return Err(RetrievalError::InvalidScope);
        }
        let mut statement = connection
            .prepare(
                r#"
                SELECT audit_id, workspace_id, relative_path, event_kind, reason_category,
                       item_count, created_at
                FROM code_index_audit WHERE workspace_id = ?1
                ORDER BY audit_id DESC LIMIT ?2
                "#,
            )
            .map_err(storage_error)?;
        let records = statement
            .query_map(
                params![workspace_id, limit.min(MAX_AUDIT_QUERY_ROWS) as i64],
                read_code_index_audit,
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(records)
    }

    fn load_code_candidates(
        &self,
        workspace_id: &str,
        source_ids: &[String],
    ) -> Result<Vec<CodeSearchCandidate>, RetrievalError> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.database.connection().map_err(database_error)?;
        // One round-trip for all candidates instead of one per source_id. `?1` previously
        // bound both `chunk.workspace_id` and `document.scope_folder` — two semantically
        // distinct columns — which could mask a cross-workspace leak; they are now separate
        // parameters even though both carry the workspace id today.
        let placeholders = (0..source_ids.len())
            .map(|index| format!("?{}", index + 3))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            r#"
            SELECT document.source_id, chunk.relative_path, chunk.start_line, chunk.end_line,
                   chunk.language, chunk.symbol_name, chunk.symbol_kind, document.content
            FROM code_index_chunks AS chunk
            JOIN retrieval_documents AS document ON document.id = chunk.document_id
            WHERE chunk.workspace_id = ?1 AND document.source_kind = 'workspace_file'
              AND document.scope_folder = ?2 AND document.source_id IN ({placeholders})
            "#,
        );
        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let params: Vec<&dyn rusqlite::ToSql> = {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&workspace_id, &workspace_id];
            for source_id in source_ids {
                params.push(source_id);
            }
            params
        };
        let candidates = statement
            .query_map(params.as_slice(), |row| {
                Ok(CodeSearchCandidate {
                    source_id: row.get(0)?,
                    file_path: row.get(1)?,
                    start_line: row.get::<_, i64>(2)? as u32,
                    end_line: row.get::<_, i64>(3)? as u32,
                    language: row.get(4)?,
                    symbol_name: row.get(5)?,
                    symbol_kind: row.get(6)?,
                    snippet: row.get(7)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(candidates)
    }

    fn list_file_manifests(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<CodeFileManifest>, RetrievalError> {
        Self::list_file_manifests(self, workspace_id)
    }

    fn replace_code_file(
        &self,
        manifest: &CodeFileManifest,
        chunks: &[CodeChunk],
        symbols: &[CodeSymbol],
    ) -> Result<(), RetrievalError> {
        self.replace_file(manifest, chunks, symbols)
    }

    fn update_file_fingerprint(&self, manifest: &CodeFileManifest) -> Result<(), RetrievalError> {
        Self::update_file_fingerprint(self, manifest)
    }

    fn delete_code_file(
        &self,
        workspace_id: &str,
        relative_path: &str,
    ) -> Result<(), RetrievalError> {
        self.delete_file(workspace_id, relative_path)
    }
}

fn read_code_index_status(row: &rusqlite::Row<'_>) -> Result<CodeIndexStatus, rusqlite::Error> {
    let phase: String = row.get(0)?;
    let total_chunks = row.get::<_, i64>(5)? as u64;
    let last_failure: Option<String> = row.get(11)?;
    let mode =
        CodeIndexMode::parse(&row.get::<_, String>(12)?).ok_or(rusqlite::Error::InvalidQuery)?;
    let local = mode == CodeIndexMode::Local;
    Ok(CodeIndexStatus {
        phase: CodeIndexPhase::parse(&phase).ok_or(rusqlite::Error::InvalidQuery)?,
        updated_at: row.get(1)?,
        total_files: row.get::<_, i64>(2)? as u64,
        processed_files: row.get::<_, i64>(3)? as u64,
        failed_files: row.get::<_, i64>(4)? as u64,
        total_chunks,
        processed_chunks: if local {
            total_chunks
        } else {
            row.get::<_, i64>(6)? as u64
        },
        pending_chunks: if local {
            0
        } else {
            row.get::<_, i64>(7)? as u64
        },
        indexed_chunks: if local {
            total_chunks
        } else {
            row.get::<_, i64>(8)? as u64
        },
        failed_chunks: if local {
            0
        } else {
            row.get::<_, i64>(9)? as u64
        },
        redaction_count: row.get::<_, i64>(10)? as u64,
        estimated_embedding_requests: if local {
            0
        } else {
            total_chunks.div_ceil(EMBEDDING_BATCH_SIZE as u64)
        },
        last_failure_category: last_failure
            .map(|category| FailureCategory::parse(&category).ok_or(rusqlite::Error::InvalidQuery))
            .transpose()?,
    })
}

fn read_code_index_audit(row: &rusqlite::Row<'_>) -> Result<CodeIndexAuditEntry, rusqlite::Error> {
    let event: String = row.get(3)?;
    let reason: Option<String> = row.get(4)?;
    Ok(CodeIndexAuditEntry {
        audit_id: row.get::<_, i64>(0)? as u64,
        workspace_id: row.get(1)?,
        relative_path: row.get(2)?,
        event: CodeIndexAuditEvent::parse(&event).ok_or(rusqlite::Error::InvalidQuery)?,
        reason: reason
            .map(|value| CodeIndexAuditReason::parse(&value).ok_or(rusqlite::Error::InvalidQuery))
            .transpose()?,
        item_count: row.get::<_, i64>(5)? as u64,
        created_at: row.get(6)?,
    })
}

fn insert_chunk(
    connection: &Connection,
    manifest: &CodeFileManifest,
    relative_path: &str,
    chunk: &CodeChunk,
    now: &str,
) -> Result<(), RetrievalError> {
    let id = document_id(SourceKind::WorkspaceFile, &chunk.source_id);
    let redacted = redact_code(&chunk.content);
    connection
        .execute(
            r#"
            INSERT INTO retrieval_documents
              (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
               created_at, updated_at)
            VALUES (?1, 'workspace_file', ?2, '', ?3, ?4, ?5, ?6, ?6)
            "#,
            params![
                id,
                chunk.source_id,
                manifest.workspace_id,
                redacted.text,
                content_hash(&redacted.text),
                now
            ],
        )
        .map_err(storage_error)?;
    connection
        .execute(
            r#"
            INSERT INTO code_index_chunks
              (document_id, workspace_id, relative_path, language, start_line, end_line,
               symbol_name, symbol_kind, chunk_ordinal, chunk_key, redaction_count, index_version)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                id,
                manifest.workspace_id,
                relative_path,
                chunk.language,
                chunk.start_line,
                chunk.end_line,
                chunk.symbol_name,
                chunk.symbol_kind,
                chunk.ordinal,
                chunk.chunk_key,
                chunk.redaction_count.saturating_add(redacted.count),
                manifest.index_version,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn load_workspace_by_root(
    connection: &Connection,
    canonical_root: &str,
) -> Result<Option<CodeWorkspace>, RetrievalError> {
    connection
        .query_row(
            &workspace_select("canonical_root = ?1 COLLATE NOCASE"),
            [canonical_root],
            read_workspace,
        )
        .optional()
        .map_err(storage_error)
}

fn workspace_requires_rebuild(
    connection: &Connection,
    workspace_id: &str,
) -> Result<bool, RetrievalError> {
    connection
        .query_row(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM code_index_workspaces AS workspace
                WHERE workspace.workspace_id = ?1
                  AND (
                    workspace.index_version <> ?2
                    OR EXISTS (
                        SELECT 1 FROM code_index_files AS file
                        WHERE file.workspace_id = workspace.workspace_id
                          AND file.index_version <> ?2
                    )
                  )
            )
            "#,
            params![workspace_id, CODE_INDEX_VERSION],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

fn workspace_select(filter: &str) -> String {
    format!(
        "SELECT workspace_id, canonical_root, display_name, origin, enabled, index_mode, selected_roots_json,
         languages_json, exclusion_patterns_json, max_file_bytes, index_version, phase, generation
         FROM code_index_workspaces WHERE {filter}"
    )
}

fn read_workspace(row: &rusqlite::Row<'_>) -> Result<CodeWorkspace, rusqlite::Error> {
    let origin: String = row.get(3)?;
    let mode: String = row.get(5)?;
    let phase: String = row.get(11)?;
    Ok(CodeWorkspace {
        workspace_id: row.get(0)?,
        canonical_root: row.get(1)?,
        display_name: row.get(2)?,
        origin: CodeWorkspaceOrigin::parse(&origin).ok_or(rusqlite::Error::InvalidQuery)?,
        enabled: row.get(4)?,
        mode: CodeIndexMode::parse(&mode).ok_or(rusqlite::Error::InvalidQuery)?,
        selected_roots: json_column(row, 6)?,
        languages: language_column(row, 7)?,
        exclusion_patterns: json_column(row, 8)?,
        max_file_bytes: row.get::<_, i64>(9)? as u64,
        index_version: row.get(10)?,
        phase: CodeIndexPhase::parse(&phase).unwrap_or(CodeIndexPhase::Unavailable),
        generation: row.get::<_, i64>(12)? as u64,
    })
}

fn normalized_relative_path(value: &str) -> Result<String, RetrievalError> {
    normalized_workspace_relative_path(value).ok_or(RetrievalError::InvalidScope)
}

fn json(values: &[String]) -> Result<String, RetrievalError> {
    serde_json::to_string(values).map_err(|error| RetrievalError::Storage(error.to_string()))
}

fn json_column(row: &rusqlite::Row<'_>, index: usize) -> Result<Vec<String>, rusqlite::Error> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn language_column(row: &rusqlite::Row<'_>, index: usize) -> Result<Vec<String>, rusqlite::Error> {
    let languages = json_column(row, index)?;
    if languages
        .iter()
        .any(|language| crate::contexts::retrieval::domain::CodeLanguage::parse(language).is_none())
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(languages)
}

fn database_error(error: DatabaseError) -> RetrievalError {
    match error {
        DatabaseError::Database(error) => storage_error(error),
        DatabaseError::Storage(message) => RetrievalError::Storage(message),
    }
}

fn storage_error(error: rusqlite::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}

fn io_error(error: std::io::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}

#[cfg(test)]
#[path = "code_index_repository_tests.rs"]
mod tests;
