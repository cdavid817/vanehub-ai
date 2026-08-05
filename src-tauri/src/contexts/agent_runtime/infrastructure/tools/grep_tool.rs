//! Searches workspace file contents by regular expression. Traversal and filtering are
//! delegated to `walk`; this module handles regex matching, the three output shapes, and
//! result/byte limits.

use super::walk::{exceeds_size_limit, is_binary, visit_workspace_files, Visit};
use super::{ToolExecutionOutcome, MAX_SEARCH_RESULTS, MAX_TOOL_OUTPUT_BYTES};
use globset::GlobBuilder;
use regex::RegexBuilder;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) const OUTPUT_MODE_FILES: &str = "files_with_matches";
pub(crate) const OUTPUT_MODE_CONTENT: &str = "content";
pub(crate) const OUTPUT_MODE_COUNT: &str = "count";

/// Carries grep's seven inputs as a struct rather than a long parameter list: it sidesteps
/// `clippy::too_many_arguments` and keeps callers from silently transposing same-typed
/// arguments (`path` vs `glob`, both `Option<&str>`) at the call site.
pub(crate) struct GrepRequest<'a> {
    pub(crate) pattern: &'a str,
    pub(crate) glob: Option<&'a str>,
    pub(crate) path: Option<&'a str>,
    pub(crate) output_mode: &'a str,
    pub(crate) context: usize,
    pub(crate) case_insensitive: bool,
    pub(crate) head_limit: Option<usize>,
}

pub(crate) fn execute_grep(
    request: GrepRequest<'_>,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
) -> ToolExecutionOutcome {
    let pattern = request.pattern.trim();
    if pattern.is_empty() {
        return error("No search pattern was provided.");
    }
    if !matches!(
        request.output_mode,
        OUTPUT_MODE_FILES | OUTPUT_MODE_CONTENT | OUTPUT_MODE_COUNT
    ) {
        return error(&format!(
            "Unknown output_mode \"{}\". Expected one of: {OUTPUT_MODE_FILES}, {OUTPUT_MODE_CONTENT}, {OUTPUT_MODE_COUNT}.",
            request.output_mode
        ));
    }
    let matcher = match RegexBuilder::new(pattern)
        .case_insensitive(request.case_insensitive)
        .build()
    {
        Ok(matcher) => matcher,
        Err(failure) => {
            return error(&format!(
                "Invalid regular expression \"{pattern}\": {failure}"
            ));
        }
    };
    let file_filter = match request.glob.map(str::trim).filter(|glob| !glob.is_empty()) {
        Some(glob) => match GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map(|compiled| compiled.compile_matcher())
        {
            Ok(compiled) => Some(compiled),
            Err(failure) => {
                return error(&format!("Invalid glob pattern \"{glob}\": {failure}"));
            }
        },
        None => None,
    };

    // `head_limit` may only lower the shared cap, never raise it: letting a model-supplied
    // value exceed `MAX_SEARCH_RESULTS` would let it enlarge its own context budget.
    let limit = request
        .head_limit
        .unwrap_or(MAX_SEARCH_RESULTS)
        .min(MAX_SEARCH_RESULTS);
    let mut lines: Vec<String> = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;

    let walk = visit_workspace_files(workspace_folder, request.path, &cancelled, &mut |file| {
        // Filtered against `scoped`, not `display`: once `path` narrows the search, `display`
        // still carries the narrowed directory's own name, which `literal_separator` matching
        // would keep an unanchored glob from ever crossing. Output still uses `display` — the
        // model hands these paths straight to `file`/`edit`, which accept only
        // workspace-relative paths.
        if let Some(filter) = &file_filter {
            if !filter.is_match(file.scoped) {
                return Visit::Continue;
            }
        }
        // The size check must happen before the read — otherwise an oversized file is already
        // in memory by the time it's judged skippable, defeating the point of the check. Skipped
        // silently rather than reported as an error: the user asked to search the whole
        // repository, and one oversized file shouldn't fail that.
        if exceeds_size_limit(file.absolute) {
            return Visit::Continue;
        }
        let Ok(raw) = std::fs::read(file.absolute) else {
            // A single unreadable file shouldn't fail the whole search.
            return Visit::Continue;
        };
        if is_binary(&raw) {
            return Visit::Continue;
        }
        let Ok(text) = String::from_utf8(raw) else {
            return Visit::Continue;
        };
        let rendered = render_file(
            &matcher,
            file.display,
            &text,
            request.output_mode,
            request.context,
        );
        for line in rendered {
            bytes += line.len() + 1;
            lines.push(line);
            if lines.len() >= limit || bytes >= MAX_TOOL_OUTPUT_BYTES {
                truncated = true;
                return Visit::Stop;
            }
        }
        Visit::Continue
    });
    if let Err(failure) = walk {
        return error(&failure);
    }
    if lines.is_empty() {
        return ToolExecutionOutcome {
            output: format!("No matches for \"{pattern}\"."),
            is_error: false,
        };
    }
    let mut output = lines.join("\n");
    if truncated {
        output.push_str(
            "\n\n[Results truncated. Narrow the pattern, add a glob filter, or scope with path.]",
        );
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}

/// Renders one file's matches into the output lines for the given `output_mode`. Returns an
/// empty `Vec` when the file has no matches.
fn render_file(
    matcher: &regex::Regex,
    relative: &str,
    text: &str,
    output_mode: &str,
    context: usize,
) -> Vec<String> {
    let all: Vec<&str> = text.lines().collect();
    let hits: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, line)| matcher.is_match(line))
        .map(|(index, _)| index)
        .collect();
    if hits.is_empty() {
        return Vec::new();
    }
    match output_mode {
        OUTPUT_MODE_FILES => vec![relative.to_string()],
        OUTPUT_MODE_COUNT => vec![format!("{relative}:{}", hits.len())],
        _ => {
            let mut wanted: Vec<usize> = Vec::new();
            for hit in &hits {
                let start = hit.saturating_sub(context);
                let end = (hit + context).min(all.len().saturating_sub(1));
                for index in start..=end {
                    if !wanted.contains(&index) {
                        wanted.push(index);
                    }
                }
            }
            wanted.sort_unstable();
            wanted
                .into_iter()
                .map(|index| format!("{relative}:{}:{}", index + 1, all[index]))
                .collect()
        }
    }
}

fn error(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_string(),
        is_error: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    // Two independent `&str` parameters feeding one struct lifetime is exactly the case
    // built-in elision refuses to guess at (E0106: could come from either parameter) — an
    // explicit `'a` tying both to the same lifetime is required, not just stylistic.
    fn request<'a>(pattern: &'a str, output_mode: &'a str) -> GrepRequest<'a> {
        GrepRequest {
            pattern,
            glob: None,
            path: None,
            output_mode,
            context: 0,
            case_insensitive: false,
            head_limit: None,
        }
    }

    fn workspace(name: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::create_dir(directory.path().join("src")).expect("mkdir src");
        std::fs::write(
            directory.path().join("src/alpha.rs"),
            "fn alpha() {}\nlet needle = 1;\n",
        )
        .expect("write alpha");
        std::fs::write(directory.path().join("src/beta.rs"), "fn beta() {}\n").expect("write beta");
        std::fs::write(directory.path().join("notes.md"), "needle in markdown\n")
            .expect("write notes");
        directory
    }

    #[test]
    fn files_with_matches_is_the_default_shape() {
        let directory = workspace("grep-files");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs"));
        assert!(outcome.output.contains("notes.md"));
        assert!(!outcome.output.contains("src/beta.rs"));
        // Filename mode must not echo file content back.
        assert!(!outcome.output.contains("let needle = 1;"));
    }

    #[test]
    fn content_mode_returns_matching_lines_with_line_numbers() {
        let directory = workspace("grep-content");
        let outcome = execute_grep(
            request("needle", "content"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs:2:let needle = 1;"));
    }

    #[test]
    fn count_mode_returns_per_file_counts() {
        let directory = workspace("grep-count");
        let outcome = execute_grep(
            request("needle", "count"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs:1"));
    }

    #[test]
    fn a_glob_filter_narrows_the_file_set() {
        let directory = workspace("grep-glob");
        let mut input = request("needle", "files_with_matches");
        input.glob = Some("**/*.rs");
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs"));
        assert!(!outcome.output.contains("notes.md"));
    }

    #[test]
    fn case_insensitive_matching_is_opt_in() {
        let directory = workspace("grep-case");
        let sensitive = execute_grep(
            request("NEEDLE", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(sensitive.output.contains("No matches"));

        let mut input = request("NEEDLE", "files_with_matches");
        input.case_insensitive = true;
        let insensitive = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(insensitive.output.contains("src/alpha.rs"));
    }

    #[test]
    fn context_lines_are_included_in_content_mode() {
        let directory = workspace("grep-context");
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("fn alpha() {}"));
    }

    #[test]
    fn an_invalid_regular_expression_is_reported_as_an_error() {
        let directory = workspace("grep-invalid");
        let outcome = execute_grep(
            request("(unclosed", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("Invalid regular expression"));
    }

    #[test]
    fn an_unknown_output_mode_is_rejected() {
        let directory = workspace("grep-mode");
        let outcome = execute_grep(
            request("needle", "sideways"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn binary_files_are_skipped_without_failing_the_search() {
        let directory = workspace("grep-binary");
        std::fs::write(directory.path().join("blob.bin"), b"needle\0\0binary").expect("write blob");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains("blob.bin"));
    }

    #[test]
    fn gitignored_files_are_not_searched() {
        let directory = workspace("grep-gitignore");
        std::fs::write(directory.path().join(".gitignore"), "hidden.txt\n")
            .expect("write gitignore");
        std::fs::write(directory.path().join("hidden.txt"), "needle here").expect("write hidden");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.output.contains("hidden.txt"));
    }

    #[test]
    fn reports_no_matches_without_an_error() {
        let directory = workspace("grep-empty");
        let outcome = execute_grep(
            request("zzz-absent-zzz", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("No matches"));
    }

    #[test]
    fn head_limit_truncates_and_says_so() {
        let directory = TempDirectory::new("grep-head-limit");
        for index in 0..20 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "needle")
                .expect("write fixture");
        }
        let mut input = request("needle", "files_with_matches");
        input.head_limit = Some(5);
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
        assert_eq!(
            outcome
                .output
                .lines()
                .filter(|line| line.starts_with('f'))
                .count(),
            5
        );
    }

    #[test]
    fn a_cancelled_search_is_reported_as_an_error() {
        let directory = workspace("grep-cancel");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            Arc::new(AtomicBool::new(true)),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            "Z:/definitely/does/not/exist",
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }
}
