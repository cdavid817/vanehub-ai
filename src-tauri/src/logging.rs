use crate::AppError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const LOG_FILE_NAME: &str = "vanehub.log";
const ARCHIVE_DIR_NAME: &str = "archive";
const RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoggingPolicy {
    pub retention_days: i64,
    pub archive_enabled: bool,
    pub redaction_enabled: bool,
    pub levels: Vec<LogLevel>,
    pub can_open_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
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

pub fn policy(can_open_directory: bool) -> LoggingPolicy {
    LoggingPolicy {
        retention_days: RETENTION_DAYS,
        archive_enabled: true,
        redaction_enabled: true,
        levels: vec![LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug],
        can_open_directory,
    }
}

pub fn default_log_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs")
}

pub fn validate_log_dir(path: &Path) -> Result<PathBuf, AppError> {
    fs::create_dir_all(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let metadata = fs::metadata(path).map_err(|error| AppError::Storage(error.to_string()))?;
    if !metadata.is_dir() {
        return Err(AppError::Validation(format!(
            "Log directory path is not a directory: {}",
            path.display()
        )));
    }
    Ok(path.to_path_buf())
}

pub fn write_entry(log_dir: &Path, entry: LogEntry) -> Result<(), AppError> {
    let log_dir = validate_log_dir(log_dir)?;
    archive_expired_logs(&log_dir)?;
    let path = log_dir.join(LOG_FILE_NAME);
    let line = serde_json::to_string(&redact_entry(entry))
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    writeln!(file, "{line}").map_err(|error| AppError::Storage(error.to_string()))
}

pub fn write_message(
    log_dir: &Path,
    level: LogLevel,
    category: &str,
    message: &str,
    context: BTreeMap<String, String>,
) -> Result<(), AppError> {
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

pub fn write_client_event(log_dir: &Path, event: ClientLogEvent) -> Result<(), AppError> {
    let mut context = event.details.unwrap_or_default();
    context.insert("source".to_string(), event.source);
    context.insert("kind".to_string(), format!("{:?}", event.kind));
    if let Some(stack) = event.stack {
        context.insert("stack".to_string(), stack);
    }
    write_message(log_dir, event.level, "frontend.client", &event.message, context)
}

pub fn open_directory(path: &Path) -> Result<(), AppError> {
    validate_log_dir(path)?;
    let spawn_result = if cfg!(target_os = "windows") {
        Command::new("explorer").arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(path).spawn()
    } else {
        Command::new("xdg-open").arg(path).spawn()
    };
    spawn_result
        .map(|_| ())
        .map_err(|error| AppError::LaunchFailed(error.to_string()))
}

pub fn archive_expired_logs(log_dir: &Path) -> Result<(), AppError> {
    let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
    let archive_dir = log_dir.join(ARCHIVE_DIR_NAME);
    for entry in fs::read_dir(log_dir).map_err(|error| AppError::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| AppError::Storage(error.to_string()))?;
        let path = entry.path();
        if !path.is_file() || path.file_name().and_then(|name| name.to_str()) == Some(LOG_FILE_NAME)
        {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("log") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let modified: DateTime<Utc> = modified.into();
        if modified >= cutoff {
            continue;
        }
        fs::create_dir_all(&archive_dir).map_err(|error| AppError::Storage(error.to_string()))?;
        let target = archive_dir.join(
            path.file_name()
                .ok_or_else(|| AppError::Storage("Log file name is unavailable".to_string()))?,
        );
        fs::rename(&path, target).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    Ok(())
}

pub fn redact_text(input: &str) -> String {
    let tokens = input.split_whitespace().collect::<Vec<_>>();
    let mut redacted = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index].eq_ignore_ascii_case("bearer") && index + 1 < tokens.len() {
            redacted.push("Bearer".to_string());
            redacted.push("[REDACTED]".to_string());
            index += 2;
            continue;
        }
        redacted.push(redact_token(tokens[index]));
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

fn redact_token(token: &str) -> String {
    if token.starts_with("sk-")
        || token.starts_with("sk-ant-")
        || token.starts_with("ghp_")
        || token.starts_with("github_pat_")
    {
        return "[REDACTED]".to_string();
    }
    let separators = ['=', ':'];
    for separator in separators {
        if let Some((key, _value)) = token.split_once(separator) {
            if is_sensitive_key(key) {
                return format!("{key}{separator}[REDACTED]");
            }
        }
    }
    token.to_string()
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

        write_message(&dir, LogLevel::Debug, "im.connector", "safe status", context)
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
    fn archives_expired_log_files() {
        let dir = temp_dir("archive");
        fs::create_dir_all(&dir).expect("dir");
        let old_log = dir.join("old.log");
        fs::write(&old_log, "old").expect("old log");

        archive_expired_logs(&dir).expect("archive pass");

        assert!(old_log.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
