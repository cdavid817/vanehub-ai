//! Searches the workspace for files whose path matches a filename glob pattern. Traversal,
//! filtering, and boundary enforcement are all delegated to `walk`; this module only matches
//! patterns and shapes output.

use super::walk::{visit_workspace_files, Visit, ZERO_FILES_VISITED_NOTE};
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
    // Distinguishes "the walk visited files but none matched" from "the walk never reached any
    // files at all" (empty `path` scope, or the whole workspace excluded by an ancestor's
    // `.gitignore` -- see `visit_workspace_files`'s doc comment). Both would otherwise report the
    // identical confident "No files matched", with no way for the caller to tell them apart.
    let mut files_visited = 0usize;
    let outcome = visit_workspace_files(workspace_folder, path, &cancelled, &mut |file| {
        files_visited += 1;
        // Matched against `scoped`, not `display`: once `path` narrows the search, `display`
        // still carries the narrowed directory's own name, which `literal_separator` matching
        // would keep any unanchored pattern from ever crossing.
        if matcher.is_match(file.scoped) {
            // Checked before pushing, not after: this is the only ordering that can tell "we
            // stopped because the cap was reached" apart from "we stopped because matches simply
            // ran out" when the count lands exactly on the cap. Checking post-push cannot
            // distinguish those, so it would raise the truncation notice even when nothing was
            // actually cut.
            if matches.len() >= MAX_SEARCH_RESULTS {
                truncated = true;
                return Visit::Stop;
            }
            matches.push(file.display.to_string());
        }
        Visit::Continue
    });
    if let Err(error) = outcome {
        return ToolExecutionOutcome {
            output: error,
            is_error: true,
        };
    }
    if matches.is_empty() {
        let note = if files_visited == 0 {
            ZERO_FILES_VISITED_NOTE
        } else {
            ""
        };
        return ToolExecutionOutcome {
            output: format!("No files matched \"{pattern}\".{note}"),
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
        // This fixture's walk visits 3 files (src/main.rs, src/lib.rs, README.md) before finding
        // no ".py" match among them -- the zero-visited note must not fire here, or every genuine
        // "searched but found nothing" answer would carry a misleading "did I even search?" caveat.
        assert!(!outcome.output.contains("visited zero files"));
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
    fn a_path_scope_matches_unanchored_patterns_against_the_narrowed_root() {
        // Regression test: matching used to run against the workspace-relative path even when
        // `path` narrowed the walk, so `literal_separator` matching meant an unanchored pattern
        // (no leading `**/`) could never cross the `docs/` component and the file was reported
        // as not found even though it exists. See `WalkedFile::scoped` in `walk.rs`.
        let directory = workspace("glob-scope-unanchored");
        std::fs::create_dir(directory.path().join("docs")).expect("mkdir docs");
        std::fs::write(directory.path().join("docs/guide.md"), "g").expect("write guide");
        let outcome = execute_glob(
            "*.md",
            Some("docs"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        // Must be the workspace-relative path, not `guide.md` — the model hands this straight to
        // the `file`/`edit` tools, which reject anything but a workspace-relative path.
        assert_eq!(outcome.output, "docs/guide.md");
    }

    #[test]
    fn a_path_scope_matches_an_anchored_pattern_against_the_narrowed_root_not_the_workspace_root() {
        // Every other `path`-scoped test above uses an unanchored pattern (no literal directory
        // component), which happens to prove the `scoped`-vs-`display` distinction only in one
        // direction. An *anchored* pattern -- one with a literal leading path segment -- proves
        // the tool's documented matching basis ("relative to `path` when `path` is given") the
        // other way: if matching ran against `display` (workspace-relative, "docs/sub/file.md")
        // instead of `scoped` (`path`-relative, "sub/file.md"), this literal "sub/*.md" prefix
        // would never match at all, since the string doesn't start with "sub/".
        let directory = workspace("glob-scope-anchored");
        std::fs::create_dir_all(directory.path().join("docs/sub")).expect("mkdir docs/sub");
        std::fs::write(directory.path().join("docs/sub/file.md"), "g").expect("write file");
        let outcome = execute_glob(
            "sub/*.md",
            Some("docs"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        // Output is still workspace-relative, per the same schema fix.
        assert_eq!(outcome.output, "docs/sub/file.md");
    }

    #[test]
    fn a_search_that_visits_zero_files_says_so_distinctly_from_a_genuine_no_match() {
        // `path` pointing at an existing-but-empty directory makes `visit_workspace_files` visit
        // zero files -- the same observable shape `parents(true)`/`git_global(true)` can produce
        // for a whole workspace nested under an ancestor's `.gitignore` (see `walk.rs`). Both would
        // otherwise report the identical confident "No files matched" a genuinely absent pattern
        // gets, with no way for the caller to tell "nothing was searched" from "nothing matched".
        let directory = TempDirectory::new("glob-zero-visited");
        std::fs::create_dir(directory.path().join("empty")).expect("mkdir empty");
        let outcome = execute_glob(
            "**/*",
            Some("empty"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("No files matched"));
        assert!(outcome.output.contains("visited zero files"));
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
        let (files, note) = outcome
            .output
            .split_once("\n\n")
            .expect("truncated output includes a blank-line-separated note");
        assert!(note.contains("truncated"));
        // The substring check above would still pass for an off-by-one or a doubled cap; only
        // counting the returned lines catches a wrong threshold.
        assert_eq!(files.lines().count(), MAX_SEARCH_RESULTS);
    }

    #[test]
    fn a_result_count_exactly_at_the_cap_is_not_reported_as_truncated() {
        // A notice that fires when nothing was actually cut teaches the model to ignore it: with
        // exactly `MAX_SEARCH_RESULTS` matches and no more behind them, `truncated` must stay
        // false.
        let directory = TempDirectory::new("glob-exact-cap");
        for index in 0..MAX_SEARCH_RESULTS {
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
        assert!(!outcome.output.contains("truncated"));
        assert_eq!(outcome.output.lines().count(), MAX_SEARCH_RESULTS);
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
