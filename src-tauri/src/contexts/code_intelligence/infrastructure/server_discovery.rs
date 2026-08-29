use crate::contexts::code_intelligence::domain::models::Language;
use crate::contexts::code_intelligence::domain::registry::{
    HostArchitecture, HostPlatform, InterpreterLaunch,
};
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
    /// The host runtime an interpreter-shaped server needs is not installed. Its own reason
    /// because the action is "install a JDK", which no other reason here asks for.
    PrerequisiteMissing,
    /// An interpreter-shaped language with no configured install directory. Distinct from a
    /// missing executable: there is nothing to search the path for.
    InstallDirectoryNotSet,
    /// The directory exists but holds no launcher matching the declared pattern.
    LauncherNotFound,
    /// Several launchers match. Refused rather than chosen -- picking one would start a server
    /// whose version the settings page does not name.
    AmbiguousInstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerDiscoveryResult {
    language: Language,
    executable: Option<PathBuf>,
    /// Which of the language's declared candidates resolved. Reported so a user with several
    /// installed servers can tell which one discovery picked.
    selected_executable_name: Option<&'static str>,
    /// The versioned launcher an interpreter-shaped language resolved. Reported rather than the
    /// install directory alone, so a reader can tell which version will run.
    resolved_launcher: Option<PathBuf>,
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

    pub(crate) fn resolved_launcher(&self) -> Option<&Path> {
        self.resolved_launcher.as_deref()
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
        self.discover_with_managed_install(
            language,
            executable_override,
            configured_arguments,
            None,
        )
    }

    /// Discovery when the caller knows whether a managed install exists.
    ///
    /// An override always wins. The managed install is a fallback, not a replacement: a user who
    /// pointed at their own copy keeps it, and the managed one stays on disk in case they switch
    /// back.
    pub(crate) fn discover_with_managed_install(
        &self,
        language: Language,
        executable_override: Option<&Path>,
        configured_arguments: Option<&Vec<String>>,
        managed_install: Option<&Path>,
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
        if let Some(launch) = language.launch.interpreter() {
            let directory = executable_override.or(managed_install);
            return self.discover_interpreter(language, arguments, launch, directory);
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

    /// The interpreter path: resolve the runtime, then the install directory, then the launcher.
    ///
    /// Ordered so the first missing thing is the one reported. A user with neither a JDK nor an
    /// install directory is told about the JDK, because that is the one they hit first anyway.
    fn discover_interpreter(
        &self,
        language: Language,
        arguments: Vec<String>,
        launch: &'static InterpreterLaunch,
        install_directory: Option<&Path>,
    ) -> ServerDiscoveryResult {
        let interpreter = language.executables.iter().find_map(|candidate| {
            self.executable_location
                .locate(candidate)
                .filter(|path| is_executable_file(path))
                .map(|path| (*candidate, path))
        });
        let Some((interpreter_name, interpreter)) = interpreter else {
            return unavailable(language, arguments, DiscoveryReason::PrerequisiteMissing);
        };
        let Some(directory) = install_directory else {
            // Not `ExecutableNotFound`: the server is a directory, so there is nothing the
            // executable search path could have been asked for.
            return unavailable(language, arguments, DiscoveryReason::InstallDirectoryNotSet);
        };
        if !directory.is_absolute() || !directory.is_dir() {
            return unavailable(language, arguments, DiscoveryReason::OverrideMissing);
        }
        match resolve_launcher(directory, launch) {
            Ok(launcher) => ServerDiscoveryResult {
                resolved_launcher: Some(launcher),
                ..available(language, arguments, Some(interpreter_name), interpreter)
            },
            Err(reason) => unavailable(language, arguments, reason),
        }
    }
}

/// The one launcher in the declared directory, or why there is not exactly one.
///
/// Prefix-and-suffix matching in a single directory rather than a glob library and rather than a
/// recursive walk: a launcher found three levels down is not the install layout the entry
/// describes, and matching it would start a server from a directory that only looks right.
pub(super) fn resolve_launcher(
    install_directory: &Path,
    launch: &InterpreterLaunch,
) -> Result<PathBuf, DiscoveryReason> {
    let Ok(entries) = std::fs::read_dir(install_directory.join(launch.launcher_directory)) else {
        return Err(DiscoveryReason::LauncherNotFound);
    };
    let mut matched = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.starts_with(launch.launcher_prefix) && name.ends_with(launch.launcher_suffix)
            })
        })
        .map(|entry| entry.path())
        // Bounded: two is already a refusal, so there is no reason to walk a directory that
        // contains thousands of files looking for a third.
        .take(2)
        .collect::<Vec<_>>();
    match matched.len() {
        0 => Err(DiscoveryReason::LauncherNotFound),
        1 => Ok(matched.remove(0)),
        _ => Err(DiscoveryReason::AmbiguousInstall),
    }
}

/// The configuration directory this host's launch needs, or `None` when the entry declares none
/// for this platform.
/// The configuration directory to launch with, preferring the host architecture's.
///
/// Existence decides between the candidates rather than the declaration alone: an archive that
/// predates the ARM variant still resolves to the one it does ship, instead of failing closed on a
/// directory the publisher never put there. Falling back to the last candidate when none exists
/// keeps the failure where it was -- a missing configuration is reported by the caller, not here.
pub(crate) fn resolve_configuration_directory(
    install_directory: &Path,
    launch: &InterpreterLaunch,
) -> Option<PathBuf> {
    let candidates =
        launch.configuration_directories_for(HostPlatform::current(), HostArchitecture::current());
    let mut resolved = None;
    for relative in candidates {
        let candidate = install_directory.join(relative);
        if candidate.is_dir() {
            return Some(candidate);
        }
        resolved = Some(candidate);
    }
    resolved
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
        resolved_launcher: None,
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
        resolved_launcher: None,
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
