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
            ] {
                candidates.push(home.join(relative).join(definition.executable_name));
            }
        }
        for base in ["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"] {
            candidates.push(PathBuf::from(base).join(definition.executable_name));
        }
    }
    candidates
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
}
