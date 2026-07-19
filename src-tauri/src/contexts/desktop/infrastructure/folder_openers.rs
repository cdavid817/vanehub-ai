use super::SqliteDesktopSettingsRepository;
use crate::platform::{logging, process};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FolderOpenerId {
    Vscode,
    FileExplorer,
    WindowsTerminal,
    GitBash,
    IntellijIdea,
    Webstorm,
}

impl FolderOpenerId {
    pub(crate) const ALL: [Self; 6] = [
        Self::Vscode,
        Self::FileExplorer,
        Self::WindowsTerminal,
        Self::GitBash,
        Self::IntellijIdea,
        Self::Webstorm,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Vscode => "vscode",
            Self::FileExplorer => "file-explorer",
            Self::WindowsTerminal => "windows-terminal",
            Self::GitBash => "git-bash",
            Self::IntellijIdea => "intellij-idea",
            Self::Webstorm => "webstorm",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == value)
    }

    fn category(self) -> &'static str {
        match self {
            Self::Vscode => "editor",
            Self::FileExplorer => "file-manager",
            Self::WindowsTerminal | Self::GitBash => "terminal",
            Self::IntellijIdea | Self::Webstorm => "ide",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FolderOpenerAvailability {
    pub(crate) id: FolderOpenerId,
    pub(crate) category: &'static str,
    pub(crate) status: &'static str,
    pub(crate) executable_path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) edition: Option<String>,
    pub(crate) detection_source: Option<&'static str>,
    pub(crate) icon_key: FolderOpenerId,
    pub(crate) reason: Option<String>,
}

impl FolderOpenerAvailability {
    fn available(id: FolderOpenerId, path: PathBuf, source: &'static str) -> Self {
        Self {
            id,
            category: id.category(),
            status: "available",
            executable_path: Some(path.to_string_lossy().to_string()),
            version: None,
            edition: None,
            detection_source: Some(source),
            icon_key: id,
            reason: None,
        }
    }

    fn missing(id: FolderOpenerId) -> Self {
        Self {
            id,
            category: id.category(),
            status: if cfg!(windows) {
                "not-installed"
            } else {
                "unsupported-platform"
            },
            executable_path: None,
            version: None,
            edition: None,
            detection_source: None,
            icon_key: id,
            reason: Some(
                if cfg!(windows) {
                    "not-installed"
                } else {
                    "unsupported-platform"
                }
                .to_string(),
            ),
        }
    }

    fn is_available(&self) -> bool {
        self.status == "available"
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FolderOpenerPreferencesView {
    pub(crate) configured_default_opener_id: FolderOpenerId,
    pub(crate) effective_default_opener_id: Option<FolderOpenerId>,
    pub(crate) enabled_opener_ids: Vec<FolderOpenerId>,
    pub(crate) fallback_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveFolderOpenerPreferences {
    pub(crate) configured_default_opener_id: FolderOpenerId,
    pub(crate) enabled_opener_ids: Vec<FolderOpenerId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenSessionFolderResult {
    pub(crate) status: &'static str,
    pub(crate) opener_id: FolderOpenerId,
    pub(crate) reason: Option<String>,
}

#[derive(Clone)]
pub(crate) struct FolderOpenerService {
    repository: SqliteDesktopSettingsRepository,
    cache: Arc<Mutex<Option<Vec<FolderOpenerAvailability>>>>,
    discovery: Arc<dyn FolderOpenerDiscoveryPort>,
    launcher: Arc<dyn FolderOpenerLaunchPort>,
}

trait FolderOpenerDiscoveryPort: Send + Sync {
    fn discover(&self) -> Vec<FolderOpenerAvailability>;
}

trait FolderOpenerLaunchPort: Send + Sync {
    fn launch(
        &self,
        executable: &Path,
        args: &[OsString],
        current_dir: &Path,
    ) -> Result<(), String>;
}

struct SystemFolderOpenerDiscovery;
impl FolderOpenerDiscoveryPort for SystemFolderOpenerDiscovery {
    fn discover(&self) -> Vec<FolderOpenerAvailability> {
        FolderOpenerId::ALL.into_iter().map(detect).collect()
    }
}

struct SystemFolderOpenerLauncher;
impl FolderOpenerLaunchPort for SystemFolderOpenerLauncher {
    fn launch(
        &self,
        executable: &Path,
        args: &[OsString],
        current_dir: &Path,
    ) -> Result<(), String> {
        process::spawn_detached(executable, args, current_dir).map_err(|error| error.to_string())
    }
}

impl FolderOpenerService {
    pub(crate) fn new(repository: SqliteDesktopSettingsRepository) -> Self {
        Self {
            repository,
            cache: Arc::new(Mutex::new(None)),
            discovery: Arc::new(SystemFolderOpenerDiscovery),
            launcher: Arc::new(SystemFolderOpenerLauncher),
        }
    }

    pub(crate) fn list(&self, refresh: bool) -> Vec<FolderOpenerAvailability> {
        if !refresh {
            if let Some(cached) = self.cache.lock().ok().and_then(|value| value.clone()) {
                return cached;
            }
        }
        let detected = self.discovery.discover();
        log_discovery(&detected);
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(detected.clone());
        }
        detected
    }

    pub(crate) fn preferences(&self) -> Result<FolderOpenerPreferencesView, String> {
        let (stored_default, stored_enabled) = self
            .repository
            .load_folder_opener_preferences()
            .map_err(|error| error.to_string())?;
        let available = self.list(false);
        let detected_ids = available
            .iter()
            .filter(|item| item.is_available())
            .map(|item| item.id)
            .collect::<BTreeSet<_>>();
        let configured = stored_default
            .as_deref()
            .and_then(FolderOpenerId::parse)
            .unwrap_or_else(|| {
                if detected_ids.contains(&FolderOpenerId::Vscode) {
                    FolderOpenerId::Vscode
                } else {
                    FolderOpenerId::FileExplorer
                }
            });
        let mut enabled = stored_enabled.as_deref().map(parse_ids).unwrap_or_else(|| {
            FolderOpenerId::ALL
                .into_iter()
                .filter(|id| detected_ids.contains(id))
                .collect()
        });
        normalize_enabled(&mut enabled);
        if !enabled.contains(&configured) {
            enabled.push(configured);
            normalize_enabled(&mut enabled);
        }
        Ok(view(configured, enabled, &detected_ids))
    }

    pub(crate) fn save_preferences(
        &self,
        input: SaveFolderOpenerPreferences,
    ) -> Result<FolderOpenerPreferencesView, String> {
        let enabled = validate_saved_enabled(input.enabled_opener_ids)?;
        if !enabled.contains(&input.configured_default_opener_id) {
            return Err("configured default must be enabled".to_string());
        }
        let available = self.list(false);
        if !available
            .iter()
            .any(|item| item.id == input.configured_default_opener_id && item.is_available())
        {
            return Err("configured default is unavailable".to_string());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let encoded = enabled
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        self.repository
            .save_folder_opener_preferences(
                input.configured_default_opener_id.as_str(),
                &encoded,
                &now,
            )
            .map_err(|error| error.to_string())?;
        let available_ids = available
            .iter()
            .filter(|item| item.is_available())
            .map(|item| item.id)
            .collect();
        Ok(view(
            input.configured_default_opener_id,
            enabled,
            &available_ids,
        ))
    }

    pub(crate) fn open_path(
        &self,
        session_id: &str,
        target: &Path,
        opener_id: FolderOpenerId,
    ) -> Result<OpenSessionFolderResult, String> {
        let target = target
            .canonicalize()
            .map_err(|_| "session-directory-missing".to_string())?;
        if !target.is_dir() {
            return Err("session-directory-missing".to_string());
        }
        let opener = self
            .list(true)
            .into_iter()
            .find(|item| item.id == opener_id && item.is_available())
            .ok_or_else(|| "opener-not-available".to_string())?;
        let executable = PathBuf::from(
            opener
                .executable_path
                .ok_or_else(|| "opener-not-available".to_string())?,
        );
        let args = launch_args(opener_id, &target);
        self.launcher
            .launch(&executable, &args, &target)
            .map_err(|error| {
                log_launch(opener_id, session_id, "process-spawn-failed");
                error.to_string()
            })?;
        log_launch(opener_id, session_id, "started");
        Ok(OpenSessionFolderResult {
            status: "opened",
            opener_id,
            reason: None,
        })
    }
}

fn normalize_enabled(enabled: &mut Vec<FolderOpenerId>) {
    enabled.push(FolderOpenerId::FileExplorer);
    let selected = enabled.iter().copied().collect::<BTreeSet<_>>();
    *enabled = FolderOpenerId::ALL
        .into_iter()
        .filter(|id| selected.contains(id))
        .collect();
}

fn validate_saved_enabled(mut enabled: Vec<FolderOpenerId>) -> Result<Vec<FolderOpenerId>, String> {
    if !enabled.contains(&FolderOpenerId::FileExplorer) {
        return Err("file explorer must remain enabled".to_string());
    }
    normalize_enabled(&mut enabled);
    Ok(enabled)
}

fn view(
    configured: FolderOpenerId,
    enabled: Vec<FolderOpenerId>,
    available: &BTreeSet<FolderOpenerId>,
) -> FolderOpenerPreferencesView {
    let effective = if enabled.contains(&configured) && available.contains(&configured) {
        Some(configured)
    } else if available.contains(&FolderOpenerId::FileExplorer) {
        Some(FolderOpenerId::FileExplorer)
    } else {
        enabled.iter().copied().find(|id| available.contains(id))
    };
    FolderOpenerPreferencesView {
        configured_default_opener_id: configured,
        effective_default_opener_id: effective,
        enabled_opener_ids: enabled,
        fallback_active: effective != Some(configured),
    }
}

fn parse_ids(value: &str) -> Vec<FolderOpenerId> {
    value.split(',').filter_map(FolderOpenerId::parse).collect()
}

fn detect(id: FolderOpenerId) -> FolderOpenerAvailability {
    if !cfg!(windows) {
        return FolderOpenerAvailability::missing(id);
    }
    let candidates = candidates(id);
    candidates
        .into_iter()
        .find(|(path, _)| path.is_file())
        .map(|(path, source)| FolderOpenerAvailability::available(id, path, source))
        .unwrap_or_else(|| FolderOpenerAvailability::missing(id))
}

fn candidates(id: FolderOpenerId) -> Vec<(PathBuf, &'static str)> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
    let mut values = Vec::new();
    match id {
        FolderOpenerId::FileExplorer => {
            values.push((PathBuf::from(r"C:\Windows\explorer.exe"), "system"))
        }
        FolderOpenerId::Vscode => {
            if let Some(path) = registry_app_path("Code.exe") {
                values.push((path, "app-paths"));
            }
            if let Some(path) = where_path("code").and_then(resolve_code_executable) {
                values.push((path, "path"));
            }
            if let Some(root) = local {
                values.push((
                    root.join(r"Programs\Microsoft VS Code\Code.exe"),
                    "known-location",
                ));
            }
            if let Some(root) = program_files {
                values.push((root.join(r"Microsoft VS Code\Code.exe"), "known-location"));
            }
        }
        FolderOpenerId::WindowsTerminal => {
            if let Some(path) = registry_app_path("wt.exe") {
                values.push((path, "app-paths"));
            }
            if let Some(path) = where_path("wt") {
                values.push((path, "path"));
            }
        }
        FolderOpenerId::GitBash => {
            if let Some(git) = where_path("git") {
                if let Some(root) = git.parent().and_then(Path::parent) {
                    values.push((root.join("git-bash.exe"), "path"));
                }
            }
            if let Some(root) = program_files {
                values.push((root.join(r"Git\git-bash.exe"), "known-location"));
            }
        }
        FolderOpenerId::IntellijIdea => {
            if let Some(path) = registry_app_path("idea64.exe") {
                values.push((path, "app-paths"));
            }
            values.extend(jetbrains_candidates("idea64.exe"));
        }
        FolderOpenerId::Webstorm => {
            if let Some(path) = registry_app_path("webstorm64.exe") {
                values.push((path, "app-paths"));
            }
            values.extend(jetbrains_candidates("webstorm64.exe"));
        }
    }
    values
}

fn where_path(name: &str) -> Option<PathBuf> {
    let output = process::ProcessAdapter
        .execute(
            &process::ProcessRequest::new("where")
                .arg(name)
                .timeout(Duration::from_secs(3)),
        )
        .ok()?;
    output
        .success()
        .then(|| {
            output
                .stdout
                .lines()
                .next()
                .map(str::trim)
                .map(PathBuf::from)
        })
        .flatten()
}

fn registry_app_path(executable: &str) -> Option<PathBuf> {
    for hive in ["HKCU", "HKLM"] {
        let key =
            format!(r"{hive}\Software\Microsoft\Windows\CurrentVersion\App Paths\{executable}");
        let output = process::ProcessAdapter
            .execute(
                &process::ProcessRequest::new("reg")
                    .args(["query", &key, "/ve"])
                    .timeout(Duration::from_secs(3)),
            )
            .ok()?;
        if !output.success() {
            continue;
        }
        if let Some(path) = parse_registry_path(&output.stdout) {
            return Some(path);
        }
    }
    None
}

fn parse_registry_path(output: &str) -> Option<PathBuf> {
    output
        .lines()
        .filter_map(|line| line.split_once("REG_SZ").map(|(_, value)| value.trim()))
        .find(|value| !value.is_empty())
        .map(|value| PathBuf::from(value.trim_matches('"')))
}

fn resolve_code_executable(path: PathBuf) -> Option<PathBuf> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
    {
        return Some(path);
    }
    path.parent()?.parent().map(|root| root.join("Code.exe"))
}

fn jetbrains_candidates(executable: &str) -> Vec<(PathBuf, &'static str)> {
    let mut roots = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join(r"JetBrains\Toolbox\apps"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("JetBrains"));
    }
    let mut found = Vec::new();
    for root in roots {
        collect_named(&root, executable, 0, &mut found);
    }
    found.sort_by(|left, right| right.cmp(left));
    found
        .into_iter()
        .map(|path| (path, "jetbrains-toolbox"))
        .collect()
}

fn collect_named(root: &Path, executable: &str, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > 5 || !root.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(executable))
        {
            found.push(path);
        } else if path.is_dir() {
            collect_named(&path, executable, depth + 1, found);
        }
    }
}

fn launch_args(id: FolderOpenerId, target: &Path) -> Vec<OsString> {
    match id {
        FolderOpenerId::WindowsTerminal => {
            vec![OsString::from("-d"), target.as_os_str().to_os_string()]
        }
        FolderOpenerId::GitBash => Vec::new(),
        _ => vec![target.as_os_str().to_os_string()],
    }
}

fn log_launch(id: FolderOpenerId, session_id: &str, result: &str) {
    let directory = logging::active_log_dir(logging::fallback_log_dir());
    let mut context = BTreeMap::new();
    context.insert("openerId".to_string(), id.as_str().to_string());
    context.insert("sessionId".to_string(), session_id.to_string());
    context.insert("result".to_string(), result.to_string());
    let _ = logging::write_message(
        &directory,
        logging::LogLevel::Info,
        "workspace.folder-opener.launch",
        "folder opener launch",
        context,
    );
}

fn log_discovery(items: &[FolderOpenerAvailability]) {
    let directory = logging::active_log_dir(logging::fallback_log_dir());
    for item in items {
        let mut context = BTreeMap::new();
        context.insert("openerId".to_string(), item.id.as_str().to_string());
        context.insert("status".to_string(), item.status.to_string());
        if let Some(source) = item.detection_source {
            context.insert("source".to_string(), source.to_string());
        }
        let level = if item.status == "detection-failed" {
            logging::LogLevel::Warn
        } else {
            logging::LogLevel::Debug
        };
        let _ = logging::write_message(
            &directory,
            level,
            "workspace.folder-opener.detection",
            "folder opener detection completed",
            context,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_keep_explorer_and_stable_order() {
        let mut enabled = vec![
            FolderOpenerId::GitBash,
            FolderOpenerId::Vscode,
            FolderOpenerId::Vscode,
        ];
        normalize_enabled(&mut enabled);
        assert_eq!(
            enabled,
            vec![
                FolderOpenerId::Vscode,
                FolderOpenerId::FileExplorer,
                FolderOpenerId::GitBash
            ]
        );
    }

    #[test]
    fn saved_preferences_reject_an_omitted_explorer_fallback() {
        assert_eq!(
            validate_saved_enabled(vec![FolderOpenerId::Vscode]),
            Err("file explorer must remain enabled".to_string())
        );
    }

    #[test]
    fn fallback_does_not_replace_configured_default() {
        let available = [FolderOpenerId::FileExplorer].into_iter().collect();
        let value = view(
            FolderOpenerId::Vscode,
            vec![FolderOpenerId::Vscode, FolderOpenerId::FileExplorer],
            &available,
        );
        assert_eq!(value.configured_default_opener_id, FolderOpenerId::Vscode);
        assert_eq!(
            value.effective_default_opener_id,
            Some(FolderOpenerId::FileExplorer)
        );
        assert!(value.fallback_active);
    }

    #[test]
    fn git_bash_plan_relies_on_literal_working_directory() {
        assert!(launch_args(FolderOpenerId::GitBash, Path::new("D:/A & B")).is_empty());
    }

    #[test]
    fn registry_parser_reads_only_string_values() {
        assert_eq!(
            parse_registry_path("    (Default)    REG_SZ    D:\\Tools\\Code.exe"),
            Some(PathBuf::from(r"D:\Tools\Code.exe"))
        );
        assert_eq!(parse_registry_path("REG_DWORD 1"), None);
    }

    #[test]
    fn git_bash_candidates_never_use_generic_bash() {
        assert!(candidates(FolderOpenerId::GitBash)
            .iter()
            .all(|(path, _)| path
                .file_name()
                .and_then(|value| value.to_str())
                .is_none_or(|value| !value.eq_ignore_ascii_case("bash.exe"))));
    }
}
