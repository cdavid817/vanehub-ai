use super::{
    active_log_dir_from_conn, command_safety, load_session, logging, AppError, RegistryStore,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

const DIRECTORY_ENTRY_LIMIT: usize = 500;
const DOCUMENT_DEPTH_LIMIT: usize = 6;
const DOCUMENT_LIMIT: usize = 300;
const FILE_BYTE_LIMIT: u64 = 1024 * 1024;
const DIFF_BYTE_LIMIT: usize = 2 * 1024 * 1024;
const LOG_PAGE_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionWorkspaceContext {
    availability: &'static str,
    root_name: Option<String>,
    reason: Option<String>,
}

impl SessionWorkspaceContext {
    fn available(root: &Path) -> Self {
        Self {
            availability: "available",
            root_name: root
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            reason: None,
        }
    }

    fn unavailable(reason: &str) -> Self {
        Self {
            availability: "unavailable",
            root_name: None,
            reason: Some(reason.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntry {
    name: String,
    path: String,
    kind: &'static str,
    size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryListing {
    context: SessionWorkspaceContext,
    path: String,
    items: Vec<DirectoryEntry>,
    truncated: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDocument {
    name: String,
    path: String,
    kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentListing {
    context: SessionWorkspaceContext,
    items: Vec<SessionDocument>,
    truncated: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileContent {
    path: String,
    name: String,
    status: &'static str,
    size: u64,
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitStatusEntry {
    path: String,
    previous_path: Option<String>,
    index: String,
    worktree: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitStatusResult {
    context: SessionWorkspaceContext,
    is_git: bool,
    branch: Option<String>,
    items: Vec<GitStatusEntry>,
    truncated: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GitDiffSource {
    Working,
    Staged,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffLine {
    kind: String,
    content: String,
    old_line_number: Option<usize>,
    new_line_number: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffHunk {
    header: String,
    old_start: usize,
    old_lines: usize,
    new_start: usize,
    new_lines: usize,
    lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffFile {
    old_path: Option<String>,
    new_path: String,
    binary: bool,
    oversized: bool,
    hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffResult {
    context: SessionWorkspaceContext,
    source: GitDiffSource,
    files: Vec<GitDiffFile>,
    truncated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogQuery {
    session_id: String,
    levels: Vec<logging::LogLevel>,
    search: String,
    cursor: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogEntry {
    id: String,
    timestamp: String,
    level: logging::LogLevel,
    category: String,
    message: String,
    context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogPage {
    items: Vec<SessionLogEntry>,
    truncated: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogExportResult {
    status: &'static str,
    path: Option<String>,
}

pub(crate) fn resolve_session_root(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<PathBuf>, AppError> {
    let session = load_session(conn, session_id)?;
    let candidate = session
        .worktree_path
        .as_ref()
        .or(session.folder.as_ref())
        .or(session.project_path.as_ref());
    canonical_workspace_root(candidate.map(String::as_str))
}

fn canonical_workspace_root(candidate: Option<&str>) -> Result<Option<PathBuf>, AppError> {
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let path = PathBuf::from(candidate);
    if !path.exists() || !path.is_dir() {
        return Ok(None);
    }
    path.canonicalize()
        .map(Some)
        .map_err(|error| AppError::Storage(error.to_string()))
}

fn validate_relative_path(path: &str) -> Result<PathBuf, AppError> {
    let relative = PathBuf::from(path);
    if relative.is_absolute() {
        return Err(AppError::Validation(
            "Session workspace paths must be relative.".to_string(),
        ));
    }
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                if name.to_string_lossy().starts_with('.') {
                    return Err(AppError::Validation(
                        "Hidden workspace paths are unavailable.".to_string(),
                    ));
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::Validation(
                    "Session workspace path escapes are not allowed.".to_string(),
                ));
            }
        }
    }
    Ok(relative)
}

fn resolve_existing_path(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let relative_path = validate_relative_path(relative)?;
    let candidate = root.join(relative_path);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    if !canonical.starts_with(root) {
        return Err(AppError::Validation(
            "Session workspace path resolves outside the session root.".to_string(),
        ));
    }
    Ok(canonical)
}

fn resolve_git_path(root: &Path, relative: &str) -> Result<(PathBuf, String), AppError> {
    let relative_path = validate_relative_path(relative)?;
    let candidate = root.join(&relative_path);
    let parent = candidate.parent().ok_or_else(|| {
        AppError::Validation("Git path has no valid parent.".to_string())
    })?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    if !canonical_parent.starts_with(root) {
        return Err(AppError::Validation(
            "Git path resolves outside the session root.".to_string(),
        ));
    }
    Ok((candidate, relative_path.to_string_lossy().replace('\\', "/")))
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, AppError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn unavailable_context() -> SessionWorkspaceContext {
    SessionWorkspaceContext::unavailable("Session workspace is unavailable.")
}

fn directory_entries_at(
    root: &Path,
    relative: &str,
) -> Result<(Vec<DirectoryEntry>, bool), AppError> {
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
        let metadata = fs::metadata(&canonical).map_err(|error| AppError::Storage(error.to_string()))?;
        let kind = if metadata.is_dir() { "directory" } else { "file" };
        entries.push(DirectoryEntry {
            name,
            path: normalized_relative(root, &canonical)?,
            kind,
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
        });
    }
    entries.sort_by(|left, right| {
        let left_rank = if left.kind == "directory" { 0 } else { 1 };
        let right_rank = if right.kind == "directory" { 0 } else { 1 };
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let truncated = entries.len() > DIRECTORY_ENTRY_LIMIT;
    entries.truncate(DIRECTORY_ENTRY_LIMIT);
    Ok((entries, truncated))
}

pub(crate) fn list_session_directory(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
) -> Result<DirectoryListing, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let Some(root) = resolve_session_root(&conn, &session_id)? else {
        return Ok(DirectoryListing {
            context: unavailable_context(),
            path,
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let (items, truncated) = directory_entries_at(&root, &path)?;
    Ok(DirectoryListing {
        context: SessionWorkspaceContext::available(&root),
        path,
        items,
        truncated,
        next_cursor: None,
    })
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
    for entry in fs::read_dir(&canonical_directory).map_err(|error| AppError::Storage(error.to_string()))? {
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
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<DocumentListing, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let Some(root) = resolve_session_root(&conn, &session_id)? else {
        return Ok(DocumentListing {
            context: unavailable_context(),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let mut documents = Vec::new();
    let truncated = collect_documents(
        &root,
        &root,
        0,
        &mut HashSet::new(),
        &mut documents,
    )?;
    documents.sort_by_key(|document| document.path.to_lowercase());
    Ok(DocumentListing {
        context: SessionWorkspaceContext::available(&root),
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
        });
    }
    match String::from_utf8(bytes) {
        Ok(content) => Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "text",
            size: metadata.len(),
            content: Some(content),
        }),
        Err(_) => Ok(FileContent {
            path: relative.to_string(),
            name,
            status: "binary",
            size: metadata.len(),
            content: None,
        }),
    }
}

pub(crate) fn read_session_file(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
) -> Result<FileContent, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let root = resolve_session_root(&conn, &session_id)?.ok_or_else(|| {
        AppError::Validation("Session workspace is unavailable.".to_string())
    })?;
    read_file_at(&root, &path)
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

fn parse_git_status(raw: &[u8]) -> (Option<String>, Vec<GitStatusEntry>) {
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
            branch = Some(value.split("...").next().unwrap_or(value).trim().to_string());
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

fn git_output(root: &Path, args: &[String]) -> Result<std::process::Output, AppError> {
    command_safety::audit_command("session.git", "git", args);
    let mut command = command_safety::std_command("git")?;
    command.current_dir(root).args(args);
    command
        .output()
        .map_err(|error| AppError::LaunchFailed(error.to_string()))
}

fn write_git_failure(
    conn: &Connection,
    session_id: &str,
    agent_id: &str,
    message: &str,
) {
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

pub(crate) fn get_session_git_status(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<GitStatusResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let session = load_session(&conn, &session_id)?;
    let Some(root) = resolve_session_root(&conn, &session_id)? else {
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
            write_git_failure(&conn, &session_id, &session.agent_id, &error.to_string());
            return Err(error);
        }
    };
    let Some((branch, mut entries)) = result else {
        return Ok(GitStatusResult {
            context: SessionWorkspaceContext::available(&root),
            is_git: false,
            branch: None,
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
    };
    let truncated = entries.len() > DIRECTORY_ENTRY_LIMIT;
    entries.truncate(DIRECTORY_ENTRY_LIMIT);
    Ok(GitStatusResult {
        context: SessionWorkspaceContext::available(&root),
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

fn parse_git_diff(raw: &str, fallback_path: &str) -> Vec<GitDiffFile> {
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
            let (kind, content, old_number, new_number) = if let Some(content) = line.strip_prefix('+') {
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
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
    source: GitDiffSource,
) -> Result<GitDiffResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let session = load_session(&conn, &session_id)?;
    let root = resolve_session_root(&conn, &session_id)?.ok_or_else(|| {
        AppError::Validation("Session workspace is unavailable.".to_string())
    })?;
    let (_candidate, normalized_path) = resolve_git_path(&root, &path)?;
    let status = match git_status_at(&root) {
        Ok(status) => status,
        Err(error) => {
            write_git_failure(&conn, &session_id, &session.agent_id, "Git status preflight failed.");
            return Err(error);
        }
    };
    let is_untracked = status
        .as_ref()
        .map(|(_, entries)| {
            entries.iter().any(|entry| {
                entry.path == normalized_path
                    && (entry.index == "untracked" || entry.worktree == "untracked")
            })
        })
        .unwrap_or(false);
    if is_untracked && source == GitDiffSource::Working {
        return Ok(GitDiffResult {
            context: SessionWorkspaceContext::available(&root),
            source,
            files: untracked_diff(&root, &normalized_path)?.into_iter().collect(),
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
            write_git_failure(&conn, &session_id, &session.agent_id, message);
            return Err(AppError::LaunchFailed(message.to_string()));
        }
        Err(error) => {
            write_git_failure(&conn, &session_id, &session.agent_id, &error.to_string());
            return Err(error);
        }
    };
    if output.stdout.len() > DIFF_BYTE_LIMIT {
        return Ok(GitDiffResult {
            context: SessionWorkspaceContext::available(&root),
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
        context: SessionWorkspaceContext::available(&root),
        source,
        files: parse_git_diff(&String::from_utf8_lossy(&output.stdout), &normalized_path),
        truncated: false,
    })
}

fn filtered_log_entries(path: &Path, input: &SessionLogQuery) -> Result<Vec<logging::LogEntry>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let search = input.search.trim().to_lowercase();
    let mut entries = Vec::new();
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(entry) = serde_json::from_str::<logging::LogEntry>(&line) else {
            continue;
        };
        if entry.context.get("sessionId") != Some(&input.session_id) {
            continue;
        }
        if !input.levels.is_empty() && !input.levels.contains(&entry.level) {
            continue;
        }
        if !search.is_empty() {
            let searchable = format!(
                "{} {} {}",
                entry.category,
                entry.message,
                serde_json::to_string(&entry.context).unwrap_or_default()
            )
            .to_lowercase();
            if !searchable.contains(&search) {
                continue;
            }
        }
        entries.push(entry);
    }
    entries.reverse();
    Ok(entries)
}

fn query_logs(log_dir: &Path, input: &SessionLogQuery) -> Result<SessionLogPage, AppError> {
    let entries = filtered_log_entries(&log_dir.join(logging::LOG_FILE_NAME), input)?;
    let offset = input
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = input.limit.unwrap_or(LOG_PAGE_LIMIT).clamp(1, LOG_PAGE_LIMIT);
    let items = entries
        .iter()
        .skip(offset)
        .take(limit)
        .enumerate()
        .map(|(index, entry)| SessionLogEntry {
            id: format!("{}-{}", entry.timestamp, offset + index),
            timestamp: entry.timestamp.clone(),
            level: entry.level,
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
        let line = serde_json::to_string(entry)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        writeln!(file, "{line}").map_err(|error| AppError::Storage(error.to_string()))?;
    }
    Ok(SessionLogExportResult {
        status: "exported",
        path: Some(path.to_string_lossy().to_string()),
    })
}

pub(crate) fn list_session_logs(
    state: State<'_, Mutex<RegistryStore>>,
    input: SessionLogQuery,
) -> Result<SessionLogPage, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &input.session_id)?;
    query_logs(&active_log_dir_from_conn(&conn)?, &input)
}

pub(crate) fn export_session_logs(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    input: SessionLogQuery,
) -> Result<SessionLogExportResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    load_session(&conn, &input.session_id)?;
    let entries = filtered_log_entries(
        &active_log_dir_from_conn(&conn)?.join(logging::LOG_FILE_NAME),
        &input,
    )?;
    let selected = app
        .dialog()
        .file()
        .set_file_name(format!("vanehub-session-{}.log", input.session_id))
        .blocking_save_file();
    let path = selected
        .map(|value| value.into_path().map_err(|error| AppError::Validation(error.to_string())))
        .transpose()?;
    export_log_entries(path, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let resolved = canonical_workspace_root(root.to_str()).expect("valid root").expect("available root");
        assert!(resolved.is_absolute());
        assert!(canonical_workspace_root(Some(root.join("missing").to_string_lossy().as_ref())).expect("missing root").is_none());
        assert!(canonical_workspace_root(None).expect("absent root").is_none());
        fs::remove_dir_all(root).expect("cleanup");
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
        let (entries, truncated) = directory_entries_at(&root, "").expect("listing");
        assert!(!truncated);
        assert_eq!(entries[0].name, "AFolder");
        assert!(entries.iter().all(|entry| entry.name != ".hidden"));
        assert_eq!(read_file_at(&root, "z-text.txt").expect("read").status, "text");
        assert_eq!(read_file_at(&root, "binary.bin").expect("read").status, "binary");
        assert_eq!(read_file_at(&root, "oversized.txt").expect("read").status, "oversized");
        assert_eq!(read_file_at(&root, "missing.txt").expect("read").status, "missing");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn directory_and_document_results_are_bounded() {
        let root = temp_dir("bounds");
        fs::create_dir_all(&root).expect("root");
        for index in 0..=DIRECTORY_ENTRY_LIMIT {
            fs::write(root.join(format!("file-{index:04}.txt")), "text").expect("fixture");
        }
        let root = root.canonicalize().expect("canonical root");
        let (entries, truncated) = directory_entries_at(&root, "").expect("listing");
        assert_eq!(entries.len(), DIRECTORY_ENTRY_LIMIT);
        assert!(truncated);
        let mut visited = HashSet::new();
        let mut documents = Vec::new();
        assert!(collect_documents(&root, &root, 0, &mut visited, &mut documents).expect("documents"));
        assert_eq!(documents.len(), DOCUMENT_LIMIT);
        fs::remove_dir_all(root).expect("cleanup");
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
        assert!(entries.iter().any(|entry| entry.path == "modified.txt" && entry.worktree == "modified"));
        assert!(entries.iter().any(|entry| entry.path == "deleted.txt" && entry.worktree == "deleted"));
        assert!(entries.iter().any(|entry| entry.path == "rename-new.txt" && entry.index == "renamed"));
        assert!(entries.iter().any(|entry| entry.path == "staged.txt" && entry.index == "added"));
        assert!(entries.iter().any(|entry| entry.path == "untracked.txt" && entry.worktree == "untracked"));
        assert!(untracked_diff(&canonical, "binary.bin").expect("binary diff").expect("binary file").binary);
        let working = git_output(&canonical, &["diff".to_string(), "--".to_string(), "modified.txt".to_string()]).expect("working diff");
        assert!(working.status.success());
        assert!(!parse_git_diff(&String::from_utf8_lossy(&working.stdout), "modified.txt").is_empty());
        let staged = git_output(&canonical, &["diff".to_string(), "--cached".to_string(), "--".to_string(), "staged.txt".to_string()]).expect("staged diff");
        assert!(staged.status.success());
        assert!(!parse_git_diff(&String::from_utf8_lossy(&staged.stdout), "staged.txt").is_empty());
        let failed = git_output(&canonical, &["not-a-real-subcommand".to_string()]).expect("failed git output");
        assert!(!failed.status.success());
        fs::remove_dir_all(canonical).expect("cleanup git");
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
                levels: vec![logging::LogLevel::Info],
                search: "safe".to_string(),
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
                levels: vec![logging::LogLevel::Info],
                search: "safe".to_string(),
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
                cursor: None,
                limit: None,
            },
        )
        .expect("filtered export entries");
        assert_eq!(export_log_entries(None, &entries).expect("cancelled export").status, "cancelled");
        let export_path = root.join("export.jsonl");
        let exported = export_log_entries(Some(export_path.clone()), &entries).expect("export");
        assert_eq!(exported.status, "exported");
        let exported_text = fs::read_to_string(export_path).expect("exported text");
        assert!(exported_text.contains("safe message"));
        assert!(!exported_text.contains("not-json"));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
