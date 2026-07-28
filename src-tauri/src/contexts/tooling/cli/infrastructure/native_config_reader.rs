use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::tooling::cli::application::{CliApplicationError, NativeConfigPort};
use rusqlite::OpenFlags;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct NativeConfigReader {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl NativeConfigReader {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }

    fn home_dir() -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn warn(&self, category: &str, message: String) {
        self.log(LogSeverity::Warn, category, message);
    }

    fn info(&self, category: &str, message: String) {
        self.log(LogSeverity::Info, category, message);
    }

    fn log(&self, severity: LogSeverity, category: &str, message: String) {
        let mut context = BTreeMap::new();
        context.insert("module".to_string(), "native-config-reader".to_string());
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: category.to_string(),
            message,
            context,
        });
    }
}

impl NativeConfigPort for NativeConfigReader {
    fn discover_model(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<String>, CliApplicationError> {
        let home = match Self::home_dir() {
            Some(dir) => dir,
            None => {
                self.info(
                    "cli.native-config",
                    "home_dir() returned None, skipping discovery".to_string(),
                );
                return Ok(None);
            }
        };

        let found = match agent_id {
            "claude-code" => discover_claude_model(&home, self, workspace_path),
            "codex-cli" => discover_codex_model(&home, self, workspace_path),
            "gemini-cli" => discover_gemini_model(&home, self),
            "opencode" => discover_opencode_model(&home, self, workspace_path),
            _ => None,
        };
        if let Some(ref model) = found {
            self.info(
                "cli.native-config",
                format!("discovered model for {agent_id}: {model}"),
            );
        }
        Ok(found)
    }
}

fn discover_claude_model(
    home: &Path,
    reader: &NativeConfigReader,
    workspace_path: Option<&str>,
) -> Option<String> {
    if let Some(model) = discover_claude_model_from_settings(home, reader) {
        return Some(model);
    }
    let model = discover_claude_model_from_project_cache(home, reader, workspace_path?);
    if model.is_some() {
        reader.info(
            "cli.native-config",
            "discovered claude model from project cache".to_string(),
        );
    }
    model
}

fn discover_claude_model_from_settings(home: &Path, reader: &NativeConfigReader) -> Option<String> {
    let path = home.join(".claude").join("settings.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .inspect_err(|_| {
            reader.warn(
                "cli.native-config",
                format!("failed to parse claude settings.json: {}", path.display()),
            );
        })
        .ok()?;
    let model = parsed
        .get("env")
        .and_then(|env| env.get("ANTHROPIC_MODEL"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if model.is_some() {
        reader.info(
            "cli.native-config",
            format!(
                "discovered model from {}: {}",
                path.display(),
                model.as_deref().unwrap_or("")
            ),
        );
    }
    model
}

/// Falls back to Claude Code's own per-project usage cache
/// (`~/.claude.json` → `projects[path].lastModelUsage`) when no explicit
/// `ANTHROPIC_MODEL` override exists. This file is Claude Code's internal,
/// undocumented state rather than a stable public config surface, so the
/// lookup only trusts a project entry when it recorded usage for exactly
/// one model — a history with multiple models has no reliable "current"
/// answer and is treated as unknown.
fn discover_claude_model_from_project_cache(
    home: &Path,
    reader: &NativeConfigReader,
    workspace_path: &str,
) -> Option<String> {
    let path = home.join(".claude.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .inspect_err(|_| {
            reader.warn(
                "cli.native-config",
                format!("failed to parse claude.json: {}", path.display()),
            );
        })
        .ok()?;
    let projects = parsed.get("projects")?.as_object()?;
    let target = normalize_project_path(workspace_path);
    let usage = projects
        .iter()
        .find(|(key, _)| normalize_project_path(key) == target)
        .and_then(|(_, entry)| entry.get("lastModelUsage"))
        .and_then(|value| value.as_object())?;
    if usage.len() != 1 {
        return None;
    }
    usage.keys().next().cloned()
}

fn normalize_project_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn discover_codex_model(
    home: &Path,
    reader: &NativeConfigReader,
    workspace_path: Option<&str>,
) -> Option<String> {
    let path = home.join(".codex").join("config.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let doc: toml::Value = toml::from_str(&content)
        .inspect_err(|_| {
            reader.warn(
                "cli.native-config",
                format!("failed to parse codex config.toml: {}", path.display()),
            );
        })
        .ok()?;

    if let Some(model) = workspace_path.and_then(|path| discover_codex_project_model(&doc, path)) {
        return Some(model);
    }

    doc.get("model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Codex CLI's `config.toml` supports a `[projects.'<path>']` table per
/// trusted project (alongside `trust_level`, it may also carry its own
/// `model` override). This checks that table before the file's top-level
/// `model` default, since a project-scoped value is more specific.
fn discover_codex_project_model(doc: &toml::Value, workspace_path: &str) -> Option<String> {
    let projects = doc.get("projects")?.as_table()?;
    let target = normalize_project_path(workspace_path);
    projects
        .iter()
        .find(|(key, _)| normalize_project_path(key) == target)
        .and_then(|(_, entry)| entry.get("model"))
        .and_then(|value| value.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn discover_gemini_model(home: &Path, _reader: &NativeConfigReader) -> Option<String> {
    let path = home.join(".gemini").join(".env");
    let content = std::fs::read_to_string(&path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("GEMINI_MODEL=") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.replace('.', "-"));
            }
        }
    }
    None
}

fn discover_opencode_model(
    home: &Path,
    reader: &NativeConfigReader,
    workspace_path: Option<&str>,
) -> Option<String> {
    if let Some(model) =
        workspace_path.and_then(|path| discover_opencode_model_from_db(home, reader, path))
    {
        return Some(model);
    }
    discover_opencode_model_from_config(home, reader)
}

/// OpenCode's `opencode.json` only declares which models a provider makes
/// *available* — it does not record which one is actually selected. The
/// selection lives in OpenCode's own SQLite state at
/// `~/.local/share/opencode/opencode.db`, in the `session` table's
/// `directory` and `model` (JSON `{"id": ..., "providerID": ...}`) columns,
/// ordered by `time_updated`. This reads that database read-only and picks
/// the most recently updated session for the matching workspace directory.
fn discover_opencode_model_from_db(
    home: &Path,
    reader: &NativeConfigReader,
    workspace_path: &str,
) -> Option<String> {
    let db_path = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    if !db_path.exists() {
        return None;
    }
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .inspect_err(|_| {
        reader.warn(
            "cli.native-config",
            format!("failed to open opencode.db: {}", db_path.display()),
        );
    })
    .ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(200));

    let target = normalize_project_path(workspace_path);
    let matched_directory = {
        let mut stmt = conn
            .prepare("SELECT DISTINCT directory FROM session WHERE directory IS NOT NULL")
            .ok()?;
        let mut rows = stmt.query([]).ok()?;
        let mut found = None;
        while let Ok(Some(row)) = rows.next() {
            let Ok(directory) = row.get::<_, String>(0) else {
                continue;
            };
            if normalize_project_path(&directory) == target {
                found = Some(directory);
                break;
            }
        }
        found?
    };

    let mut stmt = conn
        .prepare(
            "SELECT model FROM session \
             WHERE directory = ?1 AND model IS NOT NULL \
             ORDER BY time_updated DESC LIMIT 1",
        )
        .ok()?;
    let model_json: String = stmt.query_row([&matched_directory], |row| row.get(0)).ok()?;
    extract_opencode_model_id(&model_json)
}

fn extract_opencode_model_id(model_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(model_json).ok()?;
    parsed
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn discover_opencode_model_from_config(home: &Path, reader: &NativeConfigReader) -> Option<String> {
    let path = home.join(".config").join("opencode").join("opencode.json");
    let content = std::fs::read_to_string(&path).ok()?;
    // OpenCode uses JSON5 — try serde_json first, fall back to manual extraction
    let parsed: Option<serde_json::Value> = serde_json::from_str(&content).ok();
    match parsed {
        Some(json) => {
            let models = json.get("provider")?.as_object()?.values().next()?.get("models")?.as_object()?;
            models.keys().next().cloned()
        }
        None => {
            reader.warn(
                "cli.native-config",
                format!("failed to parse opencode opencode.json (json5): {}", path.display()),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationsError};
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct NullLogger(Arc<Mutex<Vec<(LogSeverity, String, String)>>>);

    impl DiagnosticLogPort for NullLogger {
        fn write_diagnostic(
            &self,
            log: DiagnosticLog,
        ) -> Result<(), OperationsError> {
            self.0.lock().unwrap().push((log.severity, log.category, log.message));
            Ok(())
        }
    }

    fn reader() -> NativeConfigReader {
        NativeConfigReader::new(Arc::new(NullLogger::default()))
    }

    fn temp_home() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.keep();
        (tempfile::tempdir().expect("workdir"), home)
    }

    #[test]
    fn claude_model_discovered_from_env_anthropic_model() {
        let (_tmp, home) = temp_home();
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("claude dir");
        let config = serde_json::json!({
            "env": {
                "ANTHROPIC_MODEL": "claude-sonnet-5",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            },
            "permissions": { "allow": ["Bash"] }
        });
        std::fs::write(claude_dir.join("settings.json"), config.to_string()).expect("write");
        assert_eq!(
            discover_claude_model(&home, &reader(), None),
            Some("claude-sonnet-5".to_string())
        );
    }

    #[test]
    fn claude_model_missing_env_key_returns_none() {
        let (_tmp, home) = temp_home();
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("claude dir");
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions": {"allow": ["Bash"]}}"#,
        )
        .expect("write");
        assert_eq!(discover_claude_model(&home, &reader(), None), None);
    }

    #[test]
    fn claude_settings_absent_returns_none() {
        let (_tmp, home) = temp_home();
        assert_eq!(discover_claude_model(&home, &reader(), None), None);
    }

    #[test]
    fn claude_project_cache_used_when_settings_absent_and_single_model() {
        let (_tmp, home) = temp_home();
        let claude_json = serde_json::json!({
            "projects": {
                "D:/cdavid/Documents/code/gemini-cli": {
                    "lastModelUsage": {
                        "deepseek-v4-pro": { "inputTokens": 100 }
                    }
                }
            }
        });
        std::fs::write(home.join(".claude.json"), claude_json.to_string()).expect("write");
        assert_eq!(
            discover_claude_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("deepseek-v4-pro".to_string())
        );
    }

    #[test]
    fn claude_project_cache_skipped_with_multiple_models() {
        let (_tmp, home) = temp_home();
        let claude_json = serde_json::json!({
            "projects": {
                "D:/cdavid/Documents/code/gemini-cli": {
                    "lastModelUsage": {
                        "deepseek-v4-pro": { "inputTokens": 100 },
                        "claude-opus-4-8": { "inputTokens": 50 }
                    }
                }
            }
        });
        std::fs::write(home.join(".claude.json"), claude_json.to_string()).expect("write");
        assert_eq!(
            discover_claude_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            None
        );
    }

    #[test]
    fn claude_project_cache_normalizes_windows_path_separators() {
        let (_tmp, home) = temp_home();
        let claude_json = serde_json::json!({
            "projects": {
                "D:/cdavid/Documents/code/gemini-cli": {
                    "lastModelUsage": {
                        "deepseek-v4-pro": { "inputTokens": 100 }
                    }
                }
            }
        });
        std::fs::write(home.join(".claude.json"), claude_json.to_string()).expect("write");
        assert_eq!(
            discover_claude_model(
                &home,
                &reader(),
                Some(r"D:\cdavid\Documents\code\gemini-cli")
            ),
            Some("deepseek-v4-pro".to_string())
        );
    }

    #[test]
    fn claude_project_cache_ignored_without_workspace_path() {
        let (_tmp, home) = temp_home();
        let claude_json = serde_json::json!({
            "projects": {
                "D:/cdavid/Documents/code/gemini-cli": {
                    "lastModelUsage": {
                        "deepseek-v4-pro": { "inputTokens": 100 }
                    }
                }
            }
        });
        std::fs::write(home.join(".claude.json"), claude_json.to_string()).expect("write");
        assert_eq!(discover_claude_model(&home, &reader(), None), None);
    }

    #[test]
    fn claude_settings_json_takes_precedence_over_project_cache() {
        let (_tmp, home) = temp_home();
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("claude dir");
        let settings = serde_json::json!({ "env": { "ANTHROPIC_MODEL": "claude-sonnet-5" } });
        std::fs::write(claude_dir.join("settings.json"), settings.to_string()).expect("write");
        let claude_json = serde_json::json!({
            "projects": {
                "D:/cdavid/Documents/code/gemini-cli": {
                    "lastModelUsage": { "deepseek-v4-pro": { "inputTokens": 100 } }
                }
            }
        });
        std::fs::write(home.join(".claude.json"), claude_json.to_string()).expect("write");
        assert_eq!(
            discover_claude_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("claude-sonnet-5".to_string())
        );
    }

    #[test]
    fn codex_model_from_top_level_toml() {
        let (_tmp, home) = temp_home();
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("codex dir");
        std::fs::write(
            codex_dir.join("config.toml"),
            "model = \"gpt-5.4\"\nmodel_provider = \"custom\"\n",
        )
        .expect("write");
        assert_eq!(
            discover_codex_model(&home, &reader(), None),
            Some("gpt-5.4".to_string())
        );
    }

    #[test]
    fn codex_config_absent_returns_none() {
        let (_tmp, home) = temp_home();
        assert_eq!(discover_codex_model(&home, &reader(), None), None);
    }

    #[test]
    fn codex_project_model_takes_precedence_over_top_level() {
        let (_tmp, home) = temp_home();
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("codex dir");
        std::fs::write(
            codex_dir.join("config.toml"),
            "model = \"gpt-5.4\"\n\n[projects.'d:\\cdavid\\documents\\code\\gemini-cli']\ntrust_level = \"trusted\"\nmodel = \"deepseek-v4-pro\"\n",
        )
        .expect("write");
        assert_eq!(
            discover_codex_model(
                &home,
                &reader(),
                Some(r"D:\cdavid\Documents\code\gemini-cli")
            ),
            Some("deepseek-v4-pro".to_string())
        );
    }

    #[test]
    fn codex_project_section_without_model_falls_back_to_top_level() {
        let (_tmp, home) = temp_home();
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("codex dir");
        std::fs::write(
            codex_dir.join("config.toml"),
            "model = \"gpt-5.4\"\n\n[projects.'d:\\cdavid\\documents\\code\\gemini-cli']\ntrust_level = \"trusted\"\n",
        )
        .expect("write");
        assert_eq!(
            discover_codex_model(
                &home,
                &reader(),
                Some(r"D:\cdavid\Documents\code\gemini-cli")
            ),
            Some("gpt-5.4".to_string())
        );
    }

    #[test]
    fn codex_project_model_ignored_when_workspace_path_does_not_match() {
        let (_tmp, home) = temp_home();
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("codex dir");
        std::fs::write(
            codex_dir.join("config.toml"),
            "model = \"gpt-5.4\"\n\n[projects.'d:\\cdavid\\documents\\code\\gemini-cli']\nmodel = \"deepseek-v4-pro\"\n",
        )
        .expect("write");
        assert_eq!(
            discover_codex_model(&home, &reader(), Some(r"D:\other\project")),
            Some("gpt-5.4".to_string())
        );
    }

    #[test]
    fn gemini_model_from_env_file() {
        let (_tmp, home) = temp_home();
        let gemini_dir = home.join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("gemini dir");
        std::fs::write(
            gemini_dir.join(".env"),
            "GEMINI_API_KEY=test-key\nGEMINI_MODEL=gemini-2.5-flash\n",
        )
        .expect("write");
        assert_eq!(
            discover_gemini_model(&home, &reader()),
            Some("gemini-2-5-flash".to_string())
        );
    }

    #[test]
    fn gemini_env_absent_returns_none() {
        let (_tmp, home) = temp_home();
        assert_eq!(discover_gemini_model(&home, &reader()), None);
    }

    #[test]
    fn opencode_first_provider_model_key() {
        let (_tmp, home) = temp_home();
        let opencode_dir = home.join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).expect("opencode dir");
        let config = serde_json::json!({
            "provider": {
                "my-provider": {
                    "npm": "@ai-sdk/openai-compatible",
                    "options": { "baseURL": "https://api.example.com/v1" },
                    "models": {
                        "gpt-5.4": { "name": "GPT-5.4" },
                        "claude-sonnet-4-6": { "name": "Claude Sonnet 4.6" }
                    }
                }
            }
        });
        std::fs::write(opencode_dir.join("opencode.json"), config.to_string()).expect("write");
        let model = discover_opencode_model(&home, &reader(), None);
        assert!(model.is_some());
        // First key in models is model discovery target
        assert!(model.unwrap().len() > 0);
    }

    #[test]
    fn opencode_config_absent_returns_none() {
        let (_tmp, home) = temp_home();
        assert_eq!(discover_opencode_model(&home, &reader(), None), None);
    }

    fn create_opencode_db(home: &Path) -> rusqlite::Connection {
        let db_dir = home.join(".local").join("share").join("opencode");
        std::fs::create_dir_all(&db_dir).expect("opencode data dir");
        let conn = rusqlite::Connection::open(db_dir.join("opencode.db")).expect("open db");
        conn.execute_batch(
            "CREATE TABLE session (
                directory TEXT,
                model TEXT,
                time_updated INTEGER
            );",
        )
        .expect("create session table");
        conn
    }

    fn insert_opencode_session(conn: &rusqlite::Connection, directory: &str, model_id: &str, time_updated: i64) {
        let model_json = serde_json::json!({ "id": model_id, "providerID": "deepseek" }).to_string();
        conn.execute(
            "INSERT INTO session (directory, model, time_updated) VALUES (?1, ?2, ?3)",
            rusqlite::params![directory, model_json, time_updated],
        )
        .expect("insert session");
    }

    #[test]
    fn opencode_db_model_used_when_directory_matches() {
        let (_tmp, home) = temp_home();
        let conn = create_opencode_db(&home);
        insert_opencode_session(
            &conn,
            "D:/cdavid/Documents/code/gemini-cli",
            "deepseek-v4-flash",
            1_000,
        );
        drop(conn);
        assert_eq!(
            discover_opencode_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("deepseek-v4-flash".to_string())
        );
    }

    #[test]
    fn opencode_db_picks_most_recent_when_multiple_sessions() {
        let (_tmp, home) = temp_home();
        let conn = create_opencode_db(&home);
        insert_opencode_session(&conn, "D:/cdavid/Documents/code/gemini-cli", "older-model", 1_000);
        insert_opencode_session(&conn, "D:/cdavid/Documents/code/gemini-cli", "newer-model", 2_000);
        drop(conn);
        assert_eq!(
            discover_opencode_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("newer-model".to_string())
        );
    }

    #[test]
    fn opencode_db_normalizes_windows_path_separators() {
        let (_tmp, home) = temp_home();
        let conn = create_opencode_db(&home);
        insert_opencode_session(
            &conn,
            "D:/cdavid/Documents/code/gemini-cli",
            "deepseek-v4-flash",
            1_000,
        );
        drop(conn);
        assert_eq!(
            discover_opencode_model(
                &home,
                &reader(),
                Some(r"D:\cdavid\Documents\code\gemini-cli")
            ),
            Some("deepseek-v4-flash".to_string())
        );
    }

    #[test]
    fn opencode_db_falls_back_to_config_when_directory_not_found() {
        let (_tmp, home) = temp_home();
        let conn = create_opencode_db(&home);
        insert_opencode_session(&conn, "D:/some/other/project", "deepseek-v4-flash", 1_000);
        drop(conn);
        let opencode_dir = home.join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).expect("opencode config dir");
        let config = serde_json::json!({
            "provider": { "my-provider": { "models": { "gpt-5.4": {} } } }
        });
        std::fs::write(opencode_dir.join("opencode.json"), config.to_string()).expect("write");
        assert_eq!(
            discover_opencode_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("gpt-5.4".to_string())
        );
    }

    #[test]
    fn opencode_db_absent_falls_back_to_config() {
        let (_tmp, home) = temp_home();
        let opencode_dir = home.join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).expect("opencode config dir");
        let config = serde_json::json!({
            "provider": { "my-provider": { "models": { "gpt-5.4": {} } } }
        });
        std::fs::write(opencode_dir.join("opencode.json"), config.to_string()).expect("write");
        assert_eq!(
            discover_opencode_model(
                &home,
                &reader(),
                Some("D:/cdavid/Documents/code/gemini-cli")
            ),
            Some("gpt-5.4".to_string())
        );
    }
}
