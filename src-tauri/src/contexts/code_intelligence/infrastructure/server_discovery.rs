use crate::contexts::code_intelligence::domain::models::ServerKind;
#[cfg(windows)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_SEARCH_PATH_ENTRIES: usize = 256;
#[cfg(windows)]
const MAX_PATH_EXTENSIONS: usize = 16;

pub(crate) trait NativeExecutableLocationPort: Send + Sync {
    fn locate(&self, executable_name: &str) -> Option<PathBuf>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiscoveryAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiscoveryReason {
    ExecutableNotFound,
    OverrideMissing,
    OverrideNotExecutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ServerCommandPreset {
    kind: ServerKind,
    executable_name: &'static str,
    arguments: &'static [&'static str],
}

impl ServerCommandPreset {
    pub(crate) const fn for_kind(kind: ServerKind) -> Self {
        match kind {
            ServerKind::RustAnalyzer => Self {
                kind,
                executable_name: "rust-analyzer",
                arguments: &[],
            },
            ServerKind::TypeScriptLanguageServer => Self {
                kind,
                executable_name: "typescript-language-server",
                arguments: &["--stdio"],
            },
        }
    }

    pub(crate) const fn executable_name(self) -> &'static str {
        self.executable_name
    }

    pub(crate) const fn arguments(self) -> &'static [&'static str] {
        self.arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerDiscoveryResult {
    preset: ServerCommandPreset,
    executable: Option<PathBuf>,
    availability: DiscoveryAvailability,
    reason: Option<DiscoveryReason>,
}

impl ServerDiscoveryResult {
    pub(crate) const fn availability(&self) -> DiscoveryAvailability {
        self.availability
    }

    pub(crate) fn executable(&self) -> Option<&Path> {
        self.executable.as_deref()
    }

    pub(crate) const fn reason(&self) -> Option<DiscoveryReason> {
        self.reason
    }

    pub(crate) const fn arguments(&self) -> &'static [&'static str] {
        self.preset.arguments()
    }

    pub(crate) const fn server_kind(&self) -> ServerKind {
        self.preset.kind
    }
}

#[derive(Clone)]
pub(crate) struct ServerDiscovery {
    executable_location: Arc<dyn NativeExecutableLocationPort>,
}

impl ServerDiscovery {
    pub(crate) fn new(executable_location: Arc<dyn NativeExecutableLocationPort>) -> Self {
        Self {
            executable_location,
        }
    }

    pub(crate) fn discover(
        &self,
        kind: ServerKind,
        executable_override: Option<&Path>,
    ) -> ServerDiscoveryResult {
        let preset = ServerCommandPreset::for_kind(kind);
        if let Some(path) = executable_override {
            return discover_override(preset, path);
        }

        let executable = self
            .executable_location
            .locate(preset.executable_name())
            .filter(|path| is_executable_file(path));
        match executable {
            Some(executable) => available(preset, executable),
            None => unavailable(preset, DiscoveryReason::ExecutableNotFound),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct SystemNativeExecutableLocator;

impl NativeExecutableLocationPort for SystemNativeExecutableLocator {
    fn locate(&self, executable_name: &str) -> Option<PathBuf> {
        std::env::var_os("PATH").and_then(|search_path| {
            locate_in_directories(std::env::split_paths(&search_path), executable_name)
        })
    }
}

pub(super) fn locate_in_directories(
    directories: impl IntoIterator<Item = PathBuf>,
    executable_name: &str,
) -> Option<PathBuf> {
    directories
        .into_iter()
        .take(MAX_SEARCH_PATH_ENTRIES)
        .flat_map(|directory| executable_candidates(&directory, executable_name))
        .find(|candidate| is_executable_file(candidate))
        .and_then(|candidate| std::fs::canonicalize(candidate).ok())
}

fn discover_override(preset: ServerCommandPreset, path: &Path) -> ServerDiscoveryResult {
    if !path.is_absolute() || !path.exists() {
        return unavailable(preset, DiscoveryReason::OverrideMissing);
    }
    if !is_executable_file(path) {
        return unavailable(preset, DiscoveryReason::OverrideNotExecutable);
    }
    match std::fs::canonicalize(path) {
        Ok(executable) => available(preset, executable),
        Err(_) => unavailable(preset, DiscoveryReason::OverrideNotExecutable),
    }
}

fn available(preset: ServerCommandPreset, executable: PathBuf) -> ServerDiscoveryResult {
    ServerDiscoveryResult {
        preset,
        executable: Some(executable),
        availability: DiscoveryAvailability::Available,
        reason: None,
    }
}

fn unavailable(preset: ServerCommandPreset, reason: DiscoveryReason) -> ServerDiscoveryResult {
    ServerDiscoveryResult {
        preset,
        executable: None,
        availability: DiscoveryAvailability::Unavailable,
        reason: Some(reason),
    }
}

fn executable_candidates(directory: &Path, executable_name: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let base = directory.join(executable_name);
        if base.extension().is_some() {
            return vec![base];
        }
        path_extensions()
            .into_iter()
            .map(|extension| directory.join(format!("{executable_name}{extension}")))
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![directory.join(executable_name)]
    }
}

#[cfg(windows)]
fn path_extensions() -> Vec<String> {
    std::env::var_os("PATHEXT")
        .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"))
        .to_string_lossy()
        .split(';')
        .map(str::trim)
        .filter(|extension| !extension.is_empty())
        .take(MAX_PATH_EXTENSIONS)
        .map(|extension| extension.to_ascii_lowercase())
        .collect()
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "com" | "exe" | "bat" | "cmd"
                )
            })
    }
}
