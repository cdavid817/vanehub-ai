use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum LogStoreError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("launch failed: {0}")]
    LaunchFailed(String),
}

pub(crate) const LOG_FILE_NAME: &str = "vanehub.log";
const ARCHIVE_DIR_NAME: &str = "archive";
const RETENTION_DAYS: i64 = 30;
const ROTATION_AGE_HOURS: i64 = 24;
const MAINTENANCE_INTERVAL_HOURS: i64 = 1;

static ACTIVE_LOG_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static LOG_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static LAST_MAINTENANCE: OnceLock<Mutex<BTreeMap<PathBuf, DateTime<Utc>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub category: String,
    pub message: String,
    pub context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientLogEventKind {
    ErrorBoundary,
    CriticalOperationFailure,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientLogEvent {
    pub level: LogLevel,
    pub kind: ClientLogEventKind,
    pub message: String,
    pub source: String,
    pub details: Option<BTreeMap<String, String>>,
    pub stack: Option<String>,
}

pub fn default_log_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs")
}

pub fn set_active_log_dir(path: PathBuf) {
    if let Ok(path) = validate_log_dir(&path) {
        if let Ok(mut active) = ACTIVE_LOG_DIR.get_or_init(|| Mutex::new(None)).lock() {
            *active = Some(path);
        }
    }
}

pub fn active_log_dir(fallback: PathBuf) -> PathBuf {
    ACTIVE_LOG_DIR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|active| active.clone())
        .unwrap_or(fallback)
}

pub fn validate_log_dir(path: &Path) -> Result<PathBuf, LogStoreError> {
    fs::create_dir_all(path).map_err(|error| LogStoreError::Storage(error.to_string()))?;
    let metadata = fs::metadata(path).map_err(|error| LogStoreError::Storage(error.to_string()))?;
    if !metadata.is_dir() {
        return Err(LogStoreError::Validation(format!(
            "Log directory path is not a directory: {}",
            path.display()
        )));
    }
    Ok(path.to_path_buf())
}

pub fn write_entry(log_dir: &Path, entry: LogEntry) -> Result<(), LogStoreError> {
    let _write_guard = LOG_WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|error| LogStoreError::Storage(error.to_string()))?;
    let log_dir = validate_log_dir(log_dir)?;
    maintain_log_dir(&log_dir, Utc::now())?;
    let path = log_dir.join(LOG_FILE_NAME);
    let line = serde_json::to_string(&redact_entry(entry))
        .map_err(|error| LogStoreError::Storage(error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| LogStoreError::Storage(error.to_string()))?;
    writeln!(file, "{line}").map_err(|error| LogStoreError::Storage(error.to_string()))
}

pub(crate) fn write_message_raw(
    log_dir: &Path,
    level: LogLevel,
    category: &str,
    message: &str,
    context: BTreeMap<String, String>,
) -> Result<(), LogStoreError> {
    write_entry(
        log_dir,
        LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level,
            category: category.to_string(),
            message: message.to_string(),
            context,
        },
    )
}

pub fn write_message(
    log_dir: &Path,
    level: LogLevel,
    category: &str,
    message: &str,
    context: BTreeMap<String, String>,
) -> Result<(), LogStoreError> {
    write_message_raw(log_dir, level, category, message, context)
}

pub(crate) fn fallback_log_dir() -> PathBuf {
    let root = std::env::var_os("VANEHUB_APP_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .map(|home| home.join(".vanehub"))
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    default_log_dir(&root)
}

pub fn write_client_event(log_dir: &Path, event: ClientLogEvent) -> Result<(), LogStoreError> {
    let mut context = event.details.unwrap_or_default();
    context.insert("source".to_string(), event.source);
    context.insert("kind".to_string(), format!("{:?}", event.kind));
    if let Some(stack) = event.stack {
        context.insert("stack".to_string(), stack);
    }
    write_message(
        log_dir,
        event.level,
        "frontend.client",
        &event.message,
        context,
    )
}

pub fn open_directory(path: &Path) -> Result<(), LogStoreError> {
    validate_log_dir(path)?;
    crate::platform::filesystem::open_directory(path).map_err(LogStoreError::LaunchFailed)
}

fn maintain_log_dir(log_dir: &Path, now: DateTime<Utc>) -> Result<(), LogStoreError> {
    let mut maintenance = LAST_MAINTENANCE
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| LogStoreError::Storage(error.to_string()))?;
    if maintenance
        .get(log_dir)
        .is_some_and(|last_run| now - *last_run < Duration::hours(MAINTENANCE_INTERVAL_HOURS))
    {
        return Ok(());
    }

    rotate_active_log(log_dir, now)?;
    archive_expired_logs_at(log_dir, now)?;
    maintenance.insert(log_dir.to_path_buf(), now);
    Ok(())
}

fn rotate_active_log(log_dir: &Path, now: DateTime<Utc>) -> Result<(), LogStoreError> {
    let active_path = log_dir.join(LOG_FILE_NAME);
    if !active_path.exists() {
        return Ok(());
    }
    let modified: DateTime<Utc> = fs::metadata(&active_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| LogStoreError::Storage(error.to_string()))?
        .into();
    if now - modified < Duration::hours(ROTATION_AGE_HOURS) {
        return Ok(());
    }

    let target = next_rotated_log_path(log_dir, modified);
    fs::rename(active_path, target).map_err(|error| LogStoreError::Storage(error.to_string()))
}

fn next_rotated_log_path(log_dir: &Path, modified: DateTime<Utc>) -> PathBuf {
    let stem = format!("vanehub-{}", modified.format("%Y%m%dT%H%M%SZ"));
    let mut suffix = 0;
    loop {
        let name = if suffix == 0 {
            format!("{stem}.log")
        } else {
            format!("{stem}-{suffix}.log")
        };
        let candidate = log_dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
        suffix += 1;
    }
}

fn archive_expired_logs_at(log_dir: &Path, now: DateTime<Utc>) -> Result<(), LogStoreError> {
    let cutoff = now - Duration::days(RETENTION_DAYS);
    let archive_dir = log_dir.join(ARCHIVE_DIR_NAME);
    for entry in fs::read_dir(log_dir).map_err(|error| LogStoreError::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| LogStoreError::Storage(error.to_string()))?;
        let path = entry.path();
        if !path.is_file() || !is_rotated_log(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|error| LogStoreError::Storage(error.to_string()))?;
        let modified: DateTime<Utc> = modified.into();
        if !is_expired(modified, cutoff) {
            continue;
        }
        fs::create_dir_all(&archive_dir)
            .map_err(|error| LogStoreError::Storage(error.to_string()))?;
        let target =
            archive_dir.join(path.file_name().ok_or_else(|| {
                LogStoreError::Storage("Log file name is unavailable".to_string())
            })?);
        fs::rename(&path, target).map_err(|error| LogStoreError::Storage(error.to_string()))?;
    }
    Ok(())
}

fn is_expired(modified: DateTime<Utc>, cutoff: DateTime<Utc>) -> bool {
    modified < cutoff
}

pub fn is_log_file(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some(LOG_FILE_NAME) || is_rotated_log(path)
}

fn is_rotated_log(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("vanehub-") && name.ends_with(".log"))
}

pub fn redact_text(input: &str) -> String {
    let tokens = input.split_whitespace().collect::<Vec<_>>();
    let mut redacted = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if token_without_punctuation(tokens[index]).eq_ignore_ascii_case("bearer")
            && index + 1 < tokens.len()
        {
            redacted.push("Bearer".to_string());
            redacted.push("[REDACTED]".to_string());
            index += 2;
            continue;
        }
        if is_provider_token(tokens[index]) {
            redacted.push("[REDACTED]".to_string());
            index += 1;
            continue;
        }
        if let Some((replacement, needs_next_value)) = redact_inline_sensitive_token(tokens[index])
        {
            redacted.push(replacement);
            if needs_next_value
                && tokens.get(index + 1).is_some_and(|value| {
                    token_without_punctuation(value).eq_ignore_ascii_case("bearer")
                })
            {
                index += 3;
            } else {
                index += if needs_next_value { 2 } else { 1 };
            }
            continue;
        }
        if is_sensitive_key(token_without_punctuation(tokens[index])) {
            if let Some(separator) = tokens
                .get(index + 1)
                .filter(|value| matches!(**value, "=" | ":"))
            {
                redacted.push(format!("{}{separator}[REDACTED]", tokens[index]));
                index += 3;
            } else {
                redacted.push(format!("{}=[REDACTED]", tokens[index]));
                index += 2;
            }
            continue;
        }
        redacted.push(tokens[index].to_string());
        index += 1;
    }
    redacted.join(" ")
}

fn redact_entry(mut entry: LogEntry) -> LogEntry {
    entry.message = redact_text(&entry.message);
    entry.context = entry
        .context
        .into_iter()
        .map(|(key, value)| {
            let redacted = if is_sensitive_key(&key) {
                "[REDACTED]".to_string()
            } else {
                redact_text(&value)
            };
            (key, redacted)
        })
        .collect();
    entry
}

fn redact_inline_sensitive_token(token: &str) -> Option<(String, bool)> {
    let separators = ['=', ':'];
    for separator in separators {
        if let Some((key, value)) = token.split_once(separator) {
            if is_sensitive_key(key) {
                return Some((
                    format!("{key}{separator}[REDACTED]"),
                    value.trim_matches(['\"', '\'', ',', '}']).is_empty(),
                ));
            }
        }
    }
    None
}

fn is_provider_token(token: &str) -> bool {
    let normalized = token_without_punctuation(token);
    normalized.starts_with("sk-")
        || normalized.starts_with("ghp_")
        || normalized.starts_with("github_pat_")
}

fn token_without_punctuation(token: &str) -> &str {
    token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_lowercase();
    normalized.contains("password")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("authorization")
        || normalized.contains("external_chat")
        || normalized.contains("external_user")
        || normalized.contains("sender_id")
        || normalized.contains("message_content")
        || normalized == "prompt"
        || normalized == "response"
        || normalized.contains("protocol_frame")
        || normalized == "headers"
        || normalized.contains("qr_payload")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.ends_with("_key")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vanehub-log-test-{name}-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn redacts_sensitive_tokens() {
        let redacted = redact_text("token=abc password:secret Bearer abc sk-test ghp_token plain");

        assert!(redacted.contains("token=[REDACTED]"));
        assert!(redacted.contains("password:[REDACTED]"));
        assert!(redacted.contains("Bearer [REDACTED]"));
        assert!(!redacted.contains("sk-test"));
        assert!(!redacted.contains("ghp_token"));
        assert!(redacted.contains("plain"));
    }

    #[test]
    fn redacts_structured_and_whitespace_separated_sensitive_values() {
        let input = r#"payload {"token": "json-secret"} api_key = spaced-secret password plain-secret Authorization: Bearer bearer-secret"#;
        let redacted = redact_text(input);

        for secret in [
            "json-secret",
            "spaced-secret",
            "plain-secret",
            "bearer-secret",
        ] {
            assert!(!redacted.contains(secret), "redaction leaked {secret}");
        }
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn writes_redacted_log_entry() {
        let dir = temp_dir("write");
        let mut context = BTreeMap::new();
        context.insert("api_key".to_string(), "secret".to_string());

        write_message(&dir, LogLevel::Error, "test", "password=abc", context).expect("write");

        let raw = fs::read_to_string(dir.join(LOG_FILE_NAME)).expect("read log");
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("secret"));
        assert!(!raw.contains("password=abc"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn operation_log_keeps_trace_id_while_redacting_output_and_context() {
        let dir = temp_dir("operation-association");
        let mut context = BTreeMap::new();
        context.insert("operationId".to_string(), "op-fixture-1".to_string());
        context.insert("relatedEntityId".to_string(), "server-fixture".to_string());
        context.insert("api_token".to_string(), "context-secret".to_string());

        write_message(
            &dir,
            LogLevel::Info,
            "operation.mcp",
            "stdout token=message-secret",
            context,
        )
        .expect("write operation log");

        let raw = fs::read_to_string(dir.join(LOG_FILE_NAME)).expect("read operation log");
        let entry: LogEntry = serde_json::from_str(raw.lines().next().expect("log line"))
            .expect("deserialize operation log");
        assert_eq!(entry.category, "operation.mcp");
        assert_eq!(
            entry.context.get("operationId").map(String::as_str),
            Some("op-fixture-1")
        );
        assert_eq!(
            entry.context.get("relatedEntityId").map(String::as_str),
            Some("server-fixture")
        );
        assert_eq!(
            entry.context.get("api_token").map(String::as_str),
            Some("[REDACTED]")
        );
        assert!(entry.message.contains("token=[REDACTED]"));
        assert!(!raw.contains("context-secret"));
        assert!(!raw.contains("message-secret"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn redacts_im_identity_content_and_protocol_context() {
        let dir = temp_dir("im-redaction");
        let mut context = BTreeMap::new();
        for key in [
            "external_chat_id",
            "sender_id",
            "message_content",
            "prompt",
            "response",
            "headers",
            "protocol_frame",
            "qr_payload",
        ] {
            context.insert(key.to_string(), format!("private-{key}"));
        }

        write_message(
            &dir,
            LogLevel::Debug,
            "im.connector",
            "safe status",
            context,
        )
        .expect("write IM log");

        let raw = fs::read_to_string(dir.join(LOG_FILE_NAME)).expect("read log");
        assert_eq!(raw.matches("[REDACTED]").count(), 8);
        assert!(!raw.contains("private-"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_redacted_client_log_event() {
        let dir = temp_dir("client");
        let mut details = BTreeMap::new();
        details.insert("token".to_string(), "secret".to_string());

        write_client_event(
            &dir,
            ClientLogEvent {
                level: LogLevel::Error,
                kind: ClientLogEventKind::ErrorBoundary,
                message: "UI failed password=abc".to_string(),
                source: "test".to_string(),
                details: Some(details),
                stack: Some("stack".to_string()),
            },
        )
        .expect("client event");

        let raw = fs::read_to_string(dir.join(LOG_FILE_NAME)).expect("read log");
        assert!(raw.contains("frontend.client"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("password=abc"));
        assert!(!raw.contains("secret"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_file_as_log_directory() {
        let dir = temp_dir("file");
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("not-dir");
        fs::write(&path, "x").expect("file");

        let result = validate_log_dir(&path);

        assert!(result.is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn identifies_rotated_logs_and_expiration() {
        let now = Utc::now();
        assert!(is_rotated_log(Path::new("vanehub-20260101T000000Z.log")));
        assert!(!is_rotated_log(Path::new(LOG_FILE_NAME)));
        assert!(is_expired(
            now - Duration::days(RETENTION_DAYS + 1),
            now - Duration::days(RETENTION_DAYS),
        ));
        assert!(!is_expired(now, now - Duration::days(RETENTION_DAYS)));
    }

    #[test]
    fn rotates_active_log_and_archives_expired_rotated_log() {
        let dir = temp_dir("rotation");
        write_message(
            &dir,
            LogLevel::Info,
            "test",
            "active entry",
            BTreeMap::new(),
        )
        .expect("write");
        let now = Utc::now();

        rotate_active_log(&dir, now + Duration::hours(ROTATION_AGE_HOURS + 1)).expect("rotate");

        assert!(!dir.join(LOG_FILE_NAME).exists());
        let rotated = fs::read_dir(&dir)
            .expect("read logs")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| is_rotated_log(path))
            .expect("rotated log");
        archive_expired_logs_at(&dir, now + Duration::days(RETENTION_DAYS + 1)).expect("archive");
        assert!(!rotated.exists());
        assert!(dir
            .join(ARCHIVE_DIR_NAME)
            .read_dir()
            .expect("archive dir")
            .next()
            .is_some());
        let _ = fs::remove_dir_all(dir);
    }
}
