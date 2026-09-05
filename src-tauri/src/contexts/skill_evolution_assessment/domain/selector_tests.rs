use super::*;

#[test]
fn clear_relevant_target_reaches_selected_with_persisted_components() {
    let catalog = TargetCatalog {
        entries: vec![target("review", TargetScope::Project, 3, 0)],
        exclusions: Vec::new(),
    };
    let index = index(&["review"]);
    let result = rank_targets(
        &catalog,
        &index,
        &matching_evidence(EvidenceAttribution::Verified),
    );

    assert_eq!(result.classification, SelectionClassification::Selected);
    assert_eq!(result.targets[0].score, 100);
    assert_eq!(result.targets[0].attribution_score, 35);
    assert_eq!(result.targets[0].participation_score, 15);
    assert_eq!(result.targets[0].compatibility_score, 20);
    assert_eq!(result.targets[0].lexical_score, 20);
    assert_eq!(result.targets[0].locality_score, 10);
    assert_eq!(result.threshold.margin, 100);
    assert!(!result.attribution_uncertain);
}

#[test]
fn tied_candidates_are_ambiguous_and_use_stable_scope_id_revision_ordering() {
    let catalog = TargetCatalog {
        entries: vec![
            target("zeta", TargetScope::User, 1, 0),
            target("alpha", TargetScope::User, 1, 0),
        ],
        exclusions: Vec::new(),
    };
    let index = index(&["alpha", "zeta"]);
    let result = rank_targets(
        &catalog,
        &index,
        &matching_evidence(EvidenceAttribution::Verified),
    );

    assert_eq!(result.classification, SelectionClassification::Ambiguous);
    assert_eq!(result.threshold.margin, 0);
    assert_eq!(result.targets[0].witness.skill_id, "alpha");
    assert_eq!(result.targets[1].witness.skill_id, "zeta");
}

#[test]
fn irrelevant_historical_and_attribution_strength_cases_remain_explainable() {
    let mut entry = target("review", TargetScope::System, 0, 1);
    entry.witness.revision_hash = "current-r2".to_string();
    let catalog = TargetCatalog {
        entries: vec![entry],
        exclusions: vec![TargetExclusion {
            skill_id: "review".to_string(),
            revision_hash: "historical-r1".to_string(),
            reason: TargetExclusionReason::HistoricalOnly,
        }],
    };
    let empty_index = build_local_lexical_index(&[]);
    let irrelevant = rank_targets(
        &catalog,
        &empty_index,
        &SelectionEvidence {
            attribution: EvidenceAttribution::Unattributed,
            skill_type: None,
            category: None,
            capabilities: Vec::new(),
            declared_tools: Vec::new(),
            lexical_terms: Vec::new(),
        },
    );
    assert_eq!(irrelevant.classification, SelectionClassification::NoTarget);
    assert!(irrelevant.targets[0]
        .exclusions
        .contains(&TargetExclusionReason::HistoricalOnly));

    for (attribution, expected_score, uncertain) in [
        (EvidenceAttribution::Correlated, 20, true),
        (EvidenceAttribution::Weak, 0, true),
        (EvidenceAttribution::Unattributed, 0, true),
    ] {
        let result = rank_targets(
            &catalog,
            &empty_index,
            &SelectionEvidence {
                attribution,
                skill_type: None,
                category: None,
                capabilities: Vec::new(),
                declared_tools: Vec::new(),
                lexical_terms: Vec::new(),
            },
        );
        assert_eq!(result.targets[0].attribution_score, expected_score);
        assert_eq!(result.attribution_uncertain, uncertain);
    }
}

#[test]
fn ranking_is_reproducible_when_catalog_and_documents_arrive_reordered() {
    let forward_catalog = TargetCatalog {
        entries: vec![
            target("project", TargetScope::Project, 1, 0),
            target("user", TargetScope::User, 1, 0),
        ],
        exclusions: Vec::new(),
    };
    let reverse_catalog = TargetCatalog {
        entries: forward_catalog.entries.iter().cloned().rev().collect(),
        exclusions: Vec::new(),
    };
    let forward_index = index(&["project", "user"]);
    let reverse_index = index(&["user", "project"]);
    let evidence = matching_evidence(EvidenceAttribution::Verified);

    assert_eq!(
        rank_targets(&forward_catalog, &forward_index, &evidence),
        rank_targets(&reverse_catalog, &reverse_index, &evidence)
    );
}

fn target(
    skill_id: &str,
    scope: TargetScope,
    verified_participation: u8,
    correlated_participation: u8,
) -> TargetCatalogEntry {
    TargetCatalogEntry {
        witness: EffectiveTargetWitness {
            skill_id: skill_id.to_string(),
            skill_type: "role".to_string(),
            revision_hash: "r1".to_string(),
            scope,
            lifecycle: TargetLifecycle::Active,
            trust: TargetTrust::Trusted,
        },
        name: skill_id.to_string(),
        skill_type: "utility".to_string(),
        category: "quality".to_string(),
        description: "Review code verification quality".to_string(),
        declared_tools: vec!["read".to_string(), "search".to_string()],
        capabilities: vec![
            "review".to_string(),
            "verification".to_string(),
            "quality".to_string(),
        ],
        verified_participation,
        correlated_participation,
    }
}

fn matching_evidence(attribution: EvidenceAttribution) -> SelectionEvidence {
    SelectionEvidence {
        attribution,
        skill_type: Some("UTILITY".to_string()),
        category: Some("quality".to_string()),
        capabilities: vec![
            "review".to_string(),
            "verification".to_string(),
            "quality".to_string(),
        ],
        declared_tools: vec!["read".to_string(), "search".to_string()],
        lexical_terms: vec![
            "review".to_string(),
            "code".to_string(),
            "verification".to_string(),
            "quality".to_string(),
        ],
    }
}

fn index(skill_ids: &[&str]) -> LocalLexicalIndex {
    build_local_lexical_index(
        &skill_ids
            .iter()
            .map(|skill_id| LexicalDocument {
                skill_id: (*skill_id).to_string(),
                revision_hash: "r1".to_string(),
                description: "Review code verification quality".to_string(),
                tags: Vec::new(),
                capabilities: Vec::new(),
                headings: Vec::new(),
                instructions: String::new(),
            })
            .collect::<Vec<_>>(),
    )
}
