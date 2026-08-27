use crate::contexts::workspaces::application::{
    detect_encoding, detect_newline, kind_rank, DirectoryCursor, DirectoryEntry,
    DirectoryFingerprint, DirectoryFingerprintState, DirectoryListing, DocumentListing,
    FileContent, FileSearchListing, GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffResult,
    GitDiffSource, GitStatusEntry, GitStatusResult, ReviewDiffFile, ReviewDiffHunk,
    ReviewFileSummary, ReviewPatch, ReviewPatchRequest, ReviewRevertReceipt, ReviewRevertRequest,
    ReviewSnapshot, SessionDocument, SessionLogEntry, SessionLogExportResult, SessionLogPage,
    SessionLogQuery, SessionWorkspaceContext, WorkspaceApplicationError as AppError,
    WorkspaceLogLevel, WorkspaceReviewPort, WorkspaceSessionQueryPort, DEFAULT_DIRECTORY_PAGE_SIZE,
    MAX_FINGERPRINT_PATHS, MAX_REVIEW_DIFF_BYTES, MAX_REVIEW_FILES, MAX_REVIEW_FILE_BYTES,
    MAX_REVIEW_PATCH_BYTES,
};
use crate::contexts::workspaces::application::{
    WorkspaceContentSearchRequest, WorkspaceContentSearchResult, WorkspacePathSearchRequest,
    WorkspacePathSearchResult,
};
use crate::contexts::workspaces::domain::{CanonicalPathBoundary, WorkspaceRelativePath};
use crate::platform;
use crate::platform::database::{NativeDatabase, PooledSqlite};
use crate::platform::logging;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::Digest;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// How many status entries one answer carries.
///
/// Its own constant now that directory listings page. The two were sharing a name and a value
/// while measuring different things — files in one folder, changed paths across a repository —
/// and a shared name is how one of them silently acquires the other's bound.
const GIT_STATUS_ENTRY_LIMIT: usize = 500;
const DOCUMENT_DEPTH_LIMIT: usize = 6;
const DOCUMENT_LIMIT: usize = 300;
const FILE_BYTE_LIMIT: u64 = 1024 * 1024;
const DIFF_BYTE_LIMIT: usize = 2 * 1024 * 1024;
const LOG_PAGE_LIMIT: usize = 200;
const LOG_QUERY_BYTE_LIMIT: u64 = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct SessionWorkspaceQueryAdapter {
    database: NativeDatabase,
    app: AppHandle,
}

impl SessionWorkspaceQueryAdapter {
    pub(crate) fn new(database: NativeDatabase, app: AppHandle) -> Self {
        Self { database, app }
    }

    fn connection(&self) -> Result<PooledSqlite, AppError> {
        self.database
            .connection()
            .map_err(|error| AppError::Repository(error.to_string()))
    }
}

impl WorkspaceSessionQueryPort for SessionWorkspaceQueryAdapter {
    fn resolve_session_root(&self, session_id: &str) -> Result<Option<String>, AppError> {
        resolve_session_root(&*self.connection()?, session_id)
            .map(|root| root.map(|path| path.to_string_lossy().to_string()))
    }

    fn resolve_session_directory(
        &self,
        session_id: &str,
        relative: &str,
    ) -> Result<Option<String>, AppError> {
        let connection = self.connection()?;
        let Some(root) = resolve_session_root(&connection, session_id)? else {
            return Ok(None);
        };
        if relative.is_empty() {
            return Ok(Some(root.to_string_lossy().to_string()));
        }
        // The same confinement every other read uses. A second one written for this caller would
        // be a second boundary, and boundaries written twice disagree.
        let resolved = resolve_existing_path(&root, relative)?;
        if !resolved.is_dir() {
            return Err(AppError::Validation(
                "Requested workspace path is not a directory.".to_string(),
            ));
        }
        Ok(Some(resolved.to_string_lossy().to_string()))
    }

    fn list_directory(&self, session_id: &str, path: &str) -> Result<DirectoryListing, AppError> {
        list_session_directory(&*self.connection()?, session_id, path)
    }

    fn list_directory_page(
        &self,
        session_id: &str,
        path: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DirectoryListing, AppError> {
        list_session_directory_page(&*self.connection()?, session_id, path, cursor, limit)
    }

    fn directory_fingerprints(
        &self,
        session_id: &str,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, AppError> {
        session_directory_fingerprints(&*self.connection()?, session_id, paths)
    }

    fn search_paths(
        &self,
        session_id: &str,
        request: &WorkspacePathSearchRequest,
    ) -> Result<WorkspacePathSearchResult, AppError> {
        super::path_search::search_session_paths(&*self.connection()?, session_id, request)
    }

    fn search_content(
        &self,
        session_id: &str,
        request: &WorkspaceContentSearchRequest,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<WorkspaceContentSearchResult, AppError> {
        super::content_search::search_session_content(
            &*self.connection()?,
            session_id,
            request,
            cancelled,
        )
    }

    fn list_documents(&self, session_id: &str) -> Result<DocumentListing, AppError> {
        list_session_documents(&*self.connection()?, session_id)
    }

    fn search_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, AppError> {
        super::session_search::search_session_files(
            &*self.connection()?,
            session_id,
            query,
            max_results,
        )
    }

    fn read_file(&self, session_id: &str, path: &str) -> Result<FileContent, AppError> {
        read_session_file(&*self.connection()?, session_id, path)
    }

    fn read_text_file(&self, session_id: &str, path: &str) -> Result<FileContent, AppError> {
        read_session_text_file(&*self.connection()?, session_id, path)
    }

    fn git_status(&self, session_id: &str) -> Result<GitStatusResult, AppError> {
        get_session_git_status(&*self.connection()?, session_id)
    }

    fn git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, AppError> {
        get_session_git_diff(&*self.connection()?, session_id, path, source)
    }

    fn list_logs(&self, query: &SessionLogQuery) -> Result<SessionLogPage, AppError> {
        list_session_logs(&*self.connection()?, query)
    }

    fn export_logs(&self, query: &SessionLogQuery) -> Result<SessionLogExportResult, AppError> {
        export_session_logs(&self.app, &*self.connection()?, query)
    }
}

impl WorkspaceReviewPort for SessionWorkspaceQueryAdapter {
    fn create_review_snapshot(&self, session_id: &str) -> Result<ReviewSnapshot, AppError> {
        let connection = self.connection()?;
        let snapshot = create_review_snapshot(&connection, session_id)?;
        write_review_event(
            &connection,
            "snapshot-created",
            session_id,
            snapshot.files.len(),
        );
        Ok(snapshot)
    }

    fn load_review_file(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<ReviewDiffFile, AppError> {
        let connection = self.connection()?;
        let file = load_review_file(&connection, session_id, path, expected_snapshot)?;
        write_review_event(&connection, "diff-loaded", session_id, file.hunks.len());
        Ok(file)
    }

    fn render_review_patch(&self, request: &ReviewPatchRequest) -> Result<ReviewPatch, AppError> {
        let connection = self.connection()?;
        render_review_patch(&connection, request)
    }

    fn revert_review_change(
        &self,
        request: &ReviewRevertRequest,
    ) -> Result<ReviewRevertReceipt, AppError> {
        let connection = self.connection()?;
        let receipt = revert_review_change(&connection, request)?;
        write_review_event(
            &connection,
            "change-reverted",
            &request.session_id,
            receipt.reverted_hunks,
        );
        Ok(receipt)
    }
}

fn write_review_event(conn: &Connection, kind: &str, session_id: &str, item_count: usize) {
    let Ok(log_dir) = active_log_dir_from_conn(conn) else {
        return;
    };
    let context = BTreeMap::from([
        ("sessionId".to_string(), session_id.to_string()),
        ("itemCount".to_string(), item_count.to_string()),
    ]);
    let _ = logging::write_message(
        &log_dir,
        logging::LogLevel::Info,
        "session.code-review",
        kind,
        context,
    );
}

struct SessionWorkspaceRecord {
    agent_id: String,
    folder: Option<String>,
    project_path: Option<String>,
    worktree_path: Option<String>,
    remote_workspace: bool,
}

fn load_session_workspace(
    conn: &Connection,
    session_id: &str,
) -> Result<SessionWorkspaceRecord, AppError> {
    conn.query_row(
        "SELECT agent_id, folder, project_path, worktree_path, remote_workspace_host, \
         remote_workspace_path, remote_workspace_display_name, remote_workspace_uri \
         FROM sessions WHERE id = ?1",
        params![session_id],
        |row| {
            let remote_host = row.get::<_, Option<String>>(4)?;
            let remote_path = row.get::<_, Option<String>>(5)?;
            let remote_display_name = row.get::<_, Option<String>>(6)?;
            let remote_uri = row.get::<_, Option<String>>(7)?;
            Ok(SessionWorkspaceRecord {
                agent_id: row.get(0)?,
                folder: row.get(1)?,
                project_path: row.get(2)?,
                worktree_path: row.get(3)?,
                remote_workspace: remote_host.is_some()
                    && remote_path.is_some()
                    && remote_display_name.is_some()
                    && remote_uri.is_some(),
            })
        },
    )
    .optional()
    .map_err(|error| AppError::Repository(error.to_string()))?
    .ok_or_else(|| AppError::SessionNotFound(session_id.to_string()))
}

pub(crate) fn resolve_session_root(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<PathBuf>, AppError> {
    let session = load_session_workspace(conn, session_id)?;
    if session.remote_workspace {
        return Ok(None);
    }
    for candidate in [
        session.worktree_path.as_deref(),
        session.folder.as_deref(),
        session.project_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(root) = canonical_workspace_root(Some(candidate))? {
            return Ok(Some(root));
        }
    }
    Ok(None)
}

fn canonical_workspace_root(candidate: Option<&str>) -> Result<Option<PathBuf>, AppError> {
    let candidate = candidate.map(Path::new);
    platform::filesystem::canonical_directory_if_available(candidate)
        .map_err(map_filesystem_storage_error)
}

fn validate_relative_path(path: &str) -> Result<PathBuf, AppError> {
    WorkspaceRelativePath::parse(path)
        .map(|path| path.into_path_buf())
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn resolve_existing_path(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let boundary = workspace_boundary(root)?;
    boundary.resolve_existing(relative).map_err(|error| {
        map_workspace_boundary_error(
            error,
            "Session workspace path resolves outside the session root.",
        )
    })
}

fn resolve_git_path(root: &Path, relative: &str) -> Result<(PathBuf, String), AppError> {
    let boundary = workspace_boundary(root)?;
    boundary
        .resolve_with_existing_parent(relative)
        .map_err(|error| {
            map_workspace_boundary_error(error, "Git path resolves outside the session root.")
        })
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, AppError> {
    CanonicalPathBoundary::new(root)
        .relative(path)
        .map_err(|_| AppError::Validation("Path resolves outside the session root.".to_string()))
}

fn workspace_boundary(root: &Path) -> Result<platform::filesystem::BoundedFilesystem, AppError> {
    platform::filesystem::BoundedFilesystem::new(root).map_err(map_filesystem_storage_error)
}

fn map_relative_path_error(error: platform::filesystem::BoundaryError) -> AppError {
    match error {
        platform::filesystem::BoundaryError::Absolute => {
            AppError::Validation("Session workspace paths must be relative.".to_string())
        }
        platform::filesystem::BoundaryError::Hidden => {
            AppError::Validation("Hidden workspace paths are unavailable.".to_string())
        }
        platform::filesystem::BoundaryError::Escape => {
            AppError::Validation("Session workspace path escapes are not allowed.".to_string())
        }
        error => map_filesystem_storage_error(error),
    }
}

fn map_workspace_boundary_error(
    error: platform::filesystem::BoundaryError,
    outside_message: &str,
) -> AppError {
    match error {
        platform::filesystem::BoundaryError::Absolute
        | platform::filesystem::BoundaryError::Hidden
        | platform::filesystem::BoundaryError::Escape => map_relative_path_error(error),
        platform::filesystem::BoundaryError::MissingParent => {
            AppError::Validation("Git path has no valid parent.".to_string())
        }
        platform::filesystem::BoundaryError::OutsideRoot => {
            AppError::Validation(outside_message.to_string())
        }
        error => map_filesystem_storage_error(error),
    }
}

fn map_filesystem_storage_error(error: platform::filesystem::BoundaryError) -> AppError {
    AppError::Storage(error.to_string())
}

fn unavailable_context() -> SessionWorkspaceContext {
    SessionWorkspaceContext::unavailable("Session workspace is unavailable.")
}

fn available_context(root: &Path) -> SessionWorkspaceContext {
    SessionWorkspaceContext::available(
        root.file_name()
            .map(|name| name.to_string_lossy().to_string()),
    )
}

/// One page of a directory, ordered and cut at the caller's bound.
///
/// The whole directory is enumerated and sorted before the cut, which is what makes the order
/// stable: a page is a window onto a total order rather than whatever the filesystem returned
/// first. That cost is the same as before - the previous version enumerated everything too and
/// then threw away all but the first 500.
fn directory_page_at(
    root: &Path,
    relative: &str,
    cursor: Option<&DirectoryCursor>,
    limit: usize,
) -> Result<(Vec<DirectoryEntry>, bool, Option<String>), AppError> {
    let directory = if relative.is_empty() {
        root.to_path_buf()
    } else {
        resolve_existing_path(root, relative)?
    };
    if !directory.is_dir() {
        return Err(AppError::Validation(
            "Requested workspace path is not a directory.".to_string(),
        ));
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| AppError::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| AppError::Storage(error.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let canonical = match entry.path().canonicalize() {
            Ok(value) if value.starts_with(root) => value,
            _ => continue,
        };
        let metadata =
            fs::metadata(&canonical).map_err(|error| AppError::Storage(error.to_string()))?;
        let kind = if metadata.is_dir() {
            "directory"
        } else {
            "file"
        };
        entries.push(DirectoryEntry {
            name,
            path: normalized_relative(root, &canonical)?,
            kind,
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
        });
    }
    entries.sort_by(|left, right| {
        kind_rank(left.kind)
            .cmp(&kind_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    // Resuming happens after the sort, so the key a cursor carries is the key the listing is
    // ordered by. Filtering before sorting would compare against an order that does not exist
    // yet and drop entries that belong on the page.
    if let Some(cursor) = cursor {
        entries.retain(|entry| cursor.precedes(entry.kind, &entry.name));
    }
    let truncated = entries.len() > limit;
    entries.truncate(limit);
    // A cursor only when there is more. Issuing one for an exhausted directory would invite a
    // caller to fetch a page that is always empty, and an empty page reads as a directory that
    // just emptied itself.
    let next_cursor = truncated.then(|| {
        entries
            .last()
            .map(|entry| DirectoryCursor::after(relative, entry.kind, &entry.name).encode())
    });
    Ok((entries, truncated, next_cursor.flatten()))
}

pub(crate) fn list_session_directory(
    conn: &Connection,
    session_id: &str,
    path: &str,
) -> Result<DirectoryListing, AppError> {
    list_session_directory_page(conn, session_id, path, None, DEFAULT_DIRECTORY_PAGE_SIZE)
}

/// One page, resuming after a cursor.
///
/// `list_session_directory` is this with the default bound and no cursor. One implementation, so
/// the ordering a cursor resumes into is the ordering the first page produced.
pub(crate) fn list_session_directory_page(
    conn: &Connection,
    session_id: &str,
    path: &str,
    cursor: Option<&str>,
    limit: usize,
) -> Result<DirectoryListing, AppError> {
    let decoded = match cursor {
        Some(encoded) => Some(
            DirectoryCursor::decode(encoded, path)
                // A cursor for another directory, or one nobody issued. Refused as a validation
                // failure so the caller starts the listing again rather than receiving a page
                // from somewhere else.
                .map_err(|_| {
                    AppError::Validation("Directory cursor is not valid here.".to_string())
                })?,
        ),
        None => None,
    };
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(DirectoryListing {
            context: unavailable_context(),
            path: path.to_string(),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let (items, truncated, next_cursor) = directory_page_at(&root, path, decoded.as_ref(), limit)?;
    Ok(DirectoryListing {
        context: available_context(&root),
        path: path.to_string(),
        items,
        truncated,
        next_cursor,
    })
}

/// A stat per directory, and no enumeration.
///
/// A directory's own modified time moves when an entry is added, removed, or renamed, which is
/// exactly the set of changes a file tree renders. Reading the directory to compare its contents
/// would answer the same question by doing the work the answer is supposed to avoid.
///
/// Paths are confined the same way every other read is. A poll is still a read, and one that
/// resolved its own paths would be a second way into the filesystem whose boundary is whatever it
/// was handed.
pub(crate) fn session_directory_fingerprints(
    conn: &Connection,
    session_id: &str,
    paths: &[String],
) -> Result<Vec<DirectoryFingerprint>, AppError> {
    let Some(root) = resolve_session_root(conn, session_id)? else {
        // No local root: every directory is unreadable rather than missing. Nothing was removed —
        // this session simply has nowhere for the poll to look.
        return Ok(paths
            .iter()
            .map(|relative_path| DirectoryFingerprint {
                relative_path: relative_path.clone(),
                state: DirectoryFingerprintState::Unreadable,
            })
            .collect());
    };
    Ok(paths
        .iter()
        .take(MAX_FINGERPRINT_PATHS)
        .map(|relative_path| DirectoryFingerprint {
            relative_path: relative_path.clone(),
            state: directory_fingerprint_at(&root, relative_path),
        })
        .collect())
}

fn directory_fingerprint_at(root: &Path, relative: &str) -> DirectoryFingerprintState {
    let directory = if relative.is_empty() {
        root.to_path_buf()
    } else {
        match resolve_existing_path(root, relative) {
            Ok(path) => path,
            // The confined resolution refuses what is not there and what escapes alike. An escape
            // cannot reach here — the provider classifies those from the request — so what is left
            // is a directory that is gone, which is itself a change worth reporting.
            Err(_) => return DirectoryFingerprintState::Missing,
        }
    };
    let Ok(metadata) = fs::metadata(&directory) else {
        return DirectoryFingerprintState::Missing;
    };
    if !metadata.is_dir() {
        return DirectoryFingerprintState::Missing;
    }
    match metadata.modified() {
        Ok(modified) => match modified.duration_since(std::time::UNIX_EPOCH) {
            Ok(since_epoch) => {
                DirectoryFingerprintState::Known(format!("{}", since_epoch.as_nanos()))
            }
            // A modified time before the epoch is a clock nobody can compare against. Unreadable
            // rather than a made-up value, which would compare equal forever.
            Err(_) => DirectoryFingerprintState::Unreadable,
        },
        // Some filesystems do not keep one. Saying so is better than substituting a constant, which
        // would report "unchanged" for every poll on that volume.
        Err(_) => DirectoryFingerprintState::Unreadable,
    }
}

fn collect_documents(
    root: &Path,
    directory: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    documents: &mut Vec<SessionDocument>,
) -> Result<bool, AppError> {
    if depth > DOCUMENT_DEPTH_LIMIT || documents.len() >= DOCUMENT_LIMIT {
        return Ok(true);
    }
    let canonical_directory = directory
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    if !canonical_directory.starts_with(root) || !visited.insert(canonical_directory.clone()) {
        return Ok(false);
    }
    let mut truncated = false;
    for entry in
        fs::read_dir(&canonical_directory).map_err(|error| AppError::Storage(error.to_string()))?
    {
        let entry = entry.map_err(|error| AppError::Storage(error.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let canonical = match entry.path().canonicalize() {
            Ok(value) if value.starts_with(root) => value,
            _ => continue,
        };
        if canonical.is_dir() {
            truncated |= collect_documents(root, &canonical, depth + 1, visited, documents)?;
        } else {
            let extension = canonical
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let kind = match extension.as_str() {
                "md" | "markdown" => Some("markdown"),
                "txt" => Some("text"),
                _ => None,
            };
            if let Some(kind) = kind {
                documents.push(SessionDocument {
                    name,
                    path: normalized_relative(root, &canonical)?,
                    kind,
                });
                if documents.len() >= DOCUMENT_LIMIT {
                    truncated = true;
                    break;
                }
            }
        }
    }
    Ok(truncated)
}

pub(crate) fn list_session_documents(
    conn: &Connection,
    session_id: &str,
) -> Result<DocumentListing, AppError> {
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(DocumentListing {
            context: unavailable_context(),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let mut documents = Vec::new();
    let truncated = collect_documents(&root, &root, 0, &mut HashSet::new(), &mut documents)?;
    documents.sort_by_key(|document| document.path.to_lowercase());
    Ok(DocumentListing {
        context: available_context(&root),
        items: documents,
        truncated,
        next_cursor: None,
    })
}

fn read_file_at(root: &Path, relative: &str) -> Result<FileContent, AppError> {
    let relative_path = validate_relative_path(relative)?;
    let candidate = root.join(&relative_path);
    if !candidate.exists() {
        return Ok(FileContent {
            path: relative.to_string(),
            name: relative_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| relative.to_string()),
            status: "missing",
            size: 0,
            content: None,
            encoding: None,
            newline: None,
        });
    }
    let path = resolve_existing_path(root, relative)?;
    if !path.is_file() {
        return Err(AppError::Validation(
            "Requested workspace path is not a file.".to_string(),
        ));
    }
    let metadata = fs::metadata(&path).map_err(|error| AppError::Storage(error.to_string()))?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| relative.to_string());
    if metadata.len() > FILE_BYTE_LIMIT {
        return Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "oversized",
            size: metadata.len(),
            content: None,
            encoding: None,
            newline: None,
        });
    }
    let bytes = fs::read(&path).map_err(|error| AppError::Storage(error.to_string()))?;
    if bytes.contains(&0) {
        return Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "binary",
            size: metadata.len(),
            content: None,
            encoding: None,
            newline: None,
        });
    }
    match String::from_utf8(bytes) {
        Ok(content) => Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "text",
            size: metadata.len(),
            // Classified from the decoded text by the shared detector, so a file reports the same
            // encoding and line endings whichever machine it is on.
            encoding: Some(detect_encoding(&content).token()),
            newline: Some(detect_newline(&content).token()),
            content: Some(content),
        }),
        Err(_) => Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "binary",
            size: metadata.len(),
            content: None,
            encoding: None,
            newline: None,
        }),
    }
}

pub(crate) fn read_session_file(
    conn: &Connection,
    session_id: &str,
    path: &str,
) -> Result<FileContent, AppError> {
    let root = resolve_session_root(conn, session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    read_file_at(&root, path)
}

pub(crate) fn read_session_text_file(
    conn: &Connection,
    session_id: &str,
    path: &str,
) -> Result<FileContent, AppError> {
    let root = resolve_session_root(conn, session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    let file = read_file_at(&root, path)?;
    if file.status != "text" {
        return Err(AppError::Validation(format!(
            "Referenced file is not readable text: {path}"
        )));
    }
    Ok(file)
}

fn git_change_kind(value: char) -> String {
    match value {
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        '?' => "untracked",
        'U' => "conflicted",
        _ => "unmodified",
    }
    .to_string()
}

pub(super) fn parse_git_status(raw: &[u8]) -> (Option<String>, Vec<GitStatusEntry>) {
    let records = raw
        .split(|value| *value == 0)
        .filter(|record| !record.is_empty())
        .map(|record| String::from_utf8_lossy(record).to_string())
        .collect::<Vec<_>>();
    let mut branch = None;
    let mut entries = Vec::new();
    let mut index = 0;
    while index < records.len() {
        let record = &records[index];
        if let Some(value) = record.strip_prefix("## ") {
            branch = Some(
                value
                    .split("...")
                    .next()
                    .unwrap_or(value)
                    .trim()
                    .to_string(),
            );
            index += 1;
            continue;
        }
        if record.len() < 3 {
            index += 1;
            continue;
        }
        let mut chars = record.chars();
        let index_code = chars.next().unwrap_or(' ');
        let worktree_code = chars.next().unwrap_or(' ');
        let path = record.get(3..).unwrap_or_default().to_string();
        let renamed = matches!(index_code, 'R' | 'C') || matches!(worktree_code, 'R' | 'C');
        let previous_path = if renamed && index + 1 < records.len() {
            index += 1;
            Some(records[index].clone())
        } else {
            None
        };
        entries.push(GitStatusEntry {
            path,
            previous_path,
            index: git_change_kind(index_code),
            worktree: git_change_kind(worktree_code),
        });
        index += 1;
    }
    (branch, entries)
}

fn git_output(root: &Path, args: &[String]) -> Result<platform::git::GitOutput, AppError> {
    platform::process::audit_command("session.git", "git", args);
    platform::git::GitAdapter::default()
        .execute(root, args, std::time::Duration::from_secs(30))
        .map_err(|error| AppError::LaunchFailed(error.to_string()))
}

fn active_log_dir_from_conn(conn: &Connection) -> Result<PathBuf, AppError> {
    let configured = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'logDirectory'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| AppError::Repository(error.to_string()))?
        .filter(|path| !path.trim().is_empty());
    let fallback = conn
        .path()
        .and_then(|path| PathBuf::from(path).parent().map(logging::default_log_dir))
        .unwrap_or_else(|| logging::active_log_dir(logging::default_log_dir(Path::new("."))));
    Ok(configured.map(PathBuf::from).unwrap_or(fallback))
}

fn write_git_failure(conn: &Connection, session_id: &str, agent_id: &str, message: &str) {
    let mut context = BTreeMap::new();
    context.insert("sessionId".to_string(), session_id.to_string());
    context.insert("agentId".to_string(), agent_id.to_string());
    let Ok(log_dir) = active_log_dir_from_conn(conn) else {
        return;
    };
    let _ = logging::write_message(
        &log_dir,
        logging::LogLevel::Warn,
        "session.git",
        message,
        context,
    );
}

type ParsedGitStatus = Option<(Option<String>, Vec<GitStatusEntry>)>;

fn git_status_at(root: &Path) -> Result<ParsedGitStatus, AppError> {
    let args = vec![
        "-c".to_string(),
        "core.quotepath=false".to_string(),
        "status".to_string(),
        "--porcelain=v1".to_string(),
        "-z".to_string(),
        "--branch".to_string(),
        "--untracked-files=all".to_string(),
    ];
    let output = git_output(root, &args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        if stderr.contains("not a git repository") {
            return Ok(None);
        }
        return Err(AppError::LaunchFailed("Git status failed.".to_string()));
    }
    Ok(Some(parse_git_status(&output.stdout)))
}

/// Whether `path` is untracked in `root`. `git ls-files --error-unmatch` succeeds only for
/// paths git tracks, so a non-zero exit (that isn't a repository error) means untracked.
/// This avoids the full-directory `git status` walk the diff path used to run just to find
/// one entry.
fn is_path_untracked(root: &Path, path: &str) -> Result<bool, AppError> {
    let args = vec![
        "-c".to_string(),
        "core.quotepath=false".to_string(),
        "ls-files".to_string(),
        "--error-unmatch".to_string(),
        "--".to_string(),
        path.to_string(),
    ];
    let output = git_output(root, &args)?;
    if output.status.success() {
        return Ok(false);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    // `did not match any files` is the expected failure for an untracked path. Anything
    // else (e.g. "not a git repository") is a real error to surface.
    if stderr.contains("did not match any files") || stderr.contains("pathspec") {
        return Ok(true);
    }
    if stderr.contains("not a git repository") {
        return Ok(true);
    }
    Err(AppError::LaunchFailed("Git ls-files failed.".to_string()))
}

pub(crate) fn get_session_git_status(
    conn: &Connection,
    session_id: &str,
) -> Result<GitStatusResult, AppError> {
    let session = load_session_workspace(conn, session_id)?;
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(GitStatusResult {
            context: unavailable_context(),
            is_git: false,
            branch: None,
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let result = match git_status_at(&root) {
        Ok(value) => value,
        Err(error) => {
            write_git_failure(conn, session_id, &session.agent_id, &error.to_string());
            return Err(error);
        }
    };
    let Some((branch, mut entries)) = result else {
        return Ok(GitStatusResult {
            context: available_context(&root),
            is_git: false,
            branch: None,
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let truncated = entries.len() > GIT_STATUS_ENTRY_LIMIT;
    entries.truncate(GIT_STATUS_ENTRY_LIMIT);
    Ok(GitStatusResult {
        context: available_context(&root),
        is_git: true,
        branch,
        items: entries,
        truncated,
        next_cursor: None,
    })
}

fn parse_range(value: &str) -> (usize, usize) {
    let value = value.trim_start_matches(['-', '+']);
    let mut parts = value.split(',');
    let start = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let count = parts.next().and_then(|part| part.parse().ok()).unwrap_or(1);
    (start, count)
}

fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    let body = header.strip_prefix("@@ ")?;
    let end = body.find(" @@")?;
    let mut ranges = body[..end].split_whitespace();
    let (old_start, old_lines) = parse_range(ranges.next()?);
    let (new_start, new_lines) = parse_range(ranges.next()?);
    Some((old_start, old_lines, new_start, new_lines))
}

fn clean_diff_path(value: &str) -> Option<String> {
    let value = value.trim();
    if value == "/dev/null" {
        None
    } else {
        Some(
            value
                .strip_prefix("a/")
                .or_else(|| value.strip_prefix("b/"))
                .unwrap_or(value)
                .to_string(),
        )
    }
}

pub(super) fn parse_git_diff(raw: &str, fallback_path: &str) -> Vec<GitDiffFile> {
    let mut files = Vec::new();
    let mut current_file: Option<GitDiffFile> = None;
    let mut current_hunk: Option<GitDiffHunk> = None;
    let mut old_line = 0usize;
    let mut new_line = 0usize;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            if let Some(mut file) = current_file.take() {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                files.push(file);
            }
            current_file = Some(GitDiffFile {
                old_path: Some(fallback_path.to_string()),
                new_path: fallback_path.to_string(),
                binary: false,
                oversized: false,
                hunks: Vec::new(),
            });
        } else if let Some(value) = line.strip_prefix("--- ") {
            if current_file.is_none() {
                current_file = Some(GitDiffFile {
                    old_path: None,
                    new_path: fallback_path.to_string(),
                    binary: false,
                    oversized: false,
                    hunks: Vec::new(),
                });
            }
            if let Some(file) = current_file.as_mut() {
                file.old_path = clean_diff_path(value);
            }
        } else if let Some(value) = line.strip_prefix("+++ ") {
            if let Some(file) = current_file.as_mut() {
                if let Some(path) = clean_diff_path(value) {
                    file.new_path = path;
                }
            }
        } else if line.starts_with("Binary files ") || line == "GIT binary patch" {
            if let Some(file) = current_file.as_mut() {
                file.binary = true;
            }
        } else if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            if let Some((old_start, old_lines, new_start, new_lines)) = parse_hunk_header(line) {
                old_line = old_start;
                new_line = new_start;
                current_hunk = Some(GitDiffHunk {
                    header: line.to_string(),
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    lines: Vec::new(),
                });
            }
        } else if let Some(hunk) = current_hunk.as_mut() {
            let (kind, content, old_number, new_number) =
                if let Some(content) = line.strip_prefix('+') {
                    let number = new_line;
                    new_line += 1;
                    ("addition", content, None, Some(number))
                } else if let Some(content) = line.strip_prefix('-') {
                    let number = old_line;
                    old_line += 1;
                    ("deletion", content, Some(number), None)
                } else {
                    let content = line.strip_prefix(' ').unwrap_or(line);
                    let old_number = old_line;
                    let new_number = new_line;
                    old_line += 1;
                    new_line += 1;
                    ("context", content, Some(old_number), Some(new_number))
                };
            hunk.lines.push(GitDiffLine {
                kind: kind.to_string(),
                content: content.to_string(),
                old_line_number: old_number,
                new_line_number: new_number,
            });
        }
    }
    if let Some(mut file) = current_file {
        if let Some(hunk) = current_hunk {
            file.hunks.push(hunk);
        }
        files.push(file);
    }
    files
}

fn untracked_diff(root: &Path, path: &str) -> Result<Option<GitDiffFile>, AppError> {
    let file = read_file_at(root, path)?;
    if file.status == "binary" || file.status == "oversized" {
        return Ok(Some(GitDiffFile {
            old_path: None,
            new_path: path.to_string(),
            binary: file.status == "binary",
            oversized: file.status == "oversized",
            hunks: Vec::new(),
        }));
    }
    let content = file.content.unwrap_or_default();
    let lines = content
        .lines()
        .enumerate()
        .map(|(index, content)| GitDiffLine {
            kind: "addition".to_string(),
            content: content.to_string(),
            old_line_number: None,
            new_line_number: Some(index + 1),
        })
        .collect::<Vec<_>>();
    Ok(Some(GitDiffFile {
        old_path: None,
        new_path: path.to_string(),
        binary: false,
        oversized: false,
        hunks: vec![GitDiffHunk {
            header: format!("@@ -0,0 +1,{} @@", lines.len()),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: lines.len(),
            lines,
        }],
    }))
}

pub(crate) fn get_session_git_diff(
    conn: &Connection,
    session_id: &str,
    path: &str,
    source: GitDiffSource,
) -> Result<GitDiffResult, AppError> {
    let session = load_session_workspace(conn, session_id)?;
    let root = resolve_session_root(conn, session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    let (_candidate, normalized_path) = resolve_git_path(&root, path)?;
    // A single-path untracked check is far cheaper than the full
    // `git status --porcelain -z --untracked-files=all` directory walk this used to run
    // just to find one entry. `git ls-files --error-unmatch -- <path>` succeeds only for
    // tracked paths, so a failure (non-zero exit, not a repository error) means untracked.
    let is_untracked = match is_path_untracked(&root, &normalized_path) {
        Ok(value) => value,
        Err(error) => {
            write_git_failure(
                conn,
                session_id,
                &session.agent_id,
                "Git status preflight failed.",
            );
            return Err(error);
        }
    };
    if is_untracked && source == GitDiffSource::Working {
        return Ok(GitDiffResult {
            context: available_context(&root),
            source,
            files: untracked_diff(&root, &normalized_path)?
                .into_iter()
                .collect(),
            truncated: false,
        });
    }
    let mut args = vec![
        "-c".to_string(),
        "core.quotepath=false".to_string(),
        "diff".to_string(),
        "--no-ext-diff".to_string(),
        "--no-color".to_string(),
        "--unified=3".to_string(),
    ];
    if source == GitDiffSource::Staged {
        args.push("--cached".to_string());
    }
    args.extend(["--".to_string(), normalized_path.clone()]);
    let output = match git_output(&root, &args) {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            let message = "Git diff failed.";
            write_git_failure(conn, session_id, &session.agent_id, message);
            return Err(AppError::LaunchFailed(message.to_string()));
        }
        Err(error) => {
            write_git_failure(conn, session_id, &session.agent_id, &error.to_string());
            return Err(error);
        }
    };
    if output.stdout.len() > DIFF_BYTE_LIMIT {
        return Ok(GitDiffResult {
            context: available_context(&root),
            source,
            files: vec![GitDiffFile {
                old_path: Some(normalized_path.clone()),
                new_path: normalized_path,
                binary: false,
                oversized: true,
                hunks: Vec::new(),
            }],
            truncated: true,
        });
    }
    Ok(GitDiffResult {
        context: available_context(&root),
        source,
        files: parse_git_diff(&String::from_utf8_lossy(&output.stdout), &normalized_path),
        truncated: false,
    })
}

fn create_review_snapshot(conn: &Connection, session_id: &str) -> Result<ReviewSnapshot, AppError> {
    let root = resolve_session_root(conn, session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    let Some((_branch, entries)) = git_status_at(&root)? else {
        return Err(AppError::Validation(
            "Session workspace is not a Git repository.".to_string(),
        ));
    };
    let truncated = entries.len() > MAX_REVIEW_FILES;
    let mut accepted_bytes = 0usize;
    let mut files = Vec::with_capacity(entries.len().min(MAX_REVIEW_FILES));
    for entry in entries.into_iter().take(MAX_REVIEW_FILES) {
        let (candidate, normalized) = resolve_git_path(&root, &entry.path)?;
        let metadata = fs::metadata(&candidate).ok();
        let size = metadata
            .as_ref()
            .map(|value| value.len() as usize)
            .unwrap_or(0);
        let oversized = size > MAX_REVIEW_FILE_BYTES;
        let binary = if metadata.as_ref().is_some_and(|value| value.is_file()) && !oversized {
            is_binary_file(&candidate)?
        } else {
            false
        };
        let new_hash = if metadata.as_ref().is_some_and(|value| value.is_file()) && !oversized {
            Some(hash_file(&candidate)?)
        } else {
            None
        };
        accepted_bytes = accepted_bytes.saturating_add(size.min(MAX_REVIEW_FILE_BYTES));
        let change_type = effective_change_type(&entry);
        files.push(ReviewFileSummary {
            path: normalized,
            previous_path: entry.previous_path,
            change_type,
            old_hash: None,
            new_hash,
            binary,
            oversized,
        });
    }
    let head_revision = git_revision(&root)?;
    let fingerprint = crate::contexts::workspaces::application::fingerprint_snapshot(&files);
    Ok(ReviewSnapshot {
        workspace_id: hash_text(&root.to_string_lossy()),
        base_revision: head_revision.clone(),
        head_revision,
        fingerprint,
        files,
        truncated,
        accepted_bytes: accepted_bytes.min(MAX_REVIEW_DIFF_BYTES),
    })
}

fn load_review_file(
    conn: &Connection,
    session_id: &str,
    path: &str,
    expected_snapshot: &str,
) -> Result<ReviewDiffFile, AppError> {
    let snapshot = create_review_snapshot(conn, session_id)?;
    if snapshot.fingerprint != expected_snapshot {
        return Err(AppError::Validation(
            "Review snapshot is stale.".to_string(),
        ));
    }
    let summary = snapshot
        .files
        .into_iter()
        .find(|file| file.path == path)
        .ok_or_else(|| AppError::Validation("Review file is unavailable.".to_string()))?;
    if summary.binary || summary.oversized {
        return Ok(ReviewDiffFile {
            summary,
            hunks: Vec::new(),
            truncated: false,
            accepted_bytes: 0,
        });
    }
    let mut diff = get_session_git_diff(conn, session_id, path, GitDiffSource::Working)?;
    if diff.files.is_empty() {
        diff = get_session_git_diff(conn, session_id, path, GitDiffSource::Staged)?;
    }
    let accepted_bytes = diff
        .files
        .iter()
        .flat_map(|file| &file.hunks)
        .flat_map(|hunk| &hunk.lines)
        .map(|line| line.content.len())
        .sum::<usize>()
        .min(MAX_REVIEW_FILE_BYTES);
    let hunks = diff
        .files
        .into_iter()
        .flat_map(|file| file.hunks)
        .map(|hunk| ReviewDiffHunk {
            fingerprint: crate::contexts::workspaces::application::fingerprint_hunk(&hunk),
            context_fingerprints: (0..hunk.lines.len())
                .map(|index| {
                    crate::contexts::workspaces::application::fingerprint_context(&hunk, index)
                })
                .collect(),
            hunk,
        })
        .collect();
    Ok(ReviewDiffFile {
        summary,
        hunks,
        truncated: diff.truncated,
        accepted_bytes,
    })
}

fn effective_change_type(entry: &GitStatusEntry) -> String {
    if entry.worktree != "unmodified" {
        entry.worktree.clone()
    } else {
        entry.index.clone()
    }
}

fn git_revision(root: &Path) -> Result<Option<String>, AppError> {
    let output = git_output(root, &["rev-parse".into(), "HEAD".into()])?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

fn is_binary_file(path: &Path) -> Result<bool, AppError> {
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let mut bytes = Vec::new();
    BufReader::new(file)
        .take(8192)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(bytes.contains(&0))
}

fn hash_file(path: &Path) -> Result<String, AppError> {
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let mut reader = BufReader::new(file).take(MAX_REVIEW_FILE_BYTES as u64);
    let mut digest = sha2::Sha256::new();
    std::io::copy(&mut reader, &mut HashWriter(&mut digest))
        .map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(digest_hex(digest))
}

struct HashWriter<'a>(&'a mut sha2::Sha256);
impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        use sha2::Digest;
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn hash_text(value: &str) -> String {
    use sha2::Digest;
    let mut digest = sha2::Sha256::new();
    digest.update(value.as_bytes());
    digest_hex(digest)
}

fn digest_hex(digest: sha2::Sha256) -> String {
    use sha2::Digest;
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The hunks one request selects, and the normalized path they belong to.
///
/// Shared by the patch render and the revert, because they answer the same three questions in the
/// same order — is the snapshot current, does the path resolve, is the hunk selection unambiguous
/// — and a second copy would drift into two answers for the same request.
fn select_review_hunks(
    conn: &Connection,
    session_id: &str,
    path: &str,
    expected_snapshot: &str,
    hunk_fingerprint: Option<&String>,
) -> Result<(String, String, ReviewDiffFile, Vec<usize>), AppError> {
    let current = create_review_snapshot(conn, session_id)?;
    if current.fingerprint != expected_snapshot {
        // The same code the hunk-decision path returns, because it is the same fact and the
        // reviewer's next move is the same: reload and look again.
        return Err(AppError::Conflict("stale_witness"));
    }
    let root = resolve_session_root(conn, session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    let (_candidate, normalized) = resolve_git_path(&root, path)?;
    let file = load_review_file(conn, session_id, &normalized, &current.fingerprint)?;
    let selected = file
        .hunks
        .iter()
        .enumerate()
        .filter(|(_, hunk)| hunk_fingerprint.is_none_or(|expected| &hunk.fingerprint == expected))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if selected.is_empty() || (hunk_fingerprint.is_some() && selected.len() != 1) {
        // Absent and ambiguous are one refusal on purpose: in both cases the request names a hunk
        // this diff cannot single out, and there is nothing a reviewer does differently about the
        // two.
        return Err(AppError::Conflict("review_hunk_unavailable"));
    }
    Ok((current.fingerprint, normalized, file, selected))
}

/// Why this file has no patch to give, or nothing.
///
/// Two codes rather than one, because they are two sentences: a binary file has no text to patch
/// and never will, while an oversized one has a change this application declined to read. A
/// reviewer does something different about each.
fn patch_refusal(file: &ReviewDiffFile) -> Option<&'static str> {
    if file.summary.binary {
        return Some("patch_unavailable_binary");
    }
    if file.summary.oversized {
        return Some("patch_too_large");
    }
    None
}

fn patch_exceeds_bound(patch: &str) -> bool {
    patch.len() > MAX_REVIEW_PATCH_BYTES
}

fn render_review_patch(
    conn: &Connection,
    request: &ReviewPatchRequest,
) -> Result<ReviewPatch, AppError> {
    let (snapshot, normalized, file, selected) = select_review_hunks(
        conn,
        &request.session_id,
        &request.path,
        &request.expected_snapshot,
        request.hunk_fingerprint.as_ref(),
    )?;
    // Before rendering, because rendering a patch for content this application never decoded
    // would produce something confident and wrong rather than something empty.
    if let Some(code) = patch_refusal(&file) {
        return Err(AppError::Conflict(code));
    }
    let hunks = selected
        .iter()
        .map(|index| &file.hunks[*index])
        .collect::<Vec<_>>();
    let patch = render_patch(&normalized, &hunks);
    // Refused rather than truncated. A patch cut short looks exactly like one that applies until
    // somebody runs it somewhere it matters.
    if patch_exceeds_bound(&patch) {
        return Err(AppError::Conflict("patch_too_large"));
    }
    Ok(ReviewPatch {
        fingerprint: crate::contexts::workspaces::application::fingerprint_patch(&patch),
        patch,
        hunks: hunks.len(),
        path: normalized,
        snapshot,
    })
}

fn revert_review_change(
    conn: &Connection,
    request: &ReviewRevertRequest,
) -> Result<ReviewRevertReceipt, AppError> {
    if !request.confirmed {
        return Err(AppError::PolicyDenied {
            session_id: request.session_id.clone(),
            action: "review-revert-confirmation".to_string(),
        });
    }
    static MUTATION_GUARD: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let guard = MUTATION_GUARD
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .map_err(|_| AppError::Storage("Workspace mutation guard is unavailable.".to_string()))?;
    let (previous_snapshot, normalized, file, selected_indexes) = select_review_hunks(
        conn,
        &request.session_id,
        &request.path,
        &request.expected_snapshot,
        request.hunk_fingerprint.as_ref(),
    )?;
    let root = resolve_session_root(conn, &request.session_id)?
        .ok_or_else(|| AppError::Validation("Session workspace is unavailable.".to_string()))?;
    let selected = selected_indexes
        .iter()
        .map(|index| &file.hunks[*index])
        .collect::<Vec<_>>();
    let patch = render_patch(&normalized, &selected);
    let mut patch_file =
        tempfile::NamedTempFile::new().map_err(|error| AppError::Storage(error.to_string()))?;
    patch_file
        .write_all(patch.as_bytes())
        .map_err(|error| AppError::Storage(error.to_string()))?;
    patch_file
        .flush()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let patch_path = patch_file.path().to_string_lossy().to_string();
    for check in [true, false] {
        let mut args = vec![
            "apply".to_string(),
            "--reverse".to_string(),
            "--whitespace=nowarn".to_string(),
        ];
        if check {
            args.push("--check".to_string());
        }
        args.push(patch_path.clone());
        let output = git_output(&root, &args)?;
        if !output.status.success() {
            return Err(AppError::Validation(
                if check {
                    "Review patch no longer applies exactly."
                } else {
                    "Review patch application failed."
                }
                .to_string(),
            ));
        }
    }
    let resulting = create_review_snapshot(conn, &request.session_id)?;
    drop(guard);
    Ok(ReviewRevertReceipt {
        path: normalized,
        previous_snapshot,
        resulting_snapshot: resulting.fingerprint,
        reverted_hunks: selected.len(),
    })
}

/// Renders selected hunks as a unified diff Git will accept.
///
/// The two `/dev/null` cases are the reason this is not a format string. A file with no old side
/// needs `--- /dev/null` and a file with no new side needs `+++ /dev/null`; naming the path on
/// both sides produces a patch that reads correctly and that `git apply` refuses, which is exactly
/// the failure a rendering assertion cannot see and `git apply --check` can.
///
/// Derived from the hunks rather than from the summary's `change_type`. The summary's `old_hash`
/// is always absent here, so it cannot distinguish an addition from a modification, and a change
/// type is a word from `git status` while this has to agree with the diff it is rendering.
fn render_patch(path: &str, hunks: &[&ReviewDiffHunk]) -> String {
    let no_old_side = hunks
        .iter()
        .all(|selected| selected.hunk.old_lines == 0 && selected.hunk.old_start == 0);
    let no_new_side = hunks
        .iter()
        .all(|selected| selected.hunk.new_lines == 0 && selected.hunk.new_start == 0);
    let old_side = if no_old_side {
        "/dev/null".to_string()
    } else {
        format!("a/{path}")
    };
    let new_side = if no_new_side {
        "/dev/null".to_string()
    } else {
        format!("b/{path}")
    };
    let mut patch = format!("diff --git a/{path} b/{path}\n");
    if no_old_side {
        patch.push_str("new file mode 100644\n");
    } else if no_new_side {
        patch.push_str("deleted file mode 100644\n");
    }
    patch.push_str(&format!("--- {old_side}\n+++ {new_side}\n"));
    for review_hunk in hunks {
        let hunk = &review_hunk.hunk;
        patch.push_str(&hunk.header);
        patch.push('\n');
        for line in &hunk.lines {
            patch.push(match line.kind.as_str() {
                "addition" => '+',
                "deletion" => '-',
                _ => ' ',
            });
            patch.push_str(&line.content);
            patch.push('\n');
        }
    }
    patch
}

fn filtered_log_entries(
    path: &Path,
    input: &SessionLogQuery,
) -> Result<Vec<logging::LogEntry>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(filter_log_entries(BufReader::new(file), input))
}

fn filtered_log_entries_tail(
    path: &Path,
    input: &SessionLogQuery,
    byte_limit: u64,
) -> Result<Vec<logging::LogEntry>, AppError> {
    let mut file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let length = file
        .metadata()
        .map_err(|error| AppError::Storage(error.to_string()))?
        .len();
    if length <= byte_limit {
        return Ok(filter_log_entries(BufReader::new(file), input));
    }
    file.seek(SeekFrom::Start(length - byte_limit))
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut discarded_partial_line = String::new();
    reader
        .read_line(&mut discarded_partial_line)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(filter_log_entries(reader, input))
}

/// Whether one already-parsed redacted record belongs in a filtered result.
///
/// Pure, and deliberately so. An export and a preview have to agree about which records are in
/// scope, and the only way two readers of the same corpus can be guaranteed to agree is if they
/// call the same function on the same parsed record. A predicate that re-derived the answer from a
/// query — one reading a file, one reading an index — would drift the first time a filter gained a
/// field, and the drift would show up as an export quietly containing more or less than the list
/// the user was looking at when they clicked it.
pub(crate) fn log_entry_matches(entry: &logging::LogEntry, input: &SessionLogQuery) -> bool {
    if entry.context.get("sessionId") != Some(&input.session_id) {
        return false;
    }
    if let Some(seat_id) = input.seat_id.as_ref() {
        if entry.context.get("seatId") != Some(seat_id) {
            return false;
        }
    }
    if !input.levels.is_empty() && !input.levels.contains(&workspace_log_level(entry.level)) {
        return false;
    }
    let search = input.search.trim().to_lowercase();
    if search.is_empty() {
        return true;
    }
    let searchable = format!(
        "{} {} {}",
        entry.category,
        entry.message,
        serde_json::to_string(&entry.context).unwrap_or_default()
    )
    .to_lowercase();
    searchable.contains(&search)
}

fn filter_log_entries(reader: impl BufRead, input: &SessionLogQuery) -> Vec<logging::LogEntry> {
    let mut entries = Vec::new();
    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(entry) = serde_json::from_str::<logging::LogEntry>(&line) else {
            continue;
        };
        if log_entry_matches(&entry, input) {
            entries.push(entry);
        }
    }
    entries
}

fn workspace_log_level(level: logging::LogLevel) -> WorkspaceLogLevel {
    match level {
        logging::LogLevel::Error => WorkspaceLogLevel::Error,
        logging::LogLevel::Warn => WorkspaceLogLevel::Warn,
        logging::LogLevel::Info => WorkspaceLogLevel::Info,
        logging::LogLevel::Debug => WorkspaceLogLevel::Debug,
    }
}

fn log_files(log_dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(log_dir)
        .map_err(|error| AppError::Storage(error.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && logging::is_log_file(path))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        let left_modified = fs::metadata(left)
            .and_then(|metadata| metadata.modified())
            .ok();
        let right_modified = fs::metadata(right)
            .and_then(|metadata| metadata.modified())
            .ok();
        right_modified.cmp(&left_modified)
    });
    Ok(files)
}

fn sort_newest_first(entries: &mut [logging::LogEntry]) {
    entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
}

fn bounded_filtered_log_entries(
    log_dir: &Path,
    input: &SessionLogQuery,
) -> Result<Vec<logging::LogEntry>, AppError> {
    let mut remaining = LOG_QUERY_BYTE_LIMIT;
    let mut entries = Vec::new();
    for path in log_files(log_dir)? {
        if remaining == 0 {
            break;
        }
        let length = fs::metadata(&path)
            .map_err(|error| AppError::Storage(error.to_string()))?
            .len();
        let read_limit = length.min(remaining);
        entries.extend(filtered_log_entries_tail(&path, input, read_limit)?);
        remaining -= read_limit;
    }
    sort_newest_first(&mut entries);
    Ok(entries)
}

/// Every record an export will write, in the order it will write them.
///
/// The one function that decides what an export contains, which is why it is reachable from a test:
/// the destination picker decides where the file goes, and this decides what goes in it.
pub(crate) fn all_filtered_log_entries(
    log_dir: &Path,
    input: &SessionLogQuery,
) -> Result<Vec<logging::LogEntry>, AppError> {
    let mut entries = Vec::new();
    for path in log_files(log_dir)? {
        entries.extend(filtered_log_entries(&path, input)?);
    }
    sort_newest_first(&mut entries);
    Ok(entries)
}

fn query_logs(log_dir: &Path, input: &SessionLogQuery) -> Result<SessionLogPage, AppError> {
    let entries = bounded_filtered_log_entries(log_dir, input)?;
    let offset = input
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = input
        .limit
        .unwrap_or(LOG_PAGE_LIMIT)
        .clamp(1, LOG_PAGE_LIMIT);
    let items = entries
        .iter()
        .skip(offset)
        .take(limit)
        .enumerate()
        .map(|(index, entry)| SessionLogEntry {
            id: format!("{}-{}", entry.timestamp, offset + index),
            timestamp: entry.timestamp.clone(),
            level: workspace_log_level(entry.level),
            category: entry.category.clone(),
            message: entry.message.clone(),
            context: entry.context.clone(),
        })
        .collect::<Vec<_>>();
    let next_offset = offset + items.len();
    let truncated = next_offset < entries.len();
    Ok(SessionLogPage {
        items,
        truncated,
        next_cursor: truncated.then(|| next_offset.to_string()),
    })
}

fn export_log_entries(
    selected: Option<PathBuf>,
    entries: &[logging::LogEntry],
) -> Result<SessionLogExportResult, AppError> {
    let Some(path) = selected else {
        return Ok(SessionLogExportResult {
            status: "cancelled",
            path: None,
        });
    };
    let mut file = fs::File::create(&path).map_err(|error| AppError::Storage(error.to_string()))?;
    for entry in entries.iter().rev() {
        let line =
            serde_json::to_string(entry).map_err(|error| AppError::Storage(error.to_string()))?;
        writeln!(file, "{line}").map_err(|error| AppError::Storage(error.to_string()))?;
    }
    Ok(SessionLogExportResult {
        status: "exported",
        path: Some(path.to_string_lossy().to_string()),
    })
}

pub(crate) fn list_session_logs(
    conn: &Connection,
    input: &SessionLogQuery,
) -> Result<SessionLogPage, AppError> {
    load_session_workspace(conn, &input.session_id)?;
    query_logs(&active_log_dir_from_conn(conn)?, input)
}

pub(crate) fn export_session_logs(
    app: &AppHandle,
    conn: &Connection,
    input: &SessionLogQuery,
) -> Result<SessionLogExportResult, AppError> {
    load_session_workspace(conn, &input.session_id)?;
    let log_dir = active_log_dir_from_conn(conn)?;
    let entries = all_filtered_log_entries(&log_dir, input)?;
    let selected = app
        .dialog()
        .file()
        .set_file_name(format!("vanehub-session-{}.log", input.session_id))
        .blocking_save_file();
    let path = selected
        .map(|value| {
            value
                .into_path()
                .map_err(|error| AppError::Validation(error.to_string()))
        })
        .transpose()?;
    export_log_entries(path, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use rusqlite::params;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("vanehub-{label}-{suffix}"))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .expect("git command");
        assert!(status.success(), "git {:?} failed", args);
    }

    use crate::contexts::workspaces::application::GitDiffLine;
    use crate::test_support::git_patch_fixture::{GitPatchFixture, PatchCheck};

    fn hunk(
        header: &str,
        old_start: usize,
        old_lines: usize,
        new_start: usize,
        new_lines: usize,
        lines: Vec<(&str, &str)>,
    ) -> ReviewDiffHunk {
        ReviewDiffHunk {
            fingerprint: format!("hunk-{header}"),
            context_fingerprints: Vec::new(),
            hunk: GitDiffHunk {
                header: header.to_string(),
                old_start,
                old_lines,
                new_start,
                new_lines,
                lines: lines
                    .into_iter()
                    .map(|(kind, content)| GitDiffLine {
                        kind: kind.to_string(),
                        content: content.to_string(),
                        old_line_number: None,
                        new_line_number: None,
                    })
                    .collect(),
            },
        }
    }

    /// The gate 0.8 exists for. A patch that reads correctly and that Git refuses is the failure a
    /// rendering assertion cannot see, and it is the only failure that matters here: what this
    /// renderer produces is handed to a reviewer to paste into `git apply`.
    #[test]
    fn a_rendered_modification_applies_to_the_repository_it_came_from() {
        let fixture =
            GitPatchFixture::committed("render-modification", &[("src/main.rs", "fn main() {}\n")]);
        let rendered = render_patch(
            "src/main.rs",
            &[&hunk(
                "@@ -1,1 +1,1 @@",
                1,
                1,
                1,
                1,
                vec![
                    ("deletion", "fn main() {}"),
                    ("addition", "fn main() { work(); }"),
                ],
            )],
        );

        assert_eq!(fixture.apply_check(&rendered), PatchCheck::Applies);
    }

    #[test]
    fn a_rendered_addition_names_dev_null_on_the_side_that_has_nothing() {
        let fixture = GitPatchFixture::committed("render-addition", &[("keep.txt", "keep\n")]);
        let rendered = render_patch(
            "added.rs",
            &[&hunk(
                "@@ -0,0 +1,1 @@",
                0,
                0,
                1,
                1,
                vec![("addition", "fn added() {}")],
            )],
        );

        // `--- a/added.rs` would name a file that has no old side, and Git refuses that. The
        // version of this renderer before 13.7 wrote exactly that, which nothing noticed because
        // its only caller applied patches in reverse against files that already existed.
        assert!(rendered.contains("--- /dev/null"));
        assert!(rendered.contains("new file mode"));
        assert_eq!(fixture.apply_check(&rendered), PatchCheck::Applies);
    }

    #[test]
    fn a_rendered_deletion_names_dev_null_on_the_side_that_has_nothing() {
        let fixture = GitPatchFixture::committed("render-deletion", &[("gone.txt", "gone\n")]);
        let rendered = render_patch(
            "gone.txt",
            &[&hunk(
                "@@ -1,1 +0,0 @@",
                1,
                1,
                0,
                0,
                vec![("deletion", "gone")],
            )],
        );

        assert!(rendered.contains("+++ /dev/null"));
        assert!(rendered.contains("deleted file mode"));
        assert_eq!(fixture.apply_check(&rendered), PatchCheck::Applies);
    }

    #[test]
    fn a_rendered_patch_for_content_that_moved_on_is_refused_by_git() {
        let fixture =
            GitPatchFixture::committed("render-stale", &[("src/main.rs", "fn main() {}\n")]);
        let rendered = render_patch(
            "src/main.rs",
            &[&hunk(
                "@@ -1,1 +1,1 @@",
                1,
                1,
                1,
                1,
                vec![
                    ("deletion", "fn main() {}"),
                    ("addition", "fn main() { work(); }"),
                ],
            )],
        );
        fixture.write("src/main.rs", "fn main() { somebody_else(); }\n");

        // Not a claim about this renderer: a claim that the check can fail. A gate that passed for
        // every input would let 13.13 pass while proving nothing.
        assert!(matches!(
            fixture.apply_check(&rendered),
            PatchCheck::Refused(_)
        ));
    }

    #[test]
    fn several_hunks_render_into_one_patch_that_applies_as_a_whole() {
        let fixture = GitPatchFixture::committed(
            "render-multi-hunk",
            &[("src/main.rs", "one\ntwo\nthree\nfour\nfive\nsix\nseven\n")],
        );
        let rendered = render_patch(
            "src/main.rs",
            &[
                &hunk(
                    "@@ -1,3 +1,3 @@",
                    1,
                    3,
                    1,
                    3,
                    vec![
                        ("deletion", "one"),
                        ("addition", "ONE"),
                        ("context", "two"),
                        ("context", "three"),
                    ],
                ),
                &hunk(
                    "@@ -5,3 +5,3 @@",
                    5,
                    3,
                    5,
                    3,
                    vec![
                        ("context", "five"),
                        ("deletion", "six"),
                        ("addition", "SIX"),
                        ("context", "seven"),
                    ],
                ),
            ],
        );

        // One file header for the file, one header per hunk. A renderer that repeated the file
        // header per hunk produces something Git reads as two patches for the same file.
        assert_eq!(rendered.matches("diff --git").count(), 1);
        assert_eq!(rendered.matches("@@ ").count(), 2);
        assert_eq!(fixture.apply_check(&rendered), PatchCheck::Applies);
    }

    fn diff_file(binary: bool, oversized: bool, hunks: Vec<ReviewDiffHunk>) -> ReviewDiffFile {
        ReviewDiffFile {
            summary: ReviewFileSummary {
                path: "src/main.rs".into(),
                previous_path: None,
                change_type: "modified".into(),
                old_hash: None,
                new_hash: Some("new".into()),
                binary,
                oversized,
            },
            hunks,
            truncated: false,
            accepted_bytes: 0,
        }
    }

    fn one_hunk() -> Vec<ReviewDiffHunk> {
        vec![hunk(
            "@@ -1,1 +1,1 @@",
            1,
            1,
            1,
            1,
            vec![
                ("deletion", "fn main() {}"),
                ("addition", "fn main() { work(); }"),
            ],
        )]
    }

    /// The four refusals, at the point they are decided rather than through a session fixture.
    ///
    /// Each is a different sentence for the reviewer -- reload it, this file is not text, this
    /// change is too big to hand over -- so each gets a code rather than one shared "unavailable".
    #[test]
    fn a_binary_file_has_no_patch_to_copy() {
        let file = diff_file(true, false, one_hunk());
        assert!(file.summary.binary);
        // Rendering first and refusing after would produce something confident and wrong for
        // content this application never decoded.
        assert_eq!(patch_refusal(&file), Some("patch_unavailable_binary"));
    }

    #[test]
    fn an_oversized_file_has_no_patch_to_copy() {
        assert_eq!(
            patch_refusal(&diff_file(false, true, one_hunk())),
            Some("patch_too_large")
        );
    }

    #[test]
    fn a_readable_file_has_no_refusal() {
        assert_eq!(patch_refusal(&diff_file(false, false, one_hunk())), None);
    }

    #[test]
    fn a_patch_over_the_bound_is_refused_rather_than_cut_short() {
        // A patch cut short looks exactly like one that applies, until somebody runs it somewhere
        // it matters.
        let oversize = "x".repeat(MAX_REVIEW_PATCH_BYTES + 1);
        assert!(patch_exceeds_bound(&oversize));
        assert!(!patch_exceeds_bound(&"x".repeat(MAX_REVIEW_PATCH_BYTES)));
    }

    #[test]
    fn relative_paths_reject_traversal_absolute_and_hidden_components() {
        assert!(validate_relative_path("src/main.rs").is_ok());
        assert!(validate_relative_path("../secret").is_err());
        assert!(validate_relative_path(".git/config").is_err());
        assert!(validate_relative_path("C:\\secret").is_err());
    }

    #[test]
    fn workspace_root_handles_valid_missing_and_absent_paths() {
        let root = temp_dir("workspace-root");
        fs::create_dir_all(&root).expect("root");
        let resolved = canonical_workspace_root(root.to_str())
            .expect("valid root")
            .expect("available root");
        assert!(resolved.is_absolute());
        assert!(
            canonical_workspace_root(Some(root.join("missing").to_string_lossy().as_ref()))
                .expect("missing root")
                .is_none()
        );
        assert!(canonical_workspace_root(None)
            .expect("absent root")
            .is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn sqlite_workspace_projection_preserves_root_priority_remote_and_missing_semantics() {
        let fixture = TempDirectory::new("workspace-query-projection");
        let folder = fixture.path().join("folder");
        let project = fixture.path().join("project");
        let worktree = fixture.path().join("worktree");
        for path in [&folder, &project, &worktree] {
            fs::create_dir_all(path).expect("workspace directory");
        }
        let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
        let connection = database.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO sessions \
                 (id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, \
                  worktree_path, pinned, archived, created_at, updated_at) \
                 VALUES ('session-local', 'Local fixture', 'codex-cli', 'cli', 'idle', ?1, ?2, ?3, \
                         0, 0, '2026-07-18T12:00:00Z', '2026-07-18T12:00:00Z')",
                params![
                    folder.to_string_lossy().as_ref(),
                    project.to_string_lossy().as_ref(),
                    worktree.to_string_lossy().as_ref(),
                ],
            )
            .expect("insert local session");
        connection
            .execute(
                "INSERT INTO sessions \
                 (id, title, agent_id, interaction_mode, lifecycle_state, remote_workspace_host, \
                  remote_workspace_path, remote_workspace_display_name, remote_workspace_uri, \
                  pinned, archived, created_at, updated_at) \
                 VALUES ('session-remote', 'Remote fixture', 'codex-cli', 'cli', 'idle', \
                         'example.com', '/work/app', 'Remote app', 'ssh://example.com/work/app', \
                         0, 0, '2026-07-18T12:00:00Z', '2026-07-18T12:00:00Z')",
                [],
            )
            .expect("insert remote session");

        assert_eq!(
            resolve_session_root(&connection, "session-local").expect("local root"),
            Some(worktree.canonicalize().expect("canonical worktree"))
        );
        fs::remove_dir_all(&worktree).expect("remove stale worktree");
        assert_eq!(
            resolve_session_root(&connection, "session-local").expect("folder fallback"),
            Some(folder.canonicalize().expect("canonical folder"))
        );
        fs::remove_dir_all(&folder).expect("remove stale folder");
        assert_eq!(
            resolve_session_root(&connection, "session-local").expect("project fallback"),
            Some(project.canonicalize().expect("canonical project"))
        );
        assert_eq!(
            resolve_session_root(&connection, "session-remote").expect("remote root"),
            None
        );
        assert_eq!(
            resolve_session_root(&connection, "missing"),
            Err(AppError::SessionNotFound("missing".to_string()))
        );
    }

    #[test]
    fn directory_listing_helpers_sort_and_bound_content_states() {
        let root = temp_dir("files");
        fs::create_dir_all(root.join("AFolder")).expect("directory");
        fs::create_dir_all(root.join(".hidden")).expect("hidden directory");
        fs::write(root.join("z-text.txt"), "hello").expect("text");
        fs::write(root.join("binary.bin"), [0, 1, 2]).expect("binary");
        let oversized = fs::File::create(root.join("oversized.txt")).expect("oversized file");
        oversized.set_len(FILE_BYTE_LIMIT + 1).expect("set length");
        let root = root.canonicalize().expect("canonical root");
        let (entries, truncated, next_cursor) =
            directory_page_at(&root, "", None, DEFAULT_DIRECTORY_PAGE_SIZE).expect("listing");
        assert!(!truncated);
        // No cursor for a directory that ended. Issuing one would invite a caller to fetch a
        // page that is always empty, which reads as a directory that just emptied itself.
        assert_eq!(next_cursor, None);
        assert_eq!(entries[0].name, "AFolder");
        assert!(entries.iter().all(|entry| entry.name != ".hidden"));
        assert_eq!(
            read_file_at(&root, "z-text.txt").expect("read").status,
            "text"
        );
        assert_eq!(
            read_file_at(&root, "binary.bin").expect("read").status,
            "binary"
        );
        assert_eq!(
            read_file_at(&root, "oversized.txt").expect("read").status,
            "oversized"
        );
        assert_eq!(
            read_file_at(&root, "missing.txt").expect("read").status,
            "missing"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn directory_and_document_results_are_bounded() {
        let root = temp_dir("bounds");
        fs::create_dir_all(&root).expect("root");
        for index in 0..=DEFAULT_DIRECTORY_PAGE_SIZE {
            fs::write(root.join(format!("file-{index:04}.txt")), "text").expect("fixture");
        }
        let root = root.canonicalize().expect("canonical root");
        let (entries, truncated, next_cursor) =
            directory_page_at(&root, "", None, DEFAULT_DIRECTORY_PAGE_SIZE).expect("listing");
        assert_eq!(entries.len(), DEFAULT_DIRECTORY_PAGE_SIZE);
        assert!(truncated);
        // A bound that reports itself and offers a way past it. The previous version stopped at
        // the same place and left the caller with no way to see the rest.
        assert!(next_cursor.is_some());
        let mut visited = HashSet::new();
        let mut documents = Vec::new();
        assert!(
            collect_documents(&root, &root, 0, &mut visited, &mut documents).expect("documents")
        );
        assert_eq!(documents.len(), DOCUMENT_LIMIT);
        fs::remove_dir_all(root).expect("cleanup");
    }

    /// The Documents tab requirement scopes this listing to Markdown and text. Mention
    /// candidate search widened its own bounds; this listing must not have moved with it.
    #[test]
    fn document_discovery_still_admits_only_markdown_and_text() {
        let fixture = TempDirectory::new("documents-scope");
        let root = fixture.path();
        for name in [
            "notes.md",
            "notes.markdown",
            "notes.txt",
            "main.rs",
            "app.ts",
            "config.json",
        ] {
            fs::write(root.join(name), "fixture").expect("fixture file");
        }
        let root = root.canonicalize().expect("canonical root");
        let mut documents = Vec::new();
        collect_documents(&root, &root, 0, &mut HashSet::new(), &mut documents).expect("documents");
        let mut names: Vec<String> = documents
            .into_iter()
            .map(|document| document.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["notes.markdown", "notes.md", "notes.txt"]);
    }

    /// Vendored trees stay visible to the Documents tab; only mention search excludes them.
    #[test]
    fn document_discovery_still_descends_into_dependency_directories() {
        let fixture = TempDirectory::new("documents-vendored");
        let root = fixture.path();
        fs::create_dir_all(root.join("node_modules/pkg")).expect("vendored directory");
        fs::write(root.join("node_modules/pkg/readme.md"), "fixture").expect("fixture file");
        let root = root.canonicalize().expect("canonical root");
        let mut documents = Vec::new();
        collect_documents(&root, &root, 0, &mut HashSet::new(), &mut documents).expect("documents");
        assert_eq!(
            documents
                .into_iter()
                .map(|document| document.path)
                .collect::<Vec<_>>(),
            vec!["node_modules/pkg/readme.md".to_string()]
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_rejected() {
        use std::os::unix::fs::symlink;
        let root = temp_dir("symlink-root");
        let outside = temp_dir("symlink-outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        fs::write(outside.join("secret.txt"), "secret").expect("secret");
        symlink(outside.join("secret.txt"), root.join("escape.txt")).expect("symlink");
        let root = root.canonicalize().expect("canonical root");
        assert!(resolve_existing_path(&root, "escape.txt").is_err());
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    #[cfg(windows)]
    #[test]
    fn symlink_escape_is_rejected_when_supported() {
        use std::os::windows::fs::symlink_file;
        let root = temp_dir("symlink-root");
        let outside = temp_dir("symlink-outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        fs::write(outside.join("secret.txt"), "secret").expect("secret");
        if symlink_file(outside.join("secret.txt"), root.join("escape.txt")).is_ok() {
            let canonical_root = root.canonicalize().expect("canonical root");
            assert!(resolve_existing_path(&canonical_root, "escape.txt").is_err());
        }
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    #[test]
    fn parses_porcelain_status_and_renames() {
        let raw = b"## main...origin/main\0 M src/main.rs\0R  new.rs\0old.rs\0?? note.txt\0";
        let (branch, entries) = parse_git_status(raw);
        assert_eq!(branch.as_deref(), Some("main"));
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].previous_path.as_deref(), Some("old.rs"));
        assert_eq!(entries[2].worktree, "untracked");
    }

    #[test]
    fn review_revert_requires_confirmation_before_workspace_access() {
        let connection = Connection::open_in_memory().expect("database");
        let request = ReviewRevertRequest {
            session_id: "session-1".into(),
            path: "src/a.rs".into(),
            expected_snapshot: "snapshot".into(),
            hunk_fingerprint: None,
            confirmed: false,
        };
        assert!(matches!(
            revert_review_change(&connection, &request),
            Err(AppError::PolicyDenied { .. })
        ));
    }

    #[test]
    fn review_patch_contains_only_selected_hunk() {
        let hunk = GitDiffHunk {
            header: "@@ -1 +1 @@".into(),
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 1,
            lines: vec![
                GitDiffLine {
                    kind: "deletion".into(),
                    content: "old".into(),
                    old_line_number: Some(1),
                    new_line_number: None,
                },
                GitDiffLine {
                    kind: "addition".into(),
                    content: "new".into(),
                    old_line_number: None,
                    new_line_number: Some(1),
                },
            ],
        };
        let review = ReviewDiffHunk {
            fingerprint: "hunk".into(),
            context_fingerprints: Vec::new(),
            hunk,
        };
        let patch = render_patch("src/a.rs", &[&review]);
        assert!(patch.contains("--- a/src/a.rs\n+++ b/src/a.rs"));
        assert!(patch.contains("-old\n+new"));
    }

    #[test]
    fn parses_structured_diff_hunks() {
        let raw = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,2 @@\n-old\n+new\n same\n";
        let files = parse_git_diff(raw, "src/a.rs");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks[0].lines[0].kind, "deletion");
        assert_eq!(files[0].hunks[0].lines[1].kind, "addition");
        assert!(parse_git_diff("malformed diff without headers", "src/a.rs").is_empty());
    }

    #[test]
    fn git_fixtures_cover_non_git_and_common_worktree_states() {
        let non_git = temp_dir("non-git");
        fs::create_dir_all(&non_git).expect("non git root");
        assert!(git_status_at(&non_git).expect("non git status").is_none());
        fs::remove_dir_all(non_git).expect("cleanup non git");

        let root = temp_dir("git-fixture");
        fs::create_dir_all(&root).expect("git root");
        run_git(&root, &["init"]);
        run_git(&root, &["config", "user.email", "tests@example.invalid"]);
        run_git(&root, &["config", "user.name", "VaneHub Tests"]);
        fs::write(root.join("modified.txt"), "before\n").expect("modified fixture");
        fs::write(root.join("rename-old.txt"), "rename\n").expect("rename fixture");
        fs::write(root.join("deleted.txt"), "delete\n").expect("delete fixture");
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-m", "fixture"]);
        fs::write(root.join("modified.txt"), "after\n").expect("modify");
        fs::remove_file(root.join("deleted.txt")).expect("delete");
        run_git(&root, &["mv", "rename-old.txt", "rename-new.txt"]);
        fs::write(root.join("staged.txt"), "staged\n").expect("staged");
        run_git(&root, &["add", "staged.txt"]);
        fs::write(root.join("untracked.txt"), "untracked\n").expect("untracked");
        fs::write(root.join("binary.bin"), [0, 1, 2]).expect("binary");

        let canonical = root.canonicalize().expect("canonical root");
        let (_, entries) = git_status_at(&canonical)
            .expect("git status")
            .expect("git repository");
        assert!(entries
            .iter()
            .any(|entry| entry.path == "modified.txt" && entry.worktree == "modified"));
        assert!(entries
            .iter()
            .any(|entry| entry.path == "deleted.txt" && entry.worktree == "deleted"));
        assert!(entries
            .iter()
            .any(|entry| entry.path == "rename-new.txt" && entry.index == "renamed"));
        assert!(entries
            .iter()
            .any(|entry| entry.path == "staged.txt" && entry.index == "added"));
        assert!(entries
            .iter()
            .any(|entry| entry.path == "untracked.txt" && entry.worktree == "untracked"));
        assert!(
            untracked_diff(&canonical, "binary.bin")
                .expect("binary diff")
                .expect("binary file")
                .binary
        );
        let working = git_output(
            &canonical,
            &[
                "diff".to_string(),
                "--".to_string(),
                "modified.txt".to_string(),
            ],
        )
        .expect("working diff");
        assert!(working.status.success());
        assert!(
            !parse_git_diff(&String::from_utf8_lossy(&working.stdout), "modified.txt").is_empty()
        );
        let staged = git_output(
            &canonical,
            &[
                "diff".to_string(),
                "--cached".to_string(),
                "--".to_string(),
                "staged.txt".to_string(),
            ],
        )
        .expect("staged diff");
        assert!(staged.status.success());
        assert!(!parse_git_diff(&String::from_utf8_lossy(&staged.stdout), "staged.txt").is_empty());
        let failed = git_output(&canonical, &["not-a-real-subcommand".to_string()])
            .expect("failed git output");
        assert!(!failed.status.success());
        fs::remove_dir_all(canonical).expect("cleanup git");
    }

    #[test]
    fn a_seat_filter_matches_only_records_carrying_that_seat() {
        let root = temp_dir("logs-seat");
        fs::create_dir_all(&root).expect("log dir");
        for (seat, message) in [
            (Some("seat-planner"), "planner shell connected"),
            (Some("seat-builder"), "builder shell connected"),
            (None, "session runtime started"),
        ] {
            let mut context = BTreeMap::new();
            context.insert("sessionId".to_string(), "session-1".to_string());
            if let Some(seat) = seat {
                context.insert("seatId".to_string(), seat.to_string());
            }
            logging::write_message(
                &root,
                logging::LogLevel::Info,
                "session.shell",
                message,
                context,
            )
            .expect("seat log");
        }

        let page = query_logs(
            &root,
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![],
                search: String::new(),
                seat_id: Some("seat-builder".to_string()),
                cursor: None,
                limit: None,
            },
        )
        .expect("seat-filtered query");

        // The uncorrelated record is not attributed to whichever seat happens to be selected.
        assert_eq!(
            page.items
                .iter()
                .map(|entry| entry.message.as_str())
                .collect::<Vec<_>>(),
            vec!["builder shell connected"]
        );

        let all_seats = query_logs(
            &root,
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![],
                search: String::new(),
                seat_id: None,
                cursor: None,
                limit: None,
            },
        )
        .expect("all-seat query");
        assert_eq!(all_seats.items.len(), 3);

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn log_query_is_session_scoped_filtered_and_bounded() {
        let root = temp_dir("logs");
        fs::create_dir_all(&root).expect("log dir");
        let mut first_context = BTreeMap::new();
        first_context.insert("sessionId".to_string(), "session-1".to_string());
        logging::write_message(
            &root,
            logging::LogLevel::Info,
            "session.runtime",
            "safe message",
            first_context,
        )
        .expect("first log");
        let mut second_context = BTreeMap::new();
        second_context.insert("sessionId".to_string(), "session-2".to_string());
        logging::write_message(
            &root,
            logging::LogLevel::Error,
            "session.runtime",
            "other message",
            second_context,
        )
        .expect("second log");
        let mut third_context = BTreeMap::new();
        third_context.insert("sessionId".to_string(), "session-1".to_string());
        logging::write_message(
            &root,
            logging::LogLevel::Info,
            "session.runtime",
            "safe newest",
            third_context,
        )
        .expect("third log");
        use std::fs::OpenOptions;
        let mut log_file = OpenOptions::new()
            .append(true)
            .open(root.join(logging::LOG_FILE_NAME))
            .expect("open log");
        writeln!(log_file, "not-json").expect("malformed line");
        let page = query_logs(
            &root,
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![WorkspaceLogLevel::Info],
                search: "safe".to_string(),
                seat_id: None,
                cursor: None,
                limit: Some(1),
            },
        )
        .expect("query");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].message, "safe newest");
        assert!(page.truncated);
        let second_page = query_logs(
            &root,
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![WorkspaceLogLevel::Info],
                search: "safe".to_string(),
                seat_id: None,
                cursor: page.next_cursor,
                limit: Some(1),
            },
        )
        .expect("second page");
        assert_eq!(second_page.items[0].message, "safe message");
        assert!(!second_page.truncated);
        let entries = filtered_log_entries(
            &root.join(logging::LOG_FILE_NAME),
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![],
                search: String::new(),
                seat_id: None,
                cursor: None,
                limit: None,
            },
        )
        .expect("filtered export entries");
        assert_eq!(
            export_log_entries(None, &entries)
                .expect("cancelled export")
                .status,
            "cancelled"
        );
        let export_path = root.join("export.jsonl");
        let exported = export_log_entries(Some(export_path.clone()), &entries).expect("export");
        assert_eq!(exported.status, "exported");
        let exported_text = fs::read_to_string(export_path).expect("exported text");
        assert!(exported_text.contains("safe message"));
        assert!(!exported_text.contains("not-json"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn bounded_log_query_reads_the_newest_complete_entries() {
        let root = temp_dir("bounded-log-tail");
        let message_padding = "x".repeat((LOG_QUERY_BYTE_LIMIT / 2 + 1024) as usize);
        for message in [
            format!("older {message_padding}"),
            format!("newest {message_padding}"),
        ] {
            let mut context = BTreeMap::new();
            context.insert("sessionId".to_string(), "session-1".to_string());
            logging::write_message(
                &root,
                logging::LogLevel::Info,
                "session.runtime",
                &message,
                context,
            )
            .expect("write log");
        }

        let entries = bounded_filtered_log_entries(
            &root,
            &SessionLogQuery {
                session_id: "session-1".to_string(),
                levels: vec![WorkspaceLogLevel::Info],
                search: String::new(),
                seat_id: None,
                cursor: None,
                limit: None,
            },
        )
        .expect("bounded query");

        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.starts_with("newest"));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
