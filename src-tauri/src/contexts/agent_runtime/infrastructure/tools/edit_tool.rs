//! Scoped, exact-string replacement within a single workspace file. `old_string` must match
//! exactly once unless `replace_all` is set — silently editing "the first occurrence" would be
//! silent corruption when the match wasn't where the caller meant, and that is far harder to
//! trace back than an error that reports how many times the string actually matched.

use super::walk::{exceeds_size_limit, is_binary, MAX_FILE_BYTES};
use super::ToolExecutionOutcome;
use crate::platform::filesystem::{BoundaryError, BoundedFilesystem};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Disambiguates temporary files from concurrent `edit` calls within the same process; combined
/// with the process id, this keeps `write_atomically`'s temp filename from colliding with
/// another in-flight edit even when two calls target the same file.
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn execute_edit(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    workspace_folder: &str,
) -> ToolExecutionOutcome {
    if old_string.is_empty() {
        return error("The old_string argument must not be empty.");
    }
    if old_string == new_string {
        return error("The old_string and new_string arguments are identical; nothing to change.");
    }
    let boundary = match BoundedFilesystem::new(Path::new(workspace_folder)) {
        Ok(boundary) => boundary,
        Err(failure) => return error(&format!("Workspace folder is unavailable: {failure}")),
    };
    let resolved = match boundary.resolve_existing(path) {
        Ok(resolved) => resolved,
        // A missing file is `edit`'s most common failure (the model guessed a path), and the
        // generic branch below surfaces the OS's raw, host-locale error string wrapped in a
        // doubled "not accessible: filesystem operation failed: ..." prefix — not actionable and
        // not a stable string to reason about. Every other `BoundaryError` variant already
        // carries a fixed English message, so only this one io-error case needs its own arm.
        Err(BoundaryError::Io(io_error)) if io_error.kind() == std::io::ErrorKind::NotFound => {
            return error(&format!(
                "\"{path}\" does not exist in this workspace. Verify the path, e.g. with the search tools, before retrying."
            ));
        }
        Err(failure) => return error(&format!("Path \"{path}\" is not accessible: {failure}")),
    };
    // The caller named this specific file, so an oversized or binary file must error rather than
    // be skipped the way `grep` skips one bad file out of a whole-repository search — a silent
    // no-op here would tell the model its edit happened when it did not.
    if exceeds_size_limit(&resolved) {
        return error(&format!(
            "\"{path}\" is larger than the {} MB edit limit.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let raw = match std::fs::read(&resolved) {
        Ok(raw) => raw,
        Err(failure) => return error(&format!("Failed to read \"{path}\": {failure}")),
    };
    if is_binary(&raw) {
        return error(&format!(
            "\"{path}\" appears to be a binary file and cannot be edited as text."
        ));
    }
    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => return error(&format!("\"{path}\" is not valid UTF-8 text.")),
    };

    let occurrences = text.matches(old_string).count();
    if occurrences == 0 {
        return error(&format!("The old_string was not found in \"{path}\"."));
    }
    if occurrences > 1 && !replace_all {
        return error(&format!(
            "The old_string matches {occurrences} times in \"{path}\". Provide more surrounding context to make it unique, or set replace_all to true."
        ));
    }
    // `text.replace`/`replacen` would allocate this unconditionally; predicting its length first
    // means an edit that would blow past the limit is rejected before that allocation runs, not
    // after. A failed `String` allocation aborts the whole process (taking every other session in
    // this app down with it, not just this tool call), so "reject after the fact" -- the way the
    // pre-read size check above works -- isn't an option here.
    let effective_replacements = if replace_all { occurrences } else { 1 };
    let projected_len = projected_replacement_len(
        text.len(),
        effective_replacements,
        old_string.len(),
        new_string.len(),
    );
    if projected_len > MAX_FILE_BYTES {
        return error(&format!(
            "The result of this edit to \"{path}\" would be larger than the {} MB edit limit.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let updated = if replace_all {
        text.replace(old_string, new_string)
    } else {
        text.replacen(old_string, new_string, 1)
    };
    match write_atomically(&resolved, &updated) {
        Ok(()) => ToolExecutionOutcome {
            output: format!("Replaced {occurrences} occurrence(s) in \"{path}\"."),
            is_error: false,
        },
        Err(failure) => error(&format!("Failed to write \"{path}\": {failure}")),
    }
}

fn error(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_string(),
        is_error: true,
    }
}

/// Predicts what `text.replace(old, new)` (or a single `replacen`) would allocate, without
/// performing the replacement. Saturating throughout: an adversarial `new_string` can make
/// `replacements * growth` overflow `u64` (or `usize` on a 32-bit target) long before the real
/// allocation would ever succeed, and overflowing here must saturate to "over the limit", never
/// wrap around to look like it fits.
fn projected_replacement_len(
    text_len: usize,
    replacements: usize,
    old_len: usize,
    new_len: usize,
) -> u64 {
    let text_len = text_len as u64;
    let replacements = replacements as u64;
    if new_len >= old_len {
        let growth = (new_len - old_len) as u64;
        text_len.saturating_add(replacements.saturating_mul(growth))
    } else {
        let shrink = (old_len - new_len) as u64;
        text_len.saturating_sub(replacements.saturating_mul(shrink))
    }
}

/// Writes `contents` to `target` via a sibling temporary file plus `fs::rename`, instead of
/// `fs::write` directly onto `target`. `fs::write` truncates its target before writing the new
/// bytes; `edit` is the only tool that modifies existing content, so an interrupted write here
/// (disk full, process killed) would otherwise leave a truncated source file behind with no
/// signal that it happened -- the model would see a plain io error, retry the same `old_string`,
/// and get "not found" against a file it had already damaged. The temp file lives in `target`'s
/// own directory so the rename stays on one filesystem, which is what makes it atomic on both
/// Windows (`std::fs::rename` calls `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`) and POSIX
/// (`rename(2)`): the replace either fully happens or `target` is left exactly as it was.
fn write_atomically(target: &Path, contents: &str) -> std::io::Result<()> {
    let parent = target.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "target path has no parent directory",
        )
    })?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_path = parent.join(format!(
        ".{file_name}.edit-tmp-{}-{sequence}",
        std::process::id()
    ));

    let result = std::fs::write(&temp_path, contents).and_then(|()| {
        // Best-effort: carry the original file's permissions onto the replacement so the atomic
        // rename doesn't silently reset them to the process's umask default (e.g. an executable
        // bit on Unix). Not fatal if this fails -- the written content is still correct either way.
        if let Ok(original) = std::fs::metadata(target) {
            let _ = std::fs::set_permissions(&temp_path, original.permissions());
        }
        std::fs::rename(&temp_path, target)
    });

    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn workspace(name: &str, contents: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::write(directory.path().join("code.rs"), contents).expect("write fixture");
        directory
    }

    #[test]
    fn replaces_a_unique_match() {
        let directory = workspace("edit-unique", "let a = 1;\nlet b = 2;\n");
        let outcome = execute_edit(
            "code.rs",
            "let a = 1;",
            "let a = 42;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "let a = 42;\nlet b = 2;\n"
        );
    }

    #[test]
    fn crlf_line_endings_and_a_missing_trailing_newline_are_preserved() {
        // This repo runs `core.autocrlf=true`, and `execute_edit` does whole-file exact-string
        // replacement rather than reasoning about lines -- pinning this now guards against a
        // future line-oriented rewrite quietly normalizing every edited file to LF. Read back as
        // raw bytes, not `read_to_string`: a lossy comparison could hide a CRLF being collapsed.
        let directory = workspace("edit-crlf", "let a = 1;\r\nlet b = 2;\r\nlast line no nl");
        let outcome = execute_edit(
            "code.rs",
            "let a = 1;",
            "let a = 42;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(
            std::fs::read(directory.path().join("code.rs")).expect("read back"),
            b"let a = 42;\r\nlet b = 2;\r\nlast line no nl"
        );
    }

    #[test]
    fn a_successful_edit_leaves_no_stray_temporary_files() {
        // `write_atomically` writes to a sibling temp file before renaming it over the target;
        // this pins that the rename actually happens (rather than, say, leaving both files
        // behind on the success path) by checking the workspace's final directory listing.
        let directory = workspace("edit-no-stray-temp", "let a = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "let a = 1;",
            "let a = 42;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        let entries: Vec<_> = std::fs::read_dir(directory.path())
            .expect("read workspace directory")
            .map(|entry| entry.expect("directory entry").file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("code.rs")]);
    }

    #[cfg(unix)]
    #[test]
    fn an_edit_preserves_the_original_file_permissions() {
        // The atomic-write path creates a brand new temp file under the process umask before
        // renaming it over the target; without copying the original mode across, an edit to an
        // executable script would silently strip its executable bit.
        use std::os::unix::fs::PermissionsExt;
        let directory = workspace("edit-permissions", "let a = 1;\n");
        let path = directory.path().join("code.rs");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("set executable permissions");
        let outcome = execute_edit(
            "code.rs",
            "let a = 1;",
            "let a = 42;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        let mode = std::fs::metadata(&path)
            .expect("read back metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn a_missing_match_is_reported_as_an_error_without_writing() {
        let directory = workspace("edit-missing", "let a = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "let z = 9;",
            "let z = 0;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("was not found"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "let a = 1;\n"
        );
    }

    #[test]
    fn multiple_matches_are_rejected_and_the_count_is_reported() {
        let directory = workspace("edit-multiple", "x = 1;\nx = 1;\nx = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "x = 1;",
            "x = 2;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        // A single-character `contains('3')` also matches a mutated "matches 13 times" (i.e.
        // `occurrences + 10`); assert the full phrase so a wrong count can't slip through.
        assert!(outcome.output.contains("matches 3 times"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "x = 1;\nx = 1;\nx = 1;\n"
        );
    }

    #[test]
    fn replace_all_replaces_every_match_and_reports_the_count() {
        let directory = workspace("edit-replace-all", "x = 1;\nx = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "x = 1;",
            "x = 2;",
            true,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        // Same masking risk as the multi-match test above: `contains('2')` also matches a
        // mutated "Replaced 12 occurrence(s)" (`occurrences + 10`).
        assert!(outcome.output.contains("Replaced 2 occurrence"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "x = 2;\nx = 2;\n"
        );
    }

    #[test]
    fn an_identical_old_and_new_string_is_rejected() {
        let directory = workspace("edit-noop", "same\n");
        let outcome = execute_edit(
            "code.rs",
            "same",
            "same",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn an_empty_old_string_is_rejected() {
        // An empty `old_string` matches at every character boundary, so
        // `"content\n".matches("").count()` is 9, not 0 — without its own guard, this falls into
        // the "matches more than once" branch instead of being silently accepted, and `is_error`
        // alone can't tell the two branches apart since both report an error. The message check
        // pins the guard actually under test; confirmed by temporarily deleting the guard and
        // observing this test still pass on `is_error` alone before adding the message assertion.
        let directory = workspace("edit-empty-old", "content\n");
        let outcome = execute_edit(
            "code.rs",
            "",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("must not be empty"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "content\n"
        );
    }

    #[test]
    fn a_path_that_escapes_the_workspace_is_rejected() {
        // `../outside.rs` must point at a file that actually exists, sitting next to (not
        // inside) the workspace root: `outside.rs` lives in `container`, the workspace is
        // `container/workspace`. Without a real file there, this can't distinguish "escape
        // rejected" from "file missing" -- both fail with `is_error: true`, one via the
        // boundary's escape check, the other via `canonicalize` failing on a path that was
        // never going to exist either way.
        let container = TempDirectory::new("edit-escape");
        let workspace_root = container.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).expect("create workspace subdirectory");
        std::fs::write(workspace_root.join("code.rs"), "content\n").expect("write fixture");
        let outside_path = container.write("outside.rs", "outside content\n");

        let outcome = execute_edit(
            "../outside.rs",
            "outside",
            "new",
            false,
            &workspace_root.to_string_lossy(),
        );
        assert!(outcome.is_error);
        // Pins the cause to the boundary's escape check specifically, matching the precedent in
        // `walk::a_relative_root_that_escapes_the_workspace_is_rejected` -- so this test can't
        // pass because of some unrelated failure (e.g. the outside file itself going missing).
        assert!(
            outcome.output.contains("path escape is not allowed"),
            "expected a path-escape error, got: {}",
            outcome.output
        );
        assert_eq!(
            std::fs::read_to_string(&outside_path).expect("read back outside file"),
            "outside content\n"
        );
    }

    #[test]
    fn a_missing_file_is_reported_with_a_stable_english_message() {
        let directory = workspace("edit-no-file", "content\n");
        let outcome = execute_edit(
            "absent.rs",
            "content",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        // Pins the exact, host-locale-independent message: the previous generic branch surfaced
        // the OS's raw (and on this machine, Chinese-language) error string doubled up behind
        // "not accessible: filesystem operation failed: ...", which is neither stable nor
        // actionable for the model to reason about.
        assert_eq!(
            outcome.output,
            "\"absent.rs\" does not exist in this workspace. Verify the path, e.g. with the search tools, before retrying."
        );
    }

    #[test]
    fn a_binary_file_is_refused() {
        let directory = TempDirectory::new("edit-binary");
        std::fs::write(directory.path().join("blob.bin"), b"abc\0def").expect("write blob");
        let outcome = execute_edit(
            "blob.bin",
            "abc",
            "xyz",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("binary"));
        // A NUL byte is itself valid single-byte UTF-8 (U+0000), so without the binary guard
        // `String::from_utf8` would succeed and the replace would proceed to write — the file
        // content check, not just `is_error`, is what would catch that.
        assert_eq!(
            std::fs::read(directory.path().join("blob.bin")).expect("read back"),
            b"abc\0def"
        );
    }

    #[test]
    fn a_file_over_the_edit_size_limit_is_rejected_without_writing() {
        // Mirrors `walk::a_file_over_the_size_limit_is_rejected`, but through `execute_edit`:
        // this pins the design decision that edit *errors* on an oversized file rather than
        // grep's silent skip, since the caller named this specific file and a silent no-op would
        // misreport that the edit happened.
        //
        // The message assertion is required, not optional: this fixture is a sparse file, and
        // sparse regions read back as zero bytes, which trips `is_binary`'s NUL-byte check too.
        // Confirmed by temporarily disabling the size guard — the call still returned
        // `is_error: true`, just with the binary-file message instead of the size-limit one, so
        // an `is_error`-only assertion would not have caught the guard's removal.
        let directory = TempDirectory::new("edit-oversized");
        let path = directory.path().join("large.rs");
        let file = std::fs::File::create(&path).expect("create fixture");
        file.set_len(MAX_FILE_BYTES + 1)
            .expect("extend fixture past the limit");
        let outcome = execute_edit(
            "large.rs",
            "anything",
            "else",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("MB edit limit"));
        assert_eq!(
            std::fs::metadata(&path).expect("read back metadata").len(),
            MAX_FILE_BYTES + 1
        );
    }

    #[test]
    fn a_projected_result_over_the_edit_size_limit_is_rejected_without_allocating_it() {
        // The pre-read check above only bounds the *input*; `text.replace` would otherwise
        // allocate `text.len() + occurrences * (new_len - old_len)` unconditionally, and a small,
        // repetitive source file with a long `new_string` can still blow past the limit --
        // that's the scenario this pins. The source fixture here is ~200 bytes, well under the
        // input cap, so this exercises the *result*-size guard specifically, not the input one.
        let directory = workspace("edit-result-too-large", &"x ".repeat(100));
        let big_replacement = "y".repeat(200_000);
        let outcome = execute_edit(
            "code.rs",
            "x",
            &big_replacement,
            true,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("MB edit limit"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "x ".repeat(100)
        );
    }

    #[test]
    fn a_non_utf8_file_is_rejected() {
        // No NUL byte (unlike the binary fixture above), so this exercises the UTF-8 validation
        // guard specifically rather than `is_binary`: 0xFF is never a valid UTF-8 leading byte.
        let directory = TempDirectory::new("edit-non-utf8");
        std::fs::write(directory.path().join("weird.rs"), [0x61, 0xFF, 0x62])
            .expect("write fixture");
        let outcome = execute_edit(
            "weird.rs",
            "a",
            "z",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("not valid UTF-8"));
        assert_eq!(
            std::fs::read(directory.path().join("weird.rs")).expect("read back"),
            [0x61, 0xFF, 0x62]
        );
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_edit(
            "code.rs",
            "content",
            "new",
            false,
            "Z:/definitely/does/not/exist",
        );
        assert!(outcome.is_error);
    }
}
