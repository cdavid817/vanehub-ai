use crate::commands::error::CommandError;
use crate::contexts::sessions::application::{ReviewSummary, ReviewView};
use crate::contexts::sessions::domain::{
    ReviewAnchor, ReviewAnchorState, ReviewComment, ReviewCommentStatus, ReviewDecision,
    ReviewFile, ReviewFileViewState, ReviewFinding, ReviewFindingSeverity, ReviewHunkDecision,
    ReviewStatus,
};
use crate::contexts::workspaces::application::{ReviewDiffFile, ReviewPatch, ReviewRevertReceipt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewAnchorInput {
    pub(crate) file_path: String,
    pub(crate) side: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) hunk_fingerprint: String,
    pub(crate) context_fingerprint: String,
}

impl ReviewAnchorInput {
    pub(crate) fn into_domain(self) -> Result<ReviewAnchor, String> {
        ReviewAnchor::try_new(
            self.file_path,
            self.side,
            self.start_line,
            self.end_line,
            self.hunk_fingerprint,
            self.context_fingerprint,
        )
        .map_err(|error| format!("{error:?}"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewAnchorDto {
    file_path: String,
    side: String,
    start_line: u32,
    end_line: u32,
    hunk_fingerprint: String,
    context_fingerprint: String,
    state: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewFileDto {
    path: String,
    previous_path: Option<String>,
    change_type: String,
    old_hash: Option<String>,
    new_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewCommentDto {
    id: String,
    anchor: ReviewAnchorDto,
    body: String,
    status: &'static str,
    selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewFindingDto {
    id: String,
    source: String,
    title: String,
    severity: &'static str,
    anchor: Option<ReviewAnchorDto>,
    operation_id: String,
    resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSessionDto {
    id: String,
    session_id: String,
    workspace_id: String,
    base_revision: Option<String>,
    head_revision: Option<String>,
    fingerprint: String,
    status: &'static str,
    decision: &'static str,
    created_at: String,
    updated_at: String,
    files: Vec<ReviewFileDto>,
    comments: Vec<ReviewCommentDto>,
    findings: Vec<ReviewFindingDto>,
    summary: ReviewSummaryDto,
}

/// The header's four numbers.
///
/// `viewedFiles` is the one a caller cannot work out for itself: the marks live in a store the
/// review does not carry, and whether a mark still applies depends on comparing its witness with
/// the file's current one. The other three are derivable from the arrays beside them, and are here
/// anyway so the header reads one shape rather than folding two lists on every render.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSummaryDto {
    changed_files: usize,
    viewed_files: usize,
    unresolved_comments: usize,
    unresolved_findings: usize,
}

impl From<ReviewSummary> for ReviewSummaryDto {
    fn from(summary: ReviewSummary) -> Self {
        Self {
            changed_files: summary.changed_files,
            viewed_files: summary.viewed_files,
            unresolved_comments: summary.unresolved_comments,
            unresolved_findings: summary.unresolved_findings,
        }
    }
}

/// Built from a view and nothing else.
///
/// There is deliberately no conversion from a bare `ReviewSession`. One would have to invent a
/// viewed count, and the only value available to invent is zero — which reads as "you have read
/// none of these files" rather than as "this path could not find out". Requiring the view means no
/// caller can produce that sentence by accident.
impl From<ReviewView> for ReviewSessionDto {
    fn from(view: ReviewView) -> Self {
        let review = view.session;
        let files = review.files().iter().cloned().map(Into::into).collect();
        let comments = review.comments().iter().cloned().map(Into::into).collect();
        let findings = review.findings().iter().cloned().map(Into::into).collect();
        Self {
            id: review.id,
            session_id: review.session_id,
            workspace_id: review.workspace_id,
            base_revision: review.base_revision,
            head_revision: review.head_revision,
            fingerprint: review.fingerprint,
            status: status(review.status),
            decision: decision(review.decision),
            created_at: review.created_at,
            updated_at: review.updated_at,
            files,
            comments,
            findings,
            summary: view.summary.into(),
        }
    }
}

impl From<ReviewFile> for ReviewFileDto {
    fn from(file: ReviewFile) -> Self {
        Self {
            path: file.path,
            previous_path: file.previous_path,
            change_type: file.change_type,
            old_hash: file.old_hash,
            new_hash: file.new_hash,
        }
    }
}

impl From<ReviewComment> for ReviewCommentDto {
    fn from(comment: ReviewComment) -> Self {
        Self {
            id: comment.id,
            anchor: comment.anchor.into(),
            body: comment.body,
            status: match comment.status {
                ReviewCommentStatus::Active => "active",
                ReviewCommentStatus::Resolved => "resolved",
            },
            selected: comment.selected,
        }
    }
}

impl From<ReviewFinding> for ReviewFindingDto {
    fn from(finding: ReviewFinding) -> Self {
        Self {
            id: finding.id,
            source: finding.source,
            title: finding.title,
            severity: match finding.severity {
                ReviewFindingSeverity::Info => "info",
                ReviewFindingSeverity::Warning => "warning",
                ReviewFindingSeverity::Error => "error",
            },
            anchor: finding.anchor.map(Into::into),
            operation_id: finding.operation_id,
            resolved: finding.resolved,
        }
    }
}

impl From<ReviewAnchor> for ReviewAnchorDto {
    fn from(anchor: ReviewAnchor) -> Self {
        Self {
            file_path: anchor.file_path,
            side: anchor.side,
            start_line: anchor.start_line,
            end_line: anchor.end_line,
            hunk_fingerprint: anchor.hunk_fingerprint,
            context_fingerprint: anchor.context_fingerprint,
            state: match anchor.state {
                ReviewAnchorState::Current => "current",
                ReviewAnchorState::Relocated => "relocated",
                ReviewAnchorState::Stale => "stale",
            },
        }
    }
}

fn status(value: ReviewStatus) -> &'static str {
    match value {
        ReviewStatus::Active => "active",
        ReviewStatus::Completed => "completed",
    }
}

/// The three values the wire carries, refused rather than defaulted.
///
/// A decision this binary does not know is not a fourth state to store; it is a caller sending
/// something else. Defaulting it to `pending` would record "nobody has decided" for a request
/// somebody deliberately made.
pub(crate) fn parse_review_decision(value: &str) -> Result<ReviewDecision, CommandError> {
    match value {
        "pending" => Ok(ReviewDecision::Pending),
        "accepted" => Ok(ReviewDecision::Accepted),
        "changes-requested" => Ok(ReviewDecision::ChangesRequested),
        _ => Err(CommandError::validation("invalid review decision")),
    }
}

/// What was recorded, echoed back so a caller does not have to assume it landed as sent.
///
/// `simulated` is false here and true in the Web adapter's fixture. The field exists so a reader
/// can tell a decision that reached a store from one that lives in a demo's memory — same shape,
/// different weight, and a UI that could not tell them apart would present both as recorded.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewHunkDecisionReceiptDto {
    review_id: String,
    relative_path: String,
    hunk_fingerprint: String,
    decision: &'static str,
    simulated: bool,
}

impl ReviewHunkDecisionReceiptDto {
    pub(crate) fn recorded(review_id: String, recorded: ReviewHunkDecision) -> Self {
        Self {
            review_id,
            relative_path: recorded.path,
            hunk_fingerprint: recorded.hunk_fingerprint,
            decision: decision(recorded.decision),
            simulated: false,
        }
    }
}

/// What was recorded about a file being read.
///
/// The witness is echoed so a caller can tell a mark that survived a refresh from one that was
/// re-made: same path, different witness means the file changed and the mark is about the new one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewFileViewedReceiptDto {
    review_id: String,
    relative_path: String,
    file_witness: String,
    viewed: bool,
    /// Absent when the file is not viewed, because there is no moment at which it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    viewed_at: Option<String>,
    simulated: bool,
}

impl ReviewFileViewedReceiptDto {
    pub(crate) fn recorded(review_id: String, state: ReviewFileViewState) -> Self {
        Self {
            review_id,
            relative_path: state.path,
            file_witness: state.file_witness,
            viewed: state.viewed,
            viewed_at: state.viewed_at,
            simulated: false,
        }
    }
}

fn decision(value: ReviewDecision) -> &'static str {
    match value {
        ReviewDecision::Pending => "pending",
        ReviewDecision::Accepted => "accepted",
        ReviewDecision::ChangesRequested => "changes-requested",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewDiffFileDto {
    path: String,
    change_type: String,
    binary: bool,
    oversized: bool,
    truncated: bool,
    accepted_bytes: usize,
    hunks: Vec<ReviewDiffHunkDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewDiffHunkDto {
    fingerprint: String,
    context_fingerprints: Vec<String>,
    header: String,
    old_start: usize,
    old_lines: usize,
    new_start: usize,
    new_lines: usize,
    lines: Vec<ReviewDiffLineDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewDiffLineDto {
    kind: String,
    content: String,
    old_line_number: Option<usize>,
    new_line_number: Option<usize>,
}

impl From<ReviewDiffFile> for ReviewDiffFileDto {
    fn from(file: ReviewDiffFile) -> Self {
        Self {
            path: file.summary.path,
            change_type: file.summary.change_type,
            binary: file.summary.binary,
            oversized: file.summary.oversized,
            truncated: file.truncated,
            accepted_bytes: file.accepted_bytes,
            hunks: file
                .hunks
                .into_iter()
                .map(|entry| ReviewDiffHunkDto {
                    fingerprint: entry.fingerprint,
                    context_fingerprints: entry.context_fingerprints,
                    header: entry.hunk.header,
                    old_start: entry.hunk.old_start,
                    old_lines: entry.hunk.old_lines,
                    new_start: entry.hunk.new_start,
                    new_lines: entry.hunk.new_lines,
                    lines: entry
                        .hunk
                        .lines
                        .into_iter()
                        .map(|line| ReviewDiffLineDto {
                            kind: line.kind,
                            content: line.content,
                            old_line_number: line.old_line_number,
                            new_line_number: line.new_line_number,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// A patch and the diff it came from.
///
/// Two identities, because they answer different questions. `snapshot` says which diff the patch
/// came from; `fingerprint` is over the patch bytes, so two renders of the same selection can be
/// recognised as the same copy. A reviewer can hold a patch on a clipboard for as long as they
/// like, and neither question is answerable from the text alone.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewPatchDto {
    path: String,
    snapshot: String,
    fingerprint: String,
    hunks: usize,
    patch: String,
}

impl From<ReviewPatch> for ReviewPatchDto {
    fn from(rendered: ReviewPatch) -> Self {
        Self {
            path: rendered.path,
            snapshot: rendered.snapshot,
            fingerprint: rendered.fingerprint,
            hunks: rendered.hunks,
            patch: rendered.patch,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewRevertReceiptDto {
    path: String,
    previous_snapshot: String,
    resulting_snapshot: String,
    reverted_hunks: usize,
    pub(crate) simulated: bool,
}

impl From<ReviewRevertReceipt> for ReviewRevertReceiptDto {
    fn from(receipt: ReviewRevertReceipt) -> Self {
        Self {
            path: receipt.path,
            previous_snapshot: receipt.previous_snapshot,
            resulting_snapshot: receipt.resulting_snapshot,
            reverted_hunks: receipt.reverted_hunks,
            simulated: false,
        }
    }
}
