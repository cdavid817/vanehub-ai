use crate::contexts::skill_evolution_generation::domain::DossierTruncationV1;

use super::AuthoritativeDossierSnapshotV1;

pub(crate) const DOSSIER_CANONICAL_SIZE_LIMIT_V1: u32 = 128 * 1024;
const SIGNAL_LIMIT_V1: usize = 100;
const TARGET_QUALITY_LIMIT_V1: usize = 32;
const EXCERPT_BYTES_LIMIT_V1: usize = 8 * 1024;
const TIMELINE_LIMIT_V1: usize = 1_000;
const SELECTION_POLICY_V1: &str = "stable_category_time_id_v1";

pub(super) struct BoundedDossierSnapshotV1 {
    pub(super) snapshot: AuthoritativeDossierSnapshotV1,
    pub(super) truncations: [DossierTruncationV1; 13],
}

pub(super) fn bounded_snapshot(
    source: &AuthoritativeDossierSnapshotV1,
) -> BoundedDossierSnapshotV1 {
    let mut snapshot = source.clone();
    snapshot.signals.sort_by(|left, right| {
        (&left.category, left.occurred_at_ms, &left.signal_id).cmp(&(
            &right.category,
            right.occurred_at_ms,
            &right.signal_id,
        ))
    });
    snapshot.targets.sort_by(|left, right| {
        (&left.skill_id, &left.revision).cmp(&(&right.skill_id, &right.revision))
    });
    snapshot.quality_checks.sort_by(|left, right| {
        (&left.code, &left.result, &left.reason_code).cmp(&(
            &right.code,
            &right.result,
            &right.reason_code,
        ))
    });
    snapshot.guidance.excerpts.sort_by(|left, right| {
        (&left.logical_location, &left.excerpt_id)
            .cmp(&(&right.logical_location, &right.excerpt_id))
    });
    snapshot.guidance.resources.sort_by(|left, right| {
        (&left.resource_kind, &left.resource_id, &left.revision).cmp(&(
            &right.resource_kind,
            &right.resource_id,
            &right.revision,
        ))
    });
    snapshot.timeline.sort_by(|left, right| {
        (&left.event_code, left.occurred_at_ms).cmp(&(&right.event_code, right.occurred_at_ms))
    });
    snapshot
        .privacy_classes
        .sort_by(|left, right| left.class_code.cmp(&right.class_code));
    snapshot
        .rationale
        .sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
    snapshot
        .verification
        .sort_by(|left, right| left.step_id.cmp(&right.step_id));
    snapshot.lineage.sort_by(|left, right| {
        (&left.source_kind, &left.source_id, &left.revision).cmp(&(
            &right.source_kind,
            &right.source_id,
            &right.revision,
        ))
    });

    let signal_total = snapshot.signals.len();
    snapshot.signals.truncate(SIGNAL_LIMIT_V1);
    let target_total = snapshot.targets.len();
    snapshot.targets.truncate(TARGET_QUALITY_LIMIT_V1);
    let quality_total = snapshot.quality_checks.len();
    snapshot.quality_checks.truncate(TARGET_QUALITY_LIMIT_V1);
    let excerpt_total = snapshot.guidance.excerpts.len();
    retain_bounded_excerpts(&mut snapshot);
    let timeline_total = snapshot.timeline.len();
    snapshot.timeline.truncate(TIMELINE_LIMIT_V1);

    let counts = [
        (snapshot.lineage.len(), snapshot.lineage.len()),
        (1, 1),
        (1, 1),
        (snapshot.signals.len(), signal_total),
        (snapshot.targets.len(), target_total),
        (snapshot.quality_checks.len(), quality_total),
        (
            usize::from(snapshot.effective_skill.is_some()),
            usize::from(snapshot.effective_skill.is_some()),
        ),
        (snapshot.guidance.excerpts.len(), excerpt_total),
        (snapshot.timeline.len(), timeline_total),
        (
            snapshot.privacy_classes.len(),
            snapshot.privacy_classes.len(),
        ),
        (snapshot.rationale.len(), snapshot.rationale.len()),
        (snapshot.verification.len(), snapshot.verification.len()),
        (snapshot.lineage.len(), snapshot.lineage.len()),
    ];
    let truncations = counts.map(|(retained, total)| DossierTruncationV1 {
        complete: retained == total,
        retained_count: retained as u32,
        total_count: total as u32,
        selection_policy: SELECTION_POLICY_V1.into(),
    });
    BoundedDossierSnapshotV1 {
        snapshot,
        truncations,
    }
}

fn retain_bounded_excerpts(snapshot: &mut AuthoritativeDossierSnapshotV1) {
    let mut retained_bytes = 0;
    snapshot.guidance.excerpts.retain(|excerpt| {
        let next = retained_bytes + excerpt.safe_text.len();
        if next > EXCERPT_BYTES_LIMIT_V1 {
            return false;
        }
        retained_bytes = next;
        true
    });
}
