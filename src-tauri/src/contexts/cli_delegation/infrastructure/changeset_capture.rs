use crate::contexts::cli_delegation::application::{
    DelegationChangeFile, DelegationChangeKind, DelegationChangeSetCapture,
    DelegationChangeSetCaptureError, DelegationChangeSetCapturePort,
};
use crate::platform::git::GitAdapter;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const GIT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) struct GitDelegationChangeSetCapture {
    git: GitAdapter,
}

impl GitDelegationChangeSetCapture {
    pub(crate) fn new() -> Self {
        Self {
            git: GitAdapter::default(),
        }
    }

    fn run(
        &self,
        workspace: &Path,
        environment: &BTreeMap<String, String>,
        args: &[&str],
    ) -> Result<Vec<u8>, DelegationChangeSetCaptureError> {
        let args = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        let output = self
            .git
            .execute_with_environment(workspace, &args, environment, GIT_TIMEOUT)
            .map_err(|_| DelegationChangeSetCaptureError::GitFailure)?;
        if !output.status.success() {
            return Err(DelegationChangeSetCaptureError::GitFailure);
        }
        Ok(output.stdout)
    }
}

impl DelegationChangeSetCapturePort for GitDelegationChangeSetCapture {
    fn capture(
        &self,
        workspace: &Path,
        control: &Path,
        expected_base: &str,
    ) -> Result<DelegationChangeSetCapture, DelegationChangeSetCaptureError> {
        if !workspace.is_absolute() || !control.is_absolute() || !control.is_dir() {
            return Err(DelegationChangeSetCaptureError::InvalidWorkspace);
        }
        let empty = BTreeMap::new();
        let head = self.run(workspace, &empty, &["rev-parse", "HEAD"])?;
        let head = String::from_utf8(head)
            .map_err(|_| DelegationChangeSetCaptureError::InvalidGitOutput)?
            .trim()
            .to_ascii_lowercase();
        if !head.eq_ignore_ascii_case(expected_base) {
            return Err(DelegationChangeSetCaptureError::BaseMismatch);
        }
        let index = prepare_private_index(workspace, control)?;
        let environment = BTreeMap::from([(
            "GIT_INDEX_FILE".to_string(),
            index.to_string_lossy().into_owned(),
        )]);
        self.run(workspace, &environment, &["add", "-A", "--", "."])?;
        let raw = self.run(
            workspace,
            &environment,
            &[
                "diff",
                "--cached",
                "--raw",
                "-z",
                "--find-renames",
                "HEAD",
                "--",
            ],
        )?;
        let numstat = self.run(
            workspace,
            &environment,
            &[
                "diff",
                "--cached",
                "--numstat",
                "-z",
                "--find-renames",
                "HEAD",
                "--",
            ],
        )?;
        let patch = self.run(
            workspace,
            &environment,
            &[
                "diff",
                "--cached",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                "--find-renames",
                "HEAD",
                "--",
            ],
        )?;
        let binary_paths = parse_binary_paths(&numstat)?;
        let files = parse_raw(&raw, &binary_paths)?;
        let diff_hash = sha256(&patch);
        Ok(DelegationChangeSetCapture {
            base_commit: head,
            files,
            canonical_patch: patch,
            diff_hash,
        })
    }
}

fn prepare_private_index(
    workspace: &Path,
    control: &Path,
) -> Result<PathBuf, DelegationChangeSetCaptureError> {
    let source = workspace.join(".git/index");
    let target = control.join("capture.index");
    if target.exists() {
        return Err(DelegationChangeSetCaptureError::StorageFailure);
    }
    fs::copy(source, &target).map_err(|_| DelegationChangeSetCaptureError::StorageFailure)?;
    Ok(target)
}

fn parse_raw(
    raw: &[u8],
    binary_paths: &[String],
) -> Result<Vec<DelegationChangeFile>, DelegationChangeSetCaptureError> {
    let fields = raw
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let metadata = std::str::from_utf8(fields[index])
            .map_err(|_| DelegationChangeSetCaptureError::InvalidGitOutput)?;
        index += 1;
        let parts = metadata.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 5 || index >= fields.len() {
            return Err(DelegationChangeSetCaptureError::InvalidGitOutput);
        }
        let status = parts[4];
        let first = utf8_path(fields[index])?;
        index += 1;
        let renamed = status.starts_with('R');
        let (previous_path, path) = if renamed {
            if index >= fields.len() {
                return Err(DelegationChangeSetCaptureError::InvalidGitOutput);
            }
            let next = utf8_path(fields[index])?;
            index += 1;
            (Some(first), next)
        } else {
            (None, first)
        };
        files.push(DelegationChangeFile {
            binary: binary_paths.iter().any(|candidate| candidate == &path),
            path,
            previous_path,
            kind: change_kind(status)?,
            before_mode: mode(parts[0]),
            after_mode: mode(parts[1]),
            before_git_hash: hash(parts[2]),
            after_git_hash: hash(parts[3]),
        });
    }
    Ok(files)
}

fn parse_binary_paths(bytes: &[u8]) -> Result<Vec<String>, DelegationChangeSetCaptureError> {
    let mut paths = Vec::new();
    let fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut index = 0;
    while index < fields.len() {
        let text = std::str::from_utf8(fields[index])
            .map_err(|_| DelegationChangeSetCaptureError::InvalidGitOutput)?;
        index += 1;
        if text.is_empty() {
            continue;
        }
        let parts = text.splitn(3, '\t').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(DelegationChangeSetCaptureError::InvalidGitOutput);
        }
        let binary = parts[0] == "-" && parts[1] == "-";
        let path = if parts[2].is_empty() {
            if index + 1 >= fields.len() {
                return Err(DelegationChangeSetCaptureError::InvalidGitOutput);
            }
            index += 2;
            utf8_path(fields[index - 1])?
        } else {
            parts[2].to_string()
        };
        if binary {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn change_kind(status: &str) -> Result<DelegationChangeKind, DelegationChangeSetCaptureError> {
    match status.chars().next() {
        Some('A') => Ok(DelegationChangeKind::Added),
        Some('M') => Ok(DelegationChangeKind::Modified),
        Some('D') => Ok(DelegationChangeKind::Deleted),
        Some('R') => Ok(DelegationChangeKind::Renamed),
        Some('T') => Ok(DelegationChangeKind::TypeChanged),
        _ => Err(DelegationChangeSetCaptureError::InvalidGitOutput),
    }
}

fn utf8_path(bytes: &[u8]) -> Result<String, DelegationChangeSetCaptureError> {
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| DelegationChangeSetCaptureError::InvalidGitOutput)
}

fn mode(value: &str) -> Option<String> {
    let value = value.trim_start_matches(':');
    (value != "000000").then(|| value.to_string())
}
fn hash(value: &str) -> Option<String> {
    (!value.bytes().all(|byte| byte == b'0')).then(|| value.to_string())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

#[cfg(test)]
#[path = "changeset_capture_tests.rs"]
mod tests;
