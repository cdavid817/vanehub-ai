//! Scoped, exact-string replacement within a single workspace file. `old_string` must match
//! exactly once unless `replace_all` is set — silently editing "the first occurrence" would be
//! silent corruption when the match wasn't where the caller meant, and that is far harder to
//! trace back than an error that reports how many times the string actually matched.

use super::walk::{exceeds_size_limit, is_binary, MAX_FILE_BYTES};
use super::ToolExecutionOutcome;
use crate::platform::filesystem::BoundedFilesystem;
use std::path::Path;

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
    let updated = if replace_all {
        text.replace(old_string, new_string)
    } else {
        text.replacen(old_string, new_string, 1)
    };
    match std::fs::write(&resolved, &updated) {
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
        assert!(outcome.output.contains('3'));
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
        assert!(outcome.output.contains('2'));
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
        let directory = workspace("edit-escape", "content\n");
        let outcome = execute_edit(
            "../outside.rs",
            "content",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_file_is_reported_as_an_error() {
        let directory = workspace("edit-no-file", "content\n");
        let outcome = execute_edit(
            "absent.rs",
            "content",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
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
