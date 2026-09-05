use super::{DossierSectionKind, DossierSectionStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const DOSSIER_SECTION_COUNT_V1: usize = 13;

pub(crate) const DOSSIER_SECTION_ORDER_V1: [DossierSectionKind; DOSSIER_SECTION_COUNT_V1] = [
    DossierSectionKind::IdentityAndProvenance,
    DossierSectionKind::ExecutiveSummary,
    DossierSectionKind::CandidateSeed,
    DossierSectionKind::SourceSignalInventory,
    DossierSectionKind::AttributionAndTargetSelection,
    DossierSectionKind::AssessmentAndQualityGates,
    DossierSectionKind::CurrentEffectiveSkillSnapshot,
    DossierSectionKind::RelevantGuidanceAndResourceContext,
    DossierSectionKind::FailureRecoveryAndVerificationTimeline,
    DossierSectionKind::PrivacyAndRedactionReport,
    DossierSectionKind::ProposedMutationRationale,
    DossierSectionKind::VerificationPlan,
    DossierSectionKind::LineageAndVersionWitnesses,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceDossierV1 {
    pub(crate) schema_version: u16,
    pub(crate) dossier_id: String,
    pub(crate) revision: u64,
    pub(crate) input_witness_hash: String,
    pub(crate) builder_version: String,
    pub(crate) sanitizer_version: String,
    pub(crate) sections: Vec<DossierSectionV1>,
    pub(crate) canonical_size_bytes: u32,
    pub(crate) content_hash: String,
    pub(crate) supersedes_dossier_id: Option<String>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSectionV1 {
    pub(crate) ordinal: u8,
    pub(crate) kind: DossierSectionKind,
    pub(crate) status: DossierSectionStatus,
    pub(crate) source_witnesses: Vec<DossierSourceWitnessV1>,
    pub(crate) records: Vec<DossierRecordV1>,
    pub(crate) truncation: DossierTruncationV1,
    pub(crate) unavailable_reason_code: Option<String>,
    pub(crate) section_hash: String,
}

impl DossierSectionV1 {
    pub(crate) fn matches_required_position(&self) -> bool {
        DOSSIER_SECTION_ORDER_V1.get(usize::from(self.ordinal)) == Some(&self.kind)
    }
}

impl EvidenceDossierV1 {
    pub(crate) fn has_exact_section_shape(&self) -> bool {
        self.sections.len() == DOSSIER_SECTION_COUNT_V1
            && self.sections.iter().enumerate().all(|(ordinal, section)| {
                usize::from(section.ordinal) == ordinal && section.matches_required_position()
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierSourceWitnessV1 {
    pub(crate) schema_version: u16,
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) revision: String,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DossierTruncationV1 {
    pub(crate) complete: bool,
    pub(crate) retained_count: u32,
    pub(crate) total_count: u32,
    pub(crate) selection_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum DossierRecordV1 {
    Identity {
        identity_kind: String,
        value: String,
    },
    Metric {
        code: String,
        value: i64,
    },
    SourceReference {
        source_id: String,
        category: String,
        occurred_at_ms: i64,
    },
    Target {
        skill_id: String,
        revision: String,
        score_bps: u16,
    },
    QualityCheck {
        code: String,
        result: String,
        reason_code: String,
    },
    SkillExcerpt {
        excerpt_id: String,
        logical_location: String,
        text: String,
    },
    TimelineBucket {
        event_code: String,
        count: u32,
        first_at_ms: i64,
        last_at_ms: i64,
    },
    PrivacyClass {
        class_code: String,
        count: u32,
    },
    LessonClaim {
        claim_id: String,
        claim_kind: String,
        text: String,
        citation_ids: Vec<String>,
    },
    VerificationStep {
        step_id: String,
        action_code: String,
        citation_ids: Vec<String>,
    },
    Witness {
        witness_kind: String,
        revision: String,
        content_hash: String,
    },
    Summary {
        codes: Vec<String>,
        metrics: BTreeMap<String, i64>,
    },
}
