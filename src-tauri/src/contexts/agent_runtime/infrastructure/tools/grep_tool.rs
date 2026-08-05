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

/// Upper bound on `context`. `render_file` computes `hit + context`, so an unclamped
/// model-supplied value would overflow `usize` (panic in debug, silent wraparound in release);
/// a merely large-but-legal value would also multiply the per-hit window-expansion cost. Not
/// enforced via the JSON schema — schema constraints aren't an enforcement boundary, since
/// nothing stops a caller from bypassing it.
const MAX_CONTEXT_LINES: usize = 20;

/// Appended to a line that had to be cut to fit `MAX_TOOL_OUTPUT_BYTES`, so the model can tell a
/// truncated fragment from a genuinely short line instead of silently reading a partial line as
/// if it were the whole match.
const LINE_TRUNCATED_MARKER: &str = " [truncated]";

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
    // Trimmed only to decide whether anything was provided at all. Matching must use the
    // original: trimming a content pattern (unlike a glob) silently changes what it means —
    // `", "` would become `","` and match a different set of lines, and `pattern.trim()` being
    // reused for regex construction is exactly what caused that.
    let pattern = request.pattern;
    if pattern.trim().is_empty() {
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
    // See `MAX_CONTEXT_LINES`: clamped here, once, rather than trusted down in `render_file`.
    let context = request.context.min(MAX_CONTEXT_LINES);
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
        let rendered = render_file(&matcher, file.display, &text, request.output_mode, context);
        for line in rendered {
            // Checked before pushing, not after: this is the only ordering that can tell "we
            // stopped because the cap was reached" apart from "we stopped because the results
            // simply ended" when the count lands exactly on the cap. Checking post-push cannot
            // distinguish those, so it would raise the truncation notice even when nothing was
            // cut — and a notice that fires for no reason teaches the model to ignore it.
            if lines.len() >= limit || bytes >= MAX_TOOL_OUTPUT_BYTES {
                truncated = true;
                return Visit::Stop;
            }
            // Reserve one byte for the `\n` that `lines.join("\n")` will insert before this
            // entry, then cut the line to whatever's left rather than pushing it whole — a
            // single 3MB matching line (e.g. a checked-in minified bundle) must not be able to
            // blow through a 64KB output budget by itself.
            let budget = MAX_TOOL_OUTPUT_BYTES
                .saturating_sub(bytes)
                .saturating_sub(1);
            let line = if line.len() > budget {
                truncated = true;
                truncate_line(line, budget)
            } else {
                line
            };
            bytes += line.len() + 1;
            lines.push(line);
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

/// Cuts `line` down to at most `budget` bytes — respecting UTF-8 char boundaries, since byte
/// length and character count diverge for multi-byte text — and appends `LINE_TRUNCATED_MARKER`.
/// Uses `saturating_sub` rather than trusting `budget > LINE_TRUNCATED_MARKER.len()`: if that
/// ever stops holding, the graceful degradation is a marker that overruns the budget by its own
/// small constant size, not a panic.
fn truncate_line(line: String, budget: usize) -> String {
    let keep = budget.saturating_sub(LINE_TRUNCATED_MARKER.len());
    let mut cut = keep.min(line.len());
    while cut > 0 && !line.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut truncated = line;
    truncated.truncate(cut);
    truncated.push_str(LINE_TRUNCATED_MARKER);
    truncated
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
    if output_mode == OUTPUT_MODE_FILES {
        // Only existence matters for this mode, not where every match is — stop at the first hit
        // instead of scanning (and indexing) the rest of the file for a result that discards the
        // rest of the information anyway.
        return if text.lines().any(|line| matcher.is_match(line)) {
            vec![relative.to_string()]
        } else {
            Vec::new()
        };
    }
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
        OUTPUT_MODE_COUNT => vec![format!("{relative}:{}", hits.len())],
        _ => {
            // A merge cursor, not a rescan: `hits` is already ascending, so a window can only
            // ever start where the previous one left off. The previous approach re-scanned
            // `wanted` with `.contains()` for every index in every window — O(matches^2) — which
            // measured ~76s (context=0) / ~217s (context=1) for 100k matching lines on this
            // machine, against ~226ms / ~218ms for this version (see the
            // `context_expansion_scales_linearly_not_quadratically` benchmark below). `next`
            // guarantees `start <= end` implies a strictly-new range, so the result is already
            // sorted and `wanted.sort_unstable()` is no longer needed.
            let mut wanted: Vec<usize> = Vec::new();
            let mut next = 0usize;
            for hit in &hits {
                let start = hit.saturating_sub(context).max(next);
                let end = (hit + context).min(all.len() - 1);
                if start > end {
                    continue;
                }
                wanted.extend(start..=end);
                next = end + 1;
            }
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
        // A dedicated single-match fixture rather than `workspace()`: `contains("src/alpha.rs:1")`
        // also passes for a count of `:10` or `:100`, so only an exact comparison against a
        // fixture with a known match count actually pins the number. `workspace()` also has a
        // second matching file (`notes.md`), and cross-file walk order isn't guaranteed, which
        // would make an exact `assert_eq!` on the full output flaky.
        let directory = TempDirectory::new("grep-count-exact");
        directory.write("src/alpha.rs", "needle\nneedle\nneedle\n");
        let outcome = execute_grep(
            request("needle", "count"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "src/alpha.rs:3");
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
        // A dedicated single-file fixture, not `workspace()`: `workspace()` has a second match in
        // `notes.md`, and cross-file order isn't guaranteed by the walk, so an exact assertion on
        // the full output needs a fixture with only one matching file. The context arm is the
        // most intricate logic in this module, so it earns an exact assertion rather than a
        // `contains`, which would pass even if extra or missing lines slipped in.
        let directory = TempDirectory::new("grep-context");
        directory.write(
            "src/alpha.rs",
            "fn alpha() {}\nlet needle = 1;\nfn done() {}\n",
        );
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(
            outcome.output,
            "src/alpha.rs:1:fn alpha() {}\nsrc/alpha.rs:2:let needle = 1;\nsrc/alpha.rs:3:fn done() {}"
        );
    }

    #[test]
    fn context_expansion_handles_a_match_on_the_first_line() {
        // Pins `hit.saturating_sub(context)`: line 0 has no line "-1" to include as leading
        // context, so the window must clamp to 0 rather than underflow.
        let directory = TempDirectory::new("grep-context-first-line");
        directory.write("a.txt", "needle here\nsecond line\nthird line\n");
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "a.txt:1:needle here\na.txt:2:second line");
    }

    #[test]
    fn context_expansion_handles_a_match_on_the_last_line() {
        // Pins `.min(all.len() - 1)`: the window's end must clamp to the file's last real line
        // rather than running past it.
        let directory = TempDirectory::new("grep-context-last-line");
        directory.write("a.txt", "first line\nsecond line\nneedle here\n");
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "a.txt:2:second line\na.txt:3:needle here");
    }

    #[test]
    fn context_windows_between_adjacent_matches_are_merged_without_duplicate_lines() {
        // The second match's own context window (lines 0..=2) is fully contained in the first
        // match's window — every line must still appear exactly once. Pins the merge-cursor
        // dedup in `render_file` (the `start > end` skip branch).
        let directory = TempDirectory::new("grep-context-overlap");
        directory.write("a.txt", "alpha\nneedle first\nneedle second\n");
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(
            outcome.output,
            "a.txt:1:alpha\na.txt:2:needle first\na.txt:3:needle second"
        );
    }

    #[test]
    fn context_larger_than_the_file_returns_the_whole_file_once() {
        let directory = TempDirectory::new("grep-context-oversized");
        directory.write("a.txt", "needle\nsecond\nthird\n");
        let mut input = request("needle", "content");
        input.context = 1000;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(
            outcome.output,
            "a.txt:1:needle\na.txt:2:second\na.txt:3:third"
        );
    }

    #[test]
    fn an_absurdly_large_context_does_not_panic_and_is_clamped() {
        // `hit + context` in `render_file` would overflow `usize` here if `context` reached
        // `execute_grep` unclamped. The point of this test is that the call below returns at all;
        // the assertion just confirms the (clamped) output is still sane rather than garbage.
        let directory = TempDirectory::new("grep-context-absurd");
        directory.write("a.txt", "needle\nsecond\nthird\n");
        let mut input = request("needle", "content");
        input.context = usize::MAX;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(
            outcome.output,
            "a.txt:1:needle\na.txt:2:second\na.txt:3:third"
        );
    }

    #[test]
    #[ignore = "timing benchmark for the render_file O(n^2)->O(n) fix; run explicitly with \
                `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::grep_tool -- \
                --ignored --nocapture`"]
    fn context_expansion_scales_linearly_not_quadratically() {
        // The pre-fix `wanted.contains()` rescan, measured on this machine with a throwaway
        // self-contained replica of the old algorithm: ~76.1s (context=0) / ~217.1s (context=1)
        // over 100k matching lines — quadratic. This version measured ~226ms / ~218ms for the
        // same input. This pins the fixed, linear behavior with a generous ceiling: still well
        // under a second, so it fails loudly if the complexity class ever regresses.
        let text = (0..100_000)
            .map(|index| format!("needle {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let matcher = RegexBuilder::new("needle").build().expect("valid pattern");

        let started = std::time::Instant::now();
        let rendered_0 = render_file(&matcher, "bench.txt", &text, OUTPUT_MODE_CONTENT, 0);
        let elapsed_0 = started.elapsed();

        let started = std::time::Instant::now();
        let rendered_1 = render_file(&matcher, "bench.txt", &text, OUTPUT_MODE_CONTENT, 1);
        let elapsed_1 = started.elapsed();

        assert_eq!(rendered_0.len(), 100_000);
        assert_eq!(rendered_1.len(), 100_000);
        println!("100k matching lines: context=0 took {elapsed_0:?}, context=1 took {elapsed_1:?}");
        assert!(
            elapsed_1.as_secs() < 1,
            "context=1 over 100k matching lines took {elapsed_1:?}, expected well under 1s"
        );
    }

    #[test]
    fn a_pattern_with_meaningful_whitespace_is_not_mangled_by_trimming() {
        // Trimming is right for the emptiness check but wrong for the regex itself: `", "`
        // becoming `","` after trimming would match commas not followed by a space too, silently
        // widening the result set.
        let directory = TempDirectory::new("grep-pattern-whitespace");
        directory.write("a.txt", "a, b\na,b\n");
        let outcome = execute_grep(
            request(", ", "content"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "a.txt:1:a, b");
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
    fn an_empty_pattern_is_rejected() {
        let directory = workspace("grep-empty-pattern");
        let outcome = execute_grep(
            request("   ", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn an_invalid_glob_filter_is_reported_as_an_error() {
        let directory = workspace("grep-invalid-glob");
        let mut input = request("needle", "files_with_matches");
        input.glob = Some("[unclosed");
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(outcome.is_error);
        assert!(outcome.output.contains("Invalid glob pattern"));
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
    fn head_limit_cannot_raise_the_shared_cap() {
        // `head_limit` may only lower `MAX_SEARCH_RESULTS`, never raise it — a model-supplied
        // value above the cap must still be clamped down to it, or a caller could enlarge its
        // own context budget just by asking.
        let directory = TempDirectory::new("grep-head-limit-cap");
        for index in 0..(MAX_SEARCH_RESULTS + 10) {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "needle")
                .expect("write fixture");
        }
        let mut input = request("needle", "files_with_matches");
        input.head_limit = Some(1000);
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(
            outcome
                .output
                .lines()
                .filter(|line| line.starts_with('f'))
                .count(),
            MAX_SEARCH_RESULTS
        );
    }

    #[test]
    fn a_result_count_exactly_at_the_head_limit_is_not_reported_as_truncated() {
        // A notice that fires when nothing was actually cut teaches the model to ignore it: with
        // exactly `head_limit` matches and no more behind them, `truncated` must stay false.
        let directory = TempDirectory::new("grep-head-limit-exact");
        for index in 0..5 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "needle")
                .expect("write fixture");
        }
        let mut input = request("needle", "files_with_matches");
        input.head_limit = Some(5);
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains("truncated"));
        assert_eq!(outcome.output.lines().count(), 5);
    }

    #[test]
    fn a_line_longer_than_the_output_budget_is_truncated_to_fit() {
        // A single line comfortably larger than `MAX_TOOL_OUTPUT_BYTES` (e.g. a checked-in
        // minified bundle matched in content mode) must not be able to blow through the output
        // budget just because it's one match on one line.
        let directory = TempDirectory::new("grep-huge-line");
        let huge_line = "x".repeat(MAX_TOOL_OUTPUT_BYTES * 2);
        directory.write("huge.txt", &huge_line);
        let outcome = execute_grep(
            request("x", "content"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        // Slack accounts for the trailing truncation notice and the per-line truncation marker —
        // both fixed-size constants, unlike the line itself, which is what this test pins: output
        // size must track the budget, not the (attacker-controlled) line length.
        assert!(
            outcome.output.len() < MAX_TOOL_OUTPUT_BYTES + 512,
            "output length {} was not bounded near the {MAX_TOOL_OUTPUT_BYTES} byte budget",
            outcome.output.len()
        );
        assert!(outcome.output.contains(LINE_TRUNCATED_MARKER.trim()));
    }

    #[test]
    fn a_path_scope_matches_unanchored_glob_filters_and_reports_workspace_relative_output() {
        // Mirrors `glob_tool`'s `a_path_scope_matches_unanchored_patterns_against_the_narrowed_root`.
        // Every other test in this module passes `path: None`, and with no `relative_root` the
        // walk makes `scoped` and `display` identical (see `scoped_equals_display_when_there_is_
        // no_relative_root` in `walk.rs`) — so none of them can tell apart a filter mistakenly
        // matched against `display`, or output mistakenly emitted as `scoped`, from correct code.
        // Only a narrowed-path test with an unanchored glob catches either direction:
        // - the filter must match `scoped` ("guide.md"), or an unanchored `*.md` could never
        //   cross the `docs/` component under `literal_separator` matching and the file would be
        //   missed even though it exists;
        // - the output must stay `display` ("docs/guide.md"), or the model would receive a
        //   scope-relative path that the `file`/`edit` tools reject outright.
        let directory = workspace("grep-scope-unanchored");
        std::fs::create_dir(directory.path().join("docs")).expect("mkdir docs");
        std::fs::write(directory.path().join("docs/guide.md"), "needle in guide\n")
            .expect("write guide");
        let mut input = request("needle", "files_with_matches");
        input.path = Some("docs");
        input.glob = Some("*.md");
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "docs/guide.md");
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
