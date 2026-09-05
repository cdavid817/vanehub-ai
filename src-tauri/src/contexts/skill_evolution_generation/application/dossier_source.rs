use crate::contexts::skill_evolution_generation::domain::{
    DossierSourceWitnessV1, FrozenGenerationInputV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoritativeDossierSnapshotV1 {
    pub(crate) sanitizer_version: String,
    pub(crate) lineage_complete: bool,
    pub(crate) identity: DossierIdentitySourceV1,
    pub(crate) seed: DossierSeedSourceV1,
    pub(crate) signals: Vec<DossierSignalSourceV1>,
    pub(crate) targets: Vec<DossierTargetSourceV1>,
    pub(crate) no_target_reason_code: Option<String>,
    pub(crate) quality_checks: Vec<DossierQualitySourceV1>,
    pub(crate) effective_skill: Option<DossierEffectiveSkillSourceV1>,
    pub(crate) guidance: DossierGuidanceSourceV1,
    pub(crate) timeline: Vec<DossierTimelineSourceV1>,
    pub(crate) privacy_classes: Vec<DossierPrivacySourceV1>,
    pub(crate) rationale: Vec<DossierClaimSourceV1>,
    pub(crate) verification: Vec<DossierVerificationSourceV1>,
    pub(crate) lineage: Vec<DossierSourceWitnessV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierIdentitySourceV1 {
    pub(crate) workspace_id: Option<String>,
    pub(crate) seed_id: String,
    pub(crate) assessment_attempt_id: String,
    pub(crate) target_skill_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierSeedSourceV1 {
    pub(crate) category: String,
    pub(crate) readiness: String,
    pub(crate) safe_summary: String,
    pub(crate) independent_run_count: u32,
    pub(crate) witness: DossierSourceWitnessV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierSignalSourceV1 {
    pub(crate) signal_id: String,
    pub(crate) category: String,
    pub(crate) occurred_at_ms: i64,
    pub(crate) witness: DossierSourceWitnessV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierTargetSourceV1 {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) score_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierQualitySourceV1 {
    pub(crate) code: String,
    pub(crate) result: String,
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierEffectiveSkillSourceV1 {
    pub(crate) skill_id: String,
    pub(crate) skill_type: String,
    pub(crate) scope: String,
    pub(crate) effective_revision: String,
    pub(crate) overlay_state: String,
    pub(crate) metadata_codes: Vec<String>,
    pub(crate) witnesses: Vec<DossierSourceWitnessV1>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DossierGuidanceSourceV1 {
    pub(crate) excerpts: Vec<DossierExcerptSourceV1>,
    pub(crate) resources: Vec<DossierResourceSourceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierExcerptSourceV1 {
    pub(crate) excerpt_id: String,
    pub(crate) logical_location: String,
    pub(crate) safe_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierResourceSourceV1 {
    pub(crate) resource_id: String,
    pub(crate) resource_kind: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierTimelineSourceV1 {
    pub(crate) event_code: String,
    pub(crate) occurred_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierPrivacySourceV1 {
    pub(crate) class_code: String,
    pub(crate) redacted_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierClaimSourceV1 {
    pub(crate) claim_id: String,
    pub(crate) claim_kind: String,
    pub(crate) safe_text: String,
    pub(crate) citation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DossierVerificationSourceV1 {
    pub(crate) step_id: String,
    pub(crate) action_code: String,
    pub(crate) citation_ids: Vec<String>,
}

pub(crate) trait GenerationDossierSourcePort {
    fn load_authoritative_snapshot(
        &self,
        input: &FrozenGenerationInputV1,
    ) -> Result<AuthoritativeDossierSnapshotV1, DossierSourceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DossierSourceError {
    Missing,
    Purged,
    Superseded,
    IncompatibleVersion,
    Storage,
}
