use crate::contexts::tooling::api::{
    project_effective_skill_catalog, EffectiveSkill, EffectiveSkillCatalogEntry, SkillAvailability,
    SkillLayer, SkillTrust, SkillType,
};

use super::{
    EffectiveTargetWitness, TargetExclusionReason, TargetLifecycle, TargetScope, TargetTrust,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetParticipation {
    pub(crate) skill_id: String,
    pub(crate) revision_hash: String,
    pub(crate) verified_runs: u8,
    pub(crate) correlated_runs: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetCatalogEntry {
    pub(crate) witness: EffectiveTargetWitness,
    pub(crate) name: String,
    pub(crate) skill_type: String,
    pub(crate) category: String,
    pub(crate) description: String,
    pub(crate) declared_tools: Vec<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) verified_participation: u8,
    pub(crate) correlated_participation: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetExclusion {
    pub(crate) skill_id: String,
    pub(crate) revision_hash: String,
    pub(crate) reason: TargetExclusionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TargetCatalog {
    pub(crate) entries: Vec<TargetCatalogEntry>,
    pub(crate) exclusions: Vec<TargetExclusion>,
}

pub(crate) fn project_target_catalog(
    skills: &[EffectiveSkill],
    participation: &[TargetParticipation],
) -> TargetCatalog {
    let projected = project_effective_skill_catalog(skills);
    project_target_catalog_entries(&projected, participation)
}

pub(super) fn project_target_catalog_entries(
    skills: &[EffectiveSkillCatalogEntry],
    participation: &[TargetParticipation],
) -> TargetCatalog {
    let mut catalog = TargetCatalog::default();
    for skill in skills {
        let lifecycle = lifecycle(skill.availability, skill.layer);
        let witness = EffectiveTargetWitness {
            skill_id: skill.skill_id.clone(),
            skill_type: skill_type(skill.skill_type).to_string(),
            revision_hash: skill.revision.clone(),
            scope: scope(skill.layer),
            lifecycle,
            trust: trust(skill.trust),
        };
        if lifecycle == TargetLifecycle::Malformed {
            catalog.exclusions.push(TargetExclusion {
                skill_id: witness.skill_id,
                revision_hash: witness.revision_hash,
                reason: TargetExclusionReason::Malformed,
            });
            continue;
        }
        let counts = participation_counts(participation, &witness);
        catalog.entries.push(TargetCatalogEntry {
            witness,
            name: skill.name.clone(),
            skill_type: skill_type(skill.skill_type).to_string(),
            category: skill.category.clone(),
            description: skill.description.clone(),
            declared_tools: skill.declared_tools.clone(),
            capabilities: skill.capabilities.clone(),
            verified_participation: counts.0,
            correlated_participation: counts.1,
        });
        catalog
            .exclusions
            .extend(skill.shadowed.iter().map(|shadowed| TargetExclusion {
                skill_id: shadowed.skill_id.clone(),
                revision_hash: shadowed.revision.clone(),
                reason: if matches!(
                    shadowed.availability,
                    SkillAvailability::Invalid | SkillAvailability::Conflicting
                ) {
                    TargetExclusionReason::Malformed
                } else {
                    TargetExclusionReason::Shadowed
                },
            }));
    }
    catalog.exclusions.extend(participation_exclusions(
        skills,
        participation,
        &catalog.exclusions,
    ));
    catalog.entries.sort_by(|left, right| {
        scope_rank(left.witness.scope)
            .cmp(&scope_rank(right.witness.scope))
            .then(left.witness.skill_id.cmp(&right.witness.skill_id))
            .then(left.witness.revision_hash.cmp(&right.witness.revision_hash))
    });
    catalog.exclusions.sort_by(|left, right| {
        left.skill_id
            .cmp(&right.skill_id)
            .then(left.revision_hash.cmp(&right.revision_hash))
            .then(exclusion_rank(left.reason).cmp(&exclusion_rank(right.reason)))
    });
    catalog.exclusions.dedup();
    catalog
}

fn participation_exclusions(
    skills: &[EffectiveSkillCatalogEntry],
    participation: &[TargetParticipation],
    existing: &[TargetExclusion],
) -> Vec<TargetExclusion> {
    participation
        .iter()
        .filter(|item| {
            !existing.iter().any(|excluded| {
                excluded.skill_id == item.skill_id && excluded.revision_hash == item.revision_hash
            }) && !skills.iter().any(|skill| {
                skill.skill_id == item.skill_id
                    && (skill.revision == item.revision_hash
                        || skill.shadowed.iter().any(|shadowed| {
                            shadowed.skill_id == item.skill_id
                                && shadowed.revision == item.revision_hash
                        }))
            })
        })
        .map(|item| TargetExclusion {
            skill_id: item.skill_id.clone(),
            revision_hash: item.revision_hash.clone(),
            reason: if skills.iter().any(|skill| skill.skill_id == item.skill_id) {
                TargetExclusionReason::HistoricalOnly
            } else {
                TargetExclusionReason::Missing
            },
        })
        .collect()
}

fn participation_counts(
    participation: &[TargetParticipation],
    witness: &EffectiveTargetWitness,
) -> (u8, u8) {
    participation
        .iter()
        .filter(|item| {
            item.skill_id == witness.skill_id && item.revision_hash == witness.revision_hash
        })
        .fold((0_u8, 0_u8), |counts, item| {
            (
                counts.0.saturating_add(item.verified_runs),
                counts.1.saturating_add(item.correlated_runs),
            )
        })
}

fn exclusion_rank(reason: TargetExclusionReason) -> u8 {
    match reason {
        TargetExclusionReason::Shadowed => 0,
        TargetExclusionReason::Missing => 1,
        TargetExclusionReason::Malformed => 2,
        TargetExclusionReason::HistoricalOnly => 3,
    }
}

fn scope(layer: SkillLayer) -> TargetScope {
    match layer {
        SkillLayer::Project => TargetScope::Project,
        SkillLayer::User => TargetScope::User,
        SkillLayer::Registry => TargetScope::Remote,
        SkillLayer::System => TargetScope::System,
    }
}

fn scope_rank(scope: TargetScope) -> u8 {
    match scope {
        TargetScope::Project => 0,
        TargetScope::User => 1,
        TargetScope::Remote => 2,
        TargetScope::System => 3,
    }
}

fn lifecycle(availability: SkillAvailability, layer: SkillLayer) -> TargetLifecycle {
    match availability {
        SkillAvailability::Invalid | SkillAvailability::Conflicting => TargetLifecycle::Malformed,
        SkillAvailability::Disabled | SkillAvailability::Unsupported => TargetLifecycle::Archived,
        SkillAvailability::Available if !layer.content_is_mutable() => TargetLifecycle::Pinned,
        SkillAvailability::Available => TargetLifecycle::Active,
    }
}

fn trust(value: SkillTrust) -> TargetTrust {
    match value {
        SkillTrust::Trusted => TargetTrust::Trusted,
        SkillTrust::Untrusted => TargetTrust::Untrusted,
    }
}

fn skill_type(value: SkillType) -> &'static str {
    match value {
        SkillType::Role => "role",
        SkillType::Utility => "utility",
    }
}
