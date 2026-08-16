use super::{GitDiffHunk, WorkspaceApplicationError};
use sha2::{Digest, Sha256};

pub(crate) const MAX_REVIEW_FILES: usize = 1_000;
pub(crate) const MAX_REVIEW_FILE_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_REVIEW_DIFF_BYTES: usize = 8 * 1024 * 1024;
const CONTEXT_RADIUS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewFileSummary {
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) change_type: String,
    pub(crate) old_hash: Option<String>,
    pub(crate) new_hash: Option<String>,
    pub(crate) binary: bool,
    pub(crate) oversized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewDiffHunk {
    pub(crate) fingerprint: String,
    pub(crate) context_fingerprints: Vec<String>,
    pub(crate) hunk: GitDiffHunk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewDiffFile {
    pub(crate) summary: ReviewFileSummary,
    pub(crate) hunks: Vec<ReviewDiffHunk>,
    pub(crate) truncated: bool,
    pub(crate) accepted_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewSnapshot {
    pub(crate) workspace_id: String,
    pub(crate) base_revision: Option<String>,
    pub(crate) head_revision: Option<String>,
    pub(crate) fingerprint: String,
    pub(crate) files: Vec<ReviewFileSummary>,
    pub(crate) truncated: bool,
    pub(crate) accepted_bytes: usize,
}

pub(crate) trait WorkspaceReviewPort: Send + Sync {
    fn create_review_snapshot(
        &self,
        session_id: &str,
    ) -> Result<ReviewSnapshot, WorkspaceApplicationError>;

    fn load_review_file(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<ReviewDiffFile, WorkspaceApplicationError>;

    fn revert_review_change(
        &self,
        request: &ReviewRevertRequest,
    ) -> Result<ReviewRevertReceipt, WorkspaceApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewRevertRequest {
    pub(crate) session_id: String,
    pub(crate) path: String,
    pub(crate) expected_snapshot: String,
    pub(crate) hunk_fingerprint: Option<String>,
    pub(crate) confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewRevertReceipt {
    pub(crate) path: String,
    pub(crate) previous_snapshot: String,
    pub(crate) resulting_snapshot: String,
    pub(crate) reverted_hunks: usize,
}

pub(crate) fn fingerprint_hunk(hunk: &GitDiffHunk) -> String {
    measured_hunk_fingerprint(hunk).0
}

pub(crate) fn measured_hunk_fingerprint(hunk: &GitDiffHunk) -> (String, usize, usize) {
    let mut digest = Sha256::new();
    update_part(&mut digest, &hunk.header);
    update_part(&mut digest, &hunk.old_start.to_string());
    update_part(&mut digest, &hunk.old_lines.to_string());
    update_part(&mut digest, &hunk.new_start.to_string());
    update_part(&mut digest, &hunk.new_lines.to_string());
    let mut accepted_bytes = 0usize;
    for line in &hunk.lines {
        update_part(&mut digest, &line.kind);
        update_part(&mut digest, &line.content);
        accepted_bytes = accepted_bytes.saturating_add(line.kind.len() + line.content.len());
    }
    (digest_hex(digest), hunk.lines.len(), accepted_bytes)
}

pub(crate) fn fingerprint_context(hunk: &GitDiffHunk, line_index: usize) -> String {
    let start = line_index.saturating_sub(CONTEXT_RADIUS);
    let end = line_index
        .saturating_add(CONTEXT_RADIUS + 1)
        .min(hunk.lines.len());
    let mut digest = Sha256::new();
    for line in &hunk.lines[start..end] {
        update_part(&mut digest, &line.kind);
        update_part(&mut digest, line.content.trim_end());
    }
    digest_hex(digest)
}

pub(crate) fn fingerprint_snapshot(files: &[ReviewFileSummary]) -> String {
    let mut ordered = files.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.path.cmp(&right.path));
    let mut digest = Sha256::new();
    for file in ordered {
        update_part(&mut digest, &file.path);
        update_part(&mut digest, file.previous_path.as_deref().unwrap_or(""));
        update_part(&mut digest, &file.change_type);
        update_part(&mut digest, file.old_hash.as_deref().unwrap_or(""));
        update_part(&mut digest, file.new_hash.as_deref().unwrap_or(""));
        update_part(&mut digest, if file.binary { "binary" } else { "text" });
        update_part(
            &mut digest,
            if file.oversized {
                "oversized"
            } else {
                "bounded"
            },
        );
    }
    digest_hex(digest)
}

fn update_part(digest: &mut Sha256, value: &str) {
    digest.update(value.len().to_le_bytes());
    digest.update(value.as_bytes());
}

fn digest_hex(digest: Sha256) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = digest.finalize();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::application::GitDiffLine;

    fn hunk() -> GitDiffHunk {
        GitDiffHunk {
            header: "@@ -1,2 +1,2 @@".into(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 2,
            lines: vec![
                GitDiffLine {
                    kind: "deletion".into(),
                    content: "old".into(),
                    old_line_number: Some(1),
                    new_line_number: None,
                },
                GitDiffLine {
                    kind: "addition".into(),
                    content: "new".into(),
                    old_line_number: None,
                    new_line_number: Some(1),
                },
            ],
        }
    }

    fn file(path: &str) -> ReviewFileSummary {
        ReviewFileSummary {
            path: path.into(),
            previous_path: None,
            change_type: "modified".into(),
            old_hash: Some("old".into()),
            new_hash: Some("new".into()),
            binary: false,
            oversized: false,
        }
    }

    #[test]
    fn fingerprints_are_deterministic_and_content_sensitive() {
        let first = hunk();
        let mut second = hunk();
        assert_eq!(fingerprint_hunk(&first), fingerprint_hunk(&second));
        second.lines[1].content = "different".into();
        assert_ne!(fingerprint_hunk(&first), fingerprint_hunk(&second));
        assert_eq!(
            fingerprint_context(&first, 0),
            fingerprint_context(&first, 1)
        );
    }

    #[test]
    fn snapshot_fingerprint_is_order_independent() {
        assert_eq!(
            fingerprint_snapshot(&[file("b"), file("a")]),
            fingerprint_snapshot(&[file("a"), file("b")])
        );
    }

    #[test]
    fn maximum_fixture_visits_each_accepted_line_once() {
        let mut fixture = hunk();
        fixture.lines = (0..20_000)
            .map(|index| GitDiffLine {
                kind: if index % 2 == 0 {
                    "addition"
                } else {
                    "deletion"
                }
                .into(),
                content: format!("bounded-line-{index}"),
                old_line_number: None,
                new_line_number: Some(index + 1),
            })
            .collect();
        let (_fingerprint, visited, accepted_bytes) = measured_hunk_fingerprint(&fixture);
        assert_eq!(visited, fixture.lines.len());
        assert_eq!(
            accepted_bytes,
            fixture
                .lines
                .iter()
                .map(|line| line.kind.len() + line.content.len())
                .sum::<usize>()
        );
    }
}
