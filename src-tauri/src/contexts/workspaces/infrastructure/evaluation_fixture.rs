use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MAX_FILES: usize = 2_000;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedEvaluationFixture {
    pub(crate) workspace_path: String,
    pub(crate) file_count: usize,
    pub(crate) byte_count: u64,
}

pub(crate) fn prepare_evaluation_fixture(
    source: &Path,
    root: &Path,
    attempt_id: &str,
) -> Result<PreparedEvaluationFixture, String> {
    if !stable_id(attempt_id) {
        return Err("invalid evaluation attempt id".into());
    }
    let source = source
        .canonicalize()
        .map_err(|_| "evaluation fixture is unavailable")?;
    if !source.is_dir() {
        return Err("evaluation fixture must be a directory".into());
    }
    fs::create_dir_all(root).map_err(|_| "evaluation root is unavailable")?;
    let destination = root.join(attempt_id);
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|_| "evaluation reset failed")?;
    }
    fs::create_dir(&destination).map_err(|_| "evaluation workspace creation failed")?;
    let mut budget = CopyBudget::default();
    if let Err(error) = copy_tree(&source, &destination, &mut budget) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }
    Ok(PreparedEvaluationFixture {
        workspace_path: destination.to_string_lossy().into_owned(),
        file_count: budget.files,
        byte_count: budget.bytes,
    })
}

pub(crate) fn cleanup_evaluation_fixture(root: &Path, attempt_id: &str) -> Result<(), String> {
    if !stable_id(attempt_id) {
        return Err("invalid evaluation attempt id".into());
    }
    let destination = root.join(attempt_id);
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(|_| "evaluation cleanup failed")?;
    }
    Ok(())
}

pub(crate) fn changed_evaluation_paths(
    source: &Path,
    workspace: &Path,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    collect_changed_paths(source, workspace, Path::new(""), &mut paths)?;
    paths.sort();
    paths.dedup();
    if paths.len() > 256 {
        return Err("evaluation diff exceeds path bounds".into());
    }
    Ok(paths)
}

#[derive(Default)]
struct CopyBudget {
    files: usize,
    bytes: u64,
}

fn copy_tree(source: &Path, destination: &Path, budget: &mut CopyBudget) -> Result<(), String> {
    for item in fs::read_dir(source).map_err(|_| "evaluation fixture cannot be read")? {
        let item = item.map_err(|_| "evaluation fixture entry is invalid")?;
        let metadata = fs::symlink_metadata(item.path())
            .map_err(|_| "evaluation fixture metadata is unavailable")?;
        if metadata.file_type().is_symlink() {
            return Err("evaluation fixture symlinks are not allowed".into());
        }
        let target = destination.join(item.file_name());
        if metadata.is_dir() {
            fs::create_dir(&target).map_err(|_| "evaluation directory copy failed")?;
            copy_tree(&item.path(), &target, budget)?;
        } else if metadata.is_file() {
            budget.files += 1;
            budget.bytes = budget.bytes.saturating_add(metadata.len());
            if budget.files > MAX_FILES || budget.bytes > MAX_BYTES {
                return Err("evaluation fixture exceeds bounds".into());
            }
            fs::copy(item.path(), target).map_err(|_| "evaluation file copy failed")?;
        } else {
            return Err("evaluation fixture special files are not allowed".into());
        }
    }
    Ok(())
}

fn collect_changed_paths(
    source: &Path,
    workspace: &Path,
    relative: &Path,
    paths: &mut Vec<String>,
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for root in [source, workspace] {
        let directory = root.join(relative);
        if !directory.exists() {
            continue;
        }
        for item in fs::read_dir(directory).map_err(|_| "evaluation diff cannot be read")? {
            let item = item.map_err(|_| "evaluation diff entry is invalid")?;
            names.insert(item.file_name());
        }
    }
    for name in names {
        let next = relative.join(name);
        let source_path = source.join(&next);
        let workspace_path = workspace.join(&next);
        for path in [&source_path, &workspace_path] {
            if path.exists()
                && fs::symlink_metadata(path)
                    .map_err(|_| "evaluation diff metadata is unavailable")?
                    .file_type()
                    .is_symlink()
            {
                return Err("evaluation diff symlinks are not allowed".into());
            }
        }
        if source_path.is_dir() || workspace_path.is_dir() {
            collect_changed_paths(source, workspace, &next, paths)?;
        } else if !source_path.is_file()
            || !workspace_path.is_file()
            || fs::read(&source_path).map_err(|_| "evaluation source cannot be read")?
                != fs::read(&workspace_path).map_err(|_| "evaluation workspace cannot be read")?
        {
            paths.push(next.to_string_lossy().replace('\\', "/"));
        }
        if paths.len() > 256 {
            return Err("evaluation diff exceeds path bounds".into());
        }
    }
    Ok(())
}

fn stable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn reset_produces_clean_distinct_attempts() {
        let base = std::env::temp_dir().join(format!("vanehub-eval-{}", std::process::id()));
        let source = base.join("source");
        let root = base.join("runs");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("input.txt"), "fixed").expect("fixture");
        let first = prepare_evaluation_fixture(&source, &root, "attempt-a").expect("first");
        fs::write(
            PathBuf::from(&first.workspace_path).join("generated.txt"),
            "dirty",
        )
        .expect("dirty");
        let reset = prepare_evaluation_fixture(&source, &root, "attempt-a").expect("reset");
        assert!(!PathBuf::from(reset.workspace_path)
            .join("generated.txt")
            .exists());
        let second = prepare_evaluation_fixture(&source, &root, "attempt-b").expect("second");
        assert_ne!(first.workspace_path, second.workspace_path);
        cleanup_evaluation_fixture(&root, "attempt-a").expect("cleanup");
        cleanup_evaluation_fixture(&root, "attempt-b").expect("cleanup");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn reports_only_bounded_relative_changed_paths() {
        let base = tempfile::tempdir().expect("temp");
        let source = base.path().join("source");
        let workspace = base.path().join("workspace");
        fs::create_dir_all(&source).expect("source");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(source.join("same.txt"), "same").expect("source same");
        fs::write(workspace.join("same.txt"), "same").expect("workspace same");
        fs::write(workspace.join("added.txt"), "added").expect("added");
        assert_eq!(
            changed_evaluation_paths(&source, &workspace).expect("diff"),
            vec!["added.txt"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_and_cleans_partial_copy() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!("vanehub-eval-link-{}", std::process::id()));
        let source = base.join("source");
        let root = base.join("runs");
        fs::create_dir_all(&source).expect("source");
        symlink("/tmp", source.join("escape")).expect("link");
        assert!(prepare_evaluation_fixture(&source, &root, "attempt-link").is_err());
        assert!(!root.join("attempt-link").exists());
        let _ = fs::remove_dir_all(base);
    }
}
