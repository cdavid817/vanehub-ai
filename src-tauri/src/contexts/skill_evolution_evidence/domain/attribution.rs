use serde::{Deserialize, Serialize};

use super::{
    AttributionStrength, EvidenceSourceEnvelope, ObservedSkillRevision, SkillAssociationKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AttributionRationale {
    ExactNativeObservation,
    ActiveCliMountSnapshot,
    ConfiguredBindingOnly,
    NoObservedSkillParticipation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetingEligibility {
    AutomatedConsideration,
    HumanReviewOnly,
    Ineligible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillAttribution {
    skill_id: String,
    revision: String,
    association_kind: SkillAssociationKind,
    observed_at: String,
    strength: AttributionStrength,
    rationale: AttributionRationale,
}

impl SkillAttribution {
    pub(crate) fn skill_id(&self) -> &str {
        &self.skill_id
    }

    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }

    pub(crate) fn association_kind(&self) -> SkillAssociationKind {
        self.association_kind
    }

    pub(crate) fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub(crate) fn strength(&self) -> AttributionStrength {
        self.strength
    }

    pub(crate) fn rationale(&self) -> AttributionRationale {
        self.rationale
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttributionResult {
    associations: Vec<SkillAttribution>,
    strength: AttributionStrength,
    rationale: AttributionRationale,
    targeting_eligibility: TargetingEligibility,
}

impl AttributionResult {
    pub(crate) fn associations(&self) -> &[SkillAttribution] {
        &self.associations
    }

    pub(crate) fn strength(&self) -> AttributionStrength {
        self.strength
    }

    pub(crate) fn rationale(&self) -> AttributionRationale {
        self.rationale
    }

    pub(crate) fn targeting_eligibility(&self) -> TargetingEligibility {
        self.targeting_eligibility
    }
}

pub(crate) fn attribute_evidence(envelope: &EvidenceSourceEnvelope) -> AttributionResult {
    match envelope {
        EvidenceSourceEnvelope::ManagedCli {
            common,
            mount_snapshot,
            configured_binding_ids,
            ..
        }
        | EvidenceSourceEnvelope::InteractiveCli {
            common,
            mount_snapshot,
            configured_binding_ids,
            ..
        } => {
            if let Some(snapshot) = mount_snapshot {
                let associations = snapshot
                    .skills
                    .iter()
                    .map(|skill| SkillAttribution {
                        skill_id: skill.skill_id.clone(),
                        revision: skill.revision.clone(),
                        association_kind: SkillAssociationKind::Mounted,
                        observed_at: common.occurred_at.clone(),
                        strength: AttributionStrength::Correlated,
                        rationale: AttributionRationale::ActiveCliMountSnapshot,
                    })
                    .collect();
                correlated(associations)
            } else if configured_binding_ids.is_empty() {
                unattributed()
            } else {
                weak()
            }
        }
        EvidenceSourceEnvelope::SkillLoading {
            common,
            skill_id,
            revision,
            ..
        } => verified_with_direct(
            &common.observed_skill_revisions,
            skill_id,
            revision,
            SkillAssociationKind::Loaded,
            &common.occurred_at,
        ),
        EvidenceSourceEnvelope::DelegatedUtility {
            common,
            utility_skill_id,
            revision,
            ..
        } => verified_with_direct(
            &common.observed_skill_revisions,
            utility_skill_id,
            revision,
            SkillAssociationKind::Delegated,
            &common.occurred_at,
        ),
        EvidenceSourceEnvelope::NativeExecution { common, .. }
        | EvidenceSourceEnvelope::RunVerification { common, .. }
        | EvidenceSourceEnvelope::ExplicitFeedback { common, .. } => {
            verified_from_observations(&common.observed_skill_revisions)
        }
    }
}

fn verified_with_direct(
    observations: &[ObservedSkillRevision],
    skill_id: &str,
    revision: &str,
    association_kind: SkillAssociationKind,
    observed_at: &str,
) -> AttributionResult {
    let mut associations = exact_associations(observations);
    if !associations
        .iter()
        .any(|item| item.skill_id == skill_id && item.revision == revision)
    {
        associations.push(SkillAttribution {
            skill_id: skill_id.to_string(),
            revision: revision.to_string(),
            association_kind,
            observed_at: observed_at.to_string(),
            strength: AttributionStrength::Verified,
            rationale: AttributionRationale::ExactNativeObservation,
        });
    }
    verified(associations)
}

fn verified_from_observations(observations: &[ObservedSkillRevision]) -> AttributionResult {
    let associations = exact_associations(observations);
    if associations.is_empty() {
        unattributed()
    } else {
        verified(associations)
    }
}

fn exact_associations(observations: &[ObservedSkillRevision]) -> Vec<SkillAttribution> {
    observations
        .iter()
        .map(|observation| SkillAttribution {
            skill_id: observation.skill_id.clone(),
            revision: observation.revision.clone(),
            association_kind: observation.association_kind,
            observed_at: observation.observed_at.clone(),
            strength: AttributionStrength::Verified,
            rationale: AttributionRationale::ExactNativeObservation,
        })
        .collect()
}

fn verified(associations: Vec<SkillAttribution>) -> AttributionResult {
    AttributionResult {
        associations,
        strength: AttributionStrength::Verified,
        rationale: AttributionRationale::ExactNativeObservation,
        targeting_eligibility: TargetingEligibility::AutomatedConsideration,
    }
}

fn correlated(associations: Vec<SkillAttribution>) -> AttributionResult {
    AttributionResult {
        associations,
        strength: AttributionStrength::Correlated,
        rationale: AttributionRationale::ActiveCliMountSnapshot,
        targeting_eligibility: TargetingEligibility::HumanReviewOnly,
    }
}

fn weak() -> AttributionResult {
    AttributionResult {
        associations: Vec::new(),
        strength: AttributionStrength::Weak,
        rationale: AttributionRationale::ConfiguredBindingOnly,
        targeting_eligibility: TargetingEligibility::Ineligible,
    }
}

fn unattributed() -> AttributionResult {
    AttributionResult {
        associations: Vec::new(),
        strength: AttributionStrength::Unattributed,
        rationale: AttributionRationale::NoObservedSkillParticipation,
        targeting_eligibility: TargetingEligibility::Ineligible,
    }
}
