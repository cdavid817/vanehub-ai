//! Searches the workspace for files whose path matches a filename glob pattern. Traversal,
//! filtering, and boundary enforcement are all delegated to `walk`; this module only matches
//! patterns and shapes output.

use super::walk::{visit_workspace_files, Visit};
use super::{ToolExecutionOutcome, MAX_SEARCH_RESULTS};
use globset::GlobBuilder;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) fn execute_glob(
    pattern: &str,
    path: Option<&str>,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
) -> ToolExecutionOutcome {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return ToolExecutionOutcome {
            output: "No glob pattern was provided.".to_string(),
            is_error: true,
        };
    }
    // `literal_separator(true)` keeps `*` from crossing a directory separator, so only `**` does —
    // matching what users conventionally expect from a filename glob.
    let matcher = match GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
    {
        Ok(matcher) => matcher,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Invalid glob pattern \"{pattern}\": {error}"),
                is_error: true,
            }
        }
    };

    let mut matches: Vec<String> = Vec::new();
    let mut truncated = false;
    let outcome = visit_workspace_files(
        workspace_folder,
        path,
        &cancelled,
        &mut |_absolute, relative| {
            if matcher.is_match(relative) {
                matches.push(relative.to_string());
                if matches.len() >= MAX_SEARCH_RESULTS {
                    truncated = true;
                    return Visit::Stop;
                }
            }
            Visit::Continue
        },
    );
    if let Err(error) = outcome {
        return ToolExecutionOutcome {
            output: error,
            is_error: true,
        };
    }
    if matches.is_empty() {
        return ToolExecutionOutcome {
            output: format!("No files matched \"{pattern}\"."),
            is_error: false,
        };
    }
    matches.sort();
    let mut output = matches.join("\n");
    if truncated {
        output.push_str(&format!(
            "\n\n[Results truncated at {MAX_SEARCH_RESULTS} files. Narrow the pattern or use the path argument.]"
        ));
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn workspace(name: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::create_dir(directory.path().join("src")).expect("mkdir src");
        std::fs::write(directory.path().join("src/main.rs"), "fn main() {}").expect("write main");
        std::fs::write(directory.path().join("src/lib.rs"), "pub fn go() {}").expect("write lib");
        std::fs::write(directory.path().join("README.md"), "# hi").expect("write readme");
        directory
    }

    #[test]
    fn matches_files_by_extension() {
        let directory = workspace("glob-extension");
        let outcome = execute_glob(
            "**/*.rs",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/main.rs"));
        assert!(outcome.output.contains("src/lib.rs"));
        assert!(!outcome.output.contains("README.md"));
    }

    #[test]
    fn reports_no_matches_without_an_error() {
        let directory = workspace("glob-none");
        let outcome = execute_glob(
            "**/*.py",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("No files matched"));
    }

    #[test]
    fn an_invalid_pattern_is_reported_as_an_error() {
        let directory = workspace("glob-invalid");
        let outcome = execute_glob(
            "[unclosed",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("Invalid glob pattern"));
    }

    #[test]
    fn an_empty_pattern_is_rejected() {
        let directory = workspace("glob-empty");
        let outcome = execute_glob(
            "   ",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_path_scope_narrows_the_search() {
        let directory = workspace("glob-scope");
        std::fs::create_dir(directory.path().join("docs")).expect("mkdir docs");
        std::fs::write(directory.path().join("docs/guide.md"), "g").expect("write guide");
        let outcome = execute_glob(
            "**/*.md",
            Some("docs"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("docs/guide.md"));
        assert!(!outcome.output.contains("README.md"));
    }

    #[test]
    fn exceeding_the_result_limit_reports_truncation() {
        let directory = TempDirectory::new("glob-truncate");
        for index in 0..(MAX_SEARCH_RESULTS + 10) {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "x")
                .expect("write fixture");
        }
        let outcome = execute_glob(
            "**/*.txt",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_glob(
            "**/*",
            None,
            "Z:/definitely/does/not/exist",
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }
}
