//! Pure decisions about a deletion request: what is a valid selection, what a choice may ask
//! for, how a request is fingerprinted for idempotency, and how group results add up.

use super::models::{
    error_code, DeletionGroupResult, DeletionGroupStatus, DeletionOutcome, DeletionPreviewWorktree,
    WorktreeDeletionChoice, WorktreeDeletionPolicy, MAX_DELETION_BATCH,
};
use crate::contexts::sessions::application::SessionsApplicationError;
use crate::contexts::sessions::domain::SessionId;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Trims, deduplicates and bounds a selection. A system-activity id is refused by the domain
/// parser, so nothing downstream ever sees one.
pub(crate) fn normalize_selection(
    session_ids: &[String],
) -> Result<Vec<String>, SessionsApplicationError> {
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();
    for raw in session_ids {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = SessionId::parse(trimmed)?;
        if seen.insert(parsed.as_str().to_string()) {
            ordered.push(parsed.as_str().to_string());
        }
    }
    if ordered.is_empty() {
        return Err(SessionsApplicationError::Validation(
            error_code::EMPTY_SELECTION.to_string(),
        ));
    }
    if ordered.len() > MAX_DELETION_BATCH {
        return Err(SessionsApplicationError::Validation(
            error_code::BATCH_TOO_LARGE.to_string(),
        ));
    }
    Ok(ordered)
}

/// The policy each preview worktree ends up with once the user's choices are applied.
///
/// A worktree with no choice keeps its directory. Every choice must name a row in the preview,
/// name it once, ask for something the preview allowed, and carry the acknowledgement the
/// preview said was required — bound to the fingerprint the preview computed, not to any
/// fingerprint the client invents.
pub(crate) fn resolve_choices(
    worktrees: &[DeletionPreviewWorktree],
    choices: &[WorktreeDeletionChoice],
) -> Result<Vec<(String, WorktreeDeletionPolicy, Option<String>)>, SessionsApplicationError> {
    let mut seen = BTreeSet::new();
    for choice in choices {
        if !seen.insert(choice.worktree_key.clone()) {
            return Err(SessionsApplicationError::Validation(
                error_code::DUPLICATE_WORKTREE_CHOICE.to_string(),
            ));
        }
        if !worktrees
            .iter()
            .any(|worktree| worktree.worktree_key == choice.worktree_key)
        {
            return Err(SessionsApplicationError::Validation(
                error_code::UNKNOWN_WORKTREE_CHOICE.to_string(),
            ));
        }
    }
    let mut resolved = Vec::new();
    for worktree in worktrees {
        let choice = choices
            .iter()
            .find(|choice| choice.worktree_key == worktree.worktree_key);
        let policy = choice
            .map(|choice| choice.policy)
            .unwrap_or(WorktreeDeletionPolicy::Keep);
        if !worktree.allowed_policies.contains(&policy) {
            return Err(SessionsApplicationError::Validation(
                error_code::POLICY_NOT_ALLOWED.to_string(),
            ));
        }
        let mut acknowledged = None;
        if policy == WorktreeDeletionPolicy::RemoveSafe {
            if worktree.worktree_id.is_none() {
                return Err(SessionsApplicationError::Validation(
                    error_code::POLICY_NOT_ALLOWED.to_string(),
                ));
            }
            if worktree.requires_ignored_acknowledgement {
                let expected = worktree
                    .ignored
                    .as_ref()
                    .map(|ignored| ignored.fingerprint.as_str());
                let supplied = choice
                    .and_then(|choice| choice.ignored_files_acknowledgement.as_ref())
                    .map(|ack| ack.fingerprint.as_str());
                match (expected, supplied) {
                    (Some(expected), Some(supplied)) if expected == supplied => {
                        acknowledged = Some(expected.to_string());
                    }
                    (_, None) => {
                        return Err(SessionsApplicationError::Validation(
                            error_code::IGNORED_ACKNOWLEDGEMENT_REQUIRED.to_string(),
                        ))
                    }
                    _ => {
                        return Err(SessionsApplicationError::Validation(
                            error_code::IGNORED_ACKNOWLEDGEMENT_STALE.to_string(),
                        ))
                    }
                }
            }
        }
        resolved.push((worktree.worktree_key.clone(), policy, acknowledged));
    }
    Ok(resolved)
}

/// One fingerprint for "the same request". Order-insensitive over sessions and choices so a
/// retransmission that happened to reorder its JSON still matches, and sensitive to every
/// element that changes what would be done.
pub(crate) fn request_hash(
    session_ids: &[String],
    resolved: &[(String, WorktreeDeletionPolicy, Option<String>)],
) -> String {
    let mut sessions: Vec<&str> = session_ids.iter().map(String::as_str).collect();
    sessions.sort_unstable();
    let mut choices: Vec<String> = resolved
        .iter()
        .map(|(key, policy, ack)| {
            format!(
                "{key}\u{1}{}\u{1}{}",
                policy.as_str(),
                ack.as_deref().unwrap_or("")
            )
        })
        .collect();
    choices.sort_unstable();
    let mut digest = Sha256::new();
    for session in sessions {
        digest.update(session.as_bytes());
        digest.update([0]);
    }
    digest.update([1]);
    for choice in choices {
        digest.update(choice.as_bytes());
        digest.update([0]);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// How group results add up. Never `Succeeded` while any group is anything else, and never a
/// plain `Failed` when part of the work is done — that is `Partial`, and the completed part is
/// kept.
pub(crate) fn aggregate_outcome(groups: &[DeletionGroupResult]) -> DeletionOutcome {
    if groups.is_empty() {
        return DeletionOutcome::Failed;
    }
    let mut succeeded = 0usize;
    let mut pending = 0usize;
    let mut attention = 0usize;
    let mut awaiting = 0usize;
    let mut failed = 0usize;
    for group in groups {
        match group.status {
            DeletionGroupStatus::Succeeded => succeeded += 1,
            DeletionGroupStatus::Pending | DeletionGroupStatus::Running => pending += 1,
            DeletionGroupStatus::FinalizePending | DeletionGroupStatus::NeedsAttention => {
                attention += 1
            }
            DeletionGroupStatus::AwaitingDecision => awaiting += 1,
            DeletionGroupStatus::Failed => failed += 1,
        }
    }
    if pending > 0 {
        return DeletionOutcome::Pending;
    }
    if attention > 0 {
        return DeletionOutcome::NeedsAttention;
    }
    if succeeded == groups.len() {
        return DeletionOutcome::Succeeded;
    }
    if succeeded > 0 {
        return DeletionOutcome::Partial;
    }
    if awaiting > 0 {
        return DeletionOutcome::AwaitingDecision;
    }
    debug_assert!(failed > 0);
    DeletionOutcome::Failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::sessions::application::deletion::models::{
        DeletionCheckCompleteness, DeletionIgnoredSummary, DeletionPhase,
        IgnoredFilesAcknowledgement, SessionDbEffect, WorktreeEffect,
    };

    fn worktree(
        key: &str,
        verified: bool,
        allow_remove: bool,
        ack: Option<&str>,
    ) -> DeletionPreviewWorktree {
        DeletionPreviewWorktree {
            worktree_key: key.to_string(),
            worktree_id: verified.then(|| key.to_string()),
            display_path: "/x".to_string(),
            branch: None,
            session_ids: vec!["s1".to_string()],
            external_references: Vec::new(),
            allowed_policies: if allow_remove {
                vec![
                    WorktreeDeletionPolicy::Keep,
                    WorktreeDeletionPolicy::RemoveSafe,
                ]
            } else {
                vec![WorktreeDeletionPolicy::Keep]
            },
            blockers: Vec::new(),
            checks: DeletionCheckCompleteness::Complete,
            changes: None,
            ignored: ack.map(|fingerprint| DeletionIgnoredSummary {
                total_entries: 1,
                samples: Vec::new(),
                samples_truncated: false,
                completeness: DeletionCheckCompleteness::Complete,
                fingerprint: fingerprint.to_string(),
            }),
            requires_ignored_acknowledgement: ack.is_some(),
            origin: "ordinary_session".to_string(),
            provenance: "verified".to_string(),
            resource_status: Some("attached".to_string()),
        }
    }

    fn choice(
        key: &str,
        policy: WorktreeDeletionPolicy,
        ack: Option<&str>,
    ) -> WorktreeDeletionChoice {
        WorktreeDeletionChoice {
            worktree_key: key.to_string(),
            policy,
            ignored_files_acknowledgement: ack.map(|fingerprint| IgnoredFilesAcknowledgement {
                fingerprint: fingerprint.to_string(),
            }),
        }
    }

    #[test]
    fn selections_are_trimmed_deduplicated_bounded_and_refuse_system_sessions() {
        let ids = vec![
            " a ".to_string(),
            "b".to_string(),
            "a".to_string(),
            "".to_string(),
        ];
        assert_eq!(normalize_selection(&ids).expect("ids"), vec!["a", "b"]);
        assert!(matches!(
            normalize_selection(&[]),
            Err(SessionsApplicationError::Validation(code)) if code == error_code::EMPTY_SELECTION
        ));
        let many: Vec<String> = (0..=MAX_DELETION_BATCH)
            .map(|index| format!("s{index}"))
            .collect();
        assert!(matches!(
            normalize_selection(&many),
            Err(SessionsApplicationError::Validation(code)) if code == error_code::BATCH_TOO_LARGE
        ));
        assert!(matches!(
            normalize_selection(&["system-activity-v1-abc".to_string()]),
            Err(SessionsApplicationError::Domain(_))
        ));
    }

    #[test]
    fn choices_default_to_keep_and_reject_unknown_duplicate_or_disallowed_targets() {
        let worktrees = vec![
            worktree("w1", true, true, None),
            worktree("w2", false, false, None),
        ];
        let resolved = resolve_choices(&worktrees, &[]).expect("defaults");
        assert!(resolved
            .iter()
            .all(|(_, policy, _)| *policy == WorktreeDeletionPolicy::Keep));

        let unknown = [choice("w9", WorktreeDeletionPolicy::Keep, None)];
        assert!(
            matches!(resolve_choices(&worktrees, &unknown), Err(SessionsApplicationError::Validation(code)) if code == error_code::UNKNOWN_WORKTREE_CHOICE)
        );

        let duplicate = [
            choice("w1", WorktreeDeletionPolicy::Keep, None),
            choice("w1", WorktreeDeletionPolicy::RemoveSafe, None),
        ];
        assert!(
            matches!(resolve_choices(&worktrees, &duplicate), Err(SessionsApplicationError::Validation(code)) if code == error_code::DUPLICATE_WORKTREE_CHOICE)
        );

        let disallowed = [choice("w2", WorktreeDeletionPolicy::RemoveSafe, None)];
        assert!(
            matches!(resolve_choices(&worktrees, &disallowed), Err(SessionsApplicationError::Validation(code)) if code == error_code::POLICY_NOT_ALLOWED)
        );

        let allowed = [choice("w1", WorktreeDeletionPolicy::RemoveSafe, None)];
        let resolved = resolve_choices(&worktrees, &allowed).expect("allowed");
        assert_eq!(resolved[0].1, WorktreeDeletionPolicy::RemoveSafe);
    }

    #[test]
    fn ignored_acknowledgements_bind_to_the_preview_fingerprint() {
        let worktrees = vec![worktree("w1", true, true, Some("fp-1"))];
        let missing = [choice("w1", WorktreeDeletionPolicy::RemoveSafe, None)];
        assert!(
            matches!(resolve_choices(&worktrees, &missing), Err(SessionsApplicationError::Validation(code)) if code == error_code::IGNORED_ACKNOWLEDGEMENT_REQUIRED)
        );
        let stale = [choice(
            "w1",
            WorktreeDeletionPolicy::RemoveSafe,
            Some("fp-0"),
        )];
        assert!(
            matches!(resolve_choices(&worktrees, &stale), Err(SessionsApplicationError::Validation(code)) if code == error_code::IGNORED_ACKNOWLEDGEMENT_STALE)
        );
        let current = [choice(
            "w1",
            WorktreeDeletionPolicy::RemoveSafe,
            Some("fp-1"),
        )];
        let resolved = resolve_choices(&worktrees, &current).expect("acknowledged");
        assert_eq!(resolved[0].2.as_deref(), Some("fp-1"));
        // Keeping never needs an acknowledgement.
        let keep = [choice("w1", WorktreeDeletionPolicy::Keep, None)];
        assert!(resolve_choices(&worktrees, &keep).is_ok());
    }

    #[test]
    fn request_hashes_ignore_order_and_notice_content() {
        let a = request_hash(
            &["s1".to_string(), "s2".to_string()],
            &[("w1".to_string(), WorktreeDeletionPolicy::Keep, None)],
        );
        let b = request_hash(
            &["s2".to_string(), "s1".to_string()],
            &[("w1".to_string(), WorktreeDeletionPolicy::Keep, None)],
        );
        assert_eq!(a, b);
        let c = request_hash(
            &["s1".to_string(), "s2".to_string()],
            &[("w1".to_string(), WorktreeDeletionPolicy::RemoveSafe, None)],
        );
        assert_ne!(a, c);
        let d = request_hash(
            &["s1".to_string()],
            &[("w1".to_string(), WorktreeDeletionPolicy::Keep, None)],
        );
        assert_ne!(a, d);
    }

    fn group(status: DeletionGroupStatus) -> DeletionGroupResult {
        DeletionGroupResult {
            group_id: "g".to_string(),
            worktree_key: None,
            worktree_id: None,
            policy: WorktreeDeletionPolicy::Keep,
            session_ids: vec!["s".to_string()],
            status,
            phase: DeletionPhase::Completed,
            worktree_effect: WorktreeEffect::NotRequested,
            db_effect: SessionDbEffect::Pending,
            error_code: None,
            retained_path: None,
            attempt: 1,
            revision: 1,
        }
    }

    #[test]
    fn aggregation_never_reports_success_while_anything_is_unfinished_or_partial() {
        use DeletionGroupStatus as S;
        assert_eq!(
            aggregate_outcome(&[group(S::Succeeded), group(S::Succeeded)]),
            DeletionOutcome::Succeeded
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Succeeded), group(S::Failed)]),
            DeletionOutcome::Partial
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Succeeded), group(S::AwaitingDecision)]),
            DeletionOutcome::Partial
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Failed), group(S::AwaitingDecision)]),
            DeletionOutcome::AwaitingDecision
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Failed)]),
            DeletionOutcome::Failed
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Succeeded), group(S::FinalizePending)]),
            DeletionOutcome::NeedsAttention
        );
        assert_eq!(
            aggregate_outcome(&[group(S::NeedsAttention), group(S::Failed)]),
            DeletionOutcome::NeedsAttention
        );
        assert_eq!(
            aggregate_outcome(&[group(S::Succeeded), group(S::Running)]),
            DeletionOutcome::Pending
        );
        assert_eq!(aggregate_outcome(&[]), DeletionOutcome::Failed);
    }
}
