use super::folder_opener_discovery as discovery;
use super::SqliteDesktopSettingsRepository;
use crate::platform::{logging, process};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    pub(super) fn available(
        id: FolderOpenerId,
        path: PathBuf,
        source: &'static str,
        version: Option<String>,
        edition: Option<String>,
    ) -> Self {
        Self {
            id,
            category: id.category(),
            status: "available",
            executable_path: Some(path.to_string_lossy().to_string()),
            version,
            edition,
            detection_source: Some(source),
            icon_key: id,
            reason: None,
        }
    }

    /// `status` is either `not-installed` (the product exists for this platform but was not
    /// found) or `unsupported-platform` (the product has no build for this platform at all).
    pub(super) fn missing(id: FolderOpenerId, status: &'static str) -> Self {
        Self {
            id,
            category: id.category(),
            status,
            executable_path: None,
            version: None,
            edition: None,
            detection_source: None,
            icon_key: id,
            reason: Some(status.to_string()),
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
        FolderOpenerId::ALL
            .into_iter()
            .map(discovery::detect)
            .collect()
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
        // The cached result is trusted as long as its launcher still exists. Re-running the whole
        // discovery here would spawn a dozen `reg`/`where` processes on Windows for every click; a
        // single existence check is what "the installation disappeared" actually needs.
        let discovered = self
            .list(false)
            .into_iter()
            .find(|item| item.id == opener_id && item.is_available())
            .and_then(|item| item.executable_path)
            .map(PathBuf::from)
            .filter(|path| path.exists())
            .or_else(|| {
                self.list(true)
                    .into_iter()
                    .find(|item| item.id == opener_id && item.is_available())
                    .and_then(|item| item.executable_path)
                    .map(PathBuf::from)
            })
            .ok_or_else(|| "opener-not-available".to_string())?;
        let (executable, args) = discovery::launch_plan(opener_id, &discovered, &target);
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
    let mut normalized = Vec::new();
    let mut seen = BTreeSet::new();
    for id in enabled
        .iter()
        .copied()
        .chain([FolderOpenerId::FileExplorer])
    {
        if FolderOpenerId::ALL.contains(&id) && seen.insert(id) {
            normalized.push(id);
        }
    }
    *enabled = normalized;
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
                FolderOpenerId::GitBash,
                FolderOpenerId::Vscode,
                FolderOpenerId::FileExplorer
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
}
