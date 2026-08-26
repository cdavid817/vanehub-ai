use crate::contexts::code_intelligence::domain::models::Language;
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
    UnsupportedOnThisPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerDiscoveryResult {
    language: Language,
    executable: Option<PathBuf>,
    /// Which of the language's declared candidates resolved. Reported so a user with several
    /// installed servers can tell which one discovery picked.
    selected_executable_name: Option<&'static str>,
    arguments: Vec<String>,
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

    pub(crate) const fn selected_executable_name(&self) -> Option<&'static str> {
        self.selected_executable_name
    }

    pub(crate) const fn reason(&self) -> Option<DiscoveryReason> {
        self.reason
    }

    pub(crate) fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub(crate) const fn language(&self) -> Language {
        self.language
    }
}

/// Resolves the arguments a server is started with: the user's list when they supplied one, the
/// registry's declared default otherwise. An empty user list is a choice, not an absence.
pub(crate) fn resolved_startup_arguments(
    language: Language,
    configured: Option<&Vec<String>>,
) -> Vec<String> {
    configured.map_or_else(
        || {
            language
                .default_startup_arguments
                .iter()
                .map(|argument| (*argument).to_string())
                .collect()
        },
        Clone::clone,
    )
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
        language: Language,
        executable_override: Option<&Path>,
        configured_arguments: Option<&Vec<String>>,
    ) -> ServerDiscoveryResult {
        let arguments = resolved_startup_arguments(language, configured_arguments);
        // A language declared for other platforms is unsupported here, which is not the same as
        // supported-but-not-installed. Reporting it as merely undiscovered would send a user
        // looking for an executable that was never going to exist on this host.
        if !language.supports_host() {
            return unavailable(
                language,
                arguments,
                DiscoveryReason::UnsupportedOnThisPlatform,
            );
        }
        if let Some(path) = executable_override {
            return discover_override(language, arguments, path);
        }

        let located = language.executables.iter().find_map(|candidate| {
            self.executable_location
                .locate(candidate)
                .filter(|path| is_executable_file(path))
                .map(|path| (*candidate, path))
        });
        match located {
            Some((name, executable)) => available(language, arguments, Some(name), executable),
            None => unavailable(language, arguments, DiscoveryReason::ExecutableNotFound),
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

fn discover_override(
    language: Language,
    arguments: Vec<String>,
    path: &Path,
) -> ServerDiscoveryResult {
    if !path.is_absolute() || !path.exists() {
        return unavailable(language, arguments, DiscoveryReason::OverrideMissing);
    }
    if !is_executable_file(path) {
        return unavailable(language, arguments, DiscoveryReason::OverrideNotExecutable);
    }
    match std::fs::canonicalize(path) {
        Ok(executable) => available(language, arguments, None, executable),
        Err(_) => unavailable(language, arguments, DiscoveryReason::OverrideNotExecutable),
    }
}

fn available(
    language: Language,
    arguments: Vec<String>,
    selected_executable_name: Option<&'static str>,
    executable: PathBuf,
) -> ServerDiscoveryResult {
    ServerDiscoveryResult {
        language,
        executable: Some(executable),
        selected_executable_name,
        arguments,
        availability: DiscoveryAvailability::Available,
        reason: None,
    }
}

fn unavailable(
    language: Language,
    arguments: Vec<String>,
    reason: DiscoveryReason,
) -> ServerDiscoveryResult {
    ServerDiscoveryResult {
        language,
        executable: None,
        selected_executable_name: None,
        arguments,
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
