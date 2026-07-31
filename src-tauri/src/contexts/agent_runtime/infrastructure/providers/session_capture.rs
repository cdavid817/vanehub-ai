use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderSessionDiscovery {
    Pending,
    Found(String),
    Ambiguous(usize),
}

#[derive(Debug, Clone)]
pub(crate) enum ProviderSessionCapture {
    Codex(CodexSessionBaseline),
    OpenCode(OpenCodeSessionBaseline),
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderSessionCaptureError {
    message: String,
}

impl std::fmt::Display for ProviderSessionCaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProviderSessionCaptureError {}

#[derive(Debug, Clone)]
pub(crate) struct CodexSessionBaseline {
    session_root: PathBuf,
    known_rollouts: HashSet<PathBuf>,
    working_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct OpenCodeSessionBaseline {
    database_path: PathBuf,
    known_ids: HashSet<String>,
    working_directory: PathBuf,
    pub(super) started_at_ms: i64,
}

#[derive(Debug, Deserialize)]
struct CodexSessionEnvelope {
    #[serde(rename = "type")]
    event_type: String,
    payload: CodexSessionMeta,
}

#[derive(Debug, Deserialize)]
struct CodexSessionMeta {
    id: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
}

pub(crate) fn prepare_provider_session_capture(
    agent_id: &str,
    working_directory: PathBuf,
) -> Result<Option<ProviderSessionCapture>, ProviderSessionCaptureError> {
    match agent_id {
        "codex-cli" => codex_session_root()
            .map(|root| {
                capture_codex_baseline(root, working_directory).map(ProviderSessionCapture::Codex)
            })
            .transpose(),
        "opencode" => opencode_database_path()
            .map(|path| {
                capture_opencode_baseline(path, working_directory)
                    .map(ProviderSessionCapture::OpenCode)
            })
            .transpose(),
        _ => Ok(None),
    }
}

impl ProviderSessionCapture {
    pub(crate) fn discover(&self) -> Result<ProviderSessionDiscovery, ProviderSessionCaptureError> {
        match self {
            Self::Codex(baseline) => discover_codex_session(baseline),
            Self::OpenCode(baseline) => discover_opencode_session(baseline),
        }
    }
}

pub(super) fn capture_codex_baseline(
    session_root: PathBuf,
    working_directory: PathBuf,
) -> Result<CodexSessionBaseline, ProviderSessionCaptureError> {
    Ok(CodexSessionBaseline {
        known_rollouts: collect_rollout_paths(&session_root)?,
        session_root,
        working_directory,
    })
}

pub(super) fn discover_codex_session(
    baseline: &CodexSessionBaseline,
) -> Result<ProviderSessionDiscovery, ProviderSessionCaptureError> {
    let mut candidates = Vec::new();
    let current_rollouts = collect_rollout_paths(&baseline.session_root)?;
    for path in current_rollouts.difference(&baseline.known_rollouts) {
        let Some(meta) = read_codex_session_meta(path)? else {
            continue;
        };
        let Some(cwd) = meta.cwd.as_deref() else {
            continue;
        };
        let Some(id) = meta
            .id
            .as_deref()
            .or(meta.session_id.as_deref())
            .filter(|id| !id.trim().is_empty())
        else {
            continue;
        };
        if paths_match(Path::new(cwd), &baseline.working_directory) {
            candidates.push(id.to_string());
        }
    }
    unique_candidate(candidates)
}

/// Post-hoc equivalent of `discover_codex_session`, used once the terminal process has
/// already exited. Matches by `cwd` (from each rollout's first-line `session_meta`) and
/// picks the most-recently-modified matching file, mirroring
/// `find_opencode_session_since`'s reasoning: no live-poll race once the process has
/// fully exited and stopped writing.
pub(crate) fn find_codex_rollout_since(
    session_root: &Path,
    working_directory: &Path,
    since_ms: i64,
) -> Result<Option<PathBuf>, ProviderSessionCaptureError> {
    let since = UNIX_EPOCH + Duration::from_millis(since_ms.saturating_sub(2_000).max(0) as u64);
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();
    for path in collect_rollout_paths(session_root)? {
        let Some(meta) = read_codex_session_meta(&path)? else {
            continue;
        };
        let Some(cwd) = meta.cwd.as_deref() else {
            continue;
        };
        if !paths_match(Path::new(cwd), working_directory) {
            continue;
        }
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .map_err(|error| capture_error("Codex", error))?;
        if modified < since {
            continue;
        }
        candidates.push((modified, path));
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    Ok(candidates.pop().map(|(_, path)| path))
}

fn collect_rollout_paths(
    session_root: &Path,
) -> Result<HashSet<PathBuf>, ProviderSessionCaptureError> {
    if !session_root.exists() {
        return Ok(HashSet::new());
    }
    let mut paths = HashSet::new();
    collect_rollout_paths_at_depth(session_root, 0, &mut paths)?;
    Ok(paths)
}

fn collect_rollout_paths_at_depth(
    directory: &Path,
    depth: usize,
    paths: &mut HashSet<PathBuf>,
) -> Result<(), ProviderSessionCaptureError> {
    if depth > 4 {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| capture_error("Codex", error))?;
    for entry in entries {
        let entry = entry.map_err(|error| capture_error("Codex", error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| capture_error("Codex", error))?;
        if file_type.is_dir() {
            collect_rollout_paths_at_depth(&entry.path(), depth + 1, paths)?;
            continue;
        }
        let path = entry.path();
        let is_rollout = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"));
        if is_rollout {
            paths.insert(path);
        }
    }
    Ok(())
}

fn read_codex_session_meta(
    path: &Path,
) -> Result<Option<CodexSessionMeta>, ProviderSessionCaptureError> {
    let file = File::open(path).map_err(|error| capture_error("Codex", error))?;
    let mut lines = BufReader::new(file).lines();
    let Some(line) = lines.next() else {
        return Ok(None);
    };
    let line = line.map_err(|error| capture_error("Codex", error))?;
    let Ok(envelope) = serde_json::from_str::<CodexSessionEnvelope>(&line) else {
        return Ok(None);
    };
    if envelope.event_type != "session_meta" {
        return Ok(None);
    }
    Ok(Some(envelope.payload))
}

pub(super) fn capture_opencode_baseline(
    database_path: PathBuf,
    working_directory: PathBuf,
) -> Result<OpenCodeSessionBaseline, ProviderSessionCaptureError> {
    Ok(OpenCodeSessionBaseline {
        known_ids: read_opencode_sessions(&database_path)?
            .into_iter()
            .map(|session| session.id)
            .collect(),
        database_path,
        working_directory,
        started_at_ms: unix_time_ms(),
    })
}

pub(super) fn discover_opencode_session(
    baseline: &OpenCodeSessionBaseline,
) -> Result<ProviderSessionDiscovery, ProviderSessionCaptureError> {
    let candidates = read_opencode_sessions(&baseline.database_path)?
        .into_iter()
        .filter(|session| !baseline.known_ids.contains(&session.id))
        .filter(|session| session.created_at_ms >= baseline.started_at_ms.saturating_sub(2_000))
        .filter(|session| {
            paths_match(
                Path::new(&session.working_directory),
                &baseline.working_directory,
            )
        })
        .map(|session| session.id)
        .collect();
    unique_candidate(candidates)
}

/// Post-hoc equivalent of `discover_opencode_session`, used once the terminal process
/// has already exited (so opencode has definitely finished writing its session row —
/// unlike the live poll in `discover_opencode_session`, which only runs while fresh PTY
/// output is arriving and can miss a session created after the last visible output).
/// Picks the most recently created match rather than requiring a unique one, since a
/// stale session from an earlier run in the same directory could otherwise be mistaken
/// for ambiguity.
pub(crate) fn find_opencode_session_since(
    database_path: &Path,
    working_directory: &Path,
    since_ms: i64,
) -> Result<Option<String>, ProviderSessionCaptureError> {
    let mut candidates: Vec<(i64, String)> = read_opencode_sessions(database_path)?
        .into_iter()
        .filter(|session| session.created_at_ms >= since_ms.saturating_sub(2_000))
        .filter(|session| paths_match(Path::new(&session.working_directory), working_directory))
        .map(|session| (session.created_at_ms, session.id))
        .collect();
    candidates.sort_by_key(|(created_at, _)| *created_at);
    Ok(candidates.pop().map(|(_, id)| id))
}

#[derive(Debug, Deserialize)]
struct GeminiProjectsRegistry {
    #[serde(default)]
    projects: HashMap<String, String>,
}

/// Post-hoc lookup for gemini-cli's own `ChatRecordingService` output, following the
/// same working-directory + start-time pattern as `find_opencode_session_since` /
/// `find_codex_rollout_since`. gemini-cli has no live `ProviderSessionCapture` poll to
/// mirror in the first place (that mechanism is codex-cli/opencode-specific), so this
/// is the only lookup, called on the same periodic timer as the other three CLIs.
///
/// The project directory is resolved via `~/.gemini/projects.json` — a
/// normalized-absolute-path -> slug map gemini-cli itself maintains and auto-populates
/// the first time it runs in a given directory (verified directly by reading the
/// installed `@google/gemini-cli` package's bundled source: `ProjectRegistry.getShortId`
/// / `Storage.getProjectIdentifier`). This recently replaced an older pure-hash scheme
/// (per a source comment: "Performs migration of legacy hash-based directories to the
/// new slug-based format"), so no hash-based assumption is used here.
pub(crate) fn find_gemini_chat_session_since(
    working_directory: &Path,
    since_ms: i64,
) -> Result<Option<PathBuf>, ProviderSessionCaptureError> {
    let Some(registry_path) = gemini_projects_registry_path() else {
        return Ok(None);
    };
    let Some(slug) = read_gemini_project_slug(&registry_path, working_directory)? else {
        return Ok(None);
    };
    let Some(gemini_home) = gemini_home_dir() else {
        return Ok(None);
    };
    let chats_dir = gemini_home.join("tmp").join(slug).join("chats");
    if !chats_dir.exists() {
        return Ok(None);
    }
    let since = UNIX_EPOCH + Duration::from_millis(since_ms.saturating_sub(2_000).max(0) as u64);
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();
    let entries = fs::read_dir(&chats_dir).map_err(|error| capture_error("Gemini", error))?;
    for entry in entries {
        let entry = entry.map_err(|error| capture_error("Gemini", error))?;
        let path = entry.path();
        let is_chat_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".jsonl"));
        if !is_chat_file {
            continue;
        }
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .map_err(|error| capture_error("Gemini", error))?;
        if modified < since {
            continue;
        }
        candidates.push((modified, path));
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    Ok(candidates.pop().map(|(_, path)| path))
}

pub(super) fn read_gemini_project_slug(
    registry_path: &Path,
    working_directory: &Path,
) -> Result<Option<String>, ProviderSessionCaptureError> {
    if !registry_path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(registry_path).map_err(|error| capture_error("Gemini", error))?;
    let Ok(registry) = serde_json::from_str::<GeminiProjectsRegistry>(&content) else {
        return Ok(None);
    };
    Ok(registry
        .projects
        .into_iter()
        .find(|(project_path, _)| paths_match(Path::new(project_path), working_directory))
        .map(|(_, slug)| slug))
}

/// `~/.gemini`, exactly as gemini-cli's own `Storage.getGlobalGeminiDir()` resolves it
/// (verified directly against source: `path.join(homedir(), ".gemini")`, no environment
/// variable override exists for this in the installed package).
fn gemini_home_dir() -> Option<PathBuf> {
    user_home().map(|home| home.join(".gemini"))
}

fn gemini_projects_registry_path() -> Option<PathBuf> {
    gemini_home_dir().map(|home| home.join("projects.json"))
}

struct OpenCodeSessionRecord {
    id: String,
    working_directory: String,
    created_at_ms: i64,
}

fn read_opencode_sessions(
    database_path: &Path,
) -> Result<Vec<OpenCodeSessionRecord>, ProviderSessionCaptureError> {
    if !database_path.exists() {
        return Ok(Vec::new());
    }
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| capture_error("OpenCode", error))?;
    let mut statement = connection
        .prepare("SELECT id, directory, time_created FROM session")
        .map_err(|error| capture_error("OpenCode", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(OpenCodeSessionRecord {
                id: row.get(0)?,
                working_directory: row.get(1)?,
                created_at_ms: row.get(2)?,
            })
        })
        .map_err(|error| capture_error("OpenCode", error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| capture_error("OpenCode", error))
}

fn unique_candidate(
    mut candidates: Vec<String>,
) -> Result<ProviderSessionDiscovery, ProviderSessionCaptureError> {
    candidates.sort();
    candidates.dedup();
    Ok(match candidates.len() {
        0 => ProviderSessionDiscovery::Pending,
        1 => ProviderSessionDiscovery::Found(candidates.remove(0)),
        count => ProviderSessionDiscovery::Ambiguous(count),
    })
}

pub(crate) fn codex_session_root() -> Option<PathBuf> {
    if let Some(root) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(root).join("sessions"));
    }
    user_home().map(|home| home.join(".codex").join("sessions"))
}

pub(crate) fn opencode_database_path() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|root| root.join("opencode").join("opencode.db"))
        .or_else(|| {
            user_home().map(|home| {
                home.join(".local")
                    .join("share")
                    .join("opencode")
                    .join("opencode.db")
            })
        })
}

fn user_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        .map(PathBuf::from)
}

fn paths_match(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => normalized_path(left) == normalized_path(right),
    }
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    let value = value
        .strip_prefix("//?/")
        .unwrap_or(&value)
        .trim_end_matches('/');
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value.to_string()
    }
}

fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

fn capture_error(provider: &str, error: impl std::fmt::Display) -> ProviderSessionCaptureError {
    ProviderSessionCaptureError {
        message: format!("{provider} session metadata could not be read: {error}"),
    }
}
