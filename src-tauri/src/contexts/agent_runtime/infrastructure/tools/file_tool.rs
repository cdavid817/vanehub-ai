//! Reads and writes a single workspace-relative file. `read` is bounded on three independent
//! axes -- line count, per-line character count, and total output bytes -- because unlike
//! `grep`/`glob`, which search *across* many files and can afford to skip an oversized one, the
//! caller here named this specific file and needs a bounded, resumable view of it rather than a
//! silent partial result.

use super::walk::{exceeds_size_limit, is_binary, MAX_FILE_BYTES};
use super::{ToolExecutionOutcome, MAX_TOOL_OUTPUT_BYTES};
use crate::platform::filesystem::BoundedFilesystem;
use std::path::Path;

/// Hard bounds on `read`. All three protect the model's context window, not the process
/// (`exceeds_size_limit`/`MAX_FILE_BYTES` already guard memory before any of this runs) -- so
/// `limit` may only make the response *smaller* than these, never larger: a model-supplied value
/// above the cap is clamped down, the same invariant `grep`'s `head_limit` and `glob`'s result
/// cap already enforce, so a caller can't enlarge its own context budget just by asking for more.
/// Use `offset` to page through the rest instead.
pub(crate) const MAX_READ_LINES: usize = 2000;
pub(crate) const MAX_READ_LINE_CHARS: usize = 2000;

/// Appended to a rendered "N\tline" entry that had to be cut short -- either because the raw
/// line exceeds `MAX_READ_LINE_CHARS`, or because including it in full would exceed the
/// remaining `MAX_TOOL_OUTPUT_BYTES` budget for the whole response. One shared marker for both:
/// the model only needs to know the entry's content is incomplete, not which of the two limits
/// did the cutting.
const LINE_TRUNCATED_MARKER: &str = "… [line truncated]";

/// Executes a file read/write tool call rooted at `workspace_folder`. Any `path` that would
/// resolve outside that folder (traversal, symlink escape, or an absolute path) is rejected by
/// `BoundedFilesystem` before touching the filesystem outside it.
pub(crate) fn execute_file(
    operation: &str,
    path: &str,
    content: Option<&str>,
    offset: Option<usize>,
    limit: Option<usize>,
    workspace_folder: &str,
) -> ToolExecutionOutcome {
    let boundary = match BoundedFilesystem::new(Path::new(workspace_folder)) {
        Ok(boundary) => boundary,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Workspace folder is unavailable: {error}"),
                is_error: true,
            }
        }
    };
    match operation {
        "read" => read_file(&boundary, path, offset.unwrap_or(0), limit),
        "write" => write_file(&boundary, path, content.unwrap_or_default()),
        other => ToolExecutionOutcome {
            output: format!("Unknown file operation \"{other}\"."),
            is_error: true,
        },
    }
}

fn read_file(
    boundary: &BoundedFilesystem,
    path: &str,
    offset: usize,
    limit: Option<usize>,
) -> ToolExecutionOutcome {
    // `limit: Some(0)` is rejected outright rather than silently rendering zero lines: the
    // caller could not otherwise tell "you asked for 0 lines and got them" apart from "your
    // input was discarded," and the trailing truncation notice would cite a fabricated resume
    // point ("line 0") on top of it. Mirrors `grep_tool`'s identical `head_limit == Some(0)`
    // guard; checked here, before touching the filesystem, because it's a pure request-shape
    // error independent of what the file actually contains.
    if limit == Some(0) {
        return ToolExecutionOutcome {
            output: "limit must be at least 1 (0 was requested, which would return zero lines)."
                .to_string(),
            is_error: true,
        };
    }
    let resolved = match boundary.resolve_existing(path) {
        Ok(resolved) => resolved,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Path \"{path}\" is not accessible: {error}"),
                is_error: true,
            }
        }
    };
    // The caller named this specific file, so an oversized or binary file must error rather
    // than be skipped the way `grep` skips one bad file out of a whole-repository search -- a
    // silent partial result here would tell the model it saw the whole file when it did not.
    // The size check must run before `fs::read`, or the memory it exists to protect is already
    // spent by the time it fires.
    if exceeds_size_limit(&resolved) {
        return ToolExecutionOutcome {
            output: format!(
                "\"{path}\" is larger than the {} MB read limit.",
                MAX_FILE_BYTES / (1024 * 1024)
            ),
            is_error: true,
        };
    }
    let raw = match std::fs::read(&resolved) {
        Ok(raw) => raw,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Failed to read \"{path}\": {error}"),
                is_error: true,
            }
        }
    };
    if is_binary(&raw) {
        return ToolExecutionOutcome {
            output: format!("\"{path}\" appears to be a binary file and cannot be read as text."),
            is_error: true,
        };
    }
    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => {
            return ToolExecutionOutcome {
                output: format!("\"{path}\" is not valid UTF-8 text."),
                is_error: true,
            }
        }
    };

    let all: Vec<&str> = text.lines().collect();
    // Validity depends on whether the file is empty: a zero-line file only accepts offset 0
    // (read the whole, empty, file); a non-empty file requires offset to name an actual line.
    // A single `offset >= all.len()` can't express both -- it would also swallow *every*
    // nonzero offset into an empty file as if it were valid, when there is no line for offset 42
    // to start from any more than there would be in a non-empty file, and the caller deserves
    // the same "beyond the end" answer either way.
    let offset_is_beyond_the_end = if all.is_empty() {
        offset > 0
    } else {
        offset >= all.len()
    };
    if offset_is_beyond_the_end {
        return ToolExecutionOutcome {
            output: format!(
                "Offset {offset} is beyond the end of \"{path}\" ({} lines).",
                all.len()
            ),
            is_error: false,
        };
    }
    let requested = limit.unwrap_or(MAX_READ_LINES).min(MAX_READ_LINES);
    let mut rendered = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    for (index, line) in all.iter().enumerate().skip(offset).take(requested) {
        // Checked before formatting/pushing, not after: this is the only ordering that can
        // tell "we stopped because the byte budget was reached" apart from "we stopped because
        // this was simply the last requested line" when the two coincide. Checking post-push
        // cannot distinguish those, so it would raise the truncation notice even when nothing
        // was actually cut -- reviewers caught exactly this shape in `grep_tool`. The per-line
        // `MAX_READ_LINE_CHARS` cap below already bounds how far a single entry can overshoot
        // the remaining budget (unlike grep's arbitrarily long source lines), so an overshoot
        // from checking post-push would be far smaller in practice here -- but the same
        // ordering is used for consistency, and because it is strictly more correct regardless
        // of severity.
        if bytes >= MAX_TOOL_OUTPUT_BYTES {
            truncated = true;
            break;
        }
        let (body, clipped) = if line.chars().count() > MAX_READ_LINE_CHARS {
            (
                line.chars().take(MAX_READ_LINE_CHARS).collect::<String>(),
                true,
            )
        } else {
            ((*line).to_string(), false)
        };
        let entry = if clipped {
            format!("{}\t{body}{LINE_TRUNCATED_MARKER}", index + 1)
        } else {
            format!("{}\t{body}", index + 1)
        };
        // Reserve one byte for the `\n` that `rendered.join("\n")` will insert before this
        // entry, then cut the entry to whatever's left rather than pushing it whole -- mirrors
        // `grep_tool::truncate_line`'s ordering and rationale: a single entry must not be able
        // to blow through the remaining output budget by itself.
        let budget = MAX_TOOL_OUTPUT_BYTES
            .saturating_sub(bytes)
            .saturating_sub(1);
        let entry = if entry.len() > budget {
            truncated = true;
            truncate_entry(entry, budget)
        } else {
            entry
        };
        bytes += entry.len() + 1;
        rendered.push(entry);
    }
    // Catches the line-count cap (`MAX_READ_LINES`/`limit`) being what stopped the loop, as
    // opposed to the in-loop byte-budget break above -- `.take(requested)` simply running out
    // never enters the loop body again, so nothing above would otherwise notice more content
    // remains. Landing exactly on the last line (no more content beyond what was rendered) must
    // not report truncation: a notice that fires when nothing was actually dropped teaches the
    // model to ignore it.
    let more_remains = offset + rendered.len() < all.len();
    if more_remains {
        truncated = true;
    }
    let mut output = rendered.join("\n");
    if truncated {
        // `offset + rendered.len()` does double duty: as a 1-indexed line number it names the
        // last line rendered above; as a 0-indexed offset it is exactly the value that resumes
        // right after that line. The two indexing bases happen to land on the same integer here,
        // but only spelling out the word "offset" next to it tells the model which meaning it
        // may reuse -- without that, "continue with offset" on its own is a coin flip between
        // this number and this number plus one, and losing it silently skips or repeats a line.
        let resume_offset = offset + rendered.len();
        if more_remains {
            output.push_str(&format!(
                "\n\n[Output truncated after line {resume_offset}. The file has {} lines; \
                 continue with offset: {resume_offset}.]",
                all.len()
            ));
        } else {
            // `more_remains` is false here only because the single entry that crossed the byte
            // budget (below) was also the file's last line: every line was rendered, just this
            // last one cut short, so there is nothing left to page to -- advising
            // `offset: resume_offset` would immediately report "beyond the end" right back. The
            // clipped entry already carries `LINE_TRUNCATED_MARKER`, so this only needs to
            // explain why the total is short of the file's real content, not offer paging advice
            // the caller can't act on.
            output.push_str(
                "\n\n[Output truncated: the file's last line was cut short by the output size \
                 limit; see the marked line above.]",
            );
        }
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}

/// Cuts an already-formatted "N\tline" entry down to at most `budget` bytes, respecting UTF-8
/// char boundaries, and marks it as cut. Mirrors `grep_tool::truncate_line`.
fn truncate_entry(entry: String, budget: usize) -> String {
    let keep = budget.saturating_sub(LINE_TRUNCATED_MARKER.len());
    let mut cut = keep.min(entry.len());
    while cut > 0 && !entry.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut truncated = entry;
    truncated.truncate(cut);
    truncated.push_str(LINE_TRUNCATED_MARKER);
    truncated
}

fn write_file(boundary: &BoundedFilesystem, path: &str, content: &str) -> ToolExecutionOutcome {
    match boundary.resolve_with_existing_parent(path) {
        Ok((resolved, _normalized)) => match std::fs::write(&resolved, content) {
            Ok(()) => ToolExecutionOutcome {
                output: format!("Wrote {} bytes to \"{path}\".", content.len()),
                is_error: false,
            },
            Err(error) => ToolExecutionOutcome {
                output: format!("Failed to write \"{path}\": {error}"),
                is_error: true,
            },
        },
        Err(error) => ToolExecutionOutcome {
            output: format!("Path \"{path}\" is not accessible: {error}"),
            is_error: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn reads_an_existing_file_within_the_workspace() {
        let directory = TempDirectory::new("file-tool-read");
        std::fs::write(directory.path().join("a.txt"), "hello").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        // Line-number prefixed, not the raw content -- see `read_output_is_prefixed_with_line_
        // numbers` below for the general case. Kept as an exact match rather than `contains`
        // per the task brief: relaxing this would stop pinning the "1\t" prefix format.
        assert_eq!(outcome.output, "1\thello");
    }

    #[test]
    fn reading_a_missing_file_is_reported_as_an_error() {
        let directory = TempDirectory::new("file-tool-read-missing");
        let outcome = execute_file(
            "read",
            "missing.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn writes_a_new_file_within_the_workspace() {
        let directory = TempDirectory::new("file-tool-write");
        let outcome = execute_file(
            "write",
            "new.txt",
            Some("written content"),
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("new.txt")).expect("read back"),
            "written content"
        );
    }

    #[test]
    fn a_path_that_escapes_the_workspace_is_rejected_without_touching_the_filesystem() {
        let directory = TempDirectory::new("file-tool-escape");
        let outcome = execute_file(
            "read",
            "../secret.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn an_unknown_operation_is_rejected() {
        let directory = TempDirectory::new("file-tool-unknown-op");
        let outcome = execute_file(
            "delete",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            "Z:/definitely/does/not/exist",
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn read_output_is_prefixed_with_line_numbers() {
        let directory = TempDirectory::new("file-tool-line-numbers");
        std::fs::write(directory.path().join("a.txt"), "first\nsecond\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("1\tfirst"));
        assert!(outcome.output.contains("2\tsecond"));
    }

    #[test]
    fn offset_and_limit_page_through_a_file() {
        let directory = TempDirectory::new("file-tool-paging");
        let body: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(5),
            Some(2),
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("6\tline6"));
        assert!(outcome.output.contains("7\tline7"));
        assert!(!outcome.output.contains("line8"));
        assert!(!outcome.output.contains("line5"));
    }

    #[test]
    fn exceeding_the_default_line_cap_reports_truncation() {
        let directory = TempDirectory::new("file-tool-line-cap");
        let body: String = (0..(MAX_READ_LINES + 50))
            .map(|index| format!("line{index}\n"))
            .collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
    }

    #[test]
    fn an_overlong_line_is_truncated_and_marked() {
        let directory = TempDirectory::new("file-tool-long-line");
        let body = format!("{}\n", "x".repeat(MAX_READ_LINE_CHARS + 100));
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("… [line truncated]"));
    }

    #[test]
    fn a_binary_file_is_refused_with_a_clear_reason() {
        let directory = TempDirectory::new("file-tool-binary");
        std::fs::write(directory.path().join("blob.bin"), b"abc\0def").expect("write blob");
        let outcome = execute_file(
            "read",
            "blob.bin",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("binary"));
    }

    #[test]
    fn an_offset_past_the_end_of_the_file_is_reported_without_an_error() {
        let directory = TempDirectory::new("file-tool-offset-past-end");
        std::fs::write(directory.path().join("a.txt"), "only\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(500),
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("beyond the end"));
    }

    #[test]
    fn an_offset_exactly_at_the_end_of_the_file_is_reported_without_an_error() {
        // Offset is 0-indexed, so offset == the file's line count means "start past the last
        // line". The brief's own fixture (offset 500 against a 1-line file) can't distinguish
        // the intended `>=` guard from an off-by-one `>` guard; this pins the exact boundary.
        let directory = TempDirectory::new("file-tool-offset-exact-end");
        std::fs::write(directory.path().join("a.txt"), "only\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(1),
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("beyond the end"));
    }

    #[test]
    fn reading_an_empty_file_succeeds_with_empty_output() {
        // The offset-beyond-the-end guard is `offset >= all.len() && !all.is_empty()`: without
        // the `!all.is_empty()` exclusion, an empty file (`all.len() == 0`) would always trip
        // "Offset 0 is beyond the end", since 0 >= 0, even though the file is simply empty, not
        // that the caller supplied a bad offset.
        let directory = TempDirectory::new("file-tool-empty");
        std::fs::write(directory.path().join("empty.txt"), "").expect("write fixture");
        let outcome = execute_file(
            "read",
            "empty.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "");
    }

    #[test]
    fn a_file_with_exactly_the_default_line_cap_is_not_reported_as_truncated() {
        // Mirrors `grep_tool::a_result_count_exactly_at_the_head_limit_is_not_reported_as_
        // truncated` / `glob_tool::a_result_count_exactly_at_the_cap_is_not_reported_as_
        // truncated`: a notice that fires when nothing was actually cut teaches the model to
        // ignore it. `exceeding_the_default_line_cap_reports_truncation` above only proves
        // truncation fires when there is more content than the cap; this proves it does *not*
        // fire when the file's length lands exactly on the cap.
        let directory = TempDirectory::new("file-tool-line-cap-exact");
        let body: String = (0..MAX_READ_LINES)
            .map(|index| format!("line{index}\n"))
            .collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains("truncated"));
        assert_eq!(
            outcome
                .output
                .lines()
                .filter(|line| line.contains('\t'))
                .count(),
            MAX_READ_LINES
        );
    }

    #[test]
    fn offset_plus_limit_landing_exactly_on_the_last_line_is_not_reported_as_truncated() {
        // The explicit-paging counterpart to the default-cap test above: a caller using
        // `offset`/`limit` to page through a file must not see a truncation notice on the
        // final page just because that page happened to be full.
        let directory = TempDirectory::new("file-tool-paging-exact-end");
        let body: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(5),
            Some(5),
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains("truncated"));
        assert_eq!(
            outcome.output,
            "6\tline6\n7\tline7\n8\tline8\n9\tline9\n10\tline10"
        );
    }

    #[test]
    fn a_limit_larger_than_the_hard_cap_cannot_enlarge_the_response() {
        // `limit` may only lower MAX_READ_LINES, never raise it -- a model-supplied value above
        // the hard cap must still be clamped down to it, mirroring
        // `grep_tool::head_limit_cannot_raise_the_shared_cap`. Without the `.min(MAX_READ_LINES)`
        // clamp, this would return MAX_READ_LINES + 100 lines instead.
        let directory = TempDirectory::new("file-tool-limit-cannot-enlarge");
        let body: String = (0..(MAX_READ_LINES + 100))
            .map(|index| format!("line{index}\n"))
            .collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            Some(MAX_READ_LINES + 100),
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(
            outcome
                .output
                .lines()
                .filter(|line| line.contains('\t'))
                .count(),
            MAX_READ_LINES
        );
        assert!(outcome
            .output
            .contains(&format!("{MAX_READ_LINES}\tline{}", MAX_READ_LINES - 1)));
        assert!(!outcome
            .output
            .contains(&format!("{}\tline{MAX_READ_LINES}", MAX_READ_LINES + 1)));
    }

    #[test]
    fn the_overlong_line_keeps_exactly_the_cap_and_no_more() {
        // `an_overlong_line_is_truncated_and_marked` above only checks the marker appears --
        // this pins the boundary precisely so an off-by-one in the `> MAX_READ_LINE_CHARS` /
        // `.take(MAX_READ_LINE_CHARS)` pairing would be caught: exactly MAX_READ_LINE_CHARS
        // characters of body survive, not one more or one fewer, and nothing else is in the
        // single-line output (no truncation notice, since this is the file's only line).
        let directory = TempDirectory::new("file-tool-long-line-exact");
        let body = format!("{}\n", "x".repeat(MAX_READ_LINE_CHARS + 100));
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        let expected = format!("1\t{}… [line truncated]", "x".repeat(MAX_READ_LINE_CHARS));
        assert_eq!(outcome.output, expected);
    }

    #[test]
    fn the_output_byte_budget_is_enforced_before_pushing_and_stays_bounded() {
        // Reproduces the exact byte accounting `read_file` uses for an unclipped (<=
        // MAX_READ_LINE_CHARS) entry, so the filler-line count needed to land just under
        // MAX_TOOL_OUTPUT_BYTES is computed rather than hand-picked, then pads a few lines past
        // that point so some line is always left over regardless of exact digit-width
        // rounding. This isolates the byte-budget cap from the far larger MAX_READ_LINES
        // line-count cap, and pins the ordering fix ported from grep_tool.rs: checking the
        // byte budget *after* accumulating and pushing an entry (the task brief's literal
        // shape, and the bug grep_tool.rs was already fixed for) lets the one entry that
        // crosses the budget overshoot it by close to its own ~2000-byte size; checking
        // *before* pushing and cutting that entry to whatever budget remains keeps the total
        // close to the budget instead.
        fn formatted_len(line_number: usize) -> usize {
            format!("{line_number}\t{}", "a".repeat(MAX_READ_LINE_CHARS)).len()
        }
        let mut cumulative = 0usize;
        let mut filler_lines = 0usize;
        while cumulative + formatted_len(filler_lines + 1) + 1 < MAX_TOOL_OUTPUT_BYTES {
            cumulative += formatted_len(filler_lines + 1) + 1;
            filler_lines += 1;
        }
        let total_lines = filler_lines + 5;

        let directory = TempDirectory::new("file-tool-byte-budget");
        let mut line = "a".repeat(MAX_READ_LINE_CHARS);
        line.push('\n');
        let body = line.repeat(total_lines);
        std::fs::write(directory.path().join("a.txt"), &body).expect("write fixture");

        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
        let rendered_lines = outcome
            .output
            .lines()
            .filter(|line| line.contains('\t'))
            .count();
        assert!(
            rendered_lines < total_lines,
            "expected the byte budget to cut off some of the {total_lines} lines, rendered {rendered_lines}"
        );
        assert!(
            rendered_lines < MAX_READ_LINES,
            "expected the {MAX_TOOL_OUTPUT_BYTES}-byte budget, not the {MAX_READ_LINES}-line \
             cap, to be what triggered truncation"
        );
        // Comfortably larger than the fixed-size trailing notice and truncation marker (well
        // under 200 bytes combined), comfortably smaller than one full entry's ~2000-byte size
        // -- so this only holds if the crossing entry was actually cut to fit the remaining
        // budget rather than pushed whole.
        assert!(
            outcome.output.len() < MAX_TOOL_OUTPUT_BYTES + 512,
            "output length {} overshot the {MAX_TOOL_OUTPUT_BYTES} byte budget by more than a \
             fixed-size notice should account for",
            outcome.output.len()
        );
    }

    #[test]
    fn a_file_over_the_read_size_limit_is_rejected() {
        // Mirrors `edit_tool::a_file_over_the_edit_size_limit_is_rejected_without_writing`: the
        // size check must run before `fs::read`, not after, or the whole point of protecting
        // memory is defeated.
        let directory = TempDirectory::new("file-tool-oversized");
        let path = directory.path().join("large.txt");
        let file = std::fs::File::create(&path).expect("create fixture");
        file.set_len(MAX_FILE_BYTES + 1)
            .expect("extend fixture past the limit");
        let outcome = execute_file(
            "read",
            "large.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("MB read limit"));
    }

    #[test]
    fn a_non_utf8_file_is_rejected_with_a_clear_reason() {
        // No NUL byte (unlike the binary fixture above), so this exercises the UTF-8
        // validation guard specifically rather than `is_binary`: 0xFF is never a valid UTF-8
        // leading byte. Mirrors `edit_tool::a_non_utf8_file_is_rejected`.
        let directory = TempDirectory::new("file-tool-non-utf8");
        std::fs::write(directory.path().join("weird.txt"), [0x61, 0xFF, 0x62])
            .expect("write fixture");
        let outcome = execute_file(
            "read",
            "weird.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("not valid UTF-8"));
    }

    #[test]
    fn a_line_of_exactly_the_per_line_char_cap_is_not_marked_truncated() {
        // Mirrors `a_file_with_exactly_the_default_line_cap_is_not_reported_as_truncated`, but
        // for the per-line character cap instead of the line-count cap: a line of exactly
        // MAX_READ_LINE_CHARS characters must render in full, unmarked. `> MAX_READ_LINE_CHARS`
        // is the only correct comparison -- flipping it to `>=` would mark this line truncated
        // (and, since `.take(MAX_READ_LINE_CHARS)` on an already-exactly-that-length iterator
        // changes nothing, cut zero actual characters while still slapping the marker on).
        // `an_overlong_line_is_truncated_and_marked` (fixture: MAX_READ_LINE_CHARS + 100 chars)
        // cannot catch that mutation, because its fixture is over the cap either way.
        let directory = TempDirectory::new("file-tool-line-cap-chars-exact");
        let body = format!("{}\n", "x".repeat(MAX_READ_LINE_CHARS));
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains(LINE_TRUNCATED_MARKER));
        assert_eq!(
            outcome.output,
            format!("1\t{}", "x".repeat(MAX_READ_LINE_CHARS))
        );
    }

    #[test]
    fn a_limit_of_zero_is_rejected_rather_than_reported_as_empty_output() {
        // Before this guard, `limit: Some(0)` silently rendered zero lines with
        // `is_error: false` and a trailing notice citing a nonexistent "line 0" -- the caller
        // could not tell "the file is genuinely empty here" apart from "your input was
        // discarded." Mirrors `grep_tool`'s
        // `a_head_limit_of_zero_is_rejected_rather_than_reported_as_no_matches`.
        let directory = TempDirectory::new("file-tool-limit-zero");
        std::fs::write(directory.path().join("a.txt"), "first\nsecond\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            Some(0),
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("limit"));
        assert!(!outcome.output.contains("line 0"));
    }

    #[test]
    fn the_truncation_notice_states_the_literal_resume_offset() {
        // Line numbers in the notice are 1-indexed; `offset` is 0-indexed. A notice that says
        // "truncated at line 4 ... continue with offset" without spelling out the number forces
        // the model to guess whether to reuse 4 or advance to 5 -- a coin flip that, lost,
        // silently skips or repeats a line. The notice must state the exact value to pass back.
        let directory = TempDirectory::new("file-tool-resume-offset");
        let body: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            Some(4),
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("continue with offset: 4"));
    }

    #[test]
    fn clipping_the_files_last_line_on_the_byte_budget_does_not_advise_an_unreachable_offset() {
        // A variant of `the_output_byte_budget_is_enforced_before_pushing_and_stays_bounded`
        // where the entry that crosses the byte budget is also the file's very last line: every
        // line still gets rendered (just the last one cut short), so
        // `offset + rendered.len() == all.len()` and there is nothing left to page to. The old
        // wording ("truncated at line N ... continue with offset") told the model to resume at
        // offset N regardless, which immediately reports "beyond the end" right back --
        // self-defeating advice for a line that already carries its own truncation marker.
        fn formatted_len(line_number: usize) -> usize {
            format!("{line_number}\t{}", "a".repeat(MAX_READ_LINE_CHARS)).len()
        }
        let mut cumulative = 0usize;
        let mut filler_lines = 0usize;
        while cumulative + formatted_len(filler_lines + 1) + 1 < MAX_TOOL_OUTPUT_BYTES {
            cumulative += formatted_len(filler_lines + 1) + 1;
            filler_lines += 1;
        }
        // Exactly one line beyond what fits whole -- unlike the byte-budget test above, that one
        // extra line must also be the file's last line, not just any later one.
        let total_lines = filler_lines + 1;

        let directory = TempDirectory::new("file-tool-byte-budget-last-line");
        let mut line = "a".repeat(MAX_READ_LINE_CHARS);
        line.push('\n');
        let body = line.repeat(total_lines);
        std::fs::write(directory.path().join("a.txt"), &body).expect("write fixture");

        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
        assert!(
            !outcome.output.contains("continue with offset"),
            "no content remains beyond the file's last (clipped) line, so paging advice is \
             unreachable: {}",
            outcome.output
        );
        let rendered_lines = outcome
            .output
            .lines()
            .filter(|line| line.contains('\t'))
            .count();
        assert_eq!(
            rendered_lines, total_lines,
            "every line should still be represented, just the last one cut short"
        );
    }

    #[test]
    fn a_multi_byte_line_is_truncated_by_character_count_not_byte_index() {
        // CJK characters are 3 bytes each in UTF-8, so a byte-oriented rewrite of the cap
        // (`line.len() > MAX_READ_LINE_CHARS`, or slicing with `&line[..MAX_READ_LINE_CHARS]`)
        // would either under-truncate -- byte length is 3x character count here -- or slice
        // mid-character and panic. Mirrors `the_overlong_line_keeps_exactly_the_cap_and_no_more`
        // above (ASCII-only) with a multi-byte fixture, exercising both the char-counting cap in
        // `read_file` and the `is_char_boundary` walk in `truncate_entry` it would fall through
        // to if the byte budget ever clipped a multi-byte entry.
        let directory = TempDirectory::new("file-tool-multibyte-long-line");
        let body = format!("{}\n", "测".repeat(MAX_READ_LINE_CHARS + 10));
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        let expected = format!(
            "1\t{}{LINE_TRUNCATED_MARKER}",
            "测".repeat(MAX_READ_LINE_CHARS)
        );
        assert_eq!(outcome.output, expected);
        // Pins the reviewer's own probe: 2000 kept CJK characters is 6000 bytes, not 2000 -- a
        // char-count assertion alone can't tell a correct char-aware cut apart from an
        // accidental byte-index cut that happened not to panic for this particular length.
        assert_eq!(outcome.output.len(), 6022);
    }

    #[test]
    fn a_nonzero_offset_on_an_empty_file_is_reported_as_beyond_the_end() {
        // The empty-file exclusion exists so offset 0 on a genuinely empty file reads as empty,
        // not as an error -- but it must not swallow every other offset too. `offset: 42` on a
        // zero-line file has no line 42 to start from and deserves the same "beyond the end"
        // answer a non-empty file would give, not a silent empty string indistinguishable from
        // "offset 0 succeeded and the file happens to be empty."
        let directory = TempDirectory::new("file-tool-empty-nonzero-offset");
        std::fs::write(directory.path().join("empty.txt"), "").expect("write fixture");
        let outcome = execute_file(
            "read",
            "empty.txt",
            None,
            Some(42),
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("beyond the end"));
    }
}
