//! Workspace-bounded traversal shared by `grep` and `glob`. Boundary concerns (path escape,
//! symlinks, cancellation, size limits) are implemented once here so neither tool has to repeat
//! them.

use crate::platform::filesystem::BoundedFilesystem;
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Per-file read cap. The output limit protects the model's context window; this one protects
/// the process itself — without it, a large log that isn't excluded by `.gitignore` would be
/// fully read into memory before any output truncation could take effect.
pub(crate) const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Sniff window for binary detection. Scanning the whole file isn't worth it for large files,
/// and a NUL byte in a text file — if one is there at all — almost always shows up near the
/// start.
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// What the visitor decides after each file: keep walking, or stop early (once a result-count
/// or byte budget the caller tracks has been reached).
pub(crate) enum Visit {
    Continue,
    Stop,
}

/// Walks the regular files under `workspace_folder` (optionally narrowed to a `relative_root`
/// subdirectory), calling `visit` with each file's absolute path and its workspace-relative
/// display form.
///
/// `absolute` is for filesystem calls only (`fs::read`, `fs::metadata`, ...). On Windows,
/// `canonicalize()` returns the `\\?\` extended-length form (see
/// `normalize_windows_extended_length_path`), so `absolute` carries that prefix too; putting it
/// in tool output would break both this tool's own absolute-path rejection and `cmd.exe`, which
/// has no concept of a UNC working directory. `display` — workspace-relative, forward-slash — is
/// the only form that should ever reach tool output or the model.
///
/// Symlinks are always skipped rather than followed-then-validated: following would require a
/// canonicalize syscall per entry, which is expensive on large repositories; skipping outright
/// eliminates the whole class of out-of-bounds reads (e.g. a link inside the repo pointing at
/// `~/.ssh/`).
///
/// `require_git(false)` is deliberate — the workspace need not be a git repository, but a
/// `.gitignore` inside it still expresses "this content isn't worth looking at." The default
/// `require_git(true)` would make a non-repository workspace fall back to searching everything.
///
/// `parents(true)` and `git_global(true)` mean results depend on ignore state *outside* the
/// workspace: a workspace nested under a directory that an ancestor repository's `.gitignore`
/// excludes will walk to zero files and return `Ok(())`, so the caller reports "no matches"
/// rather than an error. This matches ripgrep's own behavior and should stay as-is, but it is
/// worth knowing about up front rather than losing an hour to it later.
pub(crate) fn visit_workspace_files(
    workspace_folder: &str,
    relative_root: Option<&str>,
    cancelled: &AtomicBool,
    visit: &mut dyn FnMut(&Path, &str) -> Visit,
) -> Result<(), String> {
    let boundary = BoundedFilesystem::new(Path::new(workspace_folder))
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    // Canonicalizes directly rather than via `boundary.resolve_existing(".")`. `validate_relative`
    // does accept `Component::CurDir` (see the explicit arm in `platform/filesystem/mod.rs`), so
    // "." would resolve fine — but `relative_root == None` is the common case, and routing through
    // `resolve_existing` would send the workspace root back through the boundary's
    // validate/join/canonicalize/ensure-inside pipeline a second time just to arrive at the same
    // canonical path `BoundedFilesystem::new` already produced.
    let workspace_root = Path::new(workspace_folder)
        .canonicalize()
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    let root = match relative_root.map(str::trim).filter(|root| !root.is_empty()) {
        Some(relative) => boundary
            .resolve_existing(relative)
            .map_err(|error| format!("Path \"{relative}\" is not accessible: {error}"))?,
        None => workspace_root.clone(),
    };

    let walker = WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .build();

    for entry in walker {
        if cancelled.load(Ordering::SeqCst) {
            return Err("Search was cancelled.".to_string());
        }
        let Ok(entry) = entry else {
            // A single unreadable entry (permission error, a racing delete) shouldn't fail the
            // whole search.
            continue;
        };
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() || file_type.is_symlink() {
            continue;
        }
        let absolute = entry.path();
        let Ok(relative) = absolute.strip_prefix(&workspace_root) else {
            continue;
        };
        let display = relative.to_string_lossy().replace('\\', "/");
        if let Visit::Stop = visit(absolute, &display) {
            return Ok(());
        }
    }
    Ok(())
}

/// Binary detection: a NUL byte anywhere in the sniff window marks the file as binary. This is
/// more legible to the model than surfacing a UTF-8 decode error — it tells the model to pick a
/// different file rather than retry the same one.
pub(crate) fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0)
}

/// Checks whether a file exceeds `MAX_FILE_BYTES` before `std::fs::read` touches it. Returns
/// `true` when metadata can't be read at all — the failure mode is chosen to be "don't read"
/// rather than "read a file of unknown size".
pub(crate) fn exceeds_size_limit(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() > MAX_FILE_BYTES,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::sync::Arc;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn collect(directory: &TempDirectory, root: Option<&str>) -> Vec<String> {
        let folder = directory.path().to_string_lossy().to_string();
        let mut seen = Vec::new();
        visit_workspace_files(
            &folder,
            root,
            &not_cancelled(),
            &mut |_absolute, relative| {
                seen.push(relative.to_string());
                Visit::Continue
            },
        )
        .expect("walk succeeds");
        seen.sort();
        seen
    }

    #[test]
    fn visits_plain_files_under_the_workspace() {
        let directory = TempDirectory::new("walk-plain");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/b.txt"), "b").expect("write b");
        assert_eq!(collect(&directory, None), vec!["a.txt", "sub/b.txt"]);
    }

    #[test]
    fn skips_gitignored_paths_even_outside_a_git_repository() {
        let directory = TempDirectory::new("walk-gitignore");
        std::fs::write(
            directory.path().join(".gitignore"),
            "ignored.txt\nnode_modules/\n",
        )
        .expect("write gitignore");
        std::fs::write(directory.path().join("kept.txt"), "keep").expect("write kept");
        std::fs::write(directory.path().join("ignored.txt"), "drop").expect("write ignored");
        std::fs::create_dir(directory.path().join("node_modules")).expect("mkdir node_modules");
        std::fs::write(directory.path().join("node_modules/pkg.js"), "drop").expect("write pkg");
        let seen = collect(&directory, None);
        assert!(seen.contains(&"kept.txt".to_string()));
        assert!(!seen.iter().any(|path| path.contains("ignored.txt")));
        assert!(!seen.iter().any(|path| path.contains("node_modules")));
    }

    #[test]
    fn skips_hidden_entries() {
        let directory = TempDirectory::new("walk-hidden");
        std::fs::write(directory.path().join("visible.txt"), "v").expect("write visible");
        std::fs::create_dir(directory.path().join(".secret")).expect("mkdir .secret");
        std::fs::write(directory.path().join(".secret/key.txt"), "k").expect("write key");
        let seen = collect(&directory, None);
        assert_eq!(seen, vec!["visible.txt"]);
    }

    #[test]
    fn a_relative_root_narrows_the_walk() {
        let directory = TempDirectory::new("walk-root");
        std::fs::write(directory.path().join("top.txt"), "t").expect("write top");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/inner.txt"), "i").expect("write inner");
        assert_eq!(collect(&directory, Some("sub")), vec!["sub/inner.txt"]);
    }

    #[test]
    fn a_relative_root_that_escapes_the_workspace_is_rejected() {
        let directory = TempDirectory::new("walk-escape-root");
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            Some("../"),
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        let error = outcome.expect_err("a relative root outside the workspace must be rejected");
        // Pins the cause to `BoundaryError::Escape`'s message rather than accepting any error,
        // so this test can't pass because of an unrelated failure (e.g. a missing workspace).
        assert!(
            error.contains("path escape is not allowed"),
            "expected a path-escape error, got: {error}"
        );
    }

    #[test]
    fn a_cancelled_walk_stops_and_reports_an_error() {
        let directory = TempDirectory::new("walk-cancel");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        let cancelled = Arc::new(AtomicBool::new(true));
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &cancelled,
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = visit_workspace_files(
            "Z:/definitely/does/not/exist",
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_visitor_returning_stop_ends_the_walk_early() {
        let directory = TempDirectory::new("walk-stop");
        for index in 0..10 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "x")
                .expect("write fixture");
        }
        let mut count = 0usize;
        visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| {
                count += 1;
                Visit::Stop
            },
        )
        .expect("walk succeeds");
        assert_eq!(count, 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_pointing_outside_the_workspace_is_not_visited() {
        let outside = TempDirectory::new("walk-symlink-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            directory.path().join("leak.txt"),
        )
        .expect("create symlink");
        assert_eq!(collect(&directory, None), vec!["normal.txt"]);
    }

    // Creating a symlink on Windows needs Developer Mode or an elevated process. Rather than
    // skip the platform entirely, create it and no-op when the privilege isn't there, so the
    // check still runs in the common case (this is the primary development platform) without
    // failing CI/dev environments that lack the privilege. Mirrors
    // `canonical_boundary_rejects_symlinks_outside_the_root_when_supported` in
    // `platform/filesystem/mod.rs`.
    #[cfg(windows)]
    #[test]
    fn a_symlink_pointing_outside_the_workspace_is_not_visited_when_supported() {
        let outside = TempDirectory::new("walk-symlink-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        let target = outside.path().join("secret.txt");
        if std::os::windows::fs::symlink_file(target, directory.path().join("leak.txt")).is_ok() {
            assert_eq!(collect(&directory, None), vec!["normal.txt"]);
        }
    }

    // A directory symlink hands a whole foreign subtree to the walker instead of a single file —
    // a larger blast radius than the file case above — and was untested on every platform.
    #[cfg(unix)]
    #[test]
    fn a_directory_symlink_pointing_outside_the_workspace_is_not_visited() {
        let outside = TempDirectory::new("walk-symlink-dir-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink-dir");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        std::os::unix::fs::symlink(outside.path(), directory.path().join("leak"))
            .expect("create symlink");
        assert_eq!(collect(&directory, None), vec!["normal.txt"]);
    }

    #[cfg(windows)]
    #[test]
    fn a_directory_symlink_pointing_outside_the_workspace_is_not_visited_when_supported() {
        let outside = TempDirectory::new("walk-symlink-dir-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink-dir");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        if std::os::windows::fs::symlink_dir(outside.path(), directory.path().join("leak")).is_ok()
        {
            assert_eq!(collect(&directory, None), vec!["normal.txt"]);
        }
    }

    #[test]
    fn binary_content_is_detected_by_a_nul_byte() {
        assert!(is_binary(b"abc\0def"));
        assert!(!is_binary(b"plain text"));
        assert!(!is_binary(b""));
    }

    #[test]
    fn a_small_file_is_within_the_size_limit() {
        let directory = TempDirectory::new("walk-size-small");
        let path = directory.path().join("small.txt");
        std::fs::write(&path, "tiny").expect("write fixture");
        assert!(!exceeds_size_limit(&path));
    }

    #[test]
    fn a_file_over_the_size_limit_is_rejected() {
        let directory = TempDirectory::new("walk-size-large");
        let path = directory.path().join("large.bin");
        let file = std::fs::File::create(&path).expect("create fixture");
        // A sparse file: setting the logical length is instant and writes nothing to disk,
        // unlike allocating and writing a real multi-megabyte buffer on every test run.
        file.set_len(MAX_FILE_BYTES + 1)
            .expect("extend fixture past the limit");
        assert!(exceeds_size_limit(&path));
    }

    #[test]
    fn a_file_exactly_at_the_size_limit_is_not_rejected() {
        // Pins `exceeds_size_limit`'s use of `>` rather than `>=`: a file at exactly the limit
        // must still be treated as readable.
        let directory = TempDirectory::new("walk-size-boundary");
        let path = directory.path().join("boundary.bin");
        let file = std::fs::File::create(&path).expect("create fixture");
        file.set_len(MAX_FILE_BYTES)
            .expect("extend fixture to exactly the limit");
        assert!(!exceeds_size_limit(&path));
    }

    #[test]
    fn an_unreadable_path_is_treated_as_over_the_limit() {
        // Conservative on missing metadata: the caller skips or reports an error instead of
        // going on to read a file of unknown size.
        assert!(exceeds_size_limit(Path::new(
            "Z:/definitely/does/not/exist"
        )));
    }
}
