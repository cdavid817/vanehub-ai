//! Bounded, launch-free discovery of the folder-opener catalog on every supported platform.
//!
//! Every candidate list is built from an explicit [`DiscoveryEnv`] snapshot rather than the live
//! process environment, so the Linux and macOS rankings can be unit-tested against a temporary
//! directory tree on any host. Platform dispatch is a runtime value for the same reason: a Linux
//! CI runner can still assert what the macOS builder would rank first.

use super::folder_openers::{FolderOpenerAvailability, FolderOpenerId};
use crate::platform::process;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HostPlatform {
    Windows,
    MacOs,
    Linux,
}

impl HostPlatform {
    pub(super) fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }

    /// Windows Terminal and Git Bash are Windows products; every other opener has a native
    /// counterpart on each desktop platform.
    pub(super) fn supports(self, id: FolderOpenerId) -> bool {
        match id {
            FolderOpenerId::WindowsTerminal | FolderOpenerId::GitBash => self == Self::Windows,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct DiscoveryEnv {
    pub(super) home: Option<PathBuf>,
    pub(super) path_dirs: Vec<PathBuf>,
    pub(super) local_app_data: Option<PathBuf>,
    pub(super) program_files: Option<PathBuf>,
    pub(super) program_files_x86: Option<PathBuf>,
    pub(super) system_root: Option<PathBuf>,
}

impl DiscoveryEnv {
    pub(super) fn from_process() -> Self {
        Self {
            home: std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from),
            path_dirs: std::env::var_os("PATH")
                .map(|value| std::env::split_paths(&value).collect())
                .unwrap_or_default(),
            local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
            program_files: std::env::var_os("ProgramFiles").map(PathBuf::from),
            program_files_x86: std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
            system_root: std::env::var_os("SystemRoot").map(PathBuf::from),
        }
    }
}

/// Directories a GUI-launched process often lacks in `PATH` (Homebrew, `/usr/local`) but where a
/// user-installed CLI wrapper conventionally lives. Checked after the real `PATH`.
const UNIX_EXTRA_BIN_DIRS: [&str; 3] = ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"];

fn unix_bin_lookup(env: &DiscoveryEnv, name: &str) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    if let Some(path) = path_lookup(env, name) {
        values.push((path, "path"));
    }
    for dir in UNIX_EXTRA_BIN_DIRS {
        values.push((PathBuf::from(dir).join(name), "known-location"));
    }
    values
}

/// Flatpak exports a launchable wrapper per application id, so a Flatpak IDE needs no special
/// launch plan: the wrapper takes the directory argument like the native launcher does.
fn flatpak_exports(env: &DiscoveryEnv, app_ids: &[&str]) -> Vec<(PathBuf, &'static str)> {
    let mut roots = vec![PathBuf::from("/var/lib/flatpak/exports/bin")];
    if let Some(home) = &env.home {
        roots.insert(0, home.join(".local/share/flatpak/exports/bin"));
    }
    roots
        .iter()
        .flat_map(|root| app_ids.iter().map(move |id| (root.join(id), "flatpak")))
        .collect()
}

pub(super) fn detect(id: FolderOpenerId) -> FolderOpenerAvailability {
    detect_on(HostPlatform::current(), id, &DiscoveryEnv::from_process())
}

pub(super) fn detect_on(
    platform: HostPlatform,
    id: FolderOpenerId,
    env: &DiscoveryEnv,
) -> FolderOpenerAvailability {
    if !platform.supports(id) {
        return FolderOpenerAvailability::missing(id, "unsupported-platform");
    }
    candidates(platform, id, env)
        .into_iter()
        .find(|(path, _)| candidate_exists(path))
        .map(|(path, source)| {
            let (version, edition) = product_info(platform, id, &path);
            FolderOpenerAvailability::available(id, path, source, version, edition)
        })
        .unwrap_or_else(|| FolderOpenerAvailability::missing(id, "not-installed"))
}

/// A macOS application bundle is a directory, so `is_file` alone would reject every `.app`.
fn candidate_exists(path: &Path) -> bool {
    path.is_file() || (is_app_bundle(path) && path.is_dir())
}

fn is_app_bundle(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("app"))
}

pub(super) fn candidates(
    platform: HostPlatform,
    id: FolderOpenerId,
    env: &DiscoveryEnv,
) -> Vec<(PathBuf, &'static str)> {
    match platform {
        HostPlatform::Windows => windows_candidates(id, env),
        HostPlatform::MacOs => macos_candidates(id, env),
        HostPlatform::Linux => linux_candidates(id, env),
    }
}

pub(super) fn launch_plan(
    id: FolderOpenerId,
    executable: &Path,
    target: &Path,
) -> (PathBuf, Vec<OsString>) {
    launch_plan_on(HostPlatform::current(), id, executable, target)
}

pub(super) fn launch_plan_on(
    platform: HostPlatform,
    id: FolderOpenerId,
    executable: &Path,
    target: &Path,
) -> (PathBuf, Vec<OsString>) {
    let dir = target.as_os_str().to_os_string();
    match (platform, id) {
        (HostPlatform::Windows, FolderOpenerId::WindowsTerminal) => {
            (executable.to_path_buf(), vec![OsString::from("-d"), dir])
        }
        // Git Bash reads its start directory from the working directory, and a `--cd` argument
        // would reintroduce quoting of a path that is already passed as data.
        (HostPlatform::Windows, FolderOpenerId::GitBash) => (executable.to_path_buf(), Vec::new()),
        (HostPlatform::MacOs, _) if is_app_bundle(executable) => (
            PathBuf::from("/usr/bin/open"),
            vec![
                OsString::from("-a"),
                executable.as_os_str().to_os_string(),
                dir,
            ],
        ),
        _ => (executable.to_path_buf(), vec![dir]),
    }
}

// ---------------------------------------------------------------------------------------------
// Linux
// ---------------------------------------------------------------------------------------------

fn linux_candidates(id: FolderOpenerId, env: &DiscoveryEnv) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    match id {
        FolderOpenerId::FileExplorer => values.extend(unix_bin_lookup(env, "xdg-open")),
        FolderOpenerId::Vscode => {
            values.extend(unix_bin_lookup(env, "code"));
            for known in ["/usr/share/code/bin/code", "/snap/bin/code"] {
                values.push((PathBuf::from(known), "known-location"));
            }
            values.extend(flatpak_exports(env, &["com.visualstudio.code"]));
        }
        FolderOpenerId::IntellijIdea => {
            values.extend(linux_jetbrains_candidates(
                env,
                "idea",
                &["intellij-idea", "idea"],
                &[
                    "intellij-idea-ultimate",
                    "intellij-idea-community",
                    "intellij-idea-educational",
                ],
                &[
                    "com.jetbrains.IntelliJ-IDEA-Ultimate",
                    "com.jetbrains.IntelliJ-IDEA-Community",
                ],
            ));
        }
        FolderOpenerId::Webstorm => {
            values.extend(linux_jetbrains_candidates(
                env,
                "webstorm",
                &["webstorm"],
                &["webstorm"],
                &["com.jetbrains.WebStorm"],
            ));
        }
        FolderOpenerId::WindowsTerminal | FolderOpenerId::GitBash => {}
    }
    values
}

/// Toolbox-managed launchers rank before a `PATH` wrapper so that the copy Toolbox keeps current
/// wins over a distribution package that may lag behind.
fn linux_jetbrains_candidates(
    env: &DiscoveryEnv,
    launcher: &str,
    dir_prefixes: &[&str],
    snap_names: &[&str],
    flatpak_ids: &[&str],
) -> Vec<(PathBuf, &'static str)> {
    let script = format!("{launcher}.sh");
    let mut values = Vec::new();
    if let Some(home) = &env.home {
        let toolbox = home.join(".local/share/JetBrains/Toolbox");
        for root in product_dirs(&toolbox.join("apps"), dir_prefixes) {
            values.extend(launchers_under(
                &root,
                &[&script, launcher],
                "jetbrains-toolbox",
            ));
        }
        values.push((toolbox.join("scripts").join(launcher), "jetbrains-toolbox"));
    }
    for root in product_dirs(Path::new("/opt"), dir_prefixes) {
        values.push((root.join("bin").join(&script), "known-location"));
    }
    for name in snap_names {
        values.push((PathBuf::from("/snap/bin").join(name), "known-location"));
    }
    values.extend(flatpak_exports(env, flatpak_ids));
    values.extend(unix_bin_lookup(env, launcher));
    values
}

/// Toolbox 2.x lays a product out as `apps/<product>/bin/<launcher>`; Toolbox 1.x nested it under
/// `apps/<PRODUCT>/ch-<n>/<build>/bin/<launcher>`. Both are covered by a shallow walk that only
/// starts inside a product directory, never at the apps root.
fn launchers_under(
    root: &Path,
    names: &[&str],
    source: &'static str,
) -> Vec<(PathBuf, &'static str)> {
    let mut found = Vec::new();
    for name in names {
        collect_named(root, name, 0, &mut found);
    }
    // Newest build directory first when several channels are installed side by side.
    found.sort_by(|left, right| right.cmp(left));
    found.into_iter().map(|path| (path, source)).collect()
}

fn product_dirs(root: &Path, prefixes: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .is_some_and(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|left, right| right.cmp(left));
    dirs
}

fn path_lookup(env: &DiscoveryEnv, name: &str) -> Option<PathBuf> {
    env.path_dirs
        .iter()
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

// ---------------------------------------------------------------------------------------------
// macOS
// ---------------------------------------------------------------------------------------------

fn macos_candidates(id: FolderOpenerId, env: &DiscoveryEnv) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    match id {
        FolderOpenerId::FileExplorer => values.push((PathBuf::from("/usr/bin/open"), "system")),
        FolderOpenerId::Vscode => {
            for root in macos_app_roots(env) {
                values.push((
                    root.join("Visual Studio Code.app/Contents/Resources/app/bin/code"),
                    "known-location",
                ));
            }
            values.extend(unix_bin_lookup(env, "code"));
        }
        FolderOpenerId::IntellijIdea => {
            values.extend(macos_jetbrains_candidates(env, "idea", &["intellij idea"]));
        }
        FolderOpenerId::Webstorm => {
            values.extend(macos_jetbrains_candidates(env, "webstorm", &["webstorm"]));
        }
        FolderOpenerId::WindowsTerminal | FolderOpenerId::GitBash => {}
    }
    values
}

fn macos_app_roots(env: &DiscoveryEnv) -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = &env.home {
        roots.push(home.join("Applications"));
        roots.push(home.join("Applications/JetBrains Toolbox"));
    }
    roots
}

fn macos_jetbrains_candidates(
    env: &DiscoveryEnv,
    launcher: &str,
    bundle_prefixes: &[&str],
) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    for root in macos_app_roots(env) {
        for bundle in product_dirs(&root, bundle_prefixes)
            .into_iter()
            .filter(|path| is_app_bundle(path))
        {
            values.push((bundle, "app-bundle"));
        }
    }
    if let Some(home) = &env.home {
        let toolbox = home.join("Library/Application Support/JetBrains/Toolbox");
        // Toolbox 1.x kept bundles under `apps/<PRODUCT>/ch-<n>/<build>/<Name>.app`.
        for root in product_dirs(&toolbox.join("apps"), &["intellij", "idea", "webstorm"]) {
            values.extend(
                bundles_under(&root, bundle_prefixes, 0)
                    .into_iter()
                    .map(|bundle| (bundle, "jetbrains-toolbox")),
            );
        }
        values.push((toolbox.join("scripts").join(launcher), "jetbrains-toolbox"));
    }
    values.extend(unix_bin_lookup(env, launcher));
    values
}

fn bundles_under(root: &Path, prefixes: &[&str], depth: usize) -> Vec<PathBuf> {
    if depth > 3 {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if !path.is_dir() {
            continue;
        }
        let matches_prefix = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .is_some_and(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)));
        if is_app_bundle(&path) {
            if matches_prefix {
                found.push(path);
            }
        } else {
            found.extend(bundles_under(&path, prefixes, depth + 1));
        }
    }
    found.sort_by(|left, right| right.cmp(left));
    found
}

// ---------------------------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------------------------

fn windows_candidates(id: FolderOpenerId, env: &DiscoveryEnv) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    match id {
        FolderOpenerId::FileExplorer => {
            if let Some(root) = &env.system_root {
                values.push((root.join("explorer.exe"), "system"));
            }
            values.push((PathBuf::from(r"C:\Windows\explorer.exe"), "system"));
        }
        FolderOpenerId::Vscode => {
            if let Some(path) = registry_app_path("Code.exe") {
                values.push((path, "app-paths"));
            }
            if let Some(path) = where_path("code").and_then(resolve_code_executable) {
                values.push((path, "path"));
            }
            if let Some(root) = &env.local_app_data {
                values.push((
                    root.join(r"Programs\Microsoft VS Code\Code.exe"),
                    "known-location",
                ));
            }
            if let Some(root) = &env.program_files {
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
            // The Store package exposes an app-execution alias here even when it is not in PATH.
            if let Some(root) = &env.local_app_data {
                values.push((root.join(r"Microsoft\WindowsApps\wt.exe"), "known-location"));
            }
        }
        FolderOpenerId::GitBash => {
            if let Some(git) = where_path("git") {
                if let Some(root) = git.parent().and_then(Path::parent) {
                    values.push((root.join("git-bash.exe"), "path"));
                }
            }
            if let Some(root) = &env.local_app_data {
                values.push((root.join(r"Programs\Git\git-bash.exe"), "known-location"));
            }
            for root in [&env.program_files, &env.program_files_x86]
                .into_iter()
                .flatten()
            {
                values.push((root.join(r"Git\git-bash.exe"), "known-location"));
            }
        }
        FolderOpenerId::IntellijIdea => {
            if let Some(path) = registry_app_path("idea64.exe") {
                values.push((path, "app-paths"));
            }
            values.extend(windows_jetbrains_candidates(
                env,
                "idea64.exe",
                &["intellij idea", "idea", "intellij-idea"],
            ));
        }
        FolderOpenerId::Webstorm => {
            if let Some(path) = registry_app_path("webstorm64.exe") {
                values.push((path, "app-paths"));
            }
            values.extend(windows_jetbrains_candidates(
                env,
                "webstorm64.exe",
                &["webstorm"],
            ));
        }
    }
    values
}

/// Toolbox 2.x installs into `%LOCALAPPDATA%\Programs\<Product>`, Toolbox 1.x into
/// `%LOCALAPPDATA%\JetBrains\Toolbox\apps`, the standalone installer into `Program Files`, and
/// Scoop into `%USERPROFILE%\scoop\apps\<bucket-name>\current`.
fn windows_jetbrains_candidates(
    env: &DiscoveryEnv,
    executable: &str,
    dir_prefixes: &[&str],
) -> Vec<(PathBuf, &'static str)> {
    let mut values = Vec::new();
    if let Some(path) = where_path(executable.trim_end_matches(".exe")) {
        values.push((path, "path"));
    }
    if let Some(local) = &env.local_app_data {
        for root in product_dirs(&local.join("Programs"), dir_prefixes) {
            values.extend(launchers_under(&root, &[executable], "jetbrains-toolbox"));
        }
        values.extend(launchers_under(
            &local.join(r"JetBrains\Toolbox\apps"),
            &[executable],
            "jetbrains-toolbox",
        ));
    }
    for root in [&env.program_files, &env.program_files_x86]
        .into_iter()
        .flatten()
    {
        values.extend(launchers_under(
            &root.join("JetBrains"),
            &[executable],
            "known-location",
        ));
    }
    if let Some(home) = &env.home {
        for root in product_dirs(&home.join("scoop").join("apps"), dir_prefixes) {
            values.push((root.join("current").join("bin").join(executable), "scoop"));
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

pub(super) fn parse_registry_path(output: &str) -> Option<PathBuf> {
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

// ---------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------

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

/// JetBrains ships `product-info.json` at the product root (`<root>/bin/<launcher>`) and, on
/// macOS, under `Contents/Resources`. Reading it is the only way to report a version without
/// starting the IDE; any failure simply leaves the fields empty.
fn product_info(
    platform: HostPlatform,
    id: FolderOpenerId,
    launcher: &Path,
) -> (Option<String>, Option<String>) {
    if !matches!(id, FolderOpenerId::IntellijIdea | FolderOpenerId::Webstorm) {
        return (None, None);
    }
    let file = if platform == HostPlatform::MacOs && is_app_bundle(launcher) {
        launcher.join("Contents/Resources/product-info.json")
    } else {
        match launcher.parent().and_then(Path::parent) {
            Some(root) => root.join("product-info.json"),
            None => return (None, None),
        }
    };
    std::fs::read_to_string(file)
        .ok()
        .map(|text| parse_product_info(&text))
        .unwrap_or((None, None))
}

pub(super) fn parse_product_info(text: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return (None, None);
    };
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let edition = value
        .get("productCode")
        .and_then(serde_json::Value::as_str)
        .and_then(|code| match code {
            "IU" => Some("Ultimate"),
            "IC" => Some("Community"),
            "IE" => Some("Educational"),
            _ => None,
        })
        .map(str::to_string);
    (version, edition)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#!/bin/sh\n").unwrap();
    }

    #[test]
    fn windows_only_products_are_unsupported_elsewhere() {
        for platform in [HostPlatform::MacOs, HostPlatform::Linux] {
            assert!(!platform.supports(FolderOpenerId::WindowsTerminal));
            assert!(!platform.supports(FolderOpenerId::GitBash));
            assert!(platform.supports(FolderOpenerId::IntellijIdea));
            assert!(platform.supports(FolderOpenerId::FileExplorer));
        }
        assert!(HostPlatform::Windows.supports(FolderOpenerId::GitBash));
    }

    #[test]
    fn absent_supported_product_is_not_installed_rather_than_unsupported() {
        let env = DiscoveryEnv {
            home: Some(PathBuf::from("/nonexistent-home")),
            ..DiscoveryEnv::default()
        };
        let idea = detect_on(HostPlatform::Linux, FolderOpenerId::IntellijIdea, &env);
        assert_eq!(idea.status, "not-installed");
        let wt = detect_on(HostPlatform::Linux, FolderOpenerId::WindowsTerminal, &env);
        assert_eq!(wt.status, "unsupported-platform");
    }

    #[test]
    fn linux_prefers_toolbox_launcher_over_path_wrapper_and_reads_product_info() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let app = home.join(".local/share/JetBrains/Toolbox/apps/intellij-idea-ultimate");
        touch(&app.join("bin/idea.sh"));
        std::fs::write(
            app.join("product-info.json"),
            r#"{"name":"IntelliJ IDEA","version":"2025.1","productCode":"IU"}"#,
        )
        .unwrap();
        let path_dir = temp.path().join("bin");
        touch(&path_dir.join("idea"));
        let env = DiscoveryEnv {
            home: Some(home),
            path_dirs: vec![path_dir],
            ..DiscoveryEnv::default()
        };

        let detected = detect_on(HostPlatform::Linux, FolderOpenerId::IntellijIdea, &env);
        assert_eq!(detected.status, "available");
        assert_eq!(
            detected.executable_path.as_deref(),
            Some(app.join("bin/idea.sh").to_str().unwrap())
        );
        assert_eq!(detected.detection_source, Some("jetbrains-toolbox"));
        assert_eq!(detected.version.as_deref(), Some("2025.1"));
        assert_eq!(detected.edition.as_deref(), Some("Ultimate"));
    }

    #[test]
    fn linux_falls_back_to_path_wrapper_when_toolbox_is_absent() {
        let temp = tempfile::tempdir().unwrap();
        let path_dir = temp.path().join("bin");
        touch(&path_dir.join("webstorm"));
        let env = DiscoveryEnv {
            home: Some(temp.path().join("home")),
            path_dirs: vec![path_dir.clone()],
            ..DiscoveryEnv::default()
        };
        let detected = detect_on(HostPlatform::Linux, FolderOpenerId::Webstorm, &env);
        assert_eq!(detected.status, "available");
        assert_eq!(detected.detection_source, Some("path"));
        assert!(detected.version.is_none());
    }

    #[test]
    fn linux_finds_a_flatpak_export_wrapper() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let wrapper =
            home.join(".local/share/flatpak/exports/bin/com.jetbrains.IntelliJ-IDEA-Community");
        touch(&wrapper);
        let env = DiscoveryEnv {
            home: Some(home),
            ..DiscoveryEnv::default()
        };
        let detected = detect_on(HostPlatform::Linux, FolderOpenerId::IntellijIdea, &env);
        assert_eq!(detected.status, "available");
        assert_eq!(detected.detection_source, Some("flatpak"));
        assert_eq!(detected.executable_path.as_deref(), wrapper.to_str());
    }

    #[test]
    fn macos_legacy_toolbox_bundle_is_found_under_the_channel_directory() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bundle = home.join(
            "Library/Application Support/JetBrains/Toolbox/apps/WebStorm/ch-0/243.1/WebStorm.app",
        );
        std::fs::create_dir_all(bundle.join("Contents")).unwrap();
        let env = DiscoveryEnv {
            home: Some(home),
            ..DiscoveryEnv::default()
        };
        let detected = detect_on(HostPlatform::MacOs, FolderOpenerId::Webstorm, &env);
        assert_eq!(detected.status, "available");
        assert_eq!(detected.detection_source, Some("jetbrains-toolbox"));
        assert_eq!(detected.executable_path.as_deref(), bundle.to_str());
    }

    #[test]
    fn windows_scoop_and_system_root_locations_are_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let scoop = home.join("scoop").join("apps").join("idea-ultimate");
        std::fs::create_dir_all(&scoop).unwrap();
        let env = DiscoveryEnv {
            home: Some(home),
            system_root: Some(PathBuf::from(r"D:\Windows")),
            ..DiscoveryEnv::default()
        };
        let idea = candidates(HostPlatform::Windows, FolderOpenerId::IntellijIdea, &env);
        assert!(idea
            .iter()
            .any(|(path, source)| *source == "scoop" && path.starts_with(&scoop)));
        let explorer = candidates(HostPlatform::Windows, FolderOpenerId::FileExplorer, &env);
        assert_eq!(
            explorer.first().map(|(path, _)| path.clone()),
            Some(PathBuf::from(r"D:\Windows").join("explorer.exe"))
        );
    }

    #[test]
    fn linux_legacy_toolbox_channel_layout_is_walked_from_the_product_directory() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let launcher =
            home.join(".local/share/JetBrains/Toolbox/apps/IDEA-U/ch-0/241.1/bin/idea.sh");
        touch(&launcher);
        let env = DiscoveryEnv {
            home: Some(home),
            ..DiscoveryEnv::default()
        };
        let detected = detect_on(HostPlatform::Linux, FolderOpenerId::IntellijIdea, &env);
        assert_eq!(detected.executable_path.as_deref(), launcher.to_str());
    }

    #[test]
    fn macos_reports_the_bundle_and_launches_it_through_open() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bundle = home.join("Applications/IntelliJ IDEA CE.app");
        std::fs::create_dir_all(bundle.join("Contents/Resources")).unwrap();
        std::fs::write(
            bundle.join("Contents/Resources/product-info.json"),
            r#"{"version":"2024.3","productCode":"IC"}"#,
        )
        .unwrap();
        let env = DiscoveryEnv {
            home: Some(home),
            ..DiscoveryEnv::default()
        };
        let detected = detect_on(HostPlatform::MacOs, FolderOpenerId::IntellijIdea, &env);
        assert_eq!(detected.status, "available");
        assert_eq!(detected.detection_source, Some("app-bundle"));
        assert_eq!(detected.edition.as_deref(), Some("Community"));

        let (executable, args) = launch_plan_on(
            HostPlatform::MacOs,
            FolderOpenerId::IntellijIdea,
            &bundle,
            Path::new("/Users/me/A & B"),
        );
        assert_eq!(executable, PathBuf::from("/usr/bin/open"));
        assert_eq!(
            args,
            vec![
                OsString::from("-a"),
                bundle.as_os_str().to_os_string(),
                OsString::from("/Users/me/A & B")
            ]
        );
    }

    #[test]
    fn unix_launch_plan_passes_the_directory_as_one_literal_argument() {
        let (executable, args) = launch_plan_on(
            HostPlatform::Linux,
            FolderOpenerId::FileExplorer,
            Path::new("/usr/bin/xdg-open"),
            Path::new("/home/me/A & B; rm -rf"),
        );
        assert_eq!(executable, PathBuf::from("/usr/bin/xdg-open"));
        assert_eq!(args, vec![OsString::from("/home/me/A & B; rm -rf")]);
    }

    #[test]
    fn git_bash_plan_relies_on_literal_working_directory() {
        let (_, args) = launch_plan_on(
            HostPlatform::Windows,
            FolderOpenerId::GitBash,
            Path::new(r"C:\Git\git-bash.exe"),
            Path::new("D:/A & B"),
        );
        assert!(args.is_empty());
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
    fn product_info_parser_tolerates_malformed_input() {
        assert_eq!(parse_product_info("not json"), (None, None));
        assert_eq!(
            parse_product_info(r#"{"version":"2025.2","productCode":"WS"}"#),
            (Some("2025.2".to_string()), None)
        );
    }

    #[test]
    fn git_bash_candidates_never_use_generic_bash() {
        let env = DiscoveryEnv {
            program_files: Some(PathBuf::from(r"C:\Program Files")),
            ..DiscoveryEnv::default()
        };
        assert!(
            candidates(HostPlatform::Windows, FolderOpenerId::GitBash, &env)
                .iter()
                .all(|(path, _)| path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_none_or(|value| !value.eq_ignore_ascii_case("bash.exe")))
        );
    }
}
