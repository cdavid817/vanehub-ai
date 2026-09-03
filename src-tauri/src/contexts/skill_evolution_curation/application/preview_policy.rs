use crate::contexts::skill_evolution_curation::domain::*;

pub(crate) fn validate_preview_receipt(
    binding: &CuratorPreviewBinding,
    receipt: &CuratorOverlayPreviewReceipt,
) -> Result<(), &'static str> {
    let witnesses = &receipt.witnesses;
    if witnesses.candidate_hash != binding.candidate_hash
        || witnesses.draft_hash != binding.draft_hash
        || witnesses.assessment_hash != binding.assessment_hash
        || witnesses.target_revision != binding.target_revision
        || witnesses.base_instruction_hash != binding.base_instruction_hash
        || witnesses.base_package_hash != binding.base_package_hash
        || witnesses.current_effective_hash != binding.current_effective_hash
        || witnesses.overlay_revision != binding.overlay_revision
        || witnesses.pin_witness != binding.pin_witness
        || witnesses.trust_witness != binding.trust_witness
        || witnesses.conflict_witness != binding.conflict_witness
        || witnesses.policy_hash != binding.policy_hash
    {
        return Err("preview_witness_mismatch");
    }
    if !receipt.validation.scan_passed
        || !receipt.validation.can_commit
        || receipt.validation.pinned
        || !receipt.validation.trusted
        || receipt.validation.conflict_count != 0
        || witnesses.scanner_version.trim().is_empty()
    {
        return Err("preview_not_committable");
    }
    validate_diff(
        &receipt.diffs.base_to_current,
        &witnesses.base_instruction_hash,
        &witnesses.current_effective_hash,
    )?;
    validate_diff(
        &receipt.diffs.current_to_proposed,
        &witnesses.current_effective_hash,
        &witnesses.proposed_effective_hash,
    )?;
    validate_diff(
        &receipt.diffs.base_to_proposed,
        &witnesses.base_instruction_hash,
        &witnesses.proposed_effective_hash,
    )
}

pub(crate) fn page_diff(
    projection: &CuratorDiffProjection,
    cursor: Option<usize>,
    limit: usize,
) -> Result<CuratorDiffPage, &'static str> {
    if limit == 0 || limit > 100 {
        return Err("preview_page_limit_invalid");
    }
    let offset = cursor.unwrap_or(0);
    if offset > projection.hunks.len() {
        return Err("preview_page_cursor_invalid");
    }
    let end = offset.saturating_add(limit).min(projection.hunks.len());
    Ok(CuratorDiffPage {
        hunks: projection.hunks[offset..end].to_vec(),
        next_cursor: (end < projection.hunks.len()).then_some(end),
        complete: projection.complete && end == projection.hunks.len(),
    })
}

fn validate_diff(
    diff: &CuratorDiffProjection,
    expected_from: &str,
    expected_to: &str,
) -> Result<(), &'static str> {
    if diff.from_hash != expected_from
        || diff.to_hash != expected_to
        || diff.hunks.len() > 100
        || diff.hunks.iter().any(|hunk| {
            hunk.label.len() > 160
                || hunk.before.content.chars().count() > hunk.before.total_characters
                || hunk.after.content.chars().count() > hunk.after.total_characters
        })
    {
        return Err("preview_diff_projection_invalid");
    }
    Ok(())
}
