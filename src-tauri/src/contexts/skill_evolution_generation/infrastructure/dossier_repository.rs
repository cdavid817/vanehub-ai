use crate::contexts::skill_evolution_generation::domain::{DossierSectionKind, EvidenceDossierV1};
use rusqlite::{params, Connection, OptionalExtension};

use super::{canonical_json, GenerationPersistenceError, PersistGenerationOutcome};

pub(crate) struct GenerationDossierRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationDossierRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn persist(
        &self,
        dossier: &EvidenceDossierV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate_dossier(dossier)?;
        let revision = i64::try_from(dossier.revision)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let inserted = transaction.execute(
            "INSERT INTO evolution_evidence_dossiers
             (dossier_id,schema_version,revision,input_witness_hash,builder_version,
              sanitizer_version,canonical_size_bytes,content_hash,supersedes_dossier_id,created_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![dossier.dossier_id,dossier.schema_version,revision,dossier.input_witness_hash,
                dossier.builder_version,dossier.sanitizer_version,dossier.canonical_size_bytes,
                dossier.content_hash,dossier.supersedes_dossier_id,dossier.created_at_ms],
        );
        match inserted {
            Ok(_) => {}
            Err(error) if is_constraint(&error) => {
                let stored = transaction
                    .query_row(
                        "SELECT dossier_id FROM evolution_evidence_dossiers WHERE content_hash=?1",
                        [&dossier.content_hash],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|_| GenerationPersistenceError::Storage)?;
                return stored
                    .map(|id| PersistGenerationOutcome::Coalesced { id })
                    .ok_or(GenerationPersistenceError::Conflict);
            }
            Err(_) => return Err(GenerationPersistenceError::Storage),
        }
        for section in &dossier.sections {
            let witnesses = canonical_json(&section.source_witnesses)?;
            let records = canonical_json(&section.records)?;
            let truncation = canonical_json(&section.truncation)?;
            transaction
                .execute(
                    "INSERT INTO evolution_evidence_dossier_sections
                 (dossier_id,ordinal,section_kind,status,source_witnesses_json,records_json,
                  truncation_json,unavailable_reason_code,section_hash)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        dossier.dossier_id,
                        section.ordinal,
                        section_kind_name(section.kind),
                        enum_name(&section.status)?,
                        witnesses,
                        records,
                        truncation,
                        section.unavailable_reason_code,
                        section.section_hash
                    ],
                )
                .map_err(|_| GenerationPersistenceError::Storage)?;
        }
        transaction
            .commit()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        Ok(PersistGenerationOutcome::Inserted {
            id: dossier.dossier_id.clone(),
        })
    }
}

fn validate_dossier(dossier: &EvidenceDossierV1) -> Result<(), GenerationPersistenceError> {
    if dossier.dossier_id.trim().is_empty()
        || dossier.input_witness_hash.trim().is_empty()
        || dossier.content_hash.trim().is_empty()
        || dossier.revision == 0
        || !dossier.has_exact_section_shape()
        || dossier.created_at_ms < 0
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    if dossier
        .sections
        .iter()
        .any(|section| section.section_hash.trim().is_empty())
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn enum_name<T: serde::Serialize>(value: &T) -> Result<String, GenerationPersistenceError> {
    serde_json::to_value(value)
        .map_err(|_| GenerationPersistenceError::InvalidInput)?
        .as_str()
        .map(str::to_owned)
        .ok_or(GenerationPersistenceError::InvalidInput)
}

fn section_kind_name(kind: DossierSectionKind) -> &'static str {
    match kind {
        DossierSectionKind::IdentityAndProvenance => "identity_and_provenance",
        DossierSectionKind::ExecutiveSummary => "executive_summary",
        DossierSectionKind::CandidateSeed => "candidate_seed",
        DossierSectionKind::SourceSignalInventory => "source_signal_inventory",
        DossierSectionKind::AttributionAndTargetSelection => "attribution_and_target_selection",
        DossierSectionKind::AssessmentAndQualityGates => "assessment_and_quality_gates",
        DossierSectionKind::CurrentEffectiveSkillSnapshot => "current_effective_skill_snapshot",
        DossierSectionKind::RelevantGuidanceAndResourceContext => {
            "relevant_guidance_and_resource_context"
        }
        DossierSectionKind::FailureRecoveryAndVerificationTimeline => {
            "failure_recovery_and_verification_timeline"
        }
        DossierSectionKind::PrivacyAndRedactionReport => "privacy_and_redaction_report",
        DossierSectionKind::ProposedMutationRationale => "proposed_mutation_rationale",
        DossierSectionKind::VerificationPlan => "verification_plan",
        DossierSectionKind::LineageAndVersionWitnesses => "lineage_and_version_witnesses",
    }
}

fn is_constraint(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
}
