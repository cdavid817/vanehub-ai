use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolId, SkillToolKey, SkillToolLifecycle, SkillToolOwnerId, SkillToolQuarantine,
    SkillToolRevision, SkillToolSourceScope, SkillToolValidationState,
};

fn candidate(owner: &str, revision: char, kind: SkillToolOwnerKind) -> SkillToolCatalogCandidate {
    let key = SkillToolKey::new(
        SkillToolOwnerId::parse(owner).expect("owner"),
        SkillToolSourceScope::global(),
        SkillToolId::parse("check").expect("tool"),
        SkillToolRevision::parse(&revision.to_string().repeat(64)).expect("revision"),
    );
    SkillToolCatalogCandidate {
        entry: SkillToolCatalogEntry {
            canonical_name: key.canonical_name().expect("name"),
            description: "Check".to_string(),
            input_schema: serde_json::json!({"type":"object"}),
            key,
        },
        owner_kind: kind,
        lifecycle: SkillToolLifecycle {
            validation: SkillToolValidationState::Valid,
            trusted: true,
            enabled: true,
            ..SkillToolLifecycle::default()
        },
        archived: false,
        shadowed: false,
        requires_module_runtime: false,
        allow_plan: false,
    }
}

fn binding(owner: &str, revision: char) -> SkillToolBinding {
    SkillToolBinding {
        skill_id: owner.to_string(),
        revision: revision.to_string().repeat(64),
    }
}

#[test]
fn role_requires_the_exact_loaded_revision() {
    let role = candidate("review", 'a', SkillToolOwnerKind::Role);
    let not_loaded = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: Vec::new(),
        mode: SkillToolCatalogMode::Execute,
    };
    let stale = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: vec![binding("review", 'b')],
        mode: SkillToolCatalogMode::Execute,
    };
    let loaded = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: vec![binding("review", 'a')],
        mode: SkillToolCatalogMode::Execute,
    };

    assert!(project_contextual_catalog(std::slice::from_ref(&role), &not_loaded).is_empty());
    assert!(project_contextual_catalog(std::slice::from_ref(&role), &stale).is_empty());
    assert_eq!(project_contextual_catalog(&[role], &loaded).len(), 1);
}

#[test]
fn utility_is_visible_only_to_its_exact_delegated_child() {
    let utility = candidate("audit", 'a', SkillToolOwnerKind::Utility);
    let ordinary = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: vec![binding("audit", 'a')],
        mode: SkillToolCatalogMode::Execute,
    };
    let other_child = SkillToolCatalogContext::UtilityDelegation {
        workspace_path: None,
        utility: binding("other", 'a'),
        mode: SkillToolCatalogMode::Execute,
    };
    let own_child = SkillToolCatalogContext::UtilityDelegation {
        workspace_path: None,
        utility: binding("audit", 'a'),
        mode: SkillToolCatalogMode::Execute,
    };

    assert!(project_contextual_catalog(std::slice::from_ref(&utility), &ordinary).is_empty());
    assert!(project_contextual_catalog(std::slice::from_ref(&utility), &other_child).is_empty());
    assert_eq!(project_contextual_catalog(&[utility], &own_child).len(), 1);
}

#[test]
fn role_tools_never_leak_into_a_utility_child() {
    let role = candidate("review", 'a', SkillToolOwnerKind::Role);
    let child = SkillToolCatalogContext::UtilityDelegation {
        workspace_path: None,
        utility: binding("review", 'a'),
        mode: SkillToolCatalogMode::Execute,
    };
    assert!(project_contextual_catalog(&[role], &child).is_empty());
}

#[test]
fn every_lifecycle_and_execution_mode_blocker_is_excluded() {
    let context = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: vec![binding("review", 'a')],
        mode: SkillToolCatalogMode::Execute,
    };
    let base = candidate("review", 'a', SkillToolOwnerKind::Role);
    let mut blocked = Vec::new();
    for mutate in [
        |item: &mut SkillToolCatalogCandidate| item.archived = true,
        |item: &mut SkillToolCatalogCandidate| item.shadowed = true,
        |item: &mut SkillToolCatalogCandidate| item.lifecycle.enabled = false,
        |item: &mut SkillToolCatalogCandidate| item.lifecycle.trusted = false,
        |item: &mut SkillToolCatalogCandidate| {
            item.lifecycle.validation = SkillToolValidationState::Invalid
        },
        |item: &mut SkillToolCatalogCandidate| {
            item.lifecycle.quarantine = SkillToolQuarantine::Quarantined {
                reason: "trap".to_string(),
            }
        },
    ] {
        let mut item = base.clone();
        mutate(&mut item);
        blocked.push(item);
    }
    assert!(project_contextual_catalog(&blocked, &context).is_empty());

    let plan = SkillToolCatalogContext::RoleGeneration {
        workspace_path: None,
        loaded_roles: vec![binding("review", 'a')],
        mode: SkillToolCatalogMode::Plan,
    };
    assert!(project_contextual_catalog(&[base], &plan).is_empty());
}

#[test]
fn external_cli_is_explicitly_unsupported_and_receives_no_local_tools() {
    let context = SkillToolCatalogContext::ExternalCli {
        workspace_path: Some("/workspace".to_string()),
    };
    let candidate = candidate("review", 'a', SkillToolOwnerKind::Role);
    assert_eq!(context.support_state(), "unsupported-external-cli-bridge");
    assert!(project_contextual_catalog(&[candidate], &context).is_empty());
}
