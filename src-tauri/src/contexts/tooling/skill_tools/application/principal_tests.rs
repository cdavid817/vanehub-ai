use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolId, SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
};

fn key(source: SkillToolSourceScope) -> SkillToolKey {
    SkillToolKey::new(
        SkillToolOwnerId::parse("review").expect("owner"),
        source,
        SkillToolId::parse("check").expect("tool"),
        SkillToolRevision::parse(&"a".repeat(64)).expect("revision"),
    )
}

#[test]
fn principal_preserves_immutable_identity_and_bounded_context() {
    let principal = SkillToolPrincipal::new(
        "agent",
        key(SkillToolSourceScope::global()),
        Some("/workspace"),
        "session",
        "generation",
        vec!["tool:read_file".to_string()],
    )
    .expect("principal");
    assert_eq!(principal.parent_agent_id, "agent");
    assert_eq!(principal.key.owner.as_str(), "review");
    assert_eq!(principal.key.tool.as_str(), "check");
    assert_eq!(principal.key.revision.as_str(), "a".repeat(64));
    assert_eq!(principal.workspace_path.as_deref(), Some("/workspace"));
    assert_eq!(principal.session_id.as_deref(), Some("session"));
    assert_eq!(principal.generation_id, "generation");
}

#[test]
fn missing_mismatched_or_unbounded_context_fails_closed() {
    let workspace = SkillToolSourceScope::new(
        crate::contexts::tooling::skill_tools::domain::SkillToolScope::Workspace,
        Some("/workspace"),
    )
    .expect("scope");
    assert!(SkillToolPrincipal::new(
        "",
        key(workspace.clone()),
        Some("/workspace"),
        "s",
        "g",
        vec![]
    )
    .is_err());
    assert!(
        SkillToolPrincipal::new("a", key(workspace), Some("/other"), "s", "g", vec![]).is_err()
    );
    assert!(SkillToolPrincipal::new(
        "a",
        key(SkillToolSourceScope::global()),
        None,
        "",
        "g",
        vec![]
    )
    .is_err());
    assert!(SkillToolPrincipal::new(
        "a",
        key(SkillToolSourceScope::global()),
        None,
        "s",
        "",
        vec![]
    )
    .is_err());
    assert!(SkillToolPrincipal::new(
        "a",
        key(SkillToolSourceScope::global()),
        None,
        "s",
        "g",
        vec!["tool:x".to_string(); 5]
    )
    .is_err());
}
