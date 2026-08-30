use crate::contexts::skill_evolution_generation::domain::{
    DossierSectionPageRequest, DossierSectionPageV1, DossierSectionStatus, DossierSourceLinkPageV1,
    DossierSourceLinkV1, DossierTruncationV1, DOSSIER_SECTION_ORDER_V1,
};
use rusqlite::{params, Connection, OptionalExtension};

use super::GenerationPersistenceError;

const MAX_PAGE_SIZE: u16 = 100;

pub(crate) struct GenerationDossierQuery<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationDossierQuery<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn section_page(
        &self,
        request: &DossierSectionPageRequest<'_>,
    ) -> Result<DossierSectionPageV1, GenerationPersistenceError> {
        if request.dossier_id.trim().is_empty()
            || request.ordinal >= 13
            || request.limit == 0
            || request.limit > MAX_PAGE_SIZE
        {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let stored = self
            .connection
            .query_row(
                "SELECT d.revision,s.section_kind,s.status,s.source_witnesses_json,s.records_json,
             s.truncation_json,s.unavailable_reason_code,s.section_hash
             FROM evolution_evidence_dossiers d JOIN evolution_evidence_dossier_sections s
             ON s.dossier_id=d.dossier_id WHERE d.dossier_id=?1 AND s.ordinal=?2",
                params![request.dossier_id, request.ordinal],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)?
            .ok_or(GenerationPersistenceError::InvalidInput)?;
        let expected_kind = DOSSIER_SECTION_ORDER_V1[usize::from(request.ordinal)];
        if stored.1 != section_kind_name(expected_kind) {
            return Err(GenerationPersistenceError::Storage);
        }
        let offset = decode_cursor(request.cursor, &stored.7)?;
        let all_records = serde_json::from_str::<Vec<_>>(&stored.4)
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let source_witnesses =
            serde_json::from_str(&stored.3).map_err(|_| GenerationPersistenceError::Storage)?;
        let truncation: DossierTruncationV1 =
            serde_json::from_str(&stored.5).map_err(|_| GenerationPersistenceError::Storage)?;
        if offset > all_records.len() {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let end = (offset + usize::from(request.limit)).min(all_records.len());
        let records = all_records[offset..end].to_vec();
        let next_cursor = (end < all_records.len()).then(|| format!("{}:{end}", stored.7));
        let status = parse_status(&stored.2)?;
        let page_complete = next_cursor.is_none() && truncation.complete;
        Ok(DossierSectionPageV1 {
            dossier_id: request.dossier_id.into(),
            dossier_revision: u64::try_from(stored.0)
                .map_err(|_| GenerationPersistenceError::Storage)?,
            ordinal: request.ordinal,
            kind: expected_kind,
            status,
            source_witnesses,
            records,
            truncation,
            unavailable_reason_code: stored.6,
            section_hash: stored.7,
            next_cursor,
            page_complete,
        })
    }

    pub(crate) fn source_links(
        &self,
        dossier_id: &str,
        cursor: Option<&str>,
        limit: u16,
    ) -> Result<DossierSourceLinkPageV1, GenerationPersistenceError> {
        if dossier_id.trim().is_empty() || limit == 0 || limit > MAX_PAGE_SIZE {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let content_hash: String = self
            .connection
            .query_row(
                "SELECT content_hash FROM evolution_evidence_dossiers WHERE dossier_id=?1",
                [dossier_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)?
            .ok_or(GenerationPersistenceError::InvalidInput)?;
        let offset = decode_cursor(cursor, &content_hash)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT link_kind,linked_id,linked_revision,witness_hash
             FROM evolution_evidence_dossier_links WHERE dossier_id=?1
             ORDER BY link_kind,linked_id,linked_revision LIMIT ?2 OFFSET ?3",
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let rows = statement
            .query_map(
                params![dossier_id, i64::from(limit) + 1, offset as i64],
                |row| {
                    Ok(DossierSourceLinkV1 {
                        link_kind: row.get(0)?,
                        linked_id: row.get(1)?,
                        linked_revision: row.get(2)?,
                        witness_hash: row.get(3)?,
                    })
                },
            )
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let mut links = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let has_more = links.len() > usize::from(limit);
        links.truncate(usize::from(limit));
        let next_cursor = has_more.then(|| format!("{}:{}", content_hash, offset + links.len()));
        Ok(DossierSourceLinkPageV1 {
            dossier_id: dossier_id.into(),
            links,
            next_cursor,
        })
    }
}

fn decode_cursor(
    cursor: Option<&str>,
    expected_hash: &str,
) -> Result<usize, GenerationPersistenceError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let (hash, offset) = cursor
        .rsplit_once(':')
        .ok_or(GenerationPersistenceError::InvalidInput)?;
    if hash != expected_hash {
        return Err(GenerationPersistenceError::Conflict);
    }
    offset
        .parse()
        .map_err(|_| GenerationPersistenceError::InvalidInput)
}

fn parse_status(value: &str) -> Result<DossierSectionStatus, GenerationPersistenceError> {
    match value {
        "complete" => Ok(DossierSectionStatus::Complete),
        "partial" => Ok(DossierSectionStatus::Partial),
        "not_applicable" => Ok(DossierSectionStatus::NotApplicable),
        "unavailable" => Ok(DossierSectionStatus::Unavailable),
        "redacted" => Ok(DossierSectionStatus::Redacted),
        _ => Err(GenerationPersistenceError::Storage),
    }
}

fn section_kind_name(
    kind: crate::contexts::skill_evolution_generation::domain::DossierSectionKind,
) -> &'static str {
    match kind {
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::IdentityAndProvenance => "identity_and_provenance",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::ExecutiveSummary => "executive_summary",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::CandidateSeed => "candidate_seed",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::SourceSignalInventory => "source_signal_inventory",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::AttributionAndTargetSelection => "attribution_and_target_selection",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::AssessmentAndQualityGates => "assessment_and_quality_gates",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::CurrentEffectiveSkillSnapshot => "current_effective_skill_snapshot",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::RelevantGuidanceAndResourceContext => "relevant_guidance_and_resource_context",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::FailureRecoveryAndVerificationTimeline => "failure_recovery_and_verification_timeline",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::PrivacyAndRedactionReport => "privacy_and_redaction_report",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::ProposedMutationRationale => "proposed_mutation_rationale",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::VerificationPlan => "verification_plan",
        crate::contexts::skill_evolution_generation::domain::DossierSectionKind::LineageAndVersionWitnesses => "lineage_and_version_witnesses",
    }
}
