use std::collections::BTreeSet;

use unicode_normalization::UnicodeNormalization;

use super::{
    EvidenceAttribution, LexicalFieldClass, LocalLexicalIndex, RankedTarget,
    SelectionClassification, SelectionThresholdWitness, TargetCatalog, TargetCatalogEntry,
    TargetExclusionReason, TargetScope,
};

pub(crate) const SELECTED_SCORE_MINIMUM_V1: u8 = 60;
pub(crate) const AMBIGUOUS_SCORE_MINIMUM_V1: u8 = 45;
pub(crate) const SELECTED_MARGIN_MINIMUM_V1: u8 = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionEvidence {
    pub(crate) attribution: EvidenceAttribution,
    pub(crate) skill_type: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) declared_tools: Vec<String>,
    pub(crate) lexical_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetSelection {
    pub(crate) classification: SelectionClassification,
    pub(crate) targets: Vec<RankedTarget>,
    pub(crate) threshold: SelectionThresholdWitness,
    pub(crate) attribution_uncertain: bool,
}

pub(crate) fn rank_targets(
    catalog: &TargetCatalog,
    lexical_index: &LocalLexicalIndex,
    evidence: &SelectionEvidence,
) -> TargetSelection {
    let mut targets = catalog
        .entries
        .iter()
        .map(|entry| rank_target(entry, catalog, lexical_index, evidence))
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(scope_rank(left.witness.scope).cmp(&scope_rank(right.witness.scope)))
            .then(left.witness.skill_id.cmp(&right.witness.skill_id))
            .then(left.witness.revision_hash.cmp(&right.witness.revision_hash))
    });
    let leading_score = targets.first().map_or(0, |target| target.score);
    let runner_up_score = targets.get(1).map(|target| target.score);
    let margin = leading_score.saturating_sub(runner_up_score.unwrap_or(0));
    let classification =
        if leading_score >= SELECTED_SCORE_MINIMUM_V1 && margin >= SELECTED_MARGIN_MINIMUM_V1 {
            SelectionClassification::Selected
        } else if leading_score >= AMBIGUOUS_SCORE_MINIMUM_V1 {
            SelectionClassification::Ambiguous
        } else {
            SelectionClassification::NoTarget
        };
    let attribution_uncertain = targets
        .first()
        .is_none_or(|target| target.attribution_uncertain);
    TargetSelection {
        classification,
        targets,
        threshold: SelectionThresholdWitness {
            leading_score,
            runner_up_score,
            margin,
            selected_minimum: SELECTED_SCORE_MINIMUM_V1,
            ambiguous_minimum: AMBIGUOUS_SCORE_MINIMUM_V1,
            required_margin: SELECTED_MARGIN_MINIMUM_V1,
        },
        attribution_uncertain,
    }
}

fn rank_target(
    entry: &TargetCatalogEntry,
    catalog: &TargetCatalog,
    lexical_index: &LocalLexicalIndex,
    evidence: &SelectionEvidence,
) -> RankedTarget {
    let (attribution_score, attribution_uncertain) = attribution_score(entry, evidence.attribution);
    let participation_score = entry
        .verified_participation
        .saturating_add(entry.correlated_participation)
        .min(3)
        .saturating_mul(5);
    let (compatibility_score, mut matched) = compatibility_score(entry, evidence);
    let (lexical_score, lexical_fields) = lexical_index.score(
        &entry.witness.skill_id,
        &entry.witness.revision_hash,
        &evidence.lexical_terms,
    );
    matched.extend(lexical_fields.into_iter().map(lexical_class_name));
    if attribution_score > 0 {
        matched.push("attribution".to_string());
    }
    if participation_score > 0 {
        matched.push("participation".to_string());
    }
    matched.sort();
    matched.dedup();
    let locality_score = locality_score(entry.witness.scope);
    let score = attribution_score
        .saturating_add(participation_score)
        .saturating_add(compatibility_score)
        .saturating_add(lexical_score)
        .saturating_add(locality_score)
        .min(100);
    let mut exclusions = catalog
        .exclusions
        .iter()
        .filter(|excluded| excluded.skill_id == entry.witness.skill_id)
        .map(|excluded| excluded.reason)
        .collect::<Vec<_>>();
    exclusions.sort_by_key(|reason| exclusion_rank(*reason));
    exclusions.dedup();
    RankedTarget {
        witness: entry.witness.clone(),
        score,
        attribution_score,
        participation_score,
        compatibility_score,
        lexical_score,
        locality_score,
        matched_feature_classes: matched,
        exclusions,
        attribution_uncertain,
    }
}

fn attribution_score(entry: &TargetCatalogEntry, attribution: EvidenceAttribution) -> (u8, bool) {
    match attribution {
        EvidenceAttribution::Verified if entry.verified_participation > 0 => (35, false),
        EvidenceAttribution::Verified | EvidenceAttribution::Correlated
            if entry.correlated_participation > 0 || entry.verified_participation > 0 =>
        {
            (20, true)
        }
        EvidenceAttribution::Verified
        | EvidenceAttribution::Correlated
        | EvidenceAttribution::Weak
        | EvidenceAttribution::Unattributed => (0, true),
    }
}

fn compatibility_score(
    entry: &TargetCatalogEntry,
    evidence: &SelectionEvidence,
) -> (u8, Vec<String>) {
    let mut score = 0_u8;
    let mut matched = Vec::new();
    if evidence
        .skill_type
        .as_ref()
        .is_some_and(|value| normalized(value) == normalized(&entry.skill_type))
    {
        score += 5;
        matched.push("skill_type".to_string());
    }
    if evidence
        .category
        .as_ref()
        .is_some_and(|value| normalized(value) == normalized(&entry.category))
    {
        score += 5;
        matched.push("category".to_string());
    }
    let capability_matches = overlap_count(&evidence.capabilities, &entry.capabilities).min(3);
    if capability_matches > 0 {
        score += capability_matches * 2;
        matched.push("capability".to_string());
    }
    let tool_matches = overlap_count(&evidence.declared_tools, &entry.declared_tools).min(2);
    if tool_matches > 0 {
        score += tool_matches * 2;
        matched.push("declared_tool".to_string());
    }
    (score.min(20), matched)
}

fn overlap_count(left: &[String], right: &[String]) -> u8 {
    let right = right
        .iter()
        .map(|value| normalized(value))
        .collect::<BTreeSet<_>>();
    left.iter()
        .map(|value| normalized(value))
        .collect::<BTreeSet<_>>()
        .intersection(&right)
        .count()
        .try_into()
        .unwrap_or(u8::MAX)
}

fn normalized(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

fn locality_score(scope: TargetScope) -> u8 {
    match scope {
        TargetScope::Project => 10,
        TargetScope::User => 7,
        TargetScope::Remote => 4,
        TargetScope::System => 2,
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

fn lexical_class_name(field: LexicalFieldClass) -> String {
    match field {
        LexicalFieldClass::Capability => "lexical_capability",
        LexicalFieldClass::Tag => "lexical_tag",
        LexicalFieldClass::Description => "lexical_description",
        LexicalFieldClass::Heading => "lexical_heading",
        LexicalFieldClass::Instruction => "lexical_instruction",
    }
    .to_string()
}

fn exclusion_rank(reason: TargetExclusionReason) -> u8 {
    match reason {
        TargetExclusionReason::Shadowed => 0,
        TargetExclusionReason::Missing => 1,
        TargetExclusionReason::Malformed => 2,
        TargetExclusionReason::HistoricalOnly => 3,
    }
}
