use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn codex_session_root() -> Option<PathBuf> {
    if let Some(root) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(root).join("sessions"));
    }
    user_home().map(|home| home.join(".codex").join("sessions"))
}

fn opencode_database_path() -> Option<PathBuf> {
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
