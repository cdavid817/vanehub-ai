use crate::contexts::cli_delegation::application::{
    DelegationRepositoryBaseline, DelegationWorkspaceError,
};
use crate::platform::git::GitAdapter;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) fn inspect(
    git: &GitAdapter,
    source: &Path,
    expected_commit: &str,
    timeout: Duration,
) -> Result<DelegationRepositoryBaseline, DelegationWorkspaceError> {
    let top_level = run(git, source, &["rev-parse", "--show-toplevel"], timeout)?;
    let canonical_root = PathBuf::from(top_level)
        .canonicalize()
        .map_err(|_| DelegationWorkspaceError::VerificationFailure)?;
    if canonical_root != source {
        return Err(DelegationWorkspaceError::VerificationFailure);
    }
    let head = run(git, source, &["rev-parse", "HEAD"], timeout)?;
    let status = run(
        git,
        source,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        timeout,
    )?;
    if !head.eq_ignore_ascii_case(expected_commit)
        || !status.is_empty()
        || operation_in_progress(&source.join(".git"))
    {
        return Err(DelegationWorkspaceError::VerificationFailure);
    }
    let tracked = run(git, source, &["ls-files", "--stage"], timeout)?;
    let tracked_files = validate_tracked_entries(&tracked)?;
    Ok(DelegationRepositoryBaseline {
        canonical_root,
        repository_identity: format!(
            "git:{}:{}",
            source.to_string_lossy(),
            head.to_ascii_lowercase()
        ),
        head_commit: head.to_ascii_lowercase(),
        tracked_files,
    })
}

fn run(
    git: &GitAdapter,
    root: &Path,
    arguments: &[&str],
    timeout: Duration,
) -> Result<String, DelegationWorkspaceError> {
    let arguments = arguments
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let output = git
        .execute(root, &arguments, timeout)
        .map_err(|_| DelegationWorkspaceError::GitFailure)?;
    if !output.status.success() {
        return Err(DelegationWorkspaceError::GitFailure);
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| DelegationWorkspaceError::VerificationFailure)
}

fn operation_in_progress(git_dir: &Path) -> bool {
    [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_LOG",
        "rebase-apply",
        "rebase-merge",
    ]
    .iter()
    .any(|entry| git_dir.join(entry).exists())
}

fn validate_tracked_entries(entries: &str) -> Result<usize, DelegationWorkspaceError> {
    let mut folded = BTreeSet::new();
    let mut count = 0_usize;
    for entry in entries.lines().filter(|entry| !entry.is_empty()) {
        let (metadata, path) = entry
            .split_once('\t')
            .ok_or(DelegationWorkspaceError::VerificationFailure)?;
        let mode = metadata.split_whitespace().next().unwrap_or_default();
        if mode == "160000" || !admitted_path(path) {
            return Err(DelegationWorkspaceError::VerificationFailure);
        }
        if !folded.insert(path.replace('\\', "/").to_lowercase()) {
            return Err(DelegationWorkspaceError::VerificationFailure);
        }
        count = count.saturating_add(1);
    }
    Ok(count)
}

fn admitted_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.starts_with('/')
        && !path.contains('\0')
        && !path.split(['/', '\\']).any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || segment.eq_ignore_ascii_case(".git")
                || reserved_name(segment)
        })
}

fn reserved_name(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or_default();
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_reject_gitlinks_case_collisions_and_windows_device_names() {
        assert!(validate_tracked_entries("100644 hash 0\tsrc/main.rs").is_ok());
        assert!(validate_tracked_entries("160000 hash 0\tvendor/submodule").is_err());
        assert!(
            validate_tracked_entries("100644 hash 0\tsrc/File.rs\n100644 hash 0\tsrc/file.rs")
                .is_err()
        );
        assert!(validate_tracked_entries("100644 hash 0\tCON.txt").is_err());
    }
}
