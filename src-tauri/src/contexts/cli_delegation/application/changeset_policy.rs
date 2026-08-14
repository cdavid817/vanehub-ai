use super::DelegationChangeSetCapture;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelegationChangeSetLimits {
    pub(crate) files: usize,
    pub(crate) patch_bytes: usize,
    pub(crate) path_bytes: usize,
}

impl DelegationChangeSetLimits {
    pub(crate) const HARD_CEILING: Self = Self {
        files: 256,
        patch_bytes: 32 * 1024 * 1024,
        path_bytes: 4096,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationChangeSetPolicyError {
    EmptyChangeSet,
    LimitExceeded,
    UnsafePath,
    PathCollision,
    UnsupportedFileType,
    IncompleteEvidence,
}

pub(crate) struct DelegationChangeSetPolicy;

impl DelegationChangeSetPolicy {
    pub(crate) fn validate(
        capture: &DelegationChangeSetCapture,
        limits: DelegationChangeSetLimits,
    ) -> Result<(), DelegationChangeSetPolicyError> {
        if capture.files.is_empty() || capture.canonical_patch.is_empty() {
            return Err(DelegationChangeSetPolicyError::EmptyChangeSet);
        }
        if capture.files.len() > limits.files || capture.canonical_patch.len() > limits.patch_bytes
        {
            return Err(DelegationChangeSetPolicyError::LimitExceeded);
        }
        if capture.base_commit.is_empty()
            || !capture.diff_hash.starts_with("sha256:")
            || capture.diff_hash.len() != 71
        {
            return Err(DelegationChangeSetPolicyError::IncompleteEvidence);
        }
        let mut paths = BTreeSet::new();
        for file in &capture.files {
            for path in std::iter::once(file.path.as_str()).chain(file.previous_path.as_deref()) {
                if !safe_path(path, limits.path_bytes) {
                    return Err(DelegationChangeSetPolicyError::UnsafePath);
                }
                if !paths.insert(path.replace('\\', "/").to_lowercase()) {
                    return Err(DelegationChangeSetPolicyError::PathCollision);
                }
            }
            for mode in [file.before_mode.as_deref(), file.after_mode.as_deref()]
                .into_iter()
                .flatten()
            {
                if !matches!(mode, "100644" | "100755") {
                    return Err(DelegationChangeSetPolicyError::UnsupportedFileType);
                }
            }
            if file.before_mode.is_some() && file.before_git_hash.is_none()
                || file.after_mode.is_some() && file.after_git_hash.is_none()
            {
                return Err(DelegationChangeSetPolicyError::IncompleteEvidence);
            }
        }
        Ok(())
    }
}

fn safe_path(path: &str, maximum: usize) -> bool {
    !path.is_empty()
        && path.len() <= maximum
        && !path.starts_with('/')
        && !path.starts_with('\\')
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
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::cli_delegation::application::{
        DelegationChangeFile, DelegationChangeKind,
    };

    fn capture(path: &str, mode: &str) -> DelegationChangeSetCapture {
        DelegationChangeSetCapture {
            base_commit: "base".into(),
            files: vec![DelegationChangeFile {
                path: path.into(),
                previous_path: None,
                kind: DelegationChangeKind::Added,
                before_mode: None,
                after_mode: Some(mode.into()),
                before_git_hash: None,
                after_git_hash: Some("git-hash".into()),
                binary: false,
            }],
            canonical_patch: b"patch".to_vec(),
            diff_hash: format!("sha256:{}", "a".repeat(64)),
        }
    }

    #[test]
    fn ordinary_files_pass_and_links_gitlinks_or_control_paths_fail() {
        let limits = DelegationChangeSetLimits::HARD_CEILING;
        assert!(
            DelegationChangeSetPolicy::validate(&capture("src/lib.rs", "100644"), limits).is_ok()
        );
        assert_eq!(
            DelegationChangeSetPolicy::validate(&capture("link", "120000"), limits),
            Err(DelegationChangeSetPolicyError::UnsupportedFileType)
        );
        assert_eq!(
            DelegationChangeSetPolicy::validate(&capture(".git/config", "100644"), limits),
            Err(DelegationChangeSetPolicyError::UnsafePath)
        );
    }

    #[test]
    fn limits_case_collisions_and_incomplete_hashes_fail_closed() {
        let mut value = capture("src/File.rs", "100644");
        let mut duplicate = value.files[0].clone();
        duplicate.path = "src/file.rs".into();
        value.files.push(duplicate);
        assert_eq!(
            DelegationChangeSetPolicy::validate(&value, DelegationChangeSetLimits::HARD_CEILING),
            Err(DelegationChangeSetPolicyError::PathCollision)
        );
        let mut incomplete = capture("file", "100644");
        incomplete.files[0].after_git_hash = None;
        assert_eq!(
            DelegationChangeSetPolicy::validate(
                &incomplete,
                DelegationChangeSetLimits::HARD_CEILING
            ),
            Err(DelegationChangeSetPolicyError::IncompleteEvidence)
        );
    }
}
