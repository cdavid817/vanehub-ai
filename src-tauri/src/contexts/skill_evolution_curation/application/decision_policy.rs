use crate::contexts::skill_evolution_curation::domain::*;
use sha2::{Digest, Sha256};

pub(super) fn validate_actor(actor: CuratorTrustedActor) -> Result<(), &'static str> {
    if actor.occurred_at_ms() < 0 || !actor.is_interactive() {
        return Err("interactive_actor_required");
    }
    Ok(())
}

pub(super) fn validate_action_key(value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err("decision_idempotency_key_invalid");
    }
    Ok(())
}

pub(super) fn note_hash(note: Option<&str>) -> Result<Option<String>, &'static str> {
    let Some(note) = note else {
        return Ok(None);
    };
    let normalized = note.trim();
    if normalized.is_empty()
        || normalized.chars().count() > CURATOR_MAX_DECISION_NOTE_CHARACTERS
        || normalized
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Err("decision_note_invalid");
    }
    let digest = Sha256::digest(normalized.as_bytes());
    Ok(Some(format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )))
}

pub(super) fn validate_defer_time(
    now_ms: i64,
    review_after_ms: Option<i64>,
    maximum_defer_days: u16,
) -> Result<(), &'static str> {
    let Some(review_after_ms) = review_after_ms else {
        return Ok(());
    };
    let minimum = now_ms
        .checked_add(CURATOR_MIN_DEFER_MS)
        .ok_or("defer_time_invalid")?;
    if !(1..=180).contains(&maximum_defer_days) {
        return Err("defer_policy_invalid");
    }
    let maximum = now_ms
        .checked_add(i64::from(maximum_defer_days) * CURATOR_MIN_DEFER_MS)
        .ok_or("defer_time_invalid")?;
    if !(minimum..=maximum).contains(&review_after_ms) {
        return Err("defer_time_out_of_range");
    }
    Ok(())
}

pub(super) fn validate_resume(
    request: CuratorResumeRequest<'_>,
    binding: &CuratorDecisionBinding,
) -> Result<CuratorTransition, &'static str> {
    if binding.state != CuratorCandidateState::Deferred
        || request.expected_candidate_revision != binding.candidate_revision
        || request.expected_candidate_hash != binding.candidate_hash
        || request.expected_policy_hash != binding.policy_hash
        || !binding.staleness.is_empty()
    {
        return Err("resume_witness_mismatch");
    }
    match (&binding.ready_draft, request.expected_draft_revision) {
        (None, None) if request.expected_assessment_id.is_none() => {
            Ok(CuratorTransition::ResumeWithoutReadyDraft)
        }
        (Some(ready), Some(revision))
            if revision == ready.draft_revision
                && request.expected_assessment_id == Some(ready.assessment_id.as_str()) =>
        {
            Ok(CuratorTransition::ResumeWithReadyDraft)
        }
        _ => Err("resume_draft_witness_mismatch"),
    }
}

pub(super) fn validate_approval<'a>(
    request: CuratorApprovalRequest<'_>,
    binding: &'a CuratorDecisionBinding,
    now_ms: i64,
) -> Result<&'a CuratorApprovalPreviewWitness, &'static str> {
    if binding.state != CuratorCandidateState::ReadyForReview
        || request.expected_candidate_revision != binding.candidate_revision
        || !binding.staleness.is_empty()
    {
        return Err("approval_candidate_stale");
    }
    let preview = binding
        .current_preview
        .as_ref()
        .ok_or("approval_preview_missing")?;
    if now_ms < preview.issued_at_ms || now_ms >= preview.expires_at_ms {
        return Err("approval_preview_expired");
    }
    if request.confirmed_preview_hash != preview.witness_hash
        || request.confirmed_effective_diff_hash != preview.effective_diff_hash
    {
        return Err("approval_preview_mismatch");
    }
    if !preview.diffs_complete || !preview.validation_complete {
        return Err("approval_preview_incomplete");
    }
    Ok(preview)
}

pub(super) fn rejection_reason(value: CuratorRejectionReason) -> &'static str {
    match value {
        CuratorRejectionReason::IncorrectTarget => "incorrect_target",
        CuratorRejectionReason::UnsupportedLesson => "unsupported_lesson",
        CuratorRejectionReason::Duplicate => "duplicate",
        CuratorRejectionReason::TooRisky => "too_risky",
        CuratorRejectionReason::NotUseful => "not_useful",
        CuratorRejectionReason::Other => "other",
    }
}

pub(super) fn defer_reason(value: CuratorDeferReason) -> &'static str {
    match value {
        CuratorDeferReason::NeedMoreEvidence => "need_more_evidence",
        CuratorDeferReason::NeedExpertReview => "need_expert_review",
        CuratorDeferReason::WaitingForChange => "waiting_for_change",
        CuratorDeferReason::LowerPriority => "lower_priority",
        CuratorDeferReason::Other => "other",
    }
}
