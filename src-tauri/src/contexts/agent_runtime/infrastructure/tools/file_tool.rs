use super::ToolExecutionOutcome;
use crate::platform::filesystem::BoundedFilesystem;
use std::path::Path;

/// Executes a file read/write tool call rooted at `workspace_folder`. Any `path` that would
/// resolve outside that folder (traversal, symlink escape, or an absolute path) is rejected by
/// `BoundedFilesystem` before touching the filesystem outside it.
pub(crate) fn execute_file(
    operation: &str,
    path: &str,
    content: Option<&str>,
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
        "read" => read_file(&boundary, path),
        "write" => write_file(&boundary, path, content.unwrap_or_default()),
        other => ToolExecutionOutcome {
            output: format!("Unknown file operation \"{other}\"."),
            is_error: true,
        },
    }
}

fn read_file(boundary: &BoundedFilesystem, path: &str) -> ToolExecutionOutcome {
    match boundary.resolve_existing(path) {
        Ok(resolved) => match std::fs::read_to_string(&resolved) {
            Ok(content) => ToolExecutionOutcome {
                output: content,
                is_error: false,
            },
            Err(error) => ToolExecutionOutcome {
                output: format!("Failed to read \"{path}\": {error}"),
                is_error: true,
            },
        },
        Err(error) => ToolExecutionOutcome {
            output: format!("Path \"{path}\" is not accessible: {error}"),
            is_error: true,
        },
    }
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
        let outcome = execute_file("read", "a.txt", None, &directory.path().to_string_lossy());
        assert!(!outcome.is_error);
        assert_eq!(outcome.output, "hello");
    }

    #[test]
    fn reading_a_missing_file_is_reported_as_an_error() {
        let directory = TempDirectory::new("file-tool-read-missing");
        let outcome = execute_file(
            "read",
            "missing.txt",
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
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn an_unknown_operation_is_rejected() {
        let directory = TempDirectory::new("file-tool-unknown-op");
        let outcome = execute_file("delete", "a.txt", None, &directory.path().to_string_lossy());
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_file("read", "a.txt", None, "Z:/definitely/does/not/exist");
        assert!(outcome.is_error);
    }
}
