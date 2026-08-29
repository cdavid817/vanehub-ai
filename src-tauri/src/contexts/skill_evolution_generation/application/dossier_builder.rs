use crate::contexts::skill_evolution_generation::{
    application::{canonical_hash, canonical_json},
    domain::{
        DossierSectionV1, DossierTruncationV1, EvidenceDossierV1, FrozenGenerationInputV1,
        DOSSIER_SECTION_ORDER_V1, GENERATION_SCHEMA_VERSION_V1,
    },
};

use super::{
    dossier_limits::{bounded_snapshot, DOSSIER_CANONICAL_SIZE_LIMIT_V1},
    dossier_projection::{section_records, section_status, section_witnesses, unavailable_reason},
    AuthoritativeDossierSnapshotV1,
};

pub(crate) struct DossierBuildRequestV1<'a> {
    pub(crate) dossier_id: &'a str,
    pub(crate) revision: u64,
    pub(crate) builder_version: &'a str,
    pub(crate) input: &'a FrozenGenerationInputV1,
    pub(crate) snapshot: &'a AuthoritativeDossierSnapshotV1,
    pub(crate) supersedes_dossier_id: Option<&'a str>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DossierBuildError {
    InvalidInput,
    IncompatibleVersion,
    Serialization,
    SizeLimitExceeded,
}

pub(crate) fn build_dossier(
    request: &DossierBuildRequestV1<'_>,
) -> Result<EvidenceDossierV1, DossierBuildError> {
    validate_request(request)?;
    let bounded = bounded_snapshot(request.snapshot);
    let records = section_records(&bounded.snapshot);
    let witnesses = section_witnesses(&bounded.snapshot);
    let mut sections = Vec::with_capacity(DOSSIER_SECTION_ORDER_V1.len());
    for (ordinal, kind) in DOSSIER_SECTION_ORDER_V1.into_iter().enumerate() {
        let section_records = records[ordinal].clone();
        let source_witnesses = witnesses[ordinal].clone();
        let truncation = bounded.truncations[ordinal].clone();
        let status = section_status(ordinal, &section_records, &bounded.snapshot, &truncation);
        let unavailable_reason_code = unavailable_reason(status, ordinal);
        let hash = canonical_hash(&(
            GENERATION_SCHEMA_VERSION_V1,
            ordinal,
            kind,
            status,
            &source_witnesses,
            &section_records,
            &truncation,
            &unavailable_reason_code,
        ))
        .map_err(|_| DossierBuildError::Serialization)?;
        sections.push(DossierSectionV1 {
            ordinal: ordinal as u8,
            kind,
            status,
            source_witnesses,
            records: section_records,
            truncation,
            unavailable_reason_code,
            section_hash: hash,
        });
    }
    let input_witness_hash =
        canonical_hash(request.input).map_err(|_| DossierBuildError::Serialization)?;
    let section_hashes: Vec<_> = sections
        .iter()
        .map(|section| &section.section_hash)
        .collect();
    let content_hash = canonical_hash(&(
        GENERATION_SCHEMA_VERSION_V1,
        request.builder_version,
        &request.snapshot.sanitizer_version,
        &input_witness_hash,
        section_hashes,
    ))
    .map_err(|_| DossierBuildError::Serialization)?;
    let mut dossier = EvidenceDossierV1 {
        schema_version: GENERATION_SCHEMA_VERSION_V1,
        dossier_id: request.dossier_id.into(),
        revision: request.revision,
        input_witness_hash,
        builder_version: request.builder_version.into(),
        sanitizer_version: request.snapshot.sanitizer_version.clone(),
        sections,
        canonical_size_bytes: 0,
        content_hash,
        supersedes_dossier_id: request.supersedes_dossier_id.map(str::to_owned),
        created_at_ms: request.created_at_ms,
    };
    for _ in 0..3 {
        dossier.canonical_size_bytes = canonical_json(&dossier)
            .map_err(|_| DossierBuildError::Serialization)?
            .len()
            .try_into()
            .map_err(|_| DossierBuildError::InvalidInput)?;
    }
    if dossier.canonical_size_bytes > DOSSIER_CANONICAL_SIZE_LIMIT_V1 {
        return Err(DossierBuildError::SizeLimitExceeded);
    }
    Ok(dossier)
}

fn validate_request(request: &DossierBuildRequestV1<'_>) -> Result<(), DossierBuildError> {
    if request.dossier_id.trim().is_empty()
        || request.revision == 0
        || request.builder_version.trim().is_empty()
        || request.snapshot.sanitizer_version.trim().is_empty()
        || request.created_at_ms < 0
    {
        return Err(DossierBuildError::InvalidInput);
    }
    if !request.snapshot.lineage_complete
        || request.snapshot.lineage.is_empty()
        || !sanitizer_is_supported(&request.snapshot.sanitizer_version)
        || request.snapshot.sanitizer_version != request.input.evidence.sanitizer_version
        || request
            .snapshot
            .lineage
            .iter()
            .chain(std::iter::once(&request.snapshot.seed.witness))
            .chain(
                request
                    .snapshot
                    .signals
                    .iter()
                    .map(|signal| &signal.witness),
            )
            .chain(
                request
                    .snapshot
                    .effective_skill
                    .iter()
                    .flat_map(|skill| skill.witnesses.iter()),
            )
            .any(|witness| witness.schema_version != GENERATION_SCHEMA_VERSION_V1)
    {
        return Err(DossierBuildError::IncompatibleVersion);
    }
    Ok(())
}

fn sanitizer_is_supported(version: &str) -> bool {
    matches!(version, "1" | "evidence-sanitizer-v1")
}
