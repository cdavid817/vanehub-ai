use super::scope::{
    AgentId, PersonalizationPolicyScope, SessionId, WorkspaceIdentity, WorkspaceKey, WorkspaceKind,
};
use super::PersonalizationDomainError;

#[test]
fn agent_id_accepts_a_stable_registry_id() {
    // Deliberately not an assertion about *which* ids exist: the point of the newtype is that any
    // registry id passes, so a newly registered Agent needs no change here.
    for value in [
        "onepiece",
        "claude-code",
        "codex-cli",
        "acme.custom_agent-7",
    ] {
        assert!(AgentId::parse(value).is_ok(), "{value} should be accepted");
    }
}

#[test]
fn agent_id_rejects_values_that_could_escape_a_key_or_a_path() {
    // `/` and `\` are rejected because the scope key joins typed values with `/`, and because a
    // memory filename must never be derivable from an identity that can contain a separator.
    for value in [
        "",
        "   ",
        " onepiece",
        "onepiece ",
        "a/b",
        "a\\b",
        "a\u{7f}b",
        "a\nb",
    ] {
        assert!(
            matches!(
                AgentId::parse(value),
                Err(PersonalizationDomainError::InvalidAgentId(_))
            ),
            "{value:?} should be rejected"
        );
    }
}

#[test]
fn agent_id_rejects_an_overlong_value() {
    let too_long = "a".repeat(121);
    assert!(matches!(
        AgentId::parse(&too_long),
        Err(PersonalizationDomainError::InvalidAgentId(_))
    ));
    assert!(AgentId::parse(&"a".repeat(120)).is_ok());
}

#[test]
fn session_and_workspace_ids_share_the_identity_rules() {
    assert!(SessionId::parse("ses_01K2").is_ok());
    assert!(WorkspaceKey::parse("ws_9f2c").is_ok());
    assert!(matches!(
        SessionId::parse("a/b"),
        Err(PersonalizationDomainError::InvalidSessionId(_))
    ));
    assert!(matches!(
        WorkspaceKey::parse(""),
        Err(PersonalizationDomainError::InvalidWorkspaceKey(_))
    ));
}

#[test]
fn scope_keys_are_distinct_and_stable() {
    let agent = AgentId::parse("claude-code").expect("agent");
    let workspace = WorkspaceKey::parse("ws_9f2c").expect("workspace");

    let keys = [
        PersonalizationPolicyScope::Global.scope_key(),
        PersonalizationPolicyScope::Agent {
            agent_id: agent.clone(),
        }
        .scope_key(),
        PersonalizationPolicyScope::Workspace {
            workspace_key: workspace.clone(),
        }
        .scope_key(),
        PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace.clone(),
            agent_id: agent.clone(),
        }
        .scope_key(),
    ];

    assert_eq!(keys[0], "global");
    assert_eq!(keys[1], "agent/claude-code");
    assert_eq!(keys[2], "workspace/ws_9f2c");
    assert_eq!(keys[3], "workspace-agent/ws_9f2c/claude-code");

    let unique: std::collections::BTreeSet<_> = keys.iter().collect();
    assert_eq!(unique.len(), keys.len(), "scope keys must not collide");
}

#[test]
fn a_workspace_scope_key_cannot_be_confused_with_a_workspace_agent_scope_key() {
    // Regression guard for the collision a naive `format!("{a}{b}")` would produce: a workspace
    // named so that its key plus a separator reproduces another scope's key.
    let workspace = WorkspaceKey::parse("ws_9f2c").expect("workspace");
    let agent = AgentId::parse("claude-code").expect("agent");
    let workspace_scope = PersonalizationPolicyScope::Workspace {
        workspace_key: workspace.clone(),
    };
    let workspace_agent_scope = PersonalizationPolicyScope::WorkspaceAgent {
        workspace_key: workspace,
        agent_id: agent,
    };
    assert_ne!(
        workspace_scope.scope_key(),
        workspace_agent_scope.scope_key()
    );
    assert_ne!(
        workspace_scope.scope_kind(),
        workspace_agent_scope.scope_kind()
    );
}

#[test]
fn precedence_runs_global_then_agent_then_workspace_then_workspace_agent() {
    // Workspace intentionally outranks a generic Agent override; a workspace-Agent row is the
    // explicit exception for one Agent in one workspace.
    let agent = AgentId::parse("codex-cli").expect("agent");
    let workspace = WorkspaceKey::parse("ws_1").expect("workspace");
    let ordered = [
        PersonalizationPolicyScope::Global,
        PersonalizationPolicyScope::Agent {
            agent_id: agent.clone(),
        },
        PersonalizationPolicyScope::Workspace {
            workspace_key: workspace.clone(),
        },
        PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace,
            agent_id: agent,
        },
    ];
    let ranks: Vec<u8> = ordered
        .iter()
        .map(|scope| scope.precedence_rank())
        .collect();
    assert_eq!(ranks, vec![0, 1, 2, 3]);
    assert!(
        ranks.windows(2).all(|pair| pair[0] < pair[1]),
        "precedence must be strictly increasing"
    );
}

#[test]
fn scope_kind_round_trips_through_its_persisted_string() {
    let agent = AgentId::parse("onepiece").expect("agent");
    let workspace = WorkspaceKey::parse("ws_1").expect("workspace");
    for scope in [
        PersonalizationPolicyScope::Global,
        PersonalizationPolicyScope::Agent {
            agent_id: agent.clone(),
        },
        PersonalizationPolicyScope::Workspace {
            workspace_key: workspace.clone(),
        },
        PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace.clone(),
            agent_id: agent.clone(),
        },
    ] {
        let rebuilt = PersonalizationPolicyScope::from_parts(
            scope.scope_kind(),
            scope.workspace_key(),
            scope.agent_id(),
        )
        .expect("scope should rebuild from its own parts");
        assert_eq!(rebuilt, scope);
    }
}

#[test]
fn rebuilding_a_scope_rejects_columns_that_disagree_with_the_kind() {
    // The SQLite row keeps `workspace_key`/`agent_id` nullable, so the domain is the only place
    // that can reject a row whose nullable columns contradict its `scope_kind`.
    let agent = AgentId::parse("onepiece").expect("agent");
    let workspace = WorkspaceKey::parse("ws_1").expect("workspace");

    assert!(matches!(
        PersonalizationPolicyScope::from_parts("global", Some(&workspace), None),
        Err(PersonalizationDomainError::InconsistentScopeColumns { .. })
    ));
    assert!(matches!(
        PersonalizationPolicyScope::from_parts("agent", None, None),
        Err(PersonalizationDomainError::InconsistentScopeColumns { .. })
    ));
    assert!(matches!(
        PersonalizationPolicyScope::from_parts("workspace", None, Some(&agent)),
        Err(PersonalizationDomainError::InconsistentScopeColumns { .. })
    ));
    assert!(matches!(
        PersonalizationPolicyScope::from_parts("workspace-agent", Some(&workspace), None),
        Err(PersonalizationDomainError::InconsistentScopeColumns { .. })
    ));
    assert!(matches!(
        PersonalizationPolicyScope::from_parts("nonsense", None, None),
        Err(PersonalizationDomainError::UnknownScopeKind(_))
    ));
}

#[test]
fn remote_workspaces_with_equal_paths_do_not_share_a_scope() {
    // Same displayed path, different connection identity — the key must separate them, which is
    // why the resolver compares keys rather than display paths.
    let left = WorkspaceIdentity::new(
        WorkspaceKey::parse("ws_hostA").expect("key"),
        "/srv/app".to_string(),
        WorkspaceKind::Remote,
    );
    let right = WorkspaceIdentity::new(
        WorkspaceKey::parse("ws_hostB").expect("key"),
        "/srv/app".to_string(),
        WorkspaceKind::Remote,
    );
    assert_eq!(left.display_path(), right.display_path());
    assert_ne!(left.key(), right.key());
    assert_ne!(
        PersonalizationPolicyScope::Workspace {
            workspace_key: left.key().clone(),
        },
        PersonalizationPolicyScope::Workspace {
            workspace_key: right.key().clone(),
        }
    );
}
