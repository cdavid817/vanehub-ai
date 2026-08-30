use std::collections::BTreeMap;

use super::{
    api::project_effective_skill_catalog,
    application::{EffectiveSkill, SkillPackageDescriptor},
    domain::{
        RawSkillDelegation, SkillAvailability, SkillDelegationDeclaration, SkillDelivery,
        SkillLayer, SkillMetadata, SkillOrigin, SkillTrust, SkillType,
    },
};

#[test]
fn assessment_catalog_projection_exposes_normalized_safe_metadata() {
    let metadata = SkillMetadata::with_classification(
        "review",
        "Review",
        "Review changed code",
        "quality",
        "1.0.0",
        vec![
            "review".to_string(),
            "code".to_string(),
            "review".to_string(),
        ],
        Vec::new(),
        Some(SkillType::Utility),
        Some(SkillDelivery::OnDemand),
    )
    .expect("metadata")
    .with_delegation(SkillDelegationDeclaration::declared(RawSkillDelegation {
        tools: vec![
            "search".to_string(),
            "read".to_string(),
            "search".to_string(),
        ],
        fields: BTreeMap::new(),
    }));
    let effective = EffectiveSkill {
        effective: descriptor(metadata, "user-review", SkillLayer::User),
        shadowed: vec![descriptor(
            SkillMetadata::new(
                "review",
                "Review",
                "System review",
                "quality",
                "1.0.0",
                Vec::new(),
            )
            .expect("shadowed metadata"),
            "system-review",
            SkillLayer::System,
        )],
    };

    let projected = project_effective_skill_catalog(&[effective]);

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].skill_id, "review");
    assert_eq!(projected[0].name, "Review");
    assert_eq!(projected[0].capabilities, vec!["code", "review"]);
    assert_eq!(projected[0].declared_tools, vec!["read", "search"]);
    assert_eq!(projected[0].shadowed[0].revision, "system-review");
}

fn descriptor(
    metadata: SkillMetadata,
    revision: &str,
    layer: SkillLayer,
) -> SkillPackageDescriptor {
    SkillPackageDescriptor {
        package_key: revision.to_string(),
        workspace_path: None,
        metadata,
        layer,
        origin: SkillOrigin::Created,
        trust: SkillTrust::Trusted,
        availability: SkillAvailability::Available,
        revision: revision.to_string(),
        source_path: None,
    }
}
