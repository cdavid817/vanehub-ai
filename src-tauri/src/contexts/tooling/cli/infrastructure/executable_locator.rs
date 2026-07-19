use super::candidates::{CliCandidateSource, SystemCliCandidateSource};
use super::process_adapter::platform_process_runner;
use super::support::is_direct_cli_executable;
use crate::contexts::tooling::cli::application::CliExecutableLocatorPort;
use crate::contexts::tooling::cli::domain::ToolDefinition;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CliExecutableLocatorAdapter {
    candidates: Arc<dyn CliCandidateSource>,
}

impl CliExecutableLocatorAdapter {
    pub(crate) fn new() -> Self {
        Self {
            candidates: Arc::new(SystemCliCandidateSource::new(platform_process_runner())),
        }
    }

    #[cfg(test)]
    fn with_candidates(candidates: Arc<dyn CliCandidateSource>) -> Self {
        Self { candidates }
    }
}

impl CliExecutableLocatorPort for CliExecutableLocatorAdapter {
    fn resolve(&self, definition: ToolDefinition, cached_path: Option<&str>) -> Option<String> {
        cached_path
            .filter(|path| !path.trim().is_empty())
            .map(Path::new)
            .filter(|path| path.is_file() && is_direct_cli_executable(path))
            .map(|path| path.to_string_lossy().to_string())
            .or_else(|| {
                self.candidates
                    .candidates(definition)
                    .into_iter()
                    .next()
                    .map(|path| path.to_string_lossy().to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::definition;
    use std::path::PathBuf;

    struct FakeCandidates {
        paths: Vec<PathBuf>,
    }

    impl CliCandidateSource for FakeCandidates {
        fn candidates(&self, _definition: ToolDefinition) -> Vec<PathBuf> {
            self.paths.clone()
        }
    }

    #[test]
    fn resolver_uses_first_bounded_candidate_when_cache_is_invalid() {
        let expected = if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\fixture\codex.exe")
        } else {
            PathBuf::from("/fixture/codex")
        };
        let adapter = CliExecutableLocatorAdapter::with_candidates(Arc::new(FakeCandidates {
            paths: vec![expected.clone()],
        }));

        let resolved = adapter.resolve(
            definition("codex-cli").expect("definition"),
            Some("/missing/cached/path"),
        );

        assert_eq!(resolved, Some(expected.to_string_lossy().to_string()));
    }

    #[test]
    fn resolver_returns_none_when_cache_and_candidates_are_empty() {
        let adapter = CliExecutableLocatorAdapter::with_candidates(Arc::new(FakeCandidates {
            paths: Vec::new(),
        }));

        assert_eq!(
            adapter.resolve(definition("codex-cli").expect("definition"), None),
            None
        );
    }
}
