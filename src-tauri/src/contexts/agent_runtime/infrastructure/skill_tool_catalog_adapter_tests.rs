use super::*;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolBinding, SkillToolCatalogContext, SkillToolCatalogEntry, SkillToolCatalogMode,
    SkillToolCatalogSnapshot,
};
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolId, SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
};

struct Catalog(Vec<SkillToolCatalogEntry>);

impl SkillToolCatalogPort for Catalog {
    fn catalog_for(
        &self,
        _context: &SkillToolCatalogContext,
    ) -> Result<SkillToolCatalogSnapshot, SkillToolApplicationError> {
        Ok(SkillToolCatalogSnapshot {
            generation: 1,
            entries: self.0.clone(),
            lease: std::sync::Arc::new(()),
        })
    }
}

fn role_context() -> SkillToolCatalogContext {
    SkillToolCatalogContext::RoleGeneration {
        workspace_path: Some("/workspace".to_string()),
        loaded_roles: vec![SkillToolBinding {
            skill_id: "review".to_string(),
            revision: "revision-1".to_string(),
        }],
        mode: SkillToolCatalogMode::Execute,
    }
}

fn entry(owner: &str, tool: &str, revision: char) -> SkillToolCatalogEntry {
    let key = SkillToolKey::new(
        SkillToolOwnerId::parse(owner).expect("owner"),
        SkillToolSourceScope::global(),
        SkillToolId::parse(tool).expect("tool"),
        SkillToolRevision::parse(&revision.to_string().repeat(64)).expect("revision"),
    );
    SkillToolCatalogEntry {
        canonical_name: key.canonical_name().expect("name"),
        description: "Review a file".to_string(),
        input_schema: serde_json::json!({"type":"object"}),
        key,
    }
}

#[test]
fn every_native_api_interface_keeps_the_immutable_name_to_key_mapping() {
    for interface in ["anthropic", "openai-compatible"] {
        let expected = entry("review", "check", 'a');
        let expected_key = expected.key.clone();
        let resolved = resolve_skill_tool_catalog(
            &Catalog(vec![expected]),
            &role_context(),
            Vec::new(),
            interface,
        )
        .expect("catalog");
        let name = "skill__review__check__aaaaaaaaaaaa";
        assert_eq!(resolved.definitions[0].name, name);
        assert_eq!(resolved.keys_by_name.get(name), Some(&expected_key));
    }
}

#[test]
fn fixed_dynamic_and_duplicate_skill_names_collide_fail_closed() {
    let first = entry("review", "check", 'a');
    let name = first.canonical_name.clone();
    assert!(resolve_skill_tool_catalog(
        &Catalog(vec![first.clone()]),
        &role_context(),
        vec![name],
        "anthropic",
    )
    .is_err());
    assert!(resolve_skill_tool_catalog(
        &Catalog(vec![first.clone(), first]),
        &role_context(),
        Vec::new(),
        "openai-compatible",
    )
    .is_err());
}

#[test]
fn forged_display_name_and_unknown_interface_are_rejected() {
    let mut forged = entry("review", "check", 'a');
    forged.canonical_name = "skill__review__other__aaaaaaaaaaaa".to_string();
    assert!(resolve_skill_tool_catalog(
        &Catalog(vec![forged]),
        &role_context(),
        Vec::new(),
        "anthropic",
    )
    .is_err());
    assert!(resolve_skill_tool_catalog(
        &Catalog(Vec::new()),
        &role_context(),
        Vec::new(),
        "unsupported",
    )
    .is_err());
}
