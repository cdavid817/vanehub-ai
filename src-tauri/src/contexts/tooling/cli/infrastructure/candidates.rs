use super::process_adapter::{CliProcessRequest, CliProcessRunner};
use super::support::is_direct_cli_executable;
use crate::contexts::tooling::cli::domain::ToolDefinition;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const CANDIDATE_LIMIT: usize = 24;
const RESOLVER_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) trait CliCandidateSource: Send + Sync {
    fn candidates(&self, definition: ToolDefinition) -> Vec<PathBuf>;
}

pub(super) struct SystemCliCandidateSource {
    process: Arc<dyn CliProcessRunner>,
}

impl SystemCliCandidateSource {
    pub(super) fn new(process: Arc<dyn CliProcessRunner>) -> Self {
        Self { process }
    }
}

impl CliCandidateSource for SystemCliCandidateSource {
    fn candidates(&self, definition: ToolDefinition) -> Vec<PathBuf> {
        let mut paths = self
            .resolved_paths(definition.executable_name)
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        paths.extend(known_candidate_paths(definition));
        select_candidates(paths, |path| path.is_file())
    }
}

impl SystemCliCandidateSource {
    fn resolved_paths(&self, executable_name: &str) -> Vec<String> {
        let resolver = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };
        let mut args = Vec::new();
        if !cfg!(target_os = "windows") {
            args.push("-a".to_string());
        }
        args.push(executable_name.to_string());
        let Ok(output) = self.process.execute(CliProcessRequest {
            executable: resolver.to_string(),
            args,
            timeout: RESOLVER_TIMEOUT,
            audit_category: None,
        }) else {
            return Vec::new();
        };
        if !output.success {
            return Vec::new();
        }
        output
            .stdout
            .lines()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    }
}

fn known_candidate_paths(definition: ToolDefinition) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(app_data).join("npm");
            candidates.push(base.join(format!("{}.cmd", definition.executable_name)));
            candidates.push(base.join(format!("{}.exe", definition.executable_name)));
        }
        // Scoop shims. The guide recommends scoop for opencode on Windows
        // (`scoop install extras/opencode`), and scoop puts every installed program behind a shim
        // in one fixed directory rather than one per package, so this is a single entry regardless
        // of what was installed through it.
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            candidates.push(
                PathBuf::from(profile)
                    .join("scoop")
                    .join("shims")
                    .join(format!("{}.exe", definition.executable_name)),
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            candidates.push(
                base.join("Programs")
                    .join("OpenAI")
                    .join("Codex")
                    .join("bin")
                    .join(format!("{}.exe", definition.executable_name)),
            );
            candidates.push(
                base.join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join(format!("{}.exe", definition.executable_name)),
            );
            // Antigravity's Windows installer drops the binary in a directory named after the
            // executable, not after the product.
            candidates.push(
                base.join(definition.executable_name)
                    .join("bin")
                    .join(format!("{}.exe", definition.executable_name)),
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            for relative in [
                ".local/bin",
                ".npm-global/bin",
                ".volta/bin",
                ".bun/bin",
                ".opencode/bin",
                // asdf puts every managed executable behind a shim in one fixed directory, so this
                // covers whichever nodejs version is selected without naming one.
                ".asdf/shims",
            ] {
                candidates.push(home.join(relative).join(definition.executable_name));
            }
            candidates.extend(nvm_bin_paths(&home, definition.executable_name));
        }
        for base in ["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"] {
            candidates.push(PathBuf::from(base).join(definition.executable_name));
        }
    }
    candidates
}

/// Every nvm-managed node's bin directory, newest version first.
///
/// nvm is what the install guide tells people to use -- "never use sudo npm install -g; use nvm or
/// adjust the npm global prefix" -- and it installs global packages under the *selected* node
/// version, a path no fixed string can name. It is also the case this whole candidate list exists
/// for: nvm puts its bin on `PATH` from a shell profile, so a desktop app launched from an icon
/// inherits neither, and a CLI installed exactly as documented reads as not installed.
///
/// Which version is selected cannot be known without that `PATH`, so all of them are offered and
/// sorted newest-first. Ordering is lexical on the directory name, which is right for nvm's
/// zero-padded-free `vMAJOR.MINOR.PATCH` only within a major; the intent is a stable, sensible
/// order rather than a correct semver ranking, and any of these is a working install.
fn nvm_bin_paths(home: &Path, executable_name: &str) -> Vec<PathBuf> {
    let versions_root = home.join(".nvm").join("versions").join("node");
    let Ok(entries) = std::fs::read_dir(&versions_root) else {
        return Vec::new();
    };
    let mut versions: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    versions
        .into_iter()
        .map(|version| version.join("bin").join(executable_name))
        .collect()
}

fn select_candidates(
    paths: impl IntoIterator<Item = PathBuf>,
    is_file: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| is_file(path))
        .filter(|path| is_direct_cli_executable(path))
        .filter(|path| seen.insert(candidate_key(path)))
        .take(CANDIDATE_LIMIT)
        .collect()
}

fn candidate_key(path: &Path) -> String {
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(target_os = "windows")]
    let key_path = {
        let mut key_path = normalized.clone();
        let extension = normalized
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension == "cmd" || extension == "ps1" || extension.is_empty() {
            key_path.set_extension("");
        }
        key_path
    };
    #[cfg(not(target_os = "windows"))]
    let key_path = normalized;
    let key = key_path.to_string_lossy().replace('\\', "/");
    if cfg!(target_os = "windows") {
        key.to_ascii_lowercase()
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// nvm installs global packages under whichever node version is selected, so the path cannot
    /// be a fixed string and the directory has to be read. Built against a real tree rather than
    /// a mocked filesystem because the thing under test is the directory walk itself.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn nvm_versions_are_offered_newest_first_and_absent_nvm_is_not_an_error() {
        let root = std::env::temp_dir().join(format!(
            "vanehub-nvm-candidates-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        // A home with no nvm at all is the common case and must answer empty rather than fail.
        std::fs::create_dir_all(&root).expect("root");
        assert!(nvm_bin_paths(&root, "claude").is_empty());

        for version in ["v20.11.0", "v22.3.0", "v18.19.1"] {
            std::fs::create_dir_all(
                root.join(".nvm")
                    .join("versions")
                    .join("node")
                    .join(version)
                    .join("bin"),
            )
            .expect("version dir");
        }
        let paths = nvm_bin_paths(&root, "claude");
        let names: Vec<String> = paths
            .iter()
            .map(|path| {
                path.parent()
                    .and_then(Path::parent)
                    .and_then(|version| version.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .expect("version name")
            })
            .collect();

        assert_eq!(names, vec!["v22.3.0", "v20.11.0", "v18.19.1"]);
        assert!(paths.iter().all(|path| path.ends_with("bin/claude")));
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn candidate_selection_is_deduplicated_and_bounded() {
        let suffix = if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        };
        let paths = (0..30)
            .map(|index| PathBuf::from(format!("/fixture/bin/tool-{index}{suffix}")))
            .chain([PathBuf::from(format!("/fixture/bin/tool-0{suffix}"))])
            .collect::<Vec<_>>();

        let selected = select_candidates(paths, |_| true);

        assert_eq!(selected.len(), CANDIDATE_LIMIT);
        assert_eq!(
            selected[0],
            PathBuf::from(format!("/fixture/bin/tool-0{suffix}"))
        );
        assert_eq!(
            selected[23],
            PathBuf::from(format!("/fixture/bin/tool-23{suffix}"))
        );
    }

    /// Pins the layout Antigravity's own Windows installer produces — verified against a real
    /// install at `%LOCALAPPDATA%\agy\bin\agy.exe`. Without this arm the CLI is only discoverable
    /// once its installer's PATH edit reaches a freshly started shell, which it does not for an
    /// already-running desktop process.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_candidates_cover_the_executable_named_install_directory() {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let definition =
            crate::contexts::tooling::cli::domain::definition("antigravity-cli").expect("catalog");

        let candidates = known_candidate_paths(definition);

        let expected = PathBuf::from(local_app_data)
            .join("agy")
            .join("bin")
            .join("agy.exe");
        assert!(
            candidates.contains(&expected),
            "expected {expected:?} among {candidates:?}"
        );
    }
}
